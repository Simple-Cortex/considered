//! Deterministic directory traversal returning sorted, workspace-relative paths.

use considered_core::{ArtifactPath, EngineError};
use std::path::Path;
use walkdir::WalkDir;

/// Walk a directory tree and return sorted, workspace-relative paths.
///
/// - Skips hidden directories (starting with `.`) except the root itself.
/// - Skips common build/dependency directories: `node_modules`, `dist`,
///   `build`, `.next`, `.git`, `coverage`, `out`, `.turbo`, `vendor`.
/// - Returns paths sorted lexicographically for deterministic output.
/// - All returned paths are workspace-relative (root is stripped).
pub fn walk_sorted(root: &Path) -> Result<Vec<ArtifactPath>, EngineError> {
    const SKIP_DIRS: &[&str] = &[
        "node_modules",
        "dist",
        "build",
        ".next",
        ".git",
        "coverage",
        "out",
        ".turbo",
        "vendor",
    ];

    let mut paths = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        if e.depth() > 0 && name.starts_with('.') {
            return false;
        }
        if e.depth() > 0 && e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
            return false;
        }
        true
    }) {
        let entry =
            entry.map_err(|e| {
                EngineError::Io(e.into_io_error().unwrap_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::Other, "walkdir error")
                }))
            })?;

        if entry.file_type().is_file() {
            let rel = considered_core::path::normalize_relative(root, entry.path())?;
            paths.push(rel);
        }
    }

    paths.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn walk_returns_sorted_relative_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src/components")).unwrap();
        fs::write(root.join("src/App.tsx"), "").unwrap();
        fs::write(root.join("src/components/Button.tsx"), "").unwrap();
        fs::write(root.join("src/components/Card.tsx"), "").unwrap();

        let paths = walk_sorted(root).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].as_str(), "src/App.tsx");
        assert_eq!(paths[1].as_str(), "src/components/Button.tsx");
        assert_eq!(paths[2].as_str(), "src/components/Card.tsx");
    }

    #[test]
    fn walk_skips_hidden_and_build_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join(".git/HEAD"), "").unwrap();
        fs::write(root.join("node_modules/pkg/index.js"), "").unwrap();
        fs::write(root.join("src/main.ts"), "").unwrap();

        let paths = walk_sorted(root).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].as_str(), "src/main.ts");
    }

    #[test]
    fn walk_is_deterministic_across_runs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("b")).unwrap();
        fs::create_dir_all(root.join("a")).unwrap();
        fs::write(root.join("b/file.ts"), "").unwrap();
        fs::write(root.join("a/file.ts"), "").unwrap();
        fs::write(root.join("c.ts"), "").unwrap();

        let first = walk_sorted(root).unwrap();
        let second = walk_sorted(root).unwrap();
        assert_eq!(first, second);
        assert_eq!(first[0].as_str(), "a/file.ts");
    }
}
