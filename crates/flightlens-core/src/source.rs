use crate::{analyze, Artifact, SourceDescriptor};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
/// Bounds grants, including selected files that never successfully import.
pub const SOURCE_LIMIT: usize = 4096;
pub const TEXT_LIMIT: usize = 16 * 1024 * 1024;
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Default)]
pub struct SourceRegistry {
    next: u64,
    files: BTreeMap<String, PathBuf>,
    documents: BTreeMap<PathBuf, String>,
    source_documents: BTreeMap<String, BTreeSet<String>>,
}
impl SourceRegistry {
    /// Paths come from native dialogs/drop events or an entry in a native-granted
    /// workspace. No webview path grant command exists.
    pub fn register(&mut self, path: PathBuf) -> Result<SourceDescriptor, String> {
        let path = path
            .canonicalize()
            .map_err(|_| "Cannot access selected file")?;
        if !path.is_file() {
            return Err("Select a regular file".into());
        }
        let label = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if let Some((id, _)) = self
            .files
            .iter()
            .find(|(_, registered)| **registered == path)
        {
            return Ok(SourceDescriptor {
                id: id.clone(),
                label,
            });
        }
        if self.files.len() >= SOURCE_LIMIT {
            return Err("File-source limit reached. Close file-backed documents or restart FlightLens before selecting more files.".into());
        }
        self.next += 1;
        let id = format!("source-{}", self.next);
        self.files.insert(id.clone(), path);
        Ok(SourceDescriptor { id, label })
    }
    /// Associate a successfully opened file with the immutable artifact it produced.
    pub fn remember_document(&mut self, source_id: &str, document_id: &str) -> Result<(), String> {
        let path = self.path(source_id)?;
        self.documents.insert(path, document_id.into());
        self.source_documents
            .entry(source_id.into())
            .or_default()
            .insert(document_id.into());
        Ok(())
    }
    /// Remove a closed document's registered sources and saved-session references.
    pub fn forget_document(&mut self, document_id: &str, session: &mut Vec<PathBuf>) {
        session.retain(|path| self.documents.get(path).is_none_or(|id| id != document_id));
        self.source_documents.retain(|source_id, documents| {
            documents.remove(document_id);
            if documents.is_empty() {
                self.files.remove(source_id);
                false
            } else {
                true
            }
        });
        self.documents.retain(|_, id| id != document_id);
    }

    /// A previously imported file reference, without rereading potentially changed bytes.
    pub fn document_path(&self, document_id: &str) -> Result<PathBuf, String> {
        self.documents
            .iter()
            .find_map(|(path, id)| (id == document_id).then(|| path.clone()))
            .ok_or("No open file reference for this document".into())
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
    const HEADER: &[u8] = b"H Product:Blackbox";
    // Scan rather than testing byte 0 alone: a byte-order mark, leading junk, or
    // a recovered partial first session pushes the header off the start of the
    // file, and such a log must not be parsed as CLI text.
    bytes.windows(HEADER.len()).take(65536).any(|w| w == HEADER)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_limit_preserves_existing_grants_and_reuses_released_capacity() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first.dump");
        let extra = temp.path().join("extra.dump");
        std::fs::write(&first, "synthetic").unwrap();
        std::fs::write(&extra, "synthetic").unwrap();
        let mut registry = SourceRegistry::default();
        let source = registry.register(first.clone()).unwrap();
        // Fill the registry without creating thousands of files. These grants
        // intentionally have no owning document, like unattempted/failed imports.
        for index in 1..SOURCE_LIMIT {
            registry.files.insert(
                format!("reserved-{index}"),
                temp.path().join(format!("{index}.dump")),
            );
        }
        assert!(registry
            .register(extra.clone())
            .unwrap_err()
            .contains("File-source limit reached"));
        assert_eq!(registry.register(first).unwrap().id, source.id);
        assert!(registry.path(&source.id).is_ok());
        registry.remember_document(&source.id, "document").unwrap();
        registry.forget_document("document", &mut vec![]);
        let replacement = registry.register(extra).unwrap();
        assert_ne!(replacement.id, source.id);
        assert_eq!(registry.files.len(), SOURCE_LIMIT);
    }
}
