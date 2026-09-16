//! Native entry points for portable sessions. Paths never come from the renderer.
use super::*;
use flightlens_core::session::{
    reference_path, resolve_path, PortableSession, SessionComparison, SessionDocument,
};
use serde::{Deserialize, Serialize};

/// References from the most recently confirmed session that did not import.
/// Kept in native memory; cancellation and invalid manifests do not replace them.
#[derive(Clone)]
pub struct UnresolvedSession {
    session_path: PathBuf,
    directory: PathBuf,
    documents: Vec<SessionDocument>,
    pub(super) workspace: Option<String>,
    active: Option<SessionDocument>,
    comparison: Option<(Vec<SessionDocument>, Option<usize>)>,
}

fn retain_unresolved(
    manifest: &mut PortableSession,
    directory: &std::path::Path,
    unresolved: &UnresolvedSession,
) -> Result<(), String> {
    if let Some(root) = &unresolved.workspace {
        let source = resolve_path(&unresolved.directory, root)
            .map_err(|_| "Cannot relocate an unresolved foreign workspace path; choose a workspace folder before saving")?;
        manifest.workspace = Some(reference_path(directory, &source)?);
    }
    for entry in &unresolved.documents {
        retain_document(manifest, directory, unresolved, entry)?;
    }
    if let Some(entry) = &unresolved.active {
        manifest.active = Some(retain_document(manifest, directory, unresolved, entry)?);
    }
    if let Some((entries, baseline)) = &unresolved.comparison {
        let documents = entries
            .iter()
            .map(|entry| retain_document(manifest, directory, unresolved, entry))
            .collect::<Result<Vec<_>, _>>()?;
        manifest.comparison = Some(SessionComparison {
            baseline: baseline.map(|slot| documents[slot]),
            documents,
        });
    }
    // Includes retained references in the normal size/count validation.
    manifest.encode()?;
    Ok(())
}

fn retain_document(
    manifest: &mut PortableSession,
    directory: &std::path::Path,
    unresolved: &UnresolvedSession,
    entry: &SessionDocument,
) -> Result<usize, String> {
    let mut entry = entry.clone();
    let source = resolve_path(&unresolved.directory, &entry.path).map_err(|_| {
        "Cannot relocate an unresolved foreign path; use Relink session backups before saving"
    })?;
    entry.path = reference_path(directory, &source)?;
    if let Some(index) = manifest.documents.iter().position(|d| d.path == entry.path) {
        if manifest.documents[index].sha256 != entry.sha256 {
            return Err("An open backup conflicts with an unresolved saved reference. Use Relink session backups to locate the original contents before saving.".into());
        }
        return Ok(index);
    }
    let index = manifest.documents.len();
    manifest.documents.push(entry);
    Ok(index)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    #[serde(default = "preserve_selections_default")]
    preserve_unavailable_selections: bool,
    document_ids: Vec<String>,
    profiles: Vec<ProfileSelection>,
    comparison: Option<ComparisonSelection>,
    active_id: Option<String>,
    tab: String,
    theme: String,
}
fn preserve_selections_default() -> bool {
    true
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileSelection {
    document_id: String,
    rate_profile: u8,
    pid_profile: u8,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonSelection {
    documents: Vec<String>,
    baseline: Option<String>,
}

fn comparison_indices(
    selection: &ComparisonSelection,
    ids: &[String],
) -> Result<SessionComparison, String> {
    let index = |id: &String| {
        ids.iter()
            .position(|d| d == id)
            .ok_or_else(|| "Comparison references an unavailable document".to_string())
    };
    Ok(SessionComparison {
        documents: selection
            .documents
            .iter()
            .map(index)
            .collect::<Result<_, _>>()?,
        baseline: selection.baseline.as_ref().map(index).transpose()?,
    })
}

fn restore_comparison(
    selection: &SessionComparison,
    loaded: &[Option<String>],
) -> Option<ComparisonSelection> {
    let documents = selection
        .documents
        .iter()
        .map(|&i| loaded.get(i)?.clone())
        .collect::<Option<Vec<_>>>()?;
    // Identical contents share one document; do not silently collapse graph slots.
    if documents
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != documents.len()
    {
        return None;
    }
    Some(ComparisonSelection {
        documents,
        baseline: match selection.baseline {
            Some(i) => Some(loaded.get(i)?.clone()?),
            None => None,
        },
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResult {
    artifacts: Vec<ArtifactView>,
    profiles: Vec<ProfileSelection>,
    comparison: Option<ComparisonSelection>,
    active_id: Option<String>,
    tab: String,
    theme: String,
    warnings: Vec<String>,
    workspace: Option<flightlens_core::workspace::WorkspacePage>,
}

fn session_source(state: &AppState, id: &str) -> Result<PathBuf, String> {
    // Header-only binary imports use source IDs, not full-content hashes.
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Blackbox references cannot yet be saved in portable sessions; close them before saving".into());
    }
    let sources = state.sources.lock().map_err(|_| "Sources unavailable")?;
    if let Ok(doc) = document(state, id) {
        return sources.path(&doc.source_id).map_err(|_| "Pasted configurations cannot be saved as references. Save the original backup separately and open that file first.".into());
    }
    sources.document_path(id).map_err(|_| "This document has no file reference. Save the original backup separately and open that file first.".into())
}

#[tauri::command]
pub async fn save_portable_session(
    app: tauri::AppHandle,
    request: SaveRequest,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if request.document_ids.len() > 32 {
            return Err("Sessions support at most 32 documents".into());
        }
        if request.profiles.len() != request.document_ids.len()
            || request.document_ids.iter().any(|id| {
                request
                    .profiles
                    .iter()
                    .filter(|p| &p.document_id == id)
                    .count()
                    != 1
            })
        {
            return Err("Session profiles must match the open documents".into());
        }
        let comparison = request
            .comparison
            .as_ref()
            .map(|c| comparison_indices(c, &request.document_ids))
            .transpose()?;
        let state = app.state::<AppState>();
        if let Some(selection) = &request.comparison {
            for id in &selection.documents {
                document(&state, id)
                    .map_err(|_| "Comparisons require supported CLI configurations")?;
            }
        }
        let mut sources = Vec::new();
        for id in &request.document_ids {
            sources.push(session_source(&state, id)?);
        }
        save::save_new(
            &app,
            "workspace.flightlens",
            ("FlightLens session", &["flightlens"]),
            |path| {
                let directory = path.parent().ok_or("Session directory unavailable")?;
                let root = state
                    .index
                    .lock()
                    .map_err(|_| "Workspace unavailable")?
                    .as_ref()
                    .map(|i| i.root());
                let mut manifest = PortableSession {
                    format: "flightlens".into(),
                    version: 1,
                    workspace: root
                        .map(|root| reference_path(directory, &root))
                        .transpose()?,
                    documents: sources
                        .iter()
                        .zip(&request.document_ids)
                        .map(|(path, id)| {
                            let profiles = request
                                .profiles
                                .iter()
                                .find(|p| &p.document_id == id)
                                .ok_or("Session profile unavailable")?;
                            Ok(SessionDocument {
                                path: reference_path(directory, path)?,
                                sha256: id.clone(),
                                rate_profile: profiles.rate_profile,
                                pid_profile: profiles.pid_profile,
                            })
                        })
                        .collect::<Result<_, String>>()?,
                    active: request
                        .active_id
                        .as_ref()
                        .and_then(|id| request.document_ids.iter().position(|d| d == id)),
                    comparison: comparison.clone(),
                    tab: request.tab.clone(),
                    theme: request.theme.clone(),
                };
                if let Some(unresolved) = state
                    .unresolved_session
                    .lock()
                    .map_err(|_| "Session recovery unavailable")?
                    .as_ref()
                {
                    let mut recovery = unresolved.clone();
                    if !request.preserve_unavailable_selections {
                        recovery.active = None;
                        recovery.comparison = None;
                    } else if request.comparison.is_some() {
                        // An explicitly configured available comparison replaces recovery.
                        recovery.comparison = None;
                    }
                    retain_unresolved(&mut manifest, directory, &recovery)?;
                }
                manifest.encode()
            },
        )
    })
    .await
    .map_err(|_| "Session save worker failed")?
}

// Hash and parse the same bounded stable read; a replacement must be identical.
fn checked_backup(path: &std::path::Path, expected: &str) -> Result<String, String> {
    let text = source::read_stable(path)?;
    if source::hash(text.as_bytes()) != expected {
        return Err("Backup hash does not match the saved session; open it separately to inspect changed contents".into());
    }
    Ok(text)
}

#[tauri::command]
pub async fn open_portable_session(
    app: tauri::AppHandle,
    retry: bool,
    relink: bool,
) -> Result<Option<OpenResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = if retry {
            app.state::<AppState>().unresolved_session.lock().map_err(|_| "Session recovery unavailable")?
                .as_ref().map(|session| session.session_path.clone()).ok_or("Open a portable session before retrying")?
        } else {
            let Some(file) = app.dialog().file().add_filter("FlightLens session", &["flightlens"])
                .blocking_pick_file() else { return Ok(None); };
            file.into_path().map_err(|_| "Only local files are supported")?
        };
        let mut manifest = PortableSession::read(std::fs::File::open(&path).map_err(|_| "Cannot open session")?)?;
        let directory = path.parent().ok_or("Session directory unavailable")?;
        // A selected manifest is not itself a grant to every location it names.
        // Show its references in a native dialog before reading any backup.
        let mut references = manifest.documents.iter().map(|d| d.path.as_str()).collect::<Vec<_>>().join("\n");
        if let Some(root) = &manifest.workspace { references.push_str(&format!("\nWorkspace folder (recursive metadata scan): {root}")); }
        if !app.dialog().message(format!("Open the following references from this session? Relative paths use the session folder. Absolute paths and links may refer elsewhere. Existing documents stay open. An available workspace folder replaces the current explorer.\n\n{references}"))
            .title("Open FlightLens session")
            .buttons(tauri_plugin_dialog::MessageDialogButtons::OkCancel).blocking_show() { return Ok(None); }
        let state = app.state::<AppState>();
        let mut result = OpenResult { artifacts: Vec::new(), profiles: Vec::new(), comparison: None, active_id: None, tab: manifest.tab.clone(), theme: manifest.theme.clone(), warnings: Vec::new(), workspace: None };
        let mut unresolved = UnresolvedSession { session_path: path.clone(), directory: directory.to_path_buf(), documents: Vec::new(), workspace: None, active: None, comparison: None };
        let mut loaded_ids = vec![None; manifest.documents.len()];
        for (index, entry) in manifest.documents.iter_mut().enumerate() {
            let loaded = (|| -> Result<ArtifactView, String> {
                let original = resolve_path(directory, &entry.path).and_then(|path| {
                    checked_backup(&path, &entry.sha256).map(|text| (path, text))
                });
                let (path, text) = match original {
                    Ok(loaded) => loaded,
                    Err(error) if relink => {
                        let Some(file) = app.dialog().file()
                            .set_title(format!("Locate session backup: {}", entry.path))
                            .blocking_pick_file() else { return Err(format!("{error}. Relink skipped")); };
                        let replacement = file.into_path().map_err(|_| "Only local files are supported")?;
                        let text = checked_backup(&replacement, &entry.sha256)?;
                        let reference = reference_path(directory, &replacement)?;
                        entry.path = reference;
                        (replacement, text)
                    }
                    Err(error) => return Err(error),
                };
                let source = state.sources.lock().map_err(|_| "Sources unavailable")?.register(path)?;
                let artifact = flightlens_core::analyze(&text, &source.label, &source.id)?;
                let artifact = keep(&state, artifact)?;
                let id = match &artifact { ArtifactView::Config(d) => &d.id, ArtifactView::Recognized(d) => &d.id };
                remember(&app, &source.id, id);
                if matches!(&artifact, ArtifactView::Config(_)) {
                    loaded_ids[index] = Some(id.clone());
                    result.profiles.push(ProfileSelection { document_id: id.clone(), rate_profile: entry.rate_profile, pid_profile: entry.pid_profile });
                }
                if manifest.active == Some(index) { result.active_id = Some(id.clone()); }
                Ok(artifact)
            })();
            match loaded {
                Ok(artifact) => result.artifacts.push(artifact),
                Err(error) => {
                    unresolved.documents.push(entry.clone());
                    result.warnings.push(format!("{}: {error}", entry.path));
                },
            }
        }
        if let Some(comparison) = &manifest.comparison {
            result.comparison = restore_comparison(comparison, &loaded_ids);
            if result.comparison.is_none() {
                unresolved.comparison = Some((comparison.documents.iter().map(|&i| manifest.documents[i].clone()).collect(), comparison.baseline.and_then(|i| comparison.documents.iter().position(|&d| d == i))));
                result.warnings.push("Saved comparison could not be restored: a backup is unavailable, unsupported, or duplicates another backup. No replacement was selected. Save As retains its references and baseline unless you choose a new comparison or turn off keeping unavailable selections.".into());
            }
        }
        if result.active_id.is_none() {
            unresolved.active = manifest.active.map(|i| manifest.documents[i].clone());
            if unresolved.active.is_some() {
                result.warnings.push("The saved active backup is unavailable. Save As retains it unless you turn off keeping unavailable selections.".into());
            }
        }
        if let Some(root) = &manifest.workspace {
            let restored = (|| -> Result<_, String> {
                let _permit = IndexPermit::acquire(&app)?;
                let root = resolve_path(directory, root)?;
                let generation = state.index_generation.fetch_add(1, Ordering::SeqCst) + 1;
                let index = flightlens_core::workspace::WorkspaceIndex::new(root, generation)?;
                let page = index.page("", 0);
                let mut current = state.index.lock().map_err(|_| "Workspace unavailable")?;
                state.index_cancelled.store(false, Ordering::SeqCst);
                *current = Some(index);
                Ok(page)
            })();
            match restored {
                Ok(page) => result.workspace = Some(page),
                Err(error) => {
                    unresolved.workspace = Some(root.clone());
                    result.warnings.push(format!("Workspace {root}: {error}. Existing explorer retained. Save As preserves the unavailable folder instead of the current explorer folder; choose a workspace folder to replace it, or reopen a saved session to retry."));
                },
            }
        }
        if !unresolved.documents.is_empty() {
            result.warnings.push(format!("{} unresolved backup references will be retained by Save As during this app run. Reopen a saved session to retry. Opening another confirmed session replaces this recovery list.", unresolved.documents.len()));
        }
        *state.unresolved_session.lock().map_err(|_| "Session recovery unavailable")? = Some(unresolved);
        Ok(Some(result))
    }).await.map_err(|_| "Session open worker failed")?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recovery_fixture() -> (PortableSession, UnresolvedSession, PathBuf) {
        let root = std::env::temp_dir().join("flightlens-recovery-test");
        let entry = SessionDocument {
            path: "missing.dump".into(),
            sha256: "a".repeat(64),
            rate_profile: 2,
            pid_profile: 3,
        };
        let manifest = PortableSession {
            format: "flightlens".into(),
            version: 1,
            workspace: None,
            documents: vec![],
            active: None,
            comparison: None,
            tab: "Rates".into(),
            theme: "dark".into(),
        };
        (
            manifest,
            UnresolvedSession {
                session_path: root.join("old/session.flightlens"),
                directory: root.join("old"),
                documents: vec![entry],
                workspace: None,
                active: None,
                comparison: None,
            },
            root,
        )
    }

    #[test]
    fn portable_sessions_reject_pasted_config_and_recognized_text() {
        let state = AppState::default();
        for text in [
            "# Betaflight / TEST 4.5.3\nset thr_mid = 50\n",
            "# INAV / TEST 8.0.0\n",
            "# ArduPilot\n",
        ] {
            let artifact = keep(
                &state,
                flightlens_core::analyze(text, "Pasted backup", "virtual").unwrap(),
            )
            .unwrap();
            let id = match &artifact {
                ArtifactView::Config(d) => &d.id,
                ArtifactView::Recognized(d) => &d.id,
            };
            let error = session_source(&state, id).unwrap_err();
            assert!(error.contains("Save the original backup separately and open that file first"));
        }
    }

    #[test]
    fn recognized_text_session_reference_survives_missing_file_but_not_close() {
        let state = AppState::default();
        let path =
            std::env::temp_dir().join(format!("flightlens-recognized-{}.txt", std::process::id()));
        let text = "# INAV / TEST 8.0.0\n";
        std::fs::write(&path, text).unwrap();
        let registered = state
            .sources
            .lock()
            .unwrap()
            .register(path.clone())
            .unwrap();
        let artifact = source::open_path(&path, &registered.id).unwrap();
        let Artifact::Recognized(artifact) = artifact else {
            panic!("Expected recognized text");
        };
        assert_eq!(artifact.id, source::hash(text.as_bytes()));
        assert!(session_source(&state, &artifact.id).is_err());
        state
            .sources
            .lock()
            .unwrap()
            .remember_document(&registered.id, &artifact.id)
            .unwrap();
        let saved_path = session_source(&state, &artifact.id).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(session_source(&state, &artifact.id).unwrap(), saved_path);
        let (mut manifest, _, root) = recovery_fixture();
        manifest.documents.push(SessionDocument {
            path: reference_path(&root, &saved_path).unwrap(),
            sha256: artifact.id.clone(),
            rate_profile: 0,
            pid_profile: 0,
        });
        manifest.active = Some(0);
        let reopened = PortableSession::read(manifest.encode().unwrap().as_slice()).unwrap();
        assert_eq!(reopened.documents[0].sha256, artifact.id);
        assert_eq!(reopened.active, Some(0));
        state
            .sources
            .lock()
            .unwrap()
            .forget_document(&artifact.id, &mut vec![]);
        assert!(session_source(&state, &artifact.id).is_err());
        assert!(session_source(&state, &registered.id)
            .unwrap_err()
            .contains("Blackbox"));
    }

    #[test]
    fn relink_accepts_only_identical_bytes_and_preserves_saved_selections() {
        let (mut manifest, mut pending, root) = recovery_fixture();
        let replacement =
            std::env::temp_dir().join(format!("flightlens-relink-{}.dump", std::process::id()));
        std::fs::write(&replacement, b"# version\n").unwrap();
        let text = checked_backup(&replacement, &source::hash(b"# version\n")).unwrap();
        assert_eq!(text, "# version\n");
        assert!(checked_backup(&replacement, &"a".repeat(64)).is_err());
        std::fs::remove_file(&replacement).unwrap();
        assert!(checked_backup(&replacement, &source::hash(b"# version\n")).is_err());
        pending.documents[0].path = reference_path(&pending.directory, &replacement).unwrap();
        pending.active = Some(pending.documents[0].clone());
        retain_unresolved(&mut manifest, &root, &pending).unwrap();
        assert_eq!(manifest.active, Some(0));
        assert_eq!(manifest.documents[0].rate_profile, 2);
        assert_eq!(manifest.documents[0].pid_profile, 3);
        assert_eq!(
            resolve_path(&root, &manifest.documents[0].path).unwrap(),
            replacement
        );
    }

    #[test]
    fn unresolved_references_survive_moved_save_without_reading_backups() {
        let (mut manifest, pending, root) = recovery_fixture();
        let destination = root.join("new");
        retain_unresolved(&mut manifest, &destination, &pending).unwrap();
        let reopened = PortableSession::read(manifest.encode().unwrap().as_slice()).unwrap();
        assert_eq!(
            resolve_path(&destination, &reopened.documents[0].path).unwrap(),
            root.join("old/missing.dump")
        );
        assert_eq!(reopened.documents[0].sha256, "a".repeat(64));
        assert_eq!(reopened.documents[0].rate_profile, 2);
        assert_eq!(reopened.documents[0].pid_profile, 3);
        // Repeated saves neither consume nor duplicate the recovery list.
        retain_unresolved(&mut manifest, &destination, &pending).unwrap();
        assert_eq!(manifest.documents.len(), 1);
        manifest.documents[0].sha256 = "b".repeat(64);
        assert!(retain_unresolved(&mut manifest, &destination, &pending).is_err());
    }

    #[test]
    fn unavailable_workspace_survives_save_and_explicit_replacement() {
        let (mut manifest, mut pending, root) = recovery_fixture();
        pending.documents.clear();
        pending.workspace = Some("missing-folder".into());
        let current = reference_path(&root, &root.join("current")).unwrap();
        manifest.workspace = Some(current.clone());
        retain_unresolved(&mut manifest, &root, &pending).unwrap();
        let reopened = PortableSession::read(manifest.encode().unwrap().as_slice()).unwrap();
        assert_eq!(
            resolve_path(&root, reopened.workspace.as_ref().unwrap()).unwrap(),
            root.join("old/missing-folder")
        );
        // Choosing a new folder clears only workspace recovery; refreshing does not.
        pending.workspace = None;
        manifest.workspace = Some(current.clone());
        retain_unresolved(&mut manifest, &root, &pending).unwrap();
        assert_eq!(manifest.workspace, Some(current));
    }

    #[test]
    fn unavailable_selections_remap_after_open_documents_and_reopen() {
        let (mut manifest, mut pending, root) = recovery_fixture();
        let missing = pending.documents[0].clone();
        let available = SessionDocument {
            path: "available.dump".into(),
            sha256: "b".repeat(64),
            ..missing.clone()
        };
        pending.active = Some(missing.clone());
        pending.comparison = Some((vec![missing.clone(), available.clone()], Some(1)));
        manifest.documents.push(SessionDocument {
            path: reference_path(&root, &pending.directory.join(&available.path)).unwrap(),
            ..available
        });
        manifest.active = Some(0);
        retain_unresolved(&mut manifest, &root, &pending).unwrap();
        let reopened = PortableSession::read(manifest.encode().unwrap().as_slice()).unwrap();
        assert_eq!(reopened.documents.len(), 2);
        assert_eq!(reopened.active, Some(1));
        assert_eq!(reopened.comparison.as_ref().unwrap().documents, vec![1, 0]);
        assert_eq!(reopened.comparison.as_ref().unwrap().baseline, Some(0));
        let restored = restore_comparison(
            reopened.comparison.as_ref().unwrap(),
            &[Some("available".into()), Some("recovered".into())],
        )
        .unwrap();
        assert_eq!(restored.documents, vec!["recovered", "available"]);
        assert_eq!(restored.baseline.as_deref(), Some("available"));
    }

    #[test]
    fn unresolved_references_count_toward_session_limit() {
        let (mut manifest, mut pending, root) = recovery_fixture();
        let entry = pending.documents[0].clone();
        pending.documents = (0..33)
            .map(|i| SessionDocument {
                path: format!("{i}.dump"),
                ..entry.clone()
            })
            .collect();
        assert!(retain_unresolved(&mut manifest, &root, &pending).is_err());
    }

    #[test]
    fn comparison_order_baseline_and_missing_references() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let selected = ComparisonSelection {
            documents: vec!["c".into(), "a".into(), "b".into()],
            baseline: Some("a".into()),
        };
        let indices = comparison_indices(&selected, &ids).unwrap();
        assert_eq!(indices.documents, vec![2, 0, 1]);
        assert_eq!(indices.baseline, Some(0));
        let loaded = ids.into_iter().map(Some).collect::<Vec<_>>();
        let restored = restore_comparison(&indices, &loaded).unwrap();
        assert_eq!(restored.documents, selected.documents);
        assert_eq!(restored.baseline, selected.baseline);
        assert!(
            restore_comparison(&indices, &[Some("a".into()), None, Some("c".into())]).is_none()
        );
        assert!(restore_comparison(
            &indices,
            &[Some("a".into()), Some("a".into()), Some("c".into())]
        )
        .is_none());
        assert!(comparison_indices(&selected, &["a".into()]).is_err());
        let pair = SessionComparison {
            documents: vec![1, 0],
            baseline: None,
        };
        assert!(restore_comparison(&pair, &loaded)
            .unwrap()
            .baseline
            .is_none());
    }
}
