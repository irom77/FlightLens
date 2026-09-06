#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use flightlens_core::{
    analysis::{self, Inspection, Point},
    export::{self, ExportRequest, ValidatedSnippet},
    source::{self, SourceRegistry},
    Artifact, ConfigDocument, SourceDescriptor,
};
use std::{collections::BTreeMap, io::Write, sync::Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
#[derive(Default)]
struct AppState {
    sources: Mutex<SourceRegistry>,
    documents: Mutex<BTreeMap<String, ConfigDocument>>,
    pending: Mutex<Vec<SourceDescriptor>>,
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
        keep(&app.state::<AppState>(), artifact)
    })
    .await
    .map_err(|_| "Import worker failed")?
}
#[tauri::command]
fn pending_sources(state: State<AppState>) -> Result<Vec<SourceDescriptor>, String> {
    Ok(std::mem::take(
        &mut *state.pending.lock().map_err(|_| "Drop queue unavailable")?,
    ))
}
#[tauri::command]
fn close_document(config_id: String, state: State<AppState>) -> Result<(), String> {
    state
        .documents
        .lock()
        .map_err(|_| "Document repository unavailable")?
        .remove(&config_id);
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
