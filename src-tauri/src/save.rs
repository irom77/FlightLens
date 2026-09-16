//! Save new files without replacing existing paths, including collision retries.
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
use tauri_plugin_dialog::DialogExt;

fn suggestion(path: &Path) -> PathBuf {
    if path.symlink_metadata().is_err() {
        return path.to_owned();
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    for number in 1u64.. {
        let candidate = path.with_file_name(format!("{stem} ({number}){extension}"));
        if candidate.symlink_metadata().is_err() {
            return candidate;
        }
    }
    unreachable!()
}

fn save_with_picker(
    initial: PathBuf,
    mut pick: impl FnMut(&Path, bool) -> Result<Option<PathBuf>, String>,
    mut contents: impl FnMut(&Path) -> Result<Vec<u8>, String>,
) -> Result<bool, String> {
    let mut proposed = suggestion(&initial);
    let mut collision = false;
    loop {
        let Some(path) = pick(&proposed, collision)? else {
            return Ok(false);
        };
        // Encode before creating a file; sessions must resolve references again
        // if a retry chooses a different destination directory.
        let bytes = contents(&path)?;
        let mut output = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(output) => output,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                proposed = suggestion(&path);
                collision = true;
                continue;
            }
            Err(_) => {
                return Err("Cannot create file in this location. Choose a writable folder.".into())
            }
        };
        output
            .write_all(&bytes)
            .and_then(|_| output.sync_all())
            .map_err(|_| "Cannot finish saving; the new file may be incomplete")?;
        return Ok(true);
    }
}

pub fn save_new(
    app: &tauri::AppHandle,
    filename: &str,
    filter: (&str, &[&str]),
    contents: impl FnMut(&Path) -> Result<Vec<u8>, String>,
) -> Result<bool, String> {
    use tauri::Manager;
    let initial = app
        .path()
        .document_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(filename);
    save_with_picker(
        initial,
        |proposed, collision| {
            let mut dialog = app
                .dialog()
                .file()
                .add_filter(filter.0, filter.1)
                .set_title(if collision {
                    "File exists — choose a new name; existing files are never overwritten"
                } else {
                    "Save as a new file"
                });
            if let Some(parent) = proposed.parent() {
                dialog = dialog.set_directory(parent);
            }
            if let Some(name) = proposed.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy());
            }
            dialog
                .blocking_save_file()
                .map(|file| {
                    file.into_path()
                        .map_err(|_| "Only local files are supported".into())
                })
                .transpose()
        },
        contents,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collision_retries_and_rebuilds_contents_for_the_new_directory() {
        let root = std::env::temp_dir().join(format!("flightlens-save-{}", std::process::id()));
        std::fs::create_dir_all(root.join("other")).unwrap();
        let existing = root.join("workspace.flightlens");
        let destination = root.join("other/new.flightlens");
        std::fs::write(&existing, b"original").unwrap();
        let mut attempts = 0;
        let saved = save_with_picker(
            existing.clone(),
            |proposed, collision| {
                attempts += 1;
                assert_eq!(proposed, root.join("workspace (1).flightlens"));
                assert_eq!(collision, attempts == 2);
                Ok(Some(if attempts == 1 {
                    existing.clone()
                } else {
                    destination.clone()
                }))
            },
            |path| Ok(path.parent().unwrap().to_string_lossy().as_bytes().to_vec()),
        )
        .unwrap();
        assert!(saved);
        assert_eq!(attempts, 2);
        assert_eq!(std::fs::read(&existing).unwrap(), b"original");
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            root.join("other").to_string_lossy().as_bytes()
        );
        let mut attempts = 0;
        assert!(!save_with_picker(
            existing.clone(),
            |_, _| {
                attempts += 1;
                Ok((attempts == 1).then(|| existing.clone()))
            },
            |_| Ok(vec![])
        )
        .unwrap());
        assert_eq!(std::fs::read(&existing).unwrap(), b"original");
        std::fs::remove_dir_all(root).unwrap();
    }
}
