//! Bounded, metadata-only discovery. Paths are granted by a native folder picker.
use serde::Serialize;
use std::{
    fs::{self, ReadDir},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, UNIX_EPOCH},
};
use ts_rs::TS;

const ENTRY_LIMIT: usize = 10_000;
const VISIT_LIMIT: usize = 100_000;
const DEPTH_LIMIT: usize = 64;
const BATCH_LIMIT: usize = 256;

#[derive(Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
    pub id: String,
    pub relative_path: String,
    pub bytes: f64,
    pub modified_seconds: Option<f64>,
}
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePage {
    pub root: String,
    pub status: String,
    pub visited: usize,
    pub skipped: usize,
    pub indexed: usize,
    pub total: usize,
    pub offset: usize,
    pub entries: Vec<WorkspaceEntry>,
}
struct Entry {
    summary: WorkspaceEntry,
    path: PathBuf,
}
pub struct WorkspaceIndex {
    root: PathBuf,
    generation: u64,
    stack: Vec<ReadDir>,
    entries: Vec<Entry>,
    visited: usize,
    skipped: usize,
    status: String,
}
impl WorkspaceIndex {
    pub fn new(root: PathBuf, generation: u64) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|_| "Workspace folder is unavailable. Reconnect the drive and retry.")?;
        let directory = fs::read_dir(&root)
            .map_err(|_| "Cannot read workspace folder. Check access and retry.")?;
        Ok(Self {
            root,
            generation,
            stack: vec![directory],
            entries: vec![],
            visited: 0,
            skipped: 0,
            status: "scanning".into(),
        })
    }
    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }
    fn finish(&mut self, status: &str) {
        self.status = status.into();
        self.stack.clear();
        self.entries
            .sort_by(|a, b| a.summary.relative_path.cmp(&b.summary.relative_path));
    }
    /// A single worker advances a bounded batch; cancellation is checked between OS calls.
    pub fn advance(&mut self, cancelled: &AtomicBool) {
        if self.status != "scanning" {
            return;
        }
        let started = Instant::now();
        for _ in 0..BATCH_LIMIT {
            if cancelled.load(Ordering::Relaxed) {
                self.finish("cancelled");
                return;
            }
            if self.entries.len() >= ENTRY_LIMIT || self.visited >= VISIT_LIMIT {
                self.finish("limited");
                return;
            }
            if started.elapsed() >= Duration::from_millis(50) {
                return;
            }
            let Some(directory) = self.stack.last_mut() else {
                self.finish(if self.root.is_dir() {
                    "complete"
                } else {
                    "unavailable"
                });
                return;
            };
            let Some(result) = directory.next() else {
                self.stack.pop();
                continue;
            };
            self.visited += 1;
            let Ok(entry) = result else {
                self.skipped += 1;
                continue;
            };
            let Ok(kind) = entry.file_type() else {
                self.skipped += 1;
                continue;
            };
            if kind.is_symlink() {
                self.skipped += 1;
                continue;
            }
            if kind.is_dir() {
                if self.stack.len() >= DEPTH_LIMIT {
                    self.skipped += 1;
                    continue;
                }
                match fs::read_dir(entry.path()) {
                    Ok(directory) => self.stack.push(directory),
                    Err(_) => self.skipped += 1,
                }
                continue;
            }
            if !kind.is_file() || !supported(&entry.path()) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                self.skipped += 1;
                continue;
            };
            let path = entry.path();
            let relative_path = path
                .strip_prefix(&self.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let summary = WorkspaceEntry {
                id: format!("{}:{}", self.generation, self.entries.len()),
                relative_path,
                bytes: metadata.len() as f64,
                modified_seconds: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs_f64()),
            };
            self.entries.push(Entry { summary, path });
        }
    }
    pub fn page(&self, query: &str, offset: usize) -> WorkspacePage {
        let query: String = query.chars().take(512).collect::<String>().to_lowercase();
        let matching: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.summary.relative_path.to_lowercase().contains(&query))
            .collect();
        let offset = offset.min(matching.len().saturating_sub(1) / 50 * 50);
        WorkspacePage {
            root: self.root.to_string_lossy().into_owned(),
            status: self.status.clone(),
            visited: self.visited,
            skipped: self.skipped,
            indexed: self.entries.len(),
            total: matching.len(),
            offset,
            entries: matching
                .into_iter()
                .skip(offset)
                .take(50)
                .map(|e| e.summary.clone())
                .collect(),
        }
    }
    /// Resolve only an indexed identifier, checking the current path still belongs to the granted root.
    pub fn path(&self, id: &str) -> Result<PathBuf, String> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.summary.id == id)
            .ok_or("Workspace entry expired. Refresh the explorer.")?;
        let path = entry
            .path
            .canonicalize()
            .map_err(|_| "Backup unavailable. Reconnect the drive or refresh the workspace.")?;
        if !path.starts_with(&self.root) || !path.is_file() {
            return Err(
                "Backup moved outside the workspace or is no longer a regular file.".into(),
            );
        }
        Ok(path)
    }
}
fn supported(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "txt" | "diff" | "dump" | "param" | "parm" | "bbl" | "bfl"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Folder(PathBuf);
    impl Folder {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "flightlens-index-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn complete(index: &mut WorkspaceIndex) {
        while index.status == "scanning" {
            index.advance(&AtomicBool::new(false));
        }
    }
    #[test]
    fn recursive_search_pages_and_fresh_paths_without_reading_contents() {
        let folder = Folder::new();
        fs::create_dir(folder.0.join("nested")).unwrap();
        for n in 0..75 {
            fs::write(folder.0.join(format!("nested/backup-{n:02}.TXT")), [0xff]).unwrap();
        }
        fs::write(folder.0.join("ignored.png"), []).unwrap();
        let mut index = WorkspaceIndex::new(folder.0.clone(), 1).unwrap();
        complete(&mut index);
        let page = index.page("NESTED", 0);
        assert_eq!(page.total, 75);
        assert_eq!(page.entries.len(), 50);
        assert_eq!(index.page("", 50).entries.len(), 25);
        assert_eq!(index.page("absent", 999).offset, 0);
        let id = &page.entries[0].id;
        let path = index.path(id).unwrap();
        fs::remove_file(path).unwrap();
        assert!(index.path(id).is_err());
        assert!(WorkspaceIndex::new(folder.0.clone(), 2)
            .unwrap()
            .path(id)
            .is_err());
    }
    #[test]
    fn cancellation_limits_and_disconnection() {
        let folder = Folder::new();
        let mut index = WorkspaceIndex::new(folder.0.clone(), 1).unwrap();
        index.advance(&AtomicBool::new(true));
        assert_eq!(index.status, "cancelled");
        assert!(index.stack.is_empty());
        let mut index = WorkspaceIndex::new(folder.0.clone(), 2).unwrap();
        index.visited = VISIT_LIMIT;
        index.advance(&AtomicBool::new(false));
        assert_eq!(index.status, "limited");
        let mut index = WorkspaceIndex::new(folder.0.clone(), 3).unwrap();
        fs::remove_dir(&folder.0).unwrap();
        complete(&mut index);
        assert_eq!(index.status, "unavailable");
        assert!(WorkspaceIndex::new(folder.0.clone(), 4).is_err());
    }
    #[test]
    fn batches_and_depth_are_bounded_and_reconnect_can_restart() {
        let folder = Folder::new();
        for n in 0..300 {
            fs::write(folder.0.join(format!("{n}.dump")), []).unwrap();
        }
        let mut nested = folder.0.clone();
        for _ in 0..70 {
            nested = nested.join("d");
            fs::create_dir(&nested).unwrap();
        }
        fs::write(nested.join("too-deep.txt"), []).unwrap();
        let mut index = WorkspaceIndex::new(folder.0.clone(), 1).unwrap();
        index.advance(&AtomicBool::new(false));
        assert!(index.visited <= BATCH_LIMIT);
        assert_eq!(index.status, "scanning");
        complete(&mut index);
        assert_eq!(index.page("", 0).total, 300);
        assert_eq!(index.skipped, 1);
        let moved = folder.0.with_extension("disconnected");
        fs::rename(&folder.0, &moved).unwrap();
        assert!(WorkspaceIndex::new(folder.0.clone(), 2).is_err());
        fs::rename(&moved, &folder.0).unwrap();
        let mut refreshed = WorkspaceIndex::new(folder.0.clone(), 3).unwrap();
        complete(&mut refreshed);
        assert_eq!(refreshed.page("", 0).total, 300);
        assert!(refreshed.path(&index.page("", 0).entries[0].id).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn links_are_skipped_and_replaced_paths_cannot_escape_root() {
        let folder = Folder::new();
        let outside = Folder::new();
        fs::write(outside.0.join("secret.txt"), []).unwrap();
        std::os::unix::fs::symlink(&outside.0, folder.0.join("link")).unwrap();
        fs::write(folder.0.join("backup.txt"), []).unwrap();
        let mut index = WorkspaceIndex::new(folder.0.clone(), 1).unwrap();
        complete(&mut index);
        let page = index.page("", 0);
        assert_eq!(page.total, 1);
        assert_eq!(page.skipped, 1);
        fs::remove_file(folder.0.join("backup.txt")).unwrap();
        std::os::unix::fs::symlink(outside.0.join("secret.txt"), folder.0.join("backup.txt"))
            .unwrap();
        assert!(index.path(&page.entries[0].id).is_err());
    }
}
