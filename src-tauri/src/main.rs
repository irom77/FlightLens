#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod portable;
mod save;
use flightlens_core::{
    analysis::{self, Inspection, Point},
    export::{self, ExportRequest, ValidatedSnippet},
    feedback::{self, FeedbackReport, FeedbackRequest},
    source::{self, SourceRegistry},
    Artifact, ArtifactView, ConfigDocument, RestoredSession, SourceDescriptor,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
// One index worker or picker at a time, including calls queued by the renderer.
struct IndexPermit(tauri::AppHandle);
impl IndexPermit {
    fn acquire(app: &tauri::AppHandle) -> Result<Self, String> {
        app.state::<AppState>()
            .index_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Workspace operation is still running")?;
        Ok(Self(app.clone()))
    }
}
impl Drop for IndexPermit {
    fn drop(&mut self) {
        self.0
            .state::<AppState>()
            .index_busy
            .store(false, Ordering::SeqCst);
    }
}
#[tauri::command]
async fn choose_workspace(
    app: tauri::AppHandle,
    refresh: bool,
) -> Result<Option<flightlens_core::workspace::WorkspacePage>, String> {
    let permit = IndexPermit::acquire(&app)?;
    app.state::<AppState>()
        .index_cancelled
        .store(false, Ordering::SeqCst);
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let state = app.state::<AppState>();
        let root = if refresh {
            state
                .index
                .lock()
                .map_err(|_| "Workspace unavailable")?
                .as_ref()
                .map(|i| i.root())
        } else {
            app.dialog()
                .file()
                .blocking_pick_folder()
                .map(|f| {
                    f.into_path()
                        .map_err(|_| "Only local folders are supported")
                })
                .transpose()?
        };
        let Some(root) = root else {
            return Ok(None);
        };
        let generation = state.index_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let index = flightlens_core::workspace::WorkspaceIndex::new(root, generation)?;
        let page = index.page("", 0);
        *state.index.lock().map_err(|_| "Workspace unavailable")? = Some(index);
        if !refresh {
            if let Some(recovery) = state
                .unresolved_session
                .lock()
                .map_err(|_| "Session recovery unavailable")?
                .as_mut()
            {
                recovery.workspace = None;
            }
        }
        Ok(Some(page))
    })
    .await
    .map_err(|_| "Workspace worker failed")?
}
#[tauri::command]
async fn workspace_page(
    app: tauri::AppHandle,
    query: String,
    offset: usize,
) -> Result<Option<flightlens_core::workspace::WorkspacePage>, String> {
    let permit = IndexPermit::acquire(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let state = app.state::<AppState>();
        let mut index = state.index.lock().map_err(|_| "Workspace unavailable")?;
        Ok(index.as_mut().map(|index| {
            index.advance(&state.index_cancelled);
            index.page(&query, offset)
        }))
    })
    .await
    .map_err(|_| "Workspace worker failed")?
}
#[tauri::command]
fn cancel_workspace(state: State<AppState>) {
    state.index_cancelled.store(true, Ordering::SeqCst);
}
#[tauri::command]
async fn open_workspace_entry(
    app: tauri::AppHandle,
    entry_id: String,
) -> Result<ArtifactView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let path = state
            .index
            .lock()
            .map_err(|_| "Workspace unavailable")?
            .as_ref()
            .ok_or("Choose a workspace folder first")?
            .path(&entry_id)?;
        let source = state
            .sources
            .lock()
            .map_err(|_| "Source registry unavailable")?
            .register(path.clone())?;
        let artifact = keep(&state, source::open_path(&path, &source.id)?)?;
        let id = match &artifact {
            ArtifactView::Config(d) => &d.id,
            ArtifactView::Recognized(d) => &d.id,
        };
        remember(&app, &source.id, id);
        Ok(artifact)
    })
    .await
    .map_err(|_| "Import worker failed")?
}
/// How many backups a saved session may reopen. The document repository caps
/// an open workspace at 32, so a session that grew past it could never restore
/// in full anyway.
const SESSION_LIMIT: usize = 32;
#[derive(Default)]
struct AppState {
    unresolved_session: Mutex<Option<portable::UnresolvedSession>>,
    index: Mutex<Option<flightlens_core::workspace::WorkspaceIndex>>,
    index_busy: AtomicBool,
    index_cancelled: AtomicBool,
    index_generation: AtomicU64,
    sources: Mutex<SourceRegistry>,
    documents: Mutex<BTreeMap<String, Arc<ConfigDocument>>>,
    pending: Mutex<Vec<SourceDescriptor>>,
    /// Paths of the file-backed documents currently open, in the order they
    /// were opened. Written to disk on every change so the next run reopens
    /// the same workspace.
    session: Mutex<Vec<PathBuf>>,
}
/// Where the open-document list is kept between runs.
///
/// The backend owns this file entirely: it holds only paths the backend itself
/// registered from a native dialog or a drop event, and the webview never
/// supplies one. Restoring therefore grants the webview no access it did not
/// already have.
fn session_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory unavailable")?
        .join("session.json"))
}
/// Persist the open-document list. A session that cannot be written is not an
/// error the user can act on -- the workspace itself is unaffected -- so the
/// failure is swallowed rather than interrupting the import that caused it.
fn save_session(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let (Ok(path), Ok(session)) = (session_file(app), state.session.lock()) else {
        return;
    };
    let paths: Vec<&Path> = session.iter().map(PathBuf::as_path).collect();
    let (Ok(text), Some(directory)) = (serde_json::to_string_pretty(&paths), path.parent()) else {
        return;
    };
    let _ = std::fs::create_dir_all(directory).and_then(|_| std::fs::write(&path, text));
}
/// Record a file-backed source as open, so the next run reopens it.
///
/// A file opened twice appears once. Track its latest imported identity so
/// closing a document can remove every path to that content.
fn remember(app: &tauri::AppHandle, source_id: &str, document_id: &str) {
    let state = app.state::<AppState>();
    let Ok(path) = state
        .sources
        .lock()
        .map_err(|_| ())
        .and_then(|mut registry| {
            registry
                .remember_document(source_id, document_id)
                .map_err(|_| ())?;
            registry.path(source_id).map_err(|_| ())
        })
    else {
        return;
    };
    if let Ok(mut session) = state.session.lock() {
        if session.contains(&path) || session.len() >= SESSION_LIMIT {
            return;
        }
        session.push(path);
    }
    save_session(app);
}
/// Drop every file reference to a closed artifact, including duplicate content.
fn forget(app: &tauri::AppHandle, document_id: &str) {
    let state = app.state::<AppState>();
    if let (Ok(mut registry), Ok(mut session)) = (state.sources.lock(), state.session.lock()) {
        registry.forget_document(document_id, &mut session);
    }
    save_session(app);
}

fn keep(state: &AppState, artifact: Artifact) -> Result<ArtifactView, String> {
    let Artifact::Config(d) = artifact else {
        let Artifact::Recognized(d) = artifact else {
            unreachable!()
        };
        return Ok(ArtifactView::Recognized(d));
    };
    let snapshot = Arc::new(*d);
    {
        let mut docs = state
            .documents
            .lock()
            .map_err(|_| "Document repository unavailable")?;
        let total: usize = docs
            .values()
            .map(|d| d.syntax.iter().map(|l| l.raw.len()).sum::<usize>())
            .sum();
        if !docs.contains_key(&snapshot.id)
            && (docs.len() >= 32
                || total + snapshot.syntax.iter().map(|l| l.raw.len()).sum::<usize>()
                    > 128 * 1024 * 1024)
        {
            return Err(
                "Open-document limit reached. Close a document before importing more.".into(),
            );
        }
        docs.insert(snapshot.id.clone(), Arc::clone(&snapshot));
    }
    Ok(ArtifactView::Config(Box::new(snapshot.document_view())))
}
// Clone only the handle under the lock. In-flight work keeps its immutable
// snapshot alive even when the document is closed or replaced in the repository.
fn document(state: &AppState, id: &str) -> Result<Arc<ConfigDocument>, String> {
    state
        .documents
        .lock()
        .map_err(|_| "Document repository unavailable")?
        .get(id)
        .cloned()
        .ok_or("Document is not open".into())
}
const RAW_PAGE_LIMIT: usize = 500;

// Only copy the requested source window; the snapshot lookup releases the
// repository lock before any source lines are copied or serialized.
fn document_raw_page(
    state: &AppState,
    id: &str,
    offset: usize,
    count: usize,
) -> Result<Vec<flightlens_core::SyntaxLine>, String> {
    if count == 0 || count > RAW_PAGE_LIMIT {
        return Err("Raw page size must be between 1 and 500 lines".into());
    }
    let snapshot = document(state, id)?;
    let start = offset.min(snapshot.syntax.len());
    let end = start + count.min(snapshot.syntax.len() - start);
    Ok(snapshot.syntax[start..end].to_vec())
}

#[tauri::command]
fn raw_page(
    state: State<'_, AppState>,
    id: String,
    offset: usize,
    count: usize,
) -> Result<Vec<flightlens_core::SyntaxLine>, String> {
    document_raw_page(&state, &id, offset, count)
}

#[tauri::command]
async fn ingest_text(
    text: String,
    label: String,
    app: tauri::AppHandle,
) -> Result<ArtifactView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let artifact = flightlens_core::analyze(&text, &label, "virtual")?;
        keep(&app.state::<AppState>(), artifact)
    })
    .await
    .map_err(|_| "Import worker failed")?
}
#[tauri::command]
async fn choose_files(app: tauri::AppHandle) -> Result<Vec<SourceDescriptor>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let files = app
            .dialog()
            .file()
            .add_filter(
                "Flight controller backups",
                &["txt", "diff", "dump", "param", "parm", "bbl", "bfl"],
            )
            .blocking_pick_files()
            .unwrap_or_default();
        let state = app.state::<AppState>();
        let mut registry = state
            .sources
            .lock()
            .map_err(|_| "Source registry unavailable")?;
        let mut registered = Vec::new();
        for file in files {
            match file
                .into_path()
                .map_err(|_| "Only local files are supported".to_string())
                .and_then(|path| registry.register(path))
            {
                Ok(source) => registered.push(source),
                Err(error) => {
                    let _ = app.emit("source-error", error);
                }
            }
        }
        Ok(registered)
    })
    .await
    .map_err(|_| "File picker worker failed")?
}
#[tauri::command]
async fn open_source(source_id: String, app: tauri::AppHandle) -> Result<ArtifactView, String> {
    let path = app
        .state::<AppState>()
        .sources
        .lock()
        .map_err(|_| "Source registry unavailable")?
        .path(&source_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let artifact = source::open_path(&path, &source_id)?;
        let artifact = keep(&app.state::<AppState>(), artifact)?;
        let document_id = match &artifact {
            ArtifactView::Config(document) => &document.id,
            ArtifactView::Recognized(document) => &document.id,
        };
        remember(&app, &source_id, document_id);
        Ok(artifact)
    })
    .await
    .map_err(|_| "Import worker failed")?
}
/// Reopen the backups the previous run left open.
///
/// Only the paths are restored; every document is parsed again from the file
/// on disk, so a backup edited since the last run is inspected as it now
/// stands rather than as it was.
#[tauri::command]
async fn restore_session(app: tauri::AppHandle) -> Result<RestoredSession, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let saved: Vec<PathBuf> = session_file(&app)
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let state = app.state::<AppState>();
        let mut restored = RestoredSession::default();
        {
            let mut registry = state
                .sources
                .lock()
                .map_err(|_| "Source registry unavailable")?;
            let mut session = state.session.lock().map_err(|_| "Session unavailable")?;
            for path in saved.into_iter().take(SESSION_LIMIT) {
                let name = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned();
                match registry.register(path) {
                    Ok(source) => {
                        // Keep canonical paths, matching the identity associations
                        // recorded when open_source successfully imports each file.
                        if let Ok(path) = registry.path(&source.id) {
                            if !session.contains(&path) {
                                session.push(path);
                            }
                        }
                        restored.sources.push(source);
                    }
                    Err(_) => restored.unavailable.push(name),
                }
            }
        }
        // A file that moved or was deleted is dropped from the session here, so
        // the next run does not report it again.
        save_session(&app);
        Ok(restored)
    })
    .await
    .map_err(|_| "Session worker failed")?
}
#[tauri::command]
fn pending_sources(state: State<AppState>) -> Result<Vec<SourceDescriptor>, String> {
    Ok(std::mem::take(
        &mut *state.pending.lock().map_err(|_| "Drop queue unavailable")?,
    ))
}
#[tauri::command]
fn close_document(config_id: String, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .documents
        .lock()
        .map_err(|_| "Document repository unavailable")?
        .remove(&config_id);
    forget(&app, &config_id);
    Ok(())
}
#[tauri::command]
fn inspect_config(
    config_id: String,
    rate_profile: u8,
    state: State<AppState>,
) -> Result<Inspection, String> {
    Ok(analysis::inspect(
        document(&state, &config_id)?.as_ref(),
        rate_profile,
    ))
}
#[tauri::command]
fn filter_plot(
    config_id: String,
    key: String,
    scope: flightlens_core::Scope,
    sample_rate: f64,
    state: State<AppState>,
) -> Result<Vec<Point>, String> {
    let d = document(&state, &config_id)?;
    let type_key = match key.as_str() {
        "gyro_lpf1_static_hz" => "gyro_lpf1_type",
        "gyro_lpf2_static_hz" => "gyro_lpf2_type",
        "dterm_lpf1_static_hz" => "dterm_lpf1_type",
        "dterm_lpf2_static_hz" => "dterm_lpf2_type",
        _ => return Err("This filter does not have a certified response model".into()),
    };
    let cutoff = d.number(&scope, &key).ok_or("Filter cutoff is unknown")?;
    let kind = d.text(&scope, type_key).ok_or("Filter type is unknown")?;
    analysis::filter_response(&kind, cutoff, sample_rate)
}
#[tauri::command]
fn export_snippet(
    config_id: String,
    request: ExportRequest,
    state: State<AppState>,
) -> Result<ValidatedSnippet, String> {
    export::export(document(&state, &config_id)?.as_ref(), &request)
}
#[tauri::command]
async fn save_snippet(
    config_id: String,
    request: ExportRequest,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let snippet = export::export(
        document(&app.state::<AppState>(), &config_id)?.as_ref(),
        &request,
    )?;
    tauri::async_runtime::spawn_blocking(move || {
        save::save_new(
            &app,
            "flightlens-snippet.txt",
            ("Text snippet", &["txt"]),
            |_| Ok(snippet.text.as_bytes().to_vec()),
        )
    })
    .await
    .map_err(|_| "Save worker failed")?
}
/// The open backup a report attaches, or `None` when the reporter attached
/// nothing. A report can always be filed from an empty workspace.
fn reported_document(
    state: &AppState,
    config_id: Option<String>,
) -> Result<Option<Arc<ConfigDocument>>, String> {
    config_id.map(|id| document(state, &id)).transpose()
}
/// Stamp the environment the report records, overwriting whatever the webview
/// sent. The running binary knows its own version and host; a report is worth
/// less if either can be misreported from the page.
fn stamped(mut request: FeedbackRequest) -> FeedbackRequest {
    request.app_version = env!("CARGO_PKG_VERSION").into();
    request.platform = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    request
}
/// The report the dialog shows for review: the issue body, and the redacted
/// backup with the list of what redaction removed.
#[tauri::command]
fn feedback_report(
    config_id: Option<String>,
    request: FeedbackRequest,
    state: State<AppState>,
) -> Result<FeedbackReport, String> {
    let document = reported_document(&state, config_id)?;
    feedback::build_report(&stamped(request), document.as_deref())
}
/// Open the prefilled issue in the reporter's browser.
///
/// The report is assembled here from the fields the reporter typed rather than
/// from a URL the webview supplies, so the webview cannot ask the browser to
/// open an address of its own choosing. Filing the issue remains the reporter's
/// action on GitHub; FlightLens sends nothing.
#[tauri::command]
fn file_feedback_report(
    config_id: Option<String>,
    request: FeedbackRequest,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let document = reported_document(&app.state::<AppState>(), config_id)?;
    let report = feedback::build_report(&stamped(request), document.as_deref())?;
    app.opener()
        .open_url(report.url, None::<&str>)
        .map_err(|_| "Cannot open a browser for the report".into())
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                let state = window.state::<AppState>();
                if let (Ok(mut registry), Ok(mut pending)) =
                    (state.sources.lock(), state.pending.lock())
                {
                    for path in paths {
                        if pending.len() >= 256 {
                            let _ = window.emit("source-error", "Drop queue limit reached. Wait for pending imports to finish before dropping more files.");
                            break;
                        }
                        match registry.register(path.clone()) {
                            Ok(source) => {
                                if !pending.iter().any(|queued| queued.id == source.id) {
                                    pending.push(source);
                                }
                            },
                            Err(error) => {
                                let _ = window.emit("source-error", error);
                            }
                        }
                    }
                    let _ = window.emit("sources-ready", ());
                };
            }
        })
        .invoke_handler(tauri::generate_handler![
            portable::save_portable_session,
            portable::open_portable_session,
            choose_workspace,
            workspace_page,
            cancel_workspace,
            open_workspace_entry,
            ingest_text,
            choose_files,
            open_source,
            restore_session,
            pending_sources,
            close_document,
            raw_page,
            inspect_config,
            filter_plot,
            export_snippet,
            save_snippet,
            feedback_report,
            file_feedback_report
        ])
        .run(tauri::generate_context!())
        .expect("FlightLens could not start");
}

#[cfg(test)]
mod document_tests {
    use super::*;

    fn open(state: &AppState, title: &str) -> String {
        let artifact = flightlens_core::analyze(
            include_str!("../../fixtures/configs/betaflight-4.5.0.dump"),
            title,
            "virtual",
        )
        .unwrap();
        let ArtifactView::Config(document) = keep(state, artifact).unwrap() else {
            panic!("fixture must be a configuration");
        };
        document.id
    }

    #[test]
    fn imports_move_source_storage_and_return_only_the_view() {
        let state = AppState::default();
        let Artifact::Config(parsed) = flightlens_core::analyze(
            include_str!("../../fixtures/configs/betaflight-4.5.0.dump"),
            "view",
            "virtual",
        )
        .unwrap() else {
            panic!("expected config")
        };
        let source_storage = parsed.syntax.as_ptr();
        let id = parsed.id.clone();
        let view = keep(&state, Artifact::Config(parsed)).unwrap();
        let retained = document(&state, &id).unwrap();
        assert_eq!(retained.syntax.as_ptr(), source_storage);
        let json = serde_json::to_value(&view).unwrap();
        assert!(json["document"].get("syntax").is_none());
        assert_eq!(
            json["document"]["sourceEvidence"]["lineCount"],
            retained.syntax.len()
        );
        assert_eq!(
            json["document"],
            serde_json::to_value(retained.document_view()).unwrap()
        );
    }

    #[test]
    fn raw_pages_preserve_source_and_boundaries() {
        let state = AppState::default();
        let text = format!(
            "# Betaflight / STM32F405 (S405) 4.5.0\r\n{}last line",
            "# café source\r\n".repeat(1000)
        );
        let Artifact::Config(parsed) =
            flightlens_core::analyze(&text, "paging", "virtual").unwrap()
        else {
            panic!("expected configuration");
        };
        let id = parsed.id.clone();
        let expected = serde_json::to_value(&parsed.syntax).unwrap();
        keep(&state, Artifact::Config(parsed)).unwrap();
        let mut lines = Vec::new();
        for offset in [0, 500, 1000] {
            let page = document_raw_page(&state, &id, offset, 500).unwrap();
            assert!(page.len() <= RAW_PAGE_LIMIT);
            lines.extend(page);
        }
        assert_eq!(serde_json::to_value(&lines).unwrap(), expected);
        assert_eq!(
            lines
                .iter()
                .map(|line| line.raw.as_str())
                .collect::<String>(),
            text
        );
        assert!(document_raw_page(&state, &id, lines.len(), 500)
            .unwrap()
            .is_empty());
        assert!(document_raw_page(&state, &id, usize::MAX, 500)
            .unwrap()
            .is_empty());
        assert_eq!(document_raw_page(&state, &id, 499, 1).unwrap()[0].line, 500);
    }

    #[test]
    fn raw_pages_reject_invalid_sizes_and_closed_documents() {
        let state = AppState::default();
        let id = open(&state, "paging");
        for count in [0, 501, usize::MAX] {
            assert_eq!(
                document_raw_page(&state, &id, 0, count).unwrap_err(),
                "Raw page size must be between 1 and 500 lines"
            );
        }
        let page = document_raw_page(&state, &id, 0, 1).unwrap();
        state.documents.lock().unwrap().remove(&id);
        assert_eq!(
            document_raw_page(&state, &id, 0, 1).unwrap_err(),
            "Document is not open"
        );
        assert_eq!(page.len(), 1);
    }

    #[test]
    fn lookups_share_a_snapshot_that_survives_close_and_is_then_released() {
        let state = AppState::default();
        let id = open(&state, "original");
        let first = document(&state, &id).unwrap();
        let second = document(&state, &id).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let weak = Arc::downgrade(&first);
        // A returned handle must not retain the repository mutex.
        state.documents.try_lock().unwrap().remove(&id);
        assert_eq!(document(&state, &id).unwrap_err(), "Document is not open");
        assert_eq!(first.title, "original");
        assert!(!analysis::inspect(&first, 0).rates.is_empty());
        drop(first);
        assert!(weak.upgrade().is_some());
        drop(second);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn replacing_a_snapshot_does_not_mutate_in_flight_work() {
        let state = AppState::default();
        let id = open(&state, "original");
        let original = document(&state, &id).unwrap();
        assert_eq!(open(&state, "reopened"), id);
        let current = document(&state, &id).unwrap();
        assert!(!Arc::ptr_eq(&original, &current));
        assert_eq!(original.title, "original");
        assert_eq!(current.title, "reopened");
        let reported = reported_document(&state, Some(id)).unwrap().unwrap();
        assert!(Arc::ptr_eq(&current, &reported));
        assert!(reported_document(&state, None).unwrap().is_none());
    }
}
