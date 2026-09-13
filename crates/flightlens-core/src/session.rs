//! Versioned portable session metadata. Parsing never opens referenced backups.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
};

pub const SESSION_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableSession {
    pub format: String,
    pub version: u32,
    pub workspace: Option<String>,
    pub documents: Vec<SessionDocument>,
    pub active: Option<usize>,
    pub comparison: Option<SessionComparison>,
    pub tab: String,
    pub theme: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionDocument {
    pub path: String,
    pub sha256: String,
    pub rate_profile: u8,
    pub pid_profile: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionComparison {
    /// Indices into documents, in graph order; baseline is a document index too.
    pub documents: Vec<usize>,
    pub baseline: Option<usize>,
}

fn valid_path(path: &str) -> bool {
    !path.is_empty() && path.len() <= 4096 && !path.contains('\0')
}

impl PortableSession {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "flightlens" || self.version != 1 {
            return Err("Unsupported FlightLens session format or version".into());
        }
        if self.documents.len() > 32 || self.workspace.as_ref().is_some_and(|p| !valid_path(p)) {
            return Err("Invalid session workspace or document limit".into());
        }
        let mut paths = BTreeSet::new();
        for doc in &self.documents {
            if !valid_path(&doc.path)
                || !paths.insert(&doc.path)
                || doc.sha256.len() != 64
                || !doc
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err("Invalid session document reference".into());
            }
        }
        let exists = |index: usize| index < self.documents.len();
        if self.active.is_some_and(|i| !exists(i)) {
            return Err("Invalid active document reference".into());
        }
        if let Some(compare) = &self.comparison {
            let unique: BTreeSet<_> = compare.documents.iter().collect();
            if !(2..=3).contains(&compare.documents.len())
                || unique.len() != compare.documents.len()
                || !compare.documents.iter().all(|&i| exists(i))
                || compare
                    .baseline
                    .is_some_and(|i| !compare.documents.contains(&i))
            {
                return Err("Invalid comparison document references".into());
            }
        }
        if ![
            "Rates", "PID", "Filters", "Ports", "Modes", "OSD", "Raw", "Audit", "Export",
        ]
        .contains(&self.tab.as_str())
            || !["dark", "light"].contains(&self.theme.as_str())
        {
            return Err("Invalid session layout preferences".into());
        }
        Ok(())
    }

    pub fn read(reader: impl Read) -> Result<Self, String> {
        let mut bytes = Vec::new();
        reader
            .take(SESSION_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read FlightLens session")?;
        if bytes.len() as u64 > SESSION_BYTES {
            return Err("FlightLens session exceeds 256 KiB".into());
        }
        // Inspect version before strict decoding, so future fields still produce
        // the actionable unsupported-version error rather than a schema error.
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid FlightLens session JSON")?;
        if value.get("format").and_then(|v| v.as_str()) != Some("flightlens")
            || value.get("version").and_then(|v| v.as_u64()) != Some(1)
        {
            return Err("Unsupported FlightLens session format or version".into());
        }
        let session: Self =
            serde_json::from_value(value).map_err(|_| "Invalid FlightLens session fields")?;
        session.validate()?;
        Ok(session)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes =
            serde_json::to_vec_pretty(self).map_err(|_| "Cannot encode FlightLens session")?;
        if bytes.len() as u64 > SESSION_BYTES {
            return Err("FlightLens session exceeds 256 KiB".into());
        }
        Ok(bytes)
    }
}

/// Save paths beneath the manifest directory relatively. Other paths stay
/// absolute. No canonicalization or filesystem access is performed here.
pub fn reference_path(directory: &Path, source: &Path) -> Result<String, String> {
    if !directory.is_absolute() || !source.is_absolute() {
        return Err("Session paths must be absolute before saving".into());
    }
    let path = source.strip_prefix(directory).unwrap_or(source);
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    let text = path.to_str().ok_or("Session path is not valid Unicode")?;
    let text = if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text.to_owned()
    };
    if !valid_path(&text) {
        return Err("Invalid session path".into());
    }
    Ok(text)
}

/// Resolve against the manifest's containing directory, never the process cwd.
/// Foreign absolute paths need native relinking; they must not become relative.
/// The desktop caller must authorize references before reading them.
pub fn resolve_path(directory: &Path, reference: &str) -> Result<PathBuf, String> {
    if !directory.is_absolute() || !valid_path(reference) {
        return Err("Invalid session path".into());
    }
    if !cfg!(windows) && (reference.contains('\\') || reference.as_bytes().get(1) == Some(&b':')) {
        return Err("Session path requires relinking on this operating system".into());
    }
    if cfg!(windows) && reference.starts_with('/') && !reference.starts_with("//") {
        return Err("Session path requires relinking on this operating system".into());
    }
    let path = Path::new(reference);
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        directory.join(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> PortableSession {
        PortableSession {
            format: "flightlens".into(),
            version: 1,
            workspace: Some("backups".into()),
            documents: vec![SessionDocument {
                path: "backups/a.txt".into(),
                sha256: "a".repeat(64),
                rate_profile: 2,
                pid_profile: 1,
            }],
            active: Some(0),
            comparison: None,
            tab: "Rates".into(),
            theme: "dark".into(),
        }
    }
    #[test]
    fn round_trip_and_strict_compatibility() {
        let session = sample();
        assert_eq!(
            PortableSession::read(session.encode().unwrap().as_slice()).unwrap(),
            session
        );
        assert!(PortableSession::read(
            br#"{"format":"flightlens","version":2,"future":true}"#.as_slice()
        )
        .unwrap_err()
        .contains("Unsupported"));
        let mut value = serde_json::to_value(sample()).unwrap();
        value["unexpected"] = true.into();
        assert!(PortableSession::read(serde_json::to_vec(&value).unwrap().as_slice()).is_err());
        assert!(PortableSession::read(vec![b' '; SESSION_BYTES as usize + 1].as_slice()).is_err());
    }
    #[test]
    fn rejects_broken_references() {
        let mut session = sample();
        session.active = Some(1);
        assert!(session.validate().is_err());
        session.active = None;
        session.comparison = Some(SessionComparison {
            documents: vec![0, 0],
            baseline: Some(0),
        });
        assert!(session.validate().is_err());
        session.comparison = None;
        session.documents[0].sha256 = "not a hash".into();
        assert!(session.validate().is_err());
    }
    #[test]
    fn relative_references_follow_a_moved_folder_without_reading_files() {
        let old = tempfile::tempdir().unwrap();
        let new = tempfile::tempdir().unwrap();
        assert_eq!(reference_path(old.path(), old.path()).unwrap(), ".");
        let path = old.path().join("backups/a.txt");
        let reference = reference_path(old.path(), &path).unwrap();
        assert_eq!(reference, "backups/a.txt");
        assert_eq!(
            resolve_path(new.path(), &reference).unwrap(),
            new.path().join("backups/a.txt")
        );
        assert!(!path.exists());
    }
    #[cfg(unix)]
    #[test]
    fn foreign_windows_paths_require_relinking() {
        for path in [
            "C:/backups/a.txt",
            "C:\\backups\\a.txt",
            "\\\\server\\backups",
        ] {
            assert!(resolve_path(Path::new("/sessions"), path).is_err());
        }
        assert_eq!(
            resolve_path(Path::new("/sessions"), "/backups/a.txt").unwrap(),
            Path::new("/backups/a.txt")
        );
    }
}
