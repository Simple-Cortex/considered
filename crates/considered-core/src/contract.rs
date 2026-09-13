//! Contract block parser and validator.
//!
//! Ports `scripts/lint-contract.mjs` — validates the machine-readable facts
//! in a CONSIDERED-CONTRACT v1 block. This checker proves internal consistency
//! only; visual rank and state behavior belong to the independent review.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::finding::Finding;
use crate::rules::{self, RuleCatalog};

/// Required fields in a contract block.
const REQUIRED: &[&str] = &[
    "THESIS",
    "DECISION",
    "MODE",
    "QUESTIONS",
    "ZONES",
    "HIERARCHY",
    "ACTIONS",
    "DELETED",
    "ROLL",
];

/// Valid modes.
const MODES: &[&str] = &["persuade", "operate", "analyze", "read", "experience"];

/// Banned zone heading names (lowercased).
const BANNED_HEADINGS: &[&str] = &[
    "overview",
    "metrics",
    "kpis",
    "charts",
    "tables",
    "other",
    "analytics",
    "data",
    "stats",
    "misc",
];

/// A parsed question row.
#[derive(Debug, Clone)]
pub struct Question {
    pub id: String,
    pub tier: String,
    pub text: String,
    pub zone: String,
}

/// A parsed zone row.
#[derive(Debug, Clone)]
pub struct Zone {
    pub id: String,
    pub heading: String,
    pub questions: Vec<String>,
    pub elements: Vec<String>,
}

/// A parsed action row.
#[derive(Debug, Clone)]
pub struct Action {
    pub scope: u32,
    pub tier: String,
    pub label: String,
    pub risk: String,
    pub qualifier: String,
}

/// Parsed hierarchy tiers and element-to-tier mapping.
#[derive(Debug, Clone)]
pub struct Hierarchy {
    pub tiers: HashMap<String, Vec<String>>,
    /// Element-to-tier mapping, stored as a Vec to preserve insertion order
    /// (matching JS Map iteration order for deterministic output).
    pub elements: Vec<(String, String)>,
    pub errors: Vec<String>,
}

/// Contract details for output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractDetails {
    pub mode: Option<String>,
    pub questions: usize,
    pub zones: usize,
}

/// Strip comment prefixes from a line, matching the JS stripComment function.
fn strip_comment(line: &str) -> String {
    let re_prefix = Regex::new(r"^\s*(?://+|/\*+|\*+/?\s?|#+|<!--|-->)\s?").unwrap();
    let re_suffix = Regex::new(r"\s*(\*/|-->).*\s*$").unwrap();
    let s = re_prefix.replace(line, "").to_string();
    let s = re_suffix.replace(&s, "").to_string();
    // JS uses trimEnd() — Rust's trim_end() matches
    s.trim_end().to_string()
}

/// Extract the CONSIDERED-CONTRACT v1 block from source text.
pub fn extract_contract(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|line| {
        Regex::new(r"(?i)CONSIDERED-CONTRACT\s+v1")
            .unwrap()
            .is_match(line)
    })?;

    let mut body = Vec::new();
    let mut saw_roll = false;
    let re_end = Regex::new(r"(?i)CONSIDERED-CONTRACT-END").unwrap();
    let re_roll = Regex::new(r"(?i)^ROLL\s*:").unwrap();
    let re_break =
        Regex::new(r"(?i)^\s*(?:import|export|const|let|var|function|class|@|<[A-Za-z])").unwrap();

    for &line in lines.iter().skip(start + 1) {
        let raw = strip_comment(line);
        if re_end.is_match(&raw) {
            break;
        }
        if saw_roll && (raw.trim().is_empty() || re_break.is_match(&raw)) {
            break;
        }
        if re_roll.is_match(&raw) {
            saw_roll = true;
        }
        body.push(raw);
    }

    Some(body.join("\n"))
}

/// Parse FIELD: value lines from the contract block.
/// Returns (fields map, syntax errors).
pub fn parse_fields(block: &str) -> (HashMap<String, Vec<String>>, Vec<String>) {
    let mut fields: HashMap<String, Vec<String>> = HashMap::new();
    let mut syntax = Vec::new();
    let mut current: Option<String> = None;
    let re_field = Regex::new(r"^\s*([A-Z][A-Z-]{2,})\s*:\s*(.*)$").unwrap();

    for raw in block.lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if let Some(caps) = re_field.captures(line) {
            let name = caps[1].to_uppercase();
            if !REQUIRED.contains(&name.as_str()) {
                syntax.push(format!("Unknown field {name}."));
                current = None;
                continue;
            }
            let value = caps[2].trim().to_string();
            if fields.contains_key(&name) {
                syntax.push(format!("Duplicate field {name}."));
            }
            // Preserve existing values if duplicate, otherwise start fresh
            let mut values = fields.remove(&name).unwrap_or_default();
            if !value.is_empty() {
                values.push(value);
            }
            fields.insert(name.clone(), values);
            current = Some(name);
        } else if let Some(ref cur) = current {
            fields.get_mut(cur).unwrap().push(line.trim().to_string());
        } else {
            syntax.push(format!("Unattached line: {}", line.trim()));
        }
    }

    for key in REQUIRED {
        if let Some(vals) = fields.get(*key) {
            if vals.join(" ").trim().is_empty() {
                syntax.push(format!("Missing field {key}."));
            }
        } else {
            syntax.push(format!("Missing field {key}."));
        }
    }

    (fields, syntax)
}

/// Parse question rows from QUESTIONS field lines.
pub fn parse_questions(lines: &[String]) -> (Vec<Question>, Vec<String>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    let re = Regex::new(r"(?i)^Q(\d+)\s+\[(P[0-4])\]\s+(.+?)\s*->\s*([A-Za-z][\w-]*)$").unwrap();

    for line in lines {
        if let Some(caps) = re.captures(line.trim()) {
            result.push(Question {
                id: format!("Q{}", &caps[1]),
                tier: caps[2].to_uppercase(),
                text: caps[3].trim().to_string(),
                zone: caps[4].to_string(),
            });
        } else {
            errors.push(format!("Invalid question row: {line}"));
        }
    }

    let mut ids = HashSet::new();
    for q in &result {
        if !ids.insert(&q.id) {
            errors.push(format!("Duplicate question id {}.", q.id));
        }
    }

    (result, errors)
}

/// Parse zone rows from ZONES field lines.
pub fn parse_zones(lines: &[String]) -> (Vec<Zone>, Vec<String>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    let re = Regex::new(r#"^([A-Za-z][\w-]*)\s+"([^"]+)"\s*::\s*([^:]+?)\s*::\s*(.+)$"#).unwrap();

    for line in lines {
        if let Some(caps) = re.captures(line.trim()) {
            let questions: Vec<String> = caps[3]
                .split(',')
                .map(|v| v.trim().to_uppercase())
                .filter(|v| !v.is_empty())
                .collect();
            let elements: Vec<String> = caps[4]
                .split(',')
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect();
            result.push(Zone {
                id: caps[1].to_string(),
                heading: caps[2].trim().to_string(),
                questions,
                elements,
            });
        } else {
            errors.push(format!("Invalid zone row: {line}"));
        }
    }

    let mut ids = HashSet::new();
    for z in &result {
        if !ids.insert(&z.id) {
            errors.push(format!("Duplicate zone id {}.", z.id));
        }
    }

    (result, errors)
}

/// Parse hierarchy rows from HIERARCHY field lines.
pub fn parse_hierarchy(lines: &[String]) -> Hierarchy {
    let mut tiers: HashMap<String, Vec<String>> = HashMap::new();
    for t in &["P0", "P1", "P2", "P3", "P4"] {
        tiers.insert(t.to_string(), Vec::new());
    }
    let mut errors = Vec::new();
    let re = Regex::new(r"(?i)^(P[0-4])\s+(.+)$").unwrap();
    let re_chrome = Regex::new(r"(?i)^chrome\s*:").unwrap();

    for line in lines {
        if let Some(caps) = re.captures(line.trim()) {
            let tier = caps[1].to_uppercase();
            let raw = caps[2].trim().to_string();

            let parts: Vec<String> = if tier == "P4" && re_chrome.is_match(&raw) {
                re_chrome
                    .replace(&raw, "")
                    .split([',', ';'])
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect()
            } else {
                raw.split([',', ';'])
                    .map(|v| {
                        // Strip " - reason" suffix
                        let s = v.trim();
                        if let Some(idx) = s.find(" - ") {
                            s[..idx].trim().to_string()
                        } else {
                            s.to_string()
                        }
                    })
                    .filter(|v| !v.is_empty())
                    .collect()
            };

            if parts.is_empty() {
                errors.push(format!("No elements in {tier}."));
            }
            tiers.entry(tier.clone()).or_default().extend(parts);
        } else {
            errors.push(format!("Invalid hierarchy row: {line}"));
        }
    }

    let mut elements: Vec<(String, String)> = Vec::new();
    let mut element_set: HashMap<String, String> = HashMap::new();
    // Iterate tiers in fixed order to preserve insertion order (matching JS Map behavior).
    for tier_name in &["P0", "P1", "P2", "P3", "P4"] {
        if let Some(names) = tiers.get(*tier_name) {
            for name in names {
                if let Some(prev) = element_set.get(name) {
                    errors.push(format!("Element {name} appears in {prev} and {tier_name}."));
                }
                element_set.insert(name.clone(), tier_name.to_string());
                elements.push((name.clone(), tier_name.to_string()));
            }
        }
    }

    Hierarchy {
        tiers,
        elements,
        errors,
    }
}

/// Parse action rows from ACTIONS field lines.
pub fn parse_actions(lines: &[String]) -> (Vec<Action>, Vec<String>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    let re = Regex::new(
        r#"(?i)^S([0-5])\s+(primary|secondary|tertiary|overflow)\s+"([^"]+)"\s+(safe|reversible|destructive)(?:\s+(destruction-purpose|separated))?$"#
    ).unwrap();

    for line in lines {
        if let Some(caps) = re.captures(line.trim()) {
            result.push(Action {
                scope: caps[1].parse().unwrap_or(0),
                tier: caps[2].to_lowercase(),
                label: caps[3].trim().to_string(),
                risk: caps[4].to_lowercase(),
                qualifier: caps
                    .get(5)
                    .map(|m| m.as_str().to_lowercase())
                    .unwrap_or_default(),
            });
        } else {
            errors.push(format!("Invalid action row: {line}"));
        }
    }

    (result, errors)
}

/// Deck entry for roll provenance validation.
#[derive(Debug, Clone, serde::Deserialize)]
struct DeckFile {
    entries: Vec<DeckEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DeckEntry {
    id: String,
    #[serde(default)]
    tier: String,
}

/// Check roll provenance against the deck files.
fn check_roll(value: &str, root: &Path, findings: &mut Vec<Finding>, catalog: &RuleCatalog) {
    let parts: Vec<String> = value
        .split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    if parts.is_empty() {
        add_finding(
            findings,
            catalog,
            "ROLL-01",
            "ROLL must contain axis, depth, framing, direction, and cns-<8 chars>/<generation>.",
            "Copy the exact ROLL line printed by roll.mjs.",
        );
        return;
    }

    // Last element is the cns key
    let tail = parts.last().cloned().unwrap_or_default();
    let key_re = Regex::new(r"(?i)^(cns-[a-z0-9]{8})/(\d+)$").unwrap();

    let structure_parts = &parts[..parts.len() - 1];
    if structure_parts.len() != 4 || !key_re.is_match(&tail) {
        add_finding(
            findings,
            catalog,
            "ROLL-01",
            "ROLL must contain axis, depth, framing, direction, and cns-<8 chars>/<generation>.",
            "Copy the exact ROLL line printed by roll.mjs.",
        );
        return;
    }

    // Load decks
    let decks_dir = root.join("assets").join("decks");
    let structures_data = fs::read_to_string(decks_dir.join("structures.json")).unwrap_or_default();
    let directions_data = fs::read_to_string(decks_dir.join("directions.json")).unwrap_or_default();

    let structures: DeckFile =
        serde_json::from_str(&structures_data).unwrap_or(DeckFile { entries: vec![] });
    let directions: DeckFile =
        serde_json::from_str(&directions_data).unwrap_or(DeckFile { entries: vec![] });

    let tier_names = ["organizing-axis", "depth-strategy", "framing"];

    for (index, part) in structure_parts.iter().enumerate().take(3) {
        let valid = structures
            .entries
            .iter()
            .any(|e| e.id == *part && e.tier == tier_names[index]);
        if !valid {
            add_finding(
                findings,
                catalog,
                "ROLL-01",
                &format!(
                    "ROLL item \"{part}\" is not a valid {} id.",
                    tier_names[index]
                ),
                "Record the assigned ids exactly as printed by roll.mjs.",
            );
        }
    }

    // Check direction (4th structure part)
    if structure_parts.len() >= 4 {
        let dir_id = &structure_parts[3];
        if !directions.entries.iter().any(|e| e.id == *dir_id) {
            add_finding(
                findings,
                catalog,
                "ROLL-01",
                &format!("ROLL item \"{dir_id}\" is not a valid direction id."),
                "Record the assigned direction id exactly as printed by roll.mjs.",
            );
        }
    }
}

fn add_finding(
    findings: &mut Vec<Finding>,
    catalog: &RuleCatalog,
    rule: &str,
    message: &str,
    hint: &str,
) {
    let severity_str = rules::severity_for(catalog, rule);
    let severity = rules::parse_severity(&severity_str);
    findings.push(Finding {
        rule: rule.to_string(),
        severity,
        confidence: Some("high".into()),
        message: message.to_string(),
        hint: Some(hint.to_string()),
        file: None,
        line: None,
        evidence: None,
    });
}

/// Validate a contract block against the rule catalog.
///
/// Returns all findings and contract details.
pub fn validate_contract(
    text: &str,
    catalog: &RuleCatalog,
    root: &Path,
) -> (Vec<Finding>, ContractDetails) {
    let mut findings = Vec::new();
    let mut details = ContractDetails::default();

    let block = match extract_contract(text) {
        Some(b) => b,
        None => {
            add_finding(
                &mut findings,
                catalog,
                "CONTRACT-01",
                "No CONSIDERED-CONTRACT v1 block found.",
                "Run the structure phase and add a contract comment or use .considered/STRUCTURE.md.",
            );
            return (findings, details);
        }
    };

    let (fields, syntax) = parse_fields(&block);
    if !syntax.is_empty() {
        add_finding(
            &mut findings,
            catalog,
            "CONTRACT-01",
            &syntax.join(" "),
            "Use assets/templates/contract-block.md exactly.",
        );
    }

    // MODE check
    let mode = fields
        .get("MODE")
        .map(|v| v.join(" ").trim().to_lowercase())
        .unwrap_or_default();
    details.mode = if mode.is_empty() {
        None
    } else {
        Some(mode.clone())
    };

    if !mode.is_empty() && !MODES.contains(&mode.as_str()) {
        add_finding(
            &mut findings,
            catalog,
            "DASH-02",
            &format!("MODE \"{mode}\" must be exactly one supported mode."),
            &format!("Choose one of: {}.", MODES.join(", ")),
        );
    }

    // THESIS check
    let thesis = fields
        .get("THESIS")
        .map(|v| v.join(" ").to_lowercase())
        .unwrap_or_default();
    if !thesis.is_empty() {
        let re_refusal = Regex::new(r"refus|reject|avoid|not\s+").unwrap();
        if !re_refusal.is_match(&thesis) {
            add_finding(
                &mut findings,
                catalog,
                "CONTRACT-01",
                "THESIS does not state a default it refuses.",
                "Name the organizing idea and the conventional alternative it rejects.",
            );
        }
    }

    // DECISION check
    let decision = fields
        .get("DECISION")
        .map(|v| v.join(" ").to_lowercase())
        .unwrap_or_default();
    if !decision.is_empty() {
        let re_verb = Regex::new(r"decid").unwrap();
        let re_time = Regex::new(r"within|before|under|in\s+\d+").unwrap();
        let re_consequence = Regex::new(r"cost|risk|consequence|wrong").unwrap();
        if !re_verb.is_match(&decision)
            || !re_time.is_match(&decision)
            || !re_consequence.is_match(&decision)
        {
            add_finding(
                &mut findings,
                catalog,
                "CONTRACT-01",
                "DECISION lacks a decision verb, time budget, or consequence.",
                "State who decides what, within what time, and what being wrong costs.",
            );
        }
    }

    // Parse sub-sections
    let empty = Vec::new();
    let questions_lines = fields.get("QUESTIONS").unwrap_or(&empty);
    let zones_lines = fields.get("ZONES").unwrap_or(&empty);
    let hierarchy_lines = fields.get("HIERARCHY").unwrap_or(&empty);
    let actions_lines = fields.get("ACTIONS").unwrap_or(&empty);

    let (questions, q_errors) = parse_questions(questions_lines);
    let (zones, z_errors) = parse_zones(zones_lines);
    let hierarchy = parse_hierarchy(hierarchy_lines);
    let (actions, a_errors) = parse_actions(actions_lines);

    details.questions = questions.len();
    details.zones = zones.len();

    let mut syntax_errors: Vec<String> = Vec::new();
    syntax_errors.extend(q_errors);
    syntax_errors.extend(z_errors);
    syntax_errors.extend(hierarchy.errors.clone());
    syntax_errors.extend(a_errors);
    if !syntax_errors.is_empty() {
        add_finding(
            &mut findings,
            catalog,
            "CONTRACT-01",
            &syntax_errors.join(" "),
            "Correct every malformed, duplicate, or conflicting row.",
        );
    }

    // Build lookup maps
    let question_by_id: HashMap<String, &Question> =
        questions.iter().map(|q| (q.id.clone(), q)).collect();
    let zone_by_id: HashMap<String, &Zone> = zones.iter().map(|z| (z.id.clone(), z)).collect();

    let banned: HashSet<&str> = BANNED_HEADINGS.iter().copied().collect();

    // Zone checks
    let mut element_to_zone: HashMap<String, String> = HashMap::new();
    for zone in &zones {
        // IA-09: banned headings
        if banned.contains(zone.heading.to_lowercase().as_str()) {
            add_finding(
                &mut findings,
                catalog,
                "IA-09",
                &format!("Zone heading \"{}\" is a container label.", zone.heading),
                "Name the question, object, or decision served.",
            );
        }

        // IA-06: zone element count 3-7
        if zone.elements.len() < 3 || zone.elements.len() > 7 {
            add_finding(
                &mut findings,
                catalog,
                "IA-06",
                &format!(
                    "Zone \"{}\" has {} elements.",
                    zone.heading,
                    zone.elements.len()
                ),
                "Keep each zone between 3 and 7 elements.",
            );
        }

        // IA-01: zone with elements but no question
        if zone.elements.is_empty() && !zone.questions.is_empty() {
            // elements empty but questions present — not flagged in JS
        }
        if !zone.elements.is_empty() && zone.questions.is_empty() {
            add_finding(
                &mut findings,
                catalog,
                "IA-01",
                &format!(
                    "Zone \"{}\" has elements but no ranked question.",
                    zone.heading
                ),
                "Map every zone to one or more declared questions.",
            );
        }

        // IA-01: question references
        for qid in &zone.questions {
            match question_by_id.get(qid) {
                None => {
                    add_finding(
                        &mut findings,
                        catalog,
                        "IA-01",
                        &format!(
                            "Zone \"{}\" references unknown question {qid}.",
                            zone.heading
                        ),
                        "Use only question ids declared in QUESTIONS.",
                    );
                }
                Some(q) => {
                    if q.zone != zone.id {
                        add_finding(
                            &mut findings,
                            catalog,
                            "IA-01",
                            &format!("{qid} maps to {}, but appears in zone {}.", q.zone, zone.id),
                            "Keep question arrows and zone membership identical.",
                        );
                    }
                }
            }
        }

        // IA-07: duplicate elements across zones
        for element in &zone.elements {
            if let Some(prev_zone) = element_to_zone.get(element) {
                add_finding(
                    &mut findings,
                    catalog,
                    "IA-07",
                    &format!(
                        "Element \"{element}\" appears in both \"{prev_zone}\" and \"{}\".",
                        zone.heading
                    ),
                    "Assign it to one zone.",
                );
            } else {
                element_to_zone.insert(element.clone(), zone.heading.clone());
            }
        }
    }

    // IA-01/IA-17: question-to-zone mapping
    for question in &questions {
        if !zone_by_id.contains_key(&question.zone) {
            add_finding(
                &mut findings,
                catalog,
                "IA-01",
                &format!("{} maps to unknown zone {}.", question.id, question.zone),
                "Create that zone or update the question mapping.",
            );
        } else {
            let zone = zone_by_id.get(&question.zone).unwrap();
            if !zone.questions.contains(&question.id) {
                add_finding(
                    &mut findings,
                    catalog,
                    "IA-17",
                    &format!(
                        "{} is not listed in its declared zone {}.",
                        question.id, question.zone
                    ),
                    "The default zone must answer every declared ranked question.",
                );
            }
        }
    }

    // DASH-03: module budget
    let module_count: usize = zones.iter().map(|z| z.elements.len()).sum();
    if let Some(budget_map) = &catalog
        .manifests
        .get("contract")
        .and_then(|m| m.mode_module_budget.as_ref())
    {
        if let Some(budget) = budget_map.get(&mode) {
            if module_count < budget.0 || module_count > budget.1 {
                add_finding(
                    &mut findings,
                    catalog,
                    "DASH-03",
                    &format!(
                        "{module_count} declared elements is outside the {mode} module budget of {} to {}.",
                        budget.0, budget.1
                    ),
                    "Delete, defer, or add only elements that answer ranked questions.",
                );
            }
        }
    }

    // HIER-05: exactly one P0
    let p0_count = hierarchy.tiers.get("P0").map(|v| v.len()).unwrap_or(0);
    if p0_count != 1 {
        add_finding(
            &mut findings,
            catalog,
            "HIER-05",
            &format!("{p0_count} P0 elements declared."),
            "Declare exactly one P0.",
        );
    }

    // HIER-06: 3-5 P1 elements
    let p1_count = hierarchy.tiers.get("P1").map(|v| v.len()).unwrap_or(0);
    if !(3..=5).contains(&p1_count) {
        add_finding(
            &mut findings,
            catalog,
            "HIER-06",
            &format!("{p1_count} P1 elements declared."),
            "Declare 3 to 5 P1 elements.",
        );
    }

    // P0 reason check
    if p0_count == 1 {
        let p0_line = fields
            .get("HIERARCHY")
            .unwrap_or(&empty)
            .iter()
            .find(|line| Regex::new(r"(?i)^P0\b").unwrap().is_match(line.trim()))
            .cloned()
            .unwrap_or_default();
        let re_reason = Regex::new(r"(?i)\s-\s|because|beats").unwrap();
        if !re_reason.is_match(&p0_line) {
            add_finding(
                &mut findings,
                catalog,
                "CONTRACT-01",
                "P0 has no stated reason it wins the focal position.",
                "Append \" - <why it beats every rival>\" to the P0 row.",
            );
        }
    }

    // IA-01: elements without hierarchy tier
    for element in element_to_zone.keys() {
        if !hierarchy.elements.iter().any(|(e, _)| e == element) {
            add_finding(
                &mut findings,
                catalog,
                "IA-01",
                &format!("Element \"{element}\" has no hierarchy tier."),
                "Give every zone element exactly one P0 to P3 tier.",
            );
        }
    }

    // IA-01: hierarchy elements not placed in a zone
    for (element, tier) in &hierarchy.elements {
        if tier != "P4" && !element_to_zone.contains_key(element) {
            add_finding(
                &mut findings,
                catalog,
                "IA-01",
                &format!("Hierarchy element \"{element}\" is not placed in a zone."),
                "Place it in the zone that answers its question, or remove it.",
            );
        }
    }

    // Action checks
    let emphasis: HashMap<&str, i32> = HashMap::from([
        ("primary", 3),
        ("secondary", 2),
        ("tertiary", 1),
        ("overflow", 0),
    ]);

    let mut primaries_by_scope: HashMap<u32, Vec<&Action>> = HashMap::new();
    let mut by_scope: HashMap<u32, Vec<&Action>> = HashMap::new();
    let re_banned_label =
        Regex::new(r"(?i)^(submit|ok|yes|no|go|continue|manage|update|click here)$").unwrap();

    for action in &actions {
        if action.tier == "primary" {
            primaries_by_scope
                .entry(action.scope)
                .or_default()
                .push(action);
        }
        by_scope.entry(action.scope).or_default().push(action);

        // ACT-08: destructive primary
        if action.risk == "destructive"
            && action.tier == "primary"
            && action.qualifier != "destruction-purpose"
        {
            add_finding(
                &mut findings,
                catalog,
                "ACT-08",
                &format!(
                    "Destructive action \"{}\" is primary at S{}.",
                    action.label, action.scope
                ),
                "Demote it or declare a destruction-purpose scope.",
            );
        }

        // ACT-05: concise labels
        let word_count = action.label.split_whitespace().count();
        if word_count > 3 || re_banned_label.is_match(&action.label) {
            add_finding(
                &mut findings,
                catalog,
                "ACT-05",
                &format!(
                    "Action label \"{}\" does not name a concise outcome.",
                    action.label
                ),
                "Use a verb plus object in three words or fewer.",
            );
        }
    }

    // ACT-01: one primary per scope
    for (scope, rows) in &primaries_by_scope {
        if rows.len() > 1 {
            add_finding(
                &mut findings,
                catalog,
                "ACT-01",
                &format!("S{scope} has {} primary actions.", rows.len()),
                "Keep one primary action per scope.",
            );
        }
    }

    // ACT-03 and ACT-04
    let mut scopes: Vec<u32> = by_scope.keys().copied().collect();
    scopes.sort();

    for &scope in &scopes {
        let scope_actions = by_scope.get(&scope).unwrap();
        let narrower = scope_actions
            .iter()
            .map(|a| *emphasis.get(a.tier.as_str()).unwrap_or(&0))
            .max()
            .unwrap_or(0);

        // ACT-03: emphasis must decrease with scope width
        for &parent in &scopes {
            if parent >= scope {
                continue;
            }
            let parent_actions = by_scope.get(&parent).unwrap();
            let wider = parent_actions
                .iter()
                .map(|a| *emphasis.get(a.tier.as_str()).unwrap_or(&0))
                .max()
                .unwrap_or(0);
            if narrower >= wider {
                add_finding(
                    &mut findings,
                    catalog,
                    "ACT-03",
                    &format!(
                        "S{scope} has emphasis {narrower}, not strictly lower than enclosing S{parent} at {wider}."
                    ),
                    "Demote the narrower-scope action treatment.",
                );
            }
        }

        // ACT-04: max 3 visible actions per scope
        let visible: Vec<&&Action> = scope_actions
            .iter()
            .filter(|a| a.tier != "overflow")
            .collect();
        if visible.len() > 3 {
            add_finding(
                &mut findings,
                catalog,
                "ACT-04",
                &format!("S{scope} has {} visible actions.", visible.len()),
                "Move surplus actions into overflow.",
            );
        }
    }

    // LOOP-04: DELETED field format
    let deleted = fields
        .get("DELETED")
        .map(|v| v.join(" ").trim().to_string())
        .unwrap_or_default();
    let deletions: Vec<&str> = deleted
        .split(';')
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .collect();
    let re_deletion = Regex::new(r"^.+\s-\s.+$").unwrap();
    let re_none = Regex::new(r"(?i)^(none|nothing|n/a)$").unwrap();
    if deletions.is_empty()
        || deletions.iter().any(|v| !re_deletion.is_match(v))
        || re_none.is_match(&deleted)
    {
        add_finding(
            &mut findings,
            catalog,
            "LOOP-04",
            "DELETED must name one or more removed items and a reason for each.",
            "Use \"item - reason; item - reason\".",
        );
    }

    // ROLL-01: roll provenance
    let roll = fields
        .get("ROLL")
        .map(|v| v.join(" ").trim().to_string())
        .unwrap_or_default();
    check_roll(&roll, root, &mut findings, catalog);

    (findings, details)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_contract_finds_block() {
        let text = "// CONSIDERED-CONTRACT v1\n// THESIS: test\n// ROLL: x\n";
        let block = extract_contract(text);
        assert!(block.is_some());
        assert!(block.unwrap().contains("THESIS: test"));
    }

    #[test]
    fn extract_contract_returns_none_when_missing() {
        let text = "no contract here\n";
        assert!(extract_contract(text).is_none());
    }

    #[test]
    fn parse_fields_extracts_required() {
        let block = "THESIS: test thesis\nMODE: operate\nDECISION: decide\nQUESTIONS: q\nZONES: z\nHIERARCHY: h\nACTIONS: a\nDELETED: d\nROLL: r\n";
        let (fields, syntax) = parse_fields(block);
        assert!(fields.contains_key("THESIS"));
        assert!(syntax.is_empty() || syntax.iter().all(|s| !s.starts_with("Missing")));
    }

    #[test]
    fn parse_questions_valid() {
        let lines = vec!["Q1 [P0] Which queue? -> A".to_string()];
        let (questions, errors) = parse_questions(&lines);
        assert_eq!(questions.len(), 1);
        assert!(errors.is_empty());
        assert_eq!(questions[0].id, "Q1");
        assert_eq!(questions[0].zone, "A");
    }

    #[test]
    fn parse_zones_valid() {
        let lines = vec!["A \"Needs attention\" :: Q1,Q2 :: elem1,elem2,elem3".to_string()];
        let (zones, errors) = parse_zones(&lines);
        assert_eq!(zones.len(), 1);
        assert!(errors.is_empty());
        assert_eq!(zones[0].heading, "Needs attention");
        assert_eq!(zones[0].elements.len(), 3);
    }

    #[test]
    fn parse_hierarchy_extracts_tiers() {
        let lines = vec![
            "P0 breach-count - it wins".to_string(),
            "P1 a, b, c".to_string(),
        ];
        let h = parse_hierarchy(&lines);
        assert_eq!(h.tiers.get("P0").unwrap().len(), 1);
        assert_eq!(h.tiers.get("P1").unwrap().len(), 3);
        assert!(h.elements.iter().any(|(e, _)| e == "breach-count"));
    }

    #[test]
    fn parse_actions_valid() {
        let lines = vec![
            "S1 primary \"Reassign agents\" safe".to_string(),
            "S2 secondary \"Export\" safe".to_string(),
        ];
        let (actions, errors) = parse_actions(&lines);
        assert_eq!(actions.len(), 2);
        assert!(errors.is_empty());
        assert_eq!(actions[0].scope, 1);
        assert_eq!(actions[0].tier, "primary");
    }
}
