//! Relative artifact paths.
//!
//! The engine rejects absolute or escaping artifact paths before filesystem
//! access, per the implementation standards. All paths stored in findings
//! and reports are workspace-relative.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::EngineError;

/// A workspace-relative artifact path.
///
/// Constructed via `ArtifactPath::new`, which rejects absolute paths and
/// paths that escape the workspace root via `..` components.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactPath(String);

impl ArtifactPath {
    /// Create a new artifact path from a workspace-relative string.
    ///
    /// Rejects:
    /// - Absolute paths (starting with `/` or a Windows drive letter)
    /// - Paths containing `..` components that would escape the root
    /// - Empty paths
    pub fn new(path: impl Into<String>) -> Result<Self, EngineError> {
        let s = path.into();
        if s.is_empty() {
            return Err(EngineError::InvalidPath("path is empty".into()));
        }
        let p = Path::new(&s);
        if p.is_absolute() {
            return Err(EngineError::InvalidPath(format!(
                "absolute path rejected: {s}"
            )));
        }
        for component in p.components() {
            if let std::path::Component::ParentDir = component {
                return Err(EngineError::InvalidPath(format!(
                    "escaping path rejected (contains ..): {s}"
                )));
            }
        }
        Ok(ArtifactPath(s))
    }

    /// Return the path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to a PathBuf for filesystem operations.
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(&self.0)
    }

    /// Normalize path separators to forward slashes (for cross-platform
    /// deterministic output).
    pub fn normalized(&self) -> String {
        self.0.replace('\\', "/")
    }
}

/// Normalize a filesystem path to a workspace-relative forward-slash form.
///
/// Strips the workspace root prefix and converts separators.
pub fn normalize_relative(root: &Path, full: &Path) -> Result<ArtifactPath, EngineError> {
    let rel = full.strip_prefix(root).map_err(|_| {
        EngineError::InvalidPath(format!("path not under root: {}", full.display()))
    })?;
    let s = rel.to_string_lossy().replace('\\', "/");
    ArtifactPath::new(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_relative_paths() {
        assert!(ArtifactPath::new("src/App.tsx").is_ok());
        assert!(ArtifactPath::new("assets/rules/contract.json").is_ok());
    }

    #[test]
    fn rejects_absolute_paths() {
        assert!(ArtifactPath::new("/etc/passwd").is_err());
        assert!(ArtifactPath::new("/Users/test/file").is_err());
    }

    #[test]
    fn rejects_escaping_paths() {
        assert!(ArtifactPath::new("../outside").is_err());
        assert!(ArtifactPath::new("src/../../etc").is_err());
    }

    #[test]
    fn rejects_empty_paths() {
        assert!(ArtifactPath::new("").is_err());
    }

    #[test]
    fn normalized_converts_separators() {
        let p = ArtifactPath::new("src/components/Button.tsx").unwrap();
        assert_eq!(p.normalized(), "src/components/Button.tsx");
    }

    #[test]
    fn artifact_path_round_trips_through_json() {
        let p = ArtifactPath::new(".considered/surface/CONTRACT.md").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\".considered/surface/CONTRACT.md\"");
        let back: ArtifactPath = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn normalize_relative_strips_root() {
        let root = Path::new("/workspace/project");
        let full = Path::new("/workspace/project/src/App.tsx");
        let rel = normalize_relative(root, full).unwrap();
        assert_eq!(rel.as_str(), "src/App.tsx");
    }

    #[test]
    fn normalize_relative_rejects_outside_root() {
        let root = Path::new("/workspace/project");
        let full = Path::new("/other/path/file.tsx");
        assert!(normalize_relative(root, full).is_err());
    }
}
