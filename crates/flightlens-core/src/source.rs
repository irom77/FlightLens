use crate::{analyze, Artifact, SourceDescriptor};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
pub const TEXT_LIMIT: usize = 16 * 1024 * 1024;
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Default)]
pub struct SourceRegistry {
    next: u64,
    files: BTreeMap<String, PathBuf>,
}
impl SourceRegistry {
    /// Only native dialogs/drop events may call this; no webview path grant command exists.
    pub fn register(&mut self, path: PathBuf) -> Result<SourceDescriptor, String> {
        let path = path
            .canonicalize()
            .map_err(|_| "Cannot access selected file")?;
        if !path.is_file() {
            return Err("Select a regular file".into());
        }
        self.next += 1;
        let id = format!("source-{}", self.next);
        let label = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.files.insert(id.clone(), path);
        Ok(SourceDescriptor { id, label })
    }
    pub fn path(&self, id: &str) -> Result<PathBuf, String> {
        self.files
            .get(id)
            .cloned()
            .ok_or("Source is not registered".into())
    }
}
pub fn open_path(path: &Path, source_id: &str) -> Result<Artifact, String> {
    let label = path.file_name().unwrap_or_default().to_string_lossy();
    let mut prefix = Vec::new();
    File::open(path)
        .map_err(|_| "Cannot open selected source")?
        .take(65536)
        .read_to_end(&mut prefix)
        .map_err(|_| "Cannot identify selected source")?;
    if is_blackbox(&prefix) {
        return Ok(Artifact::Recognized(crate::RecognizedArtifact{id:source_id.into(),title:label.into(),family:"blackbox".into(),message:"Telemetry decoding available in Phase 3. Binary data has not been parsed as CLI text.".into()}));
    }
    let text = read_stable(path)?;
    analyze(&text, &label, source_id)
}
pub fn is_blackbox(bytes: &[u8]) -> bool {
    bytes.starts_with(b"H Product:Blackbox")
        || bytes
            .windows(19)
            .take(65536)
            .any(|w| w == b"H Product:Blackbox f")
}
pub fn read_stable(path: &Path) -> Result<String, String> {
    for _ in 0..2 {
        let mut file = File::open(path).map_err(|_| "Cannot open selected source")?;
        let before = file.metadata().map_err(|_| "Cannot read file metadata")?;
        if !before.is_file() {
            return Err("Source is not a regular file".into());
        }
        if before.len() > TEXT_LIMIT as u64 {
            return Err("Text imports are limited to 16 MiB".into());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(TEXT_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read selected source")?;
        if bytes.len() > TEXT_LIMIT {
            return Err("Text imports are limited to 16 MiB".into());
        }
        let after = file.metadata().map_err(|_| "Source became unavailable")?;
        let current = std::fs::metadata(path).map_err(|_| "Source became unavailable")?;
        if before.len() == after.len()
            && after.len() == current.len()
            && before.modified().ok() == after.modified().ok()
            && after.modified().ok() == current.modified().ok()
        {
            return String::from_utf8(bytes)
                .map_err(|_| "Source is not UTF-8 text; binary decoding is unavailable".into());
        }
    }
    Err("Source changed while reading. Try again when writing has stopped.".into())
}
