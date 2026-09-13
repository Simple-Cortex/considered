//! Experimental context manifest: bounded route manifest for agent consumption.
//!
//! Lists all SKILL.md files and their commands, references, and templates.
//! Output is JSON for agent consumption.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// A bounded route manifest for agent consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManifest {
    /// Engine version.
    pub engine_version: String,
    /// All SKILL.md files found and their commands.
    pub skills: Vec<SkillEntry>,
    /// All reference files found.
    pub references: Vec<ReferenceEntry>,
    /// All template files found.
    pub templates: Vec<TemplateEntry>,
    /// Available commands summary.
    pub commands: BTreeMap<String, CommandEntry>,
}

/// A SKILL.md file and its declared commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub path: String,
    pub commands: Vec<String>,
}

/// A reference file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceEntry {
    pub path: String,
    pub name: String,
}

/// A template file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEntry {
    pub path: String,
    pub name: String,
}

/// A command entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEntry {
    pub name: String,
    pub description: String,
}

/// Build the context manifest from the project root.
pub fn build_manifest(root: &Path) -> ContextManifest {
    let engine_version = env!("CARGO_PKG_VERSION").to_string();

    let skills = find_skill_files(root);
    let references = find_reference_files(root);
    let templates = find_template_files(root);
    let commands = build_command_index();

    ContextManifest {
        engine_version,
        skills,
        references,
        templates,
        commands,
    }
}

fn find_skill_files(root: &Path) -> Vec<SkillEntry> {
    let mut entries = Vec::new();

    // Check root SKILL.md.
    let root_skill = root.join("SKILL.md");
    if root_skill.exists() {
        let commands = extract_commands_from_skill(&root_skill);
        entries.push(SkillEntry {
            path: "SKILL.md".to_string(),
            commands,
        });
    }

    // Check .considered/ for surface-level SKILL.md files.
    let considered_dir = root.join(".considered");
    if considered_dir.is_dir() {
        if let Ok(dir_entries) = fs::read_dir(&considered_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_path = path.join("SKILL.md");
                    if skill_path.exists() {
                        let rel = skill_path
                            .strip_prefix(root)
                            .unwrap_or(&skill_path)
                            .to_string_lossy()
                            .to_string();
                        let commands = extract_commands_from_skill(&skill_path);
                        entries.push(SkillEntry {
                            path: rel,
                            commands,
                        });
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

fn extract_commands_from_skill(path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut commands = Vec::new();
    // Look for lines starting with `considered <command>` or `## <command>`.
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("considered ") {
            if let Some(cmd) = rest.split_whitespace().next() {
                if !commands.contains(&cmd.to_string()) && !cmd.starts_with('#') && cmd.len() < 30 {
                    commands.push(cmd.to_string());
                }
            }
        }
    }
    commands.sort();
    commands
}

fn find_reference_files(root: &Path) -> Vec<ReferenceEntry> {
    let mut entries = Vec::new();

    // Check assets/ for reference files.
    let assets_dir = root.join("assets");
    if assets_dir.is_dir() {
        collect_files(&assets_dir, root, &mut entries, &["md", "json"]);
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

fn find_template_files(root: &Path) -> Vec<TemplateEntry> {
    let mut entries = Vec::new();

    let templates_dir = root.join("assets").join("templates");
    if templates_dir.is_dir() {
        if let Ok(dir_entries) = fs::read_dir(&templates_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let rel = path
                        .strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    let name = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    entries.push(TemplateEntry { path: rel, name });
                }
            }
        }
    }

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

fn collect_files(dir: &Path, root: &Path, entries: &mut Vec<ReferenceEntry>, extensions: &[&str]) {
    if let Ok(dir_entries) = fs::read_dir(dir) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if extensions.contains(&ext) {
                    let rel = path
                        .strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    let name = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    entries.push(ReferenceEntry { path: rel, name });
                }
            } else if path.is_dir() {
                collect_files(&path, root, entries, extensions);
            }
        }
    }
}

fn build_command_index() -> BTreeMap<String, CommandEntry> {
    let mut commands = BTreeMap::new();
    let defs = [
        ("contract", "Validate a CONSIDERED-CONTRACT v1 block"),
        ("lint", "Heuristic source audit"),
        (
            "gate",
            "Evaluate the ship gate from checker reports and review",
        ),
        ("inventory", "Report existing design system in a project"),
        ("roll", "Assign a structure and direction for a build"),
        (
            "validate",
            "Validate rule manifests, decks, and reference coverage",
        ),
        ("status", "Report current surface state"),
        ("verify", "Verify artifact integrity and consistency"),
        ("context", "Emit bounded route manifest (experimental)"),
    ];
    for (name, desc) in defs {
        commands.insert(
            name.to_string(),
            CommandEntry {
                name: name.to_string(),
                description: desc.to_string(),
            },
        );
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_index_has_all_commands() {
        let idx = build_command_index();
        assert!(idx.contains_key("contract"));
        assert!(idx.contains_key("lint"));
        assert!(idx.contains_key("gate"));
        assert!(idx.contains_key("inventory"));
        assert!(idx.contains_key("roll"));
        assert!(idx.contains_key("validate"));
        assert!(idx.contains_key("status"));
        assert!(idx.contains_key("verify"));
        assert!(idx.contains_key("context"));
    }

    #[test]
    fn manifest_serializes_to_json() {
        let manifest = ContextManifest {
            engine_version: "0.6.0".into(),
            skills: vec![],
            references: vec![],
            templates: vec![],
            commands: build_command_index(),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("engine_version"));
        assert!(json.contains("commands"));
    }
}
