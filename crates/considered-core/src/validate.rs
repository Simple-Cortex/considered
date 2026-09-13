//! Validate rule manifests, gate policy, decks, and reference coverage.
//! Ports `scripts/validate-assets.mjs`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::rules::{load_gate_policy, load_rule_catalog, RuleCatalog};

const MODES: &[&str] = &["persuade", "operate", "analyze", "read", "experience"];
const STRUCTURE_TIERS: &[&str] = &["organizing-axis", "depth-strategy", "framing"];
const TAUGHT_IN: &[&str] = &["references", "assets/templates", "docs", "evals"];

/// Result of asset validation.
pub struct ValidateResult {
    pub failures: Vec<String>,
    pub rule_count: usize,
    pub min_score: f64,
    pub max_score: f64,
}

impl ValidateResult {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Run all asset validations.
pub fn validate_assets(root: &Path) -> ValidateResult {
    let mut failures = Vec::new();

    // 1. Rule manifests
    let catalog = match load_rule_catalog(root) {
        Ok(c) => Some(c),
        Err(e) => {
            failures.push(format!("Rule manifests: {e}"));
            None
        }
    };

    if let Some(ref cat) = catalog {
        for (kind, data) in &cat.manifests {
            if data.rule_count != data.rules.len() {
                failures.push(format!("{kind}: ruleCount does not match rules length."));
            }
            for rule in &data.rules {
                if !["S1", "S2", "S3", "S4"].contains(&rule.severity.as_str()) {
                    failures.push(format!("{kind}: {} has invalid severity.", rule.id));
                }
            }
        }
    }

    // 2. Gate policy
    let gate_policy = match load_gate_policy(root) {
        Ok(p) => Some(p),
        Err(e) => {
            failures.push(format!("Gate policy: {e}"));
            None
        }
    };

    if let Some(ref policy) = gate_policy {
        let review = &policy.review;
        if review.minimum_score > review.max_score {
            failures.push("gate: minimumScore exceeds maxScore.".into());
        }
        if review.dimension_floor < 0.0 {
            failures.push("gate: dimensionFloor must not be negative.".into());
        }
        let mut seen = HashSet::new();
        for dim in &review.critical_dimensions {
            if !seen.insert(dim.clone()) {
                failures.push(format!("gate: duplicate critical dimension {dim}."));
            }
        }
    }

    // 3. Decks
    for name in &["structures", "directions"] {
        validate_deck(root, name, &mut failures);
    }

    // 4. Reference coverage
    if let Some(ref cat) = catalog {
        validate_reference_coverage(root, cat, &mut failures);
    }

    let (rule_count, min_score, max_score) = match (&catalog, &gate_policy) {
        (Some(c), Some(p)) => (c.by_id_len(), p.review.minimum_score, p.review.max_score),
        _ => (0, 0.0, 0.0),
    };

    ValidateResult {
        failures,
        rule_count,
        min_score,
        max_score,
    }
}

fn validate_deck(root: &Path, name: &str, failures: &mut Vec<String>) {
    let path = root
        .join("assets")
        .join("decks")
        .join(format!("{name}.json"));
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            failures.push(format!("{name}: {e}"));
            return;
        }
    };
    let deck: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            failures.push(format!("{name}: {e}"));
            return;
        }
    };

    if deck.get("kind").and_then(|v| v.as_str()) != Some(name) {
        failures.push(format!("{name}: invalid deck kind."));
        return;
    }
    let entries = match deck.get("entries").and_then(|v| v.as_array()) {
        Some(e) => e,
        None => {
            failures.push(format!("{name}: invalid entries."));
            return;
        }
    };

    let mut ids = HashSet::new();
    for entry in entries {
        let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() || !ids.insert(id.to_string()) {
            failures.push(format!("{name}: duplicate or empty id \"{id}\"."));
        }
        let rating = entry.get("rating").and_then(|v| v.as_i64()).unwrap_or(0);
        if !(1..=3).contains(&rating) {
            failures.push(format!("{name}: {id} rating must be 1 to 3."));
        }
        if let Some(modes) = entry.get("modes").and_then(|v| v.as_array()) {
            for mode in modes {
                if let Some(m) = mode.as_str() {
                    if !MODES.contains(&m) {
                        failures.push(format!("{name}: {id} has invalid modes."));
                    }
                }
            }
        }
        if name == "structures" {
            let tier = entry.get("tier").and_then(|v| v.as_str()).unwrap_or("");
            if !STRUCTURE_TIERS.contains(&tier) {
                failures.push(format!("{name}: {id} has invalid tier."));
            }
        }
        let laws = entry.get("laws").and_then(|v| v.as_object());
        if entry.get("thesis").and_then(|v| v.as_str()).is_none()
            || laws.is_none()
            || laws.map(|l| l.len()).unwrap_or(0) < 4
        {
            failures.push(format!("{name}: {id} is missing thesis or laws."));
        }
    }
}

fn validate_reference_coverage(root: &Path, catalog: &RuleCatalog, failures: &mut Vec<String>) {
    let rule_id_re = Regex::new(r"\b([A-Z][A-Z0-9]{1,8})-(\d{2}[a-z]?)\b").unwrap();
    let mut citations: HashMap<String, HashSet<String>> = HashMap::new();

    for dir in TAUGHT_IN {
        let dir_path = root.join(dir);
        if !dir_path.exists() {
            continue;
        }
        collect_citations(&dir_path, &rule_id_re, &mut citations);
    }

    // Rules defined but taught in no reference
    let mut untaught: Vec<String> = catalog
        .by_id_keys()
        .filter(|id| !citations.contains_key(*id))
        .map(|s| s.to_string())
        .collect();
    untaught.sort();
    if !untaught.is_empty() {
        failures.push(format!(
            "Rules defined but taught in no reference: {}.",
            untaught.join(", ")
        ));
    }

    // Cited but not defined (excluding common non-rule patterns)
    let non_rule_re = Regex::new(r"^[AQBFDEGP]-?\d").unwrap();
    let mut dangling: Vec<String> = citations
        .keys()
        .filter(|id| !catalog.has_rule(id) && !non_rule_re.is_match(id))
        .map(|s| s.to_string())
        .collect();
    dangling.sort();
    if !dangling.is_empty() {
        failures.push(format!(
            "Rule ids cited but defined in no manifest: {}.",
            dangling.join(", ")
        ));
    }
}

fn collect_citations(dir: &Path, re: &Regex, citations: &mut HashMap<String, HashSet<String>>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_citations(&path, re, citations);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") && !name.ends_with(".json") {
            continue;
        }
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for cap in re.captures_iter(&text) {
            let id = format!("{}-{}", &cap[1], &cap[2]);
            citations
                .entry(id)
                .or_default()
                .insert(path.to_string_lossy().to_string());
        }
    }
}
