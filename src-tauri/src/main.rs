#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use flightlens_core::{
    analysis::{self, Inspection, Point},
    export::{self, ExportRequest, ValidatedSnippet},
    source::{self, SourceRegistry},
    Artifact, ConfigDocument, RestoredSession, SourceDescriptor,
};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
/// How many backups a saved session may reopen. The document repository caps
/// an open workspace at 32, so a session that grew past it could never restore
/// in full anyway.
const SESSION_LIMIT: usize = 32;
#[derive(Default)]
struct AppState {
    sources: Mutex<SourceRegistry>,
    documents: Mutex<BTreeMap<String, ConfigDocument>>,
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
/// A file opened twice appears once: the same path always parses to the same
/// document, and the workspace deduplicates by content hash.
fn remember(app: &tauri::AppHandle, source_id: &str) {
    let state = app.state::<AppState>();
    let Ok(path) = state
        .sources
        .lock()
        .map_err(|_| ())
        .and_then(|registry| registry.path(source_id).map_err(|_| ()))
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
/// Drop a source from the saved session, so closing a document keeps it closed.
fn forget(app: &tauri::AppHandle, source_id: &str) {
    let state = app.state::<AppState>();
    let Ok(path) = state
        .sources
        .lock()
        .map_err(|_| ())
        .and_then(|registry| registry.path(source_id).map_err(|_| ()))
    else {
        return;
    };
    if let Ok(mut session) = state.session.lock() {
        session.retain(|open| open != &path);
    }
    save_session(app);
}
fn keep(state: &AppState, artifact: Artifact) -> Result<Artifact, String> {
    if let Artifact::Config(d) = &artifact {
        let mut docs = state
            .documents
            .lock()
            .map_err(|_| "Document repository unavailable")?;
        let total: usize = docs
            .values()
            .map(|d| d.syntax.iter().map(|l| l.raw.len()).sum::<usize>())
            .sum();
        if !docs.contains_key(&d.id)
            && (docs.len() >= 32
                || total + d.syntax.iter().map(|l| l.raw.len()).sum::<usize>() > 128 * 1024 * 1024)
        {
            return Err(
                "Open-document limit reached. Close a document before importing more.".into(),
            );
        }
        docs.insert(d.id.clone(), d.as_ref().clone());
    }
    Ok(artifact)
}
fn document(state: &AppState, id: &str) -> Result<ConfigDocument, String> {
    state
        .documents
        .lock()
        .map_err(|_| "Document repository unavailable")?
        .get(id)
        .cloned()
        .ok_or("Document is not open".into())
}
#[tauri::command]
async fn ingest_text(
    text: String,
    label: String,
    app: tauri::AppHandle,
) -> Result<Artifact, String> {
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
async fn open_source(source_id: String, app: tauri::AppHandle) -> Result<Artifact, String> {
    let path = app
        .state::<AppState>()
        .sources
        .lock()
        .map_err(|_| "Source registry unavailable")?
        .path(&source_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let artifact = source::open_path(&path, &source_id)?;
        let artifact = keep(&app.state::<AppState>(), artifact)?;
        remember(&app, &source_id);
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
                        // The registry canonicalizes, and `forget` looks the
                        // path back up through it, so the session holds the
                        // canonical form rather than the one on file.
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
    let closed = state
        .documents
        .lock()
        .map_err(|_| "Document repository unavailable")?
        .remove(&config_id);
    // A recognized artifact -- a Blackbox log, say -- is never kept in the
    // document repository and carries its source id as its own, so the
    // identifier the webview closed stands in for one.
    let source_id = closed.map_or(config_id, |d| d.source_id);
    forget(&app, &source_id);
    Ok(())
}
#[tauri::command]
fn inspect_config(
    config_id: String,
    rate_profile: u8,
    state: State<AppState>,
) -> Result<Inspection, String> {
    Ok(analysis::inspect(
        &document(&state, &config_id)?,
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
    export::export(&document(&state, &config_id)?, &request)
}
#[tauri::command]
async fn save_snippet(
    config_id: String,
    request: ExportRequest,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let snippet = export::export(&document(&app.state::<AppState>(), &config_id)?, &request)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .set_file_name("flightlens-snippet.txt")
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = file
            .into_path()
            .map_err(|_| "Only local files are supported")?;
        // Never truncate an existing backup (or any other file).
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| "Choose a new filename. Existing files are never overwritten.")?;
        output
            .write_all(snippet.text.as_bytes())
            .map_err(|_| "Cannot save snippet")?;
        Ok(true)
    })
    .await
    .map_err(|_| "Save worker failed")?
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                let state = window.state::<AppState>();
                if let (Ok(mut registry), Ok(mut pending)) =
                    (state.sources.lock(), state.pending.lock())
                {
                    for path in paths {
                        match registry.register(path.clone()) {
                            Ok(source) => pending.push(source),
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
            ingest_text,
            choose_files,
            open_source,
            restore_session,
            pending_sources,
            close_document,
            inspect_config,
            filter_plot,
            export_snippet,
            save_snippet
        ])
        .run(tauri::generate_context!())
        .expect("FlightLens could not start");
}
