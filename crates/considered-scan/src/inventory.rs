//! Port of `scripts/inventory.mjs` — report the design system that already exists.
//!
//! Walks a directory tree, collects code files, extracts design tokens (CSS custom
//! properties, fonts, spacing, radii), detects components and configs, and returns
//! a structured [`InventoryReport`]. The crate never prints or terminates.

use considered_core::EngineError;
use regex::Regex;
use serde::ser::{SerializeMap, SerializeStruct};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

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

const CODE_EXTENSIONS: &[&str] = &[
    ".css", ".scss", ".sass", ".less", ".html", ".jsx", ".tsx", ".js", ".mjs", ".cjs", ".ts",
    ".mts", ".cts", ".vue", ".svelte", ".astro", ".mdx",
];

const CONFIG_FILES: &[&str] = &[
    "tailwind.config.js",
    "tailwind.config.ts",
    "tailwind.config.mjs",
    "tailwind.config.cjs",
    "theme.js",
    "theme.ts",
    "panda.config.ts",
    "uno.config.ts",
];

const CONTEXT_DOCS: &[&str] = &[
    "DESIGN.md",
    "PRODUCT.md",
    "STRUCTURE.md",
    "design.md",
    "product.md",
];

const TOKEN_PREFIXES: &[&str] = &[
    "color", "bg", "surface", "text", "border", "accent", "primary", "danger", "success", "warning",
];

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// The full inventory report returned to callers.
#[derive(Debug, Clone, Deserialize)]
pub struct InventoryReport {
    pub target: String,
    pub target_kind: String,
    pub files: usize,
    pub tokens: BTreeMap<String, String>,
    pub fonts: Vec<String>,
    pub spacing: Vec<String>,
    pub radii: Vec<String>,
    pub components: BTreeMap<String, Vec<String>>,
    pub configs: Vec<String>,
    pub docs: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(skip)]
    token_order: Vec<String>,
    #[serde(skip)]
    component_order: Vec<String>,
    #[serde(skip)]
    component_dirs: Vec<String>,
}

impl InventoryReport {
    /// Token groups in the same first-seen order used by the Node renderer.
    pub fn ordered_token_groups(&self) -> Vec<(String, Vec<String>)> {
        let mut groups: Vec<(String, Vec<String>)> = Vec::new();
        for name in &self.token_order {
            let Some(value) = self.tokens.get(name) else {
                continue;
            };
            let Some(prefix) = TOKEN_PREFIXES.iter().find(|prefix| name.contains(**prefix)) else {
                continue;
            };
            if let Some((_, values)) = groups.iter_mut().find(|(key, _)| key == prefix) {
                values.push(format!("{name}: {value}"));
            } else {
                groups.push((prefix.to_string(), vec![format!("{name}: {value}")]));
            }
        }
        groups
    }

    /// Component directories in first-seen order, for compatibility rendering.
    pub fn component_dirs(&self) -> &[String] {
        &self.component_dirs
    }
}

struct OrderedMap<'a, V> {
    order: &'a [String],
    values: &'a BTreeMap<String, V>,
}

impl<V: Serialize> Serialize for OrderedMap<'_, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.order.len()))?;
        for key in self.order {
            if let Some(value) = self.values.get(key) {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}

impl Serialize for InventoryReport {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("InventoryReport", 11)?;
        state.serialize_field("target", &self.target)?;
        state.serialize_field("targetKind", &self.target_kind)?;
        state.serialize_field("files", &self.files)?;
        state.serialize_field(
            "tokens",
            &OrderedMap {
                order: &self.token_order,
                values: &self.tokens,
            },
        )?;
        state.serialize_field("fonts", &self.fonts)?;
        state.serialize_field("spacing", &self.spacing)?;
        state.serialize_field("radii", &self.radii)?;
        state.serialize_field(
            "components",
            &OrderedMap {
                order: &self.component_order,
                values: &self.components,
            },
        )?;
        state.serialize_field("configs", &self.configs)?;
        state.serialize_field("docs", &self.docs)?;
        state.serialize_field("warnings", &self.warnings)?;
        state.end()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let ext = format!(".{e}");
            CODE_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

fn collect_files(root: &Path) -> (Vec<PathBuf>, Vec<String>) {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        if e.depth() > 0 && name.starts_with('.') && name != ".storybook" {
            return false;
        }
        if e.depth() > 0 && e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
            return false;
        }
        true
    }) {
        match entry {
            Ok(e) if e.file_type().is_file() => files.push(e.into_path()),
            Err(err) => warnings.push(format!("Cannot read directory: {err}")),
            _ => {}
        }
    }

    files.sort();
    (files, warnings)
}

// ---------------------------------------------------------------------------
// Token extraction
// ---------------------------------------------------------------------------

struct TokenExtraction {
    tokens: BTreeMap<String, String>,
    token_order: Vec<String>,
    fonts: Vec<String>,
    spacing: Vec<String>,
    radii: Vec<String>,
    warnings: Vec<String>,
}

fn extract_tokens(code_files: &[PathBuf], root: &Path) -> TokenExtraction {
    let re_custom_prop = Regex::new(r"(--[a-zA-Z0-9-]+)\s*:\s*([^;{}]+)").unwrap();
    let re_font_family = Regex::new(r"(?i)font-family\s*:\s*([^;{}]+)").unwrap();
    let re_spacing = Regex::new(r"(?i)(?:margin|padding|gap)(?:-[a-z]+)?\s*:\s*([^;{}]+)").unwrap();
    let re_spacing_val = Regex::new(r"(\d+(?:\.\d+)?(?:px|rem))").unwrap();
    let re_radius = Regex::new(r"(?i)border-radius\s*:\s*([^;{}]+)").unwrap();

    let mut tokens: BTreeMap<String, String> = BTreeMap::new();
    let mut token_order = Vec::new();
    let mut fonts = Vec::new();
    let mut spacing = Vec::new();
    let mut radii = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for file in code_files {
        let text = match fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) => {
                let rel = file.strip_prefix(root).unwrap_or(file);
                warnings.push(format!("Cannot read file {}: {e}", rel.display()));
                continue;
            }
        };

        for cap in re_custom_prop.captures_iter(&text) {
            let name = cap[1].to_string();
            let value = cap[2].trim().to_string();
            if !tokens.contains_key(&name) {
                token_order.push(name.clone());
                tokens.insert(name, value);
            }
        }

        for cap in re_font_family.captures_iter(&text) {
            let val = cap[1].trim().replace(['"', '\''], "");
            if !fonts.contains(&val) {
                fonts.push(val);
            }
        }

        for cap in re_spacing.captures_iter(&text) {
            for v in re_spacing_val.captures_iter(&cap[1]) {
                let value = v[1].to_string();
                if !spacing.contains(&value) {
                    spacing.push(value);
                }
            }
        }

        for cap in re_radius.captures_iter(&text) {
            let value = cap[1].trim().to_string();
            if !radii.contains(&value) {
                radii.push(value);
            }
        }
    }

    TokenExtraction {
        tokens,
        token_order,
        fonts,
        spacing,
        radii,
        warnings,
    }
}

// ---------------------------------------------------------------------------
// Component detection
// ---------------------------------------------------------------------------

fn detect_components(
    code_files: &[PathBuf],
    root: &Path,
    target_kind: &str,
) -> (BTreeMap<String, Vec<String>>, Vec<String>, Vec<String>) {
    let re_component_path = Regex::new(r"(?i)components?|ui/|design-system|primitives").unwrap();
    let re_skip_name = Regex::new(r"(?i)^(index|types|utils|constants)$").unwrap();
    let re_variant1 =
        Regex::new(r#"(?:variant|appearance|kind|intent|tone)\s*[:?]?\s*["']([a-zA-Z-]+)["']"#)
            .unwrap();
    let re_variant2 = Regex::new(
        r#"["'](primary|secondary|tertiary|ghost|subtle|outline|destructive|danger|link)["']"#,
    )
    .unwrap();

    let mut components: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut component_order = Vec::new();
    let mut component_dirs = Vec::new();

    for file in code_files {
        let rel = if target_kind == "file" {
            let parent = file
                .parent()
                .and_then(|p| p.file_name())
                .unwrap_or_default();
            let name = file.file_name().unwrap_or_default();
            format!("{}/{}", parent.to_string_lossy(), name.to_string_lossy())
        } else {
            let r = file.strip_prefix(root).unwrap_or(file);
            r.to_string_lossy().replace('\\', "/")
        };

        if !re_component_path.is_match(&rel) {
            continue;
        }

        let dir = if target_kind == "file" {
            ".".to_string()
        } else {
            let d = file.parent().unwrap_or(root);
            d.strip_prefix(root)
                .unwrap_or(d)
                .to_string_lossy()
                .replace('\\', "/")
        };
        let dir = if dir.is_empty() { ".".to_string() } else { dir };
        if !component_dirs.contains(&dir) {
            component_dirs.push(dir);
        }

        let stem = file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if re_skip_name.is_match(&stem) {
            continue;
        }

        let text = match fs::read_to_string(file) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let mut variants = Vec::new();
        for cap in re_variant1.captures_iter(&text) {
            let value = cap[1].to_string();
            if !variants.contains(&value) {
                variants.push(value);
            }
        }
        for cap in re_variant2.captures_iter(&text) {
            let value = cap[1].to_string();
            if !variants.contains(&value) {
                variants.push(value);
            }
        }

        if !components.contains_key(&stem) {
            component_order.push(stem.clone());
        }
        components.insert(stem, variants);
    }

    (components, component_order, component_dirs)
}

// ---------------------------------------------------------------------------
// Config & doc detection
// ---------------------------------------------------------------------------

fn detect_configs_and_docs(
    root: &Path,
    target_kind: &str,
    target_file: Option<&Path>,
) -> (Vec<String>, Vec<String>) {
    let mut docs = Vec::new();
    let mut configs = Vec::new();

    if target_kind == "directory" {
        for doc in CONTEXT_DOCS {
            if root.join(doc).exists() {
                docs.push(doc.to_string());
            }
        }
        for cfg in CONFIG_FILES {
            if root.join(cfg).exists() {
                configs.push(cfg.to_string());
            }
        }
    } else if let Some(file) = target_file {
        let name = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let name_ref = name.as_str();
        if CONTEXT_DOCS.contains(&name_ref) {
            docs.push(name.clone());
        }
        if CONFIG_FILES.contains(&name_ref) {
            configs.push(name);
        }
    }

    docs.sort();
    configs.sort();
    (configs, docs)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the inventory scan on `target`.
///
/// `target` may be a directory or a single file. Returns a structured
/// [`InventoryReport`] with all detected design-system artifacts.
pub fn inventory(target: &Path) -> Result<InventoryReport, EngineError> {
    let meta = fs::symlink_metadata(target)
        .map_err(|e| EngineError::Usage(format!("Cannot inspect {}: {e}", target.display())))?;
    if meta.file_type().is_symlink() {
        return Err(EngineError::Usage(format!(
            "refusing symlink target: {}",
            target.display()
        )));
    }

    let (target_kind, scan_root, code_files) = if meta.is_dir() {
        let (all, _) = collect_files(target);
        let code: Vec<PathBuf> = all.into_iter().filter(|f| is_code_file(f)).collect();
        ("directory".to_string(), target.to_path_buf(), code)
    } else if meta.is_file() {
        let code = if is_code_file(target) {
            vec![target.to_path_buf()]
        } else {
            Vec::new()
        };
        let root = target.parent().unwrap_or(Path::new(".")).to_path_buf();
        ("file".to_string(), root, code)
    } else {
        return Err(EngineError::Usage(format!(
            "Cannot inventory {}: expected a directory or regular file.",
            target.display()
        )));
    };

    let extracted = extract_tokens(&code_files, &scan_root);
    let (components, component_order, component_dirs) =
        detect_components(&code_files, &scan_root, &target_kind);
    let (configs, docs) = detect_configs_and_docs(
        target,
        &target_kind,
        if target_kind == "file" {
            Some(target)
        } else {
            None
        },
    );

    let mut warnings = extracted.warnings;
    warnings.sort();
    warnings.dedup();

    Ok(InventoryReport {
        target: target.to_string_lossy().to_string(),
        target_kind,
        files: code_files.len(),
        tokens: extracted.tokens,
        fonts: extracted.fonts,
        spacing: extracted.spacing,
        radii: extracted.radii,
        components,
        configs,
        docs,
        warnings,
        token_order: extracted.token_order,
        component_order,
        component_dirs,
    })
}

/// Group tokens by semantic prefix (color, bg, surface, text, border, etc.).
pub fn group_tokens_by_prefix(tokens: &BTreeMap<String, String>) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, value) in tokens {
        for prefix in TOKEN_PREFIXES {
            if name.contains(prefix) {
                out.entry(prefix.to_string())
                    .or_default()
                    .push(format!("{name}: {value}"));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn inventory_empty_dir_returns_zero_files() {
        let dir = tempfile::tempdir().unwrap();
        let report = inventory(dir.path()).unwrap();
        assert_eq!(report.files, 0);
        assert_eq!(report.target_kind, "directory");
    }

    #[test]
    fn inventory_detects_css_tokens() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("theme.css"),
            ":root { --color-primary: #007bff; font-family: Inter; margin: 8px; border-radius: 4px; }",
        )
        .unwrap();

        let report = inventory(dir.path()).unwrap();
        assert_eq!(report.files, 1);
        assert!(report.tokens.contains_key("--color-primary"));
        assert!(report.fonts.contains(&"Inter".to_string()));
        assert!(report.spacing.contains(&"8px".to_string()));
        assert!(report.radii.contains(&"4px".to_string()));
    }

    #[test]
    fn inventory_detects_config_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("tailwind.config.js"), "").unwrap();
        fs::write(dir.path().join("DESIGN.md"), "").unwrap();

        let report = inventory(dir.path()).unwrap();
        assert!(report.configs.contains(&"tailwind.config.js".to_string()));
        assert!(report.docs.contains(&"DESIGN.md".to_string()));
    }

    #[test]
    fn inventory_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("app.css");
        fs::write(&file, ":root { --bg-surface: #fff; }").unwrap();

        let report = inventory(&file).unwrap();
        assert_eq!(report.target_kind, "file");
        assert_eq!(report.files, 1);
        assert!(report.tokens.contains_key("--bg-surface"));
    }

    #[test]
    fn group_tokens_by_prefix_works() {
        let mut tokens = BTreeMap::new();
        tokens.insert("--color-primary".to_string(), "#007bff".to_string());
        tokens.insert("--bg-surface".to_string(), "#fff".to_string());
        tokens.insert("--spacing-unit".to_string(), "4px".to_string());

        let grouped = group_tokens_by_prefix(&tokens);
        assert!(grouped.contains_key("color"));
        assert!(grouped.contains_key("bg"));
        assert!(!grouped.contains_key("spacing"));
    }

    #[test]
    fn inventory_nonexistent_target_returns_error() {
        let result = inventory(Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn inventory_rejects_symlink_root() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.css"), ":root {}").unwrap();
        let link = dir.path().join("linked");
        symlink(outside.path(), &link).unwrap();
        assert!(inventory(&link).is_err());
    }
}
