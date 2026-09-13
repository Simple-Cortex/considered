//! Validate the skill package for drift and correctness.
//!
//! Checks:
//! 1. Root SKILL.md frontmatter exists with required fields
//! 2. Command routing — every command in root router has a matching skill
//! 3. Reference paths — every referenced file path exists on disk
//! 4. Duplicate/missing commands — no duplicates, no missing skills
//! 5. Runtime manifest agreement — include/exclude consistent with disk
//! 6. Version agreement — package.json, README, CHANGELOG, manifest match
//! 7. Scaffold placeholders — no unresolved `<placeholder>` patterns
//! 8. Stale documentation — no docs describing shipped behavior as future work

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use regex::Regex;
use serde::Serialize;

/// Result of a single check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub messages: Vec<String>,
}

/// Aggregate result of all skill validation checks.
#[derive(Debug, Clone, Serialize)]
pub struct SkillValidationResult {
    pub checks: Vec<CheckResult>,
    pub passed: bool,
}

impl SkillValidationResult {
    pub fn exit_code(&self) -> i32 {
        if self.passed {
            0
        } else {
            1
        }
    }
}

/// Run all skill validation checks against the project root.
pub fn validate_skill(root: &Path) -> SkillValidationResult {
    let checks = vec![
        check_root_frontmatter(root),
        check_command_routing(root),
        check_reference_paths(root),
        check_duplicate_missing_commands(root),
        check_runtime_manifest(root),
        check_version_agreement(root),
        check_scaffold_placeholders(root),
        check_stale_documentation(root),
    ];

    let passed = checks.iter().all(|c| c.passed);

    SkillValidationResult { checks, passed }
}

// ---------------------------------------------------------------------------
// Check 1: Root SKILL.md frontmatter
// ---------------------------------------------------------------------------

fn check_root_frontmatter(root: &Path) -> CheckResult {
    let mut messages = Vec::new();
    let skill_path = root.join("SKILL.md");

    let content = match fs::read_to_string(&skill_path) {
        Ok(c) => c,
        Err(_) => {
            messages.push("Root SKILL.md not found".into());
            return CheckResult {
                name: "root-frontmatter".into(),
                passed: false,
                messages,
            };
        }
    };

    // Parse YAML frontmatter between --- markers
    let frontmatter = match extract_frontmatter(&content) {
        Some(fm) => fm,
        None => {
            messages.push("Root SKILL.md has no frontmatter (missing --- delimiters)".into());
            return CheckResult {
                name: "root-frontmatter".into(),
                passed: false,
                messages,
            };
        }
    };

    // Check required fields: name, description
    if !has_yaml_field(&frontmatter, "name") {
        messages.push("Root SKILL.md frontmatter missing required field: name".into());
    }
    if !has_yaml_field(&frontmatter, "description") {
        messages.push("Root SKILL.md frontmatter missing required field: description".into());
    }

    CheckResult {
        name: "root-frontmatter".into(),
        passed: messages.is_empty(),
        messages,
    }
}

// ---------------------------------------------------------------------------
// Check 2: Command routing
// ---------------------------------------------------------------------------

fn check_command_routing(root: &Path) -> CheckResult {
    let mut messages = Vec::new();
    let skill_path = root.join("SKILL.md");

    let content = match fs::read_to_string(&skill_path) {
        Ok(c) => c,
        Err(_) => {
            messages.push("Root SKILL.md not found".into());
            return CheckResult {
                name: "command-routing".into(),
                passed: false,
                messages,
            };
        }
    };

    let commands = extract_commands_from_table(&content);
    if commands.is_empty() {
        messages.push("No commands found in root SKILL.md command table".into());
        return CheckResult {
            name: "command-routing".into(),
            passed: false,
            messages,
        };
    }

    for cmd in &commands {
        let skill_file = root.join("skills").join(cmd).join("SKILL.md");
        if !skill_file.exists() {
            messages.push(format!(
                "Command '{}' listed in root router but skills/{}/SKILL.md not found",
                cmd, cmd
            ));
        }
    }

    CheckResult {
        name: "command-routing".into(),
        passed: messages.is_empty(),
        messages,
    }
}

// ---------------------------------------------------------------------------
// Check 3: Reference paths
// ---------------------------------------------------------------------------

fn check_reference_paths(root: &Path) -> CheckResult {
    let mut messages = Vec::new();

    // Collect all SKILL.md files to scan
    let mut skill_files = vec![root.join("SKILL.md")];
    if let Ok(entries) = fs::read_dir(root.join("skills")) {
        for entry in entries.flatten() {
            let skill_md = entry.path().join("SKILL.md");
            if skill_md.exists() {
                skill_files.push(skill_md);
            }
        }
    }

    let path_re = Regex::new(r"`((?:references|assets|skills|scripts|bin)/[^`]+)`").unwrap();

    for skill_file in &skill_files {
        let content = match fs::read_to_string(skill_file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let base_dir = skill_file.parent().unwrap_or(root);

        for cap in path_re.captures_iter(&content) {
            let raw_path = &cap[1];
            // Strip section references like §1, §2 etc from the path
            let clean_path = raw_path.split('§').next().unwrap_or(raw_path).trim();
            // Skip paths with wildcards or variables
            if clean_path.contains('*') || clean_path.contains('<') || clean_path.contains('|') {
                continue;
            }
            // Resolve relative to the skill file's directory
            let resolved = if clean_path.starts_with("../../") || clean_path.starts_with("../") {
                base_dir.join(clean_path)
            } else {
                root.join(clean_path)
            };

            // For paths with wildcards in the original (already filtered above),
            // check if the parent directory exists for glob patterns
            if !resolved.exists() {
                let rel = resolved.strip_prefix(root).unwrap_or(&resolved);
                messages.push(format!(
                    "Referenced path does not exist: {} (from {})",
                    rel.display(),
                    skill_file
                        .strip_prefix(root)
                        .unwrap_or(skill_file)
                        .display()
                ));
            }
        }
    }

    messages.sort();
    messages.dedup();

    CheckResult {
        name: "reference-paths".into(),
        passed: messages.is_empty(),
        messages,
    }
}

// ---------------------------------------------------------------------------
// Check 4: Duplicate/missing commands
// ---------------------------------------------------------------------------

fn check_duplicate_missing_commands(root: &Path) -> CheckResult {
    let mut messages = Vec::new();
    let skill_path = root.join("SKILL.md");

    let content = match fs::read_to_string(&skill_path) {
        Ok(c) => c,
        Err(_) => {
            messages.push("Root SKILL.md not found".into());
            return CheckResult {
                name: "duplicate-missing-commands".into(),
                passed: false,
                messages,
            };
        }
    };

    let commands = extract_commands_from_table(&content);

    // Check for duplicate command names in the router
    let mut seen = BTreeSet::new();
    for cmd in &commands {
        if !seen.insert(cmd.clone()) {
            messages.push(format!("Duplicate command in root router: '{}'", cmd));
        }
    }

    // Check for skill directories that don't correspond to any command
    let skills_dir = root.join("skills");
    if skills_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    let skill_md = entry.path().join("SKILL.md");
                    if !skill_md.exists() {
                        messages.push(format!(
                            "Skill directory 'skills/{}' has no SKILL.md",
                            dir_name
                        ));
                    }
                }
            }
        }
    }

    // Check that every command has a matching skill directory
    for cmd in &commands {
        let skill_dir = skills_dir.join(cmd);
        if !skill_dir.is_dir() {
            messages.push(format!(
                "Command '{}' has no matching skills/{}/ directory",
                cmd, cmd
            ));
        }
    }

    CheckResult {
        name: "duplicate-missing-commands".into(),
        passed: messages.is_empty(),
        messages,
    }
}

// ---------------------------------------------------------------------------
// Check 5: Runtime manifest agreement
// ---------------------------------------------------------------------------

fn check_runtime_manifest(root: &Path) -> CheckResult {
    let mut messages = Vec::new();
    let manifest_path = root.join("runtime-manifest.json");

    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => {
            messages.push("runtime-manifest.json not found".into());
            return CheckResult {
                name: "runtime-manifest".into(),
                passed: false,
                messages,
            };
        }
    };

    let manifest: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            messages.push(format!("runtime-manifest.json is not valid JSON: {}", e));
            return CheckResult {
                name: "runtime-manifest".into(),
                passed: false,
                messages,
            };
        }
    };

    // Check schema version
    let schema = manifest
        .get("$schema")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !schema.starts_with("considered-runtime-manifest/") {
        messages.push("runtime-manifest.json has unexpected $schema value".into());
    }

    // Check that key files are included
    let include_strs: Vec<&str> = manifest
        .get("include")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let required_includes = ["SKILL.md", "runtime-manifest.json"];
    for req in &required_includes {
        if !include_strs.iter().any(|s| s == req) {
            messages.push(format!(
                "runtime-manifest.json include list missing required entry: {}",
                req
            ));
        }
    }

    // Check that skills/*/SKILL.md glob is present
    if !include_strs.iter().any(|s| *s == "skills/*/SKILL.md") {
        messages.push("runtime-manifest.json include list missing skills/*/SKILL.md".into());
    }

    CheckResult {
        name: "runtime-manifest".into(),
        passed: messages.is_empty(),
        messages,
    }
}

// ---------------------------------------------------------------------------
// Check 6: Version agreement
// ---------------------------------------------------------------------------

fn check_version_agreement(root: &Path) -> CheckResult {
    let mut messages = Vec::new();
    let mut versions: BTreeMap<String, String> = BTreeMap::new();

    // 1. package.json version
    if let Ok(content) = fs::read_to_string(root.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(v) = pkg.get("version").and_then(|v| v.as_str()) {
                versions.insert("package.json".into(), v.to_string());
            }
        }
    }

    // 2. README.md status section
    if let Ok(content) = fs::read_to_string(root.join("README.md")) {
        if let Some(v) = extract_readme_version(&content) {
            versions.insert("README.md".into(), v);
        }
    }

    // 3. CHANGELOG.md latest version
    if let Ok(content) = fs::read_to_string(root.join("CHANGELOG.md")) {
        if let Some(v) = extract_changelog_version(&content) {
            versions.insert("CHANGELOG.md".into(), v);
        }
    }

    // 4. runtime-manifest.json version
    if let Ok(content) = fs::read_to_string(root.join("runtime-manifest.json")) {
        if let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(v) = manifest.get("version").and_then(|v| v.as_str()) {
                versions.insert("runtime-manifest.json".into(), v.to_string());
            }
        }
    }

    // Check all versions match
    let unique_versions: BTreeSet<&str> = versions.values().map(|s| s.as_str()).collect();
    if unique_versions.len() > 1 {
        let detail: Vec<String> = versions
            .iter()
            .map(|(source, ver)| format!("{}: {}", source, ver))
            .collect();
        messages.push(format!(
            "Version mismatch across sources: {}",
            detail.join(", ")
        ));
    }

    if versions.is_empty() {
        messages.push("Could not find version in any source".into());
    }

    CheckResult {
        name: "version-agreement".into(),
        passed: messages.is_empty(),
        messages,
    }
}

// ---------------------------------------------------------------------------
// Check 7: Scaffold placeholders
// ---------------------------------------------------------------------------

fn check_scaffold_placeholders(root: &Path) -> CheckResult {
    let mut messages = Vec::new();

    // Scan shipped files for unresolved <placeholder> patterns
    let shipped_dirs = ["references", "assets", "skills", "scripts", "bin"];
    let placeholder_re = Regex::new(r"<[A-Z][A-Z0-9_-]+>").unwrap();

    // Also check root SKILL.md
    check_file_for_placeholders(&root.join("SKILL.md"), root, &placeholder_re, &mut messages);

    for dir_name in &shipped_dirs {
        let dir = root.join(dir_name);
        if dir.is_dir() {
            scan_dir_for_placeholders(&dir, root, &placeholder_re, &mut messages);
        }
    }

    messages.sort();

    CheckResult {
        name: "scaffold-placeholders".into(),
        passed: messages.is_empty(),
        messages,
    }
}

fn scan_dir_for_placeholders(dir: &Path, root: &Path, re: &Regex, messages: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_for_placeholders(&path, root, re, messages);
            continue;
        }
        check_file_for_placeholders(&path, root, re, messages);
    }
}

fn check_file_for_placeholders(path: &Path, root: &Path, re: &Regex, messages: &mut Vec<String>) {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Only scan text files
    if ![
        "md", "mjs", "json", "toml", "txt", "yaml", "yml", "ts", "js",
    ]
    .contains(&ext)
    {
        return;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for line in content.lines() {
        // Skip lines that are clearly code examples or HTML tags
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("<!--") || trimmed.starts_with("-->") {
            continue;
        }

        for mat in re.find_iter(line) {
            let matched = mat.as_str();
            // Filter out common false positives: XML/HTML tags, JSON schema types, etc.
            let false_positives = [
                "<br>",
                "<hr>",
                "<code>",
                "<pre>",
                "<em>",
                "<strong>",
                "<!--",
                "-->",
                "<!DOCTYPE",
            ];
            if false_positives.iter().any(|fp| matched.starts_with(fp)) {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(path);
            messages.push(format!(
                "Unresolved placeholder '{}' in {}",
                matched,
                rel.display()
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Check 8: Stale documentation
// ---------------------------------------------------------------------------

fn check_stale_documentation(root: &Path) -> CheckResult {
    let mut messages = Vec::new();

    // Scan reference and skill docs for "future work" language about shipped features
    let stale_patterns = [
        (r"(?i)future\s+work", "future work"),
        (r"(?i)not\s+yet\s+implemented", "not yet implemented"),
        (r"(?i)coming\s+soon", "coming soon"),
        (
            r"(?i)planned\s+(?:feature|addition|release)",
            "planned feature/addition/release",
        ),
        (
            r"(?i)to\s+be\s+(?:implemented|added|released)",
            "to be implemented/added/released",
        ),
    ];

    let scan_dirs = ["references", "skills"];
    for dir_name in &scan_dirs {
        let dir = root.join(dir_name);
        if dir.is_dir() {
            scan_dir_for_stale_docs(&dir, root, &stale_patterns, &mut messages);
        }
    }

    messages.sort();

    CheckResult {
        name: "stale-documentation".into(),
        passed: messages.is_empty(),
        messages,
    }
}

fn scan_dir_for_stale_docs(
    dir: &Path,
    root: &Path,
    patterns: &[(&str, &str)],
    messages: &mut Vec<String>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir_for_stale_docs(&path, root, patterns, messages);
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (pattern, label) in patterns {
            let re = match Regex::new(pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if let Some(mat) = re.find(&content) {
                // Find line number
                let line_num = content[..mat.start()].matches('\n').count() + 1;
                let rel = path.strip_prefix(root).unwrap_or(&path);
                messages.push(format!(
                    "Stale documentation: '{}' found in {} (line {})",
                    label,
                    rel.display(),
                    line_num
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract YAML frontmatter content between --- delimiters.
fn extract_frontmatter(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    after_first
        .find("\n---")
        .map(|end| after_first[..end].to_string())
}

/// Check if a YAML frontmatter block contains a top-level field.
fn has_yaml_field(frontmatter: &str, field: &str) -> bool {
    let pattern = format!("^{}:", field);
    let re = Regex::new(&pattern).unwrap();
    frontmatter.lines().any(|line| re.is_match(line.trim()))
}

/// Extract command names from the Commands table in root SKILL.md.
/// Looks for table rows with backtick-quoted command names.
fn extract_commands_from_table(content: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let cmd_re = Regex::new(r"`(\w+)`").unwrap();

    let mut in_commands_section = false;
    let mut in_table = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect "## Commands" section
        if trimmed.starts_with("## Commands") {
            in_commands_section = true;
            continue;
        }

        // Stop at next ## section
        if in_commands_section && trimmed.starts_with("## ") && !trimmed.starts_with("## Commands")
        {
            break;
        }

        if !in_commands_section {
            continue;
        }

        // Detect table header separator
        if trimmed.starts_with("|---") || trimmed.starts_with("| ---") {
            in_table = true;
            continue;
        }

        // Parse table rows
        if in_table && trimmed.starts_with('|') {
            // Extract the first backtick-quoted word from the row
            if let Some(cap) = cmd_re.captures(trimmed) {
                let cmd = cap[1].to_string();
                // Skip header words
                if cmd != "Command" && cmd != "Read" {
                    commands.push(cmd);
                }
            }
        }
    }

    commands
}

/// Extract version from README.md Status section.
fn extract_readme_version(content: &str) -> Option<String> {
    let re = Regex::new(r"\*\*v(\d+\.\d+\.\d+)\.\*\*").unwrap();
    re.captures(content).map(|cap| cap[1].to_string())
}

/// Extract the latest version from CHANGELOG.md (first versioned heading).
fn extract_changelog_version(content: &str) -> Option<String> {
    let re = Regex::new(r"## \[(\d+\.\d+\.\d+)\]").unwrap();
    re.captures(content).map(|cap| cap[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_frontmatter_basic() {
        let content = "---\nname: test\ndescription: hello\n---\n# Title\n";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("name: test"));
        assert!(fm.contains("description: hello"));
    }

    #[test]
    fn extract_frontmatter_missing() {
        let content = "# Title\nNo frontmatter here.\n";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn has_yaml_field_present() {
        let fm = "name: test\ndescription: hello\n";
        assert!(has_yaml_field(fm, "name"));
        assert!(has_yaml_field(fm, "description"));
    }

    #[test]
    fn has_yaml_field_absent() {
        let fm = "name: test\n";
        assert!(!has_yaml_field(fm, "description"));
    }

    #[test]
    fn extract_commands_from_skill_md() {
        let content = r#"
## Commands

| Command | Required outcome | Read next |
| --- | --- | --- |
| `init` | Capture context | `references/workflow.md` |
| `frame` | State decision | `references/workflow.md` |
| `compose` | Build surface | `references/workflow.md` |

## Other section
"#;
        let commands = extract_commands_from_table(content);
        assert_eq!(commands, vec!["init", "frame", "compose"]);
    }

    #[test]
    fn extract_readme_version_works() {
        let content = "## Status\n\n**v0.6.0.** The corpus is at 190 rules.\n";
        assert_eq!(extract_readme_version(content), Some("0.6.0".into()));
    }

    #[test]
    fn extract_changelog_version_works() {
        let content = "# Changelog\n\n## [Unreleased]\n\n## [0.6.0] - 2026-08-04\n";
        assert_eq!(extract_changelog_version(content), Some("0.6.0".into()));
    }

    #[test]
    fn validate_skill_passes_on_project_root() {
        // Find the project root by looking for SKILL.md + assets/rules/
        let mut dir = std::env::current_dir().unwrap();
        // Walk up to find the project root
        loop {
            if dir.join("SKILL.md").exists() && dir.join("assets").join("rules").exists() {
                break;
            }
            if !dir.pop() {
                // Can't find root, skip test
                return;
            }
        }

        let result = validate_skill(&dir);
        assert!(
            result.passed,
            "Validation failed on project root: {:?}",
            result
                .checks
                .iter()
                .filter(|c| !c.passed)
                .map(|c| format!("{}: {:?}", c.name, c.messages))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn check_result_serializes() {
        let result = CheckResult {
            name: "test".into(),
            passed: true,
            messages: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"passed\":true"));
    }

    #[test]
    fn skill_validation_result_exit_code() {
        let pass = SkillValidationResult {
            checks: vec![CheckResult {
                name: "a".into(),
                passed: true,
                messages: vec![],
            }],
            passed: true,
        };
        assert_eq!(pass.exit_code(), 0);

        let fail = SkillValidationResult {
            checks: vec![CheckResult {
                name: "a".into(),
                passed: false,
                messages: vec!["err".into()],
            }],
            passed: false,
        };
        assert_eq!(fail.exit_code(), 1);
    }
}
