//! CLI entry point for the Considered Rust engine.
//!
//! Implements `considered-rs` subcommands: contract, gate, roll, validate,
//! inventory, lint, status, verify, and context (experimental).
//! The CLI owns serialization, stderr, and exit-code mapping.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use considered_core::{
    build_manifest, check_artifacts, compute_status, discover_surfaces, evaluate_gate, hash_corpus,
    hash_file_content, latest_run, load_gate_policy, load_rule_catalog, validate_contract,
    write_check_artifact, ArtifactPath, CacheKey, ContractDetails, Finding, Severity,
};
use serde::{Deserialize, Serialize};

/// The Considered design-reasoning engine (Rust accelerator).
#[derive(Debug, Parser)]
#[command(
    name = "considered-rs",
    version = env!("CARGO_PKG_VERSION"),
    about = "Considered design-reasoning engine (Rust accelerator)",
    after_help = "Exit codes: 0 clean, 1 blocking findings or failed gate, 2 usage error."
)]
struct Cli {
    /// Print the native delegation protocol version and exit.
    #[arg(long)]
    protocol_version: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Output format for commands that produce findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Current default behavior (human-readable or JSON with --json).
    Compatibility,
    /// Status, counts, report path, next action only.
    Compact,
    /// Full JSON output.
    Json,
    /// One finding per line (lint/contract only).
    Ndjson,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Validate a CONSIDERED-CONTRACT v1 block.
    Contract {
        /// Source file to validate.
        file: String,
        /// Output as JSON (shorthand for --format json).
        #[arg(long)]
        json: bool,
        /// Apply gate thresholds (exit 1 on blocking findings).
        #[arg(long)]
        gate: bool,
        /// Output format.
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
        /// Only show findings at these severities (comma-separated, e.g. S1,S2).
        #[arg(long, value_delimiter = ',')]
        severity: Option<Vec<String>>,
        /// Only show findings in files under this path.
        #[arg(long)]
        path: Option<String>,
        /// Limit output to N findings.
        #[arg(long)]
        max_findings: Option<usize>,
    },
    /// Evaluate the ship gate from checker reports and a review.
    Gate {
        /// Report JSON files (from `contract --json` or `lint --json`).
        reports: Vec<String>,
        /// Path to the review JSON file.
        #[arg(long)]
        review: String,
        /// Output as JSON (shorthand for --format json).
        #[arg(long)]
        json: bool,
        /// Output format.
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
    },
    /// Assign a structure and direction for a build.
    Roll {
        /// Mode filter (persuade, operate, analyze, read, experience).
        #[arg(long)]
        mode: Option<String>,
        /// Reproduction key.
        #[arg(long)]
        key: Option<String>,
        /// Replay generation n.
        #[arg(long)]
        gen: Option<u64>,
        /// Advance to generation 1 (requires --key).
        #[arg(long)]
        reroll: bool,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Validate rule manifests, decks, and reference coverage.
    Validate,
    /// Validate the skill package for drift and correctness.
    ValidateSkill {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
        /// Project root to validate (defaults to auto-detection).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Report the design system that already exists in a project.
    Inventory {
        /// Source directory or file (default: .).
        path: Option<String>,
        /// Output as JSON (shorthand for --format json).
        #[arg(long)]
        json: bool,
        /// Output format.
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
    },
    /// Heuristic source audit (review leads, not proof).
    Lint {
        /// Source directory or file (default: .).
        path: Option<String>,
        /// Output as JSON (shorthand for --format json).
        #[arg(long)]
        json: bool,
        /// Exit 1 when findings exceed gate limits.
        #[arg(long)]
        enforce_heuristics: bool,
        /// Output format.
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
        /// Only show findings at these severities (comma-separated, e.g. S1,S2).
        #[arg(long, value_delimiter = ',')]
        severity: Option<Vec<String>>,
        /// Only show findings in files under this path.
        #[arg(long)]
        path_filter: Option<String>,
        /// Limit output to N findings.
        #[arg(long)]
        max_findings: Option<usize>,
    },
    /// Report current surface state from .considered/ directory.
    Status {
        /// Surface ID (default: auto-detect).
        surface: Option<String>,
        /// Output as JSON (shorthand for --format json).
        #[arg(long)]
        json: bool,
        /// Output format.
        #[arg(long, value_enum)]
        format: Option<OutputFormat>,
    },
    /// Verify artifact integrity and consistency.
    Verify {
        /// Surface ID (default: auto-detect).
        surface: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Emit a bounded route manifest for agent consumption (experimental).
    Context {
        /// Must be passed to enable this experimental command.
        #[arg(long)]
        experimental: bool,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Find the project root by walking up from cwd looking for assets/rules/.
fn find_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("assets").join("rules").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Find the .considered directory from cwd.
fn find_considered_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".considered");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Resolve the effective output format from --json and --format flags.
fn resolve_format(json: bool, format: Option<OutputFormat>) -> OutputFormat {
    if json {
        OutputFormat::Json
    } else {
        format.unwrap_or(OutputFormat::Compatibility)
    }
}

/// Parse severity filter strings into Severity values.
fn parse_severity_filter(severities: &[String]) -> Vec<Severity> {
    severities
        .iter()
        .filter_map(|s| match s.as_str() {
            "S1" => Some(Severity::S1),
            "S2" => Some(Severity::S2),
            "S3" => Some(Severity::S3),
            "S4" => Some(Severity::S4),
            _ => None,
        })
        .collect()
}

/// Apply filters to a list of findings.
fn apply_filters(
    findings: &[Finding],
    severity_filter: &[Severity],
    path_filter: Option<&str>,
    max_findings: Option<usize>,
) -> Vec<Finding> {
    let mut filtered: Vec<Finding> = findings
        .iter()
        .filter(|f| {
            if !severity_filter.is_empty() && !severity_filter.contains(&f.severity) {
                return false;
            }
            if let Some(pf) = path_filter {
                if let Some(ref file) = f.file {
                    if !file.starts_with(pf) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    if let Some(max) = max_findings {
        filtered.truncate(max);
    }
    filtered
}

/// Contract audit output (human + JSON shape).
#[derive(Debug, Serialize)]
struct ContractOutput {
    file: String,
    checker: &'static str,
    scope: &'static str,
    details: ContractDetailsOutput,
    counts: serde_json::Value,
    findings: Vec<Finding>,
    /// Path to the check artifact, if written.
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_path: Option<String>,
}

/// JSON compatibility shape for `lint-source.mjs`.
///
/// Field declaration order is intentional: JSON member order is part of the
/// frozen Node compatibility stream, even though JSON objects are otherwise
/// unordered by specification.
#[derive(Debug, Serialize)]
struct LintCompatibilityOutput {
    checker: String,
    scope: String,
    scanned: usize,
    skipped: Vec<considered_scan::lint_source::SkippedFile>,
    counts: serde_json::Value,
    findings: Vec<LintFindingCompatibility>,
}

/// Finding order mirrors the legacy JavaScript object construction exactly.
#[derive(Debug, Serialize)]
struct LintFindingCompatibility {
    rule: String,
    severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    line: u32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
}

/// Frozen JSON assignment shape emitted by `scripts/roll.mjs`.
#[derive(Debug, Serialize)]
struct RollCompatibilityOutput<'a> {
    key: &'a str,
    generation: u64,
    mode: &'a Option<String>,
    capacity: usize,
    warnings: &'a [String],
    structure: RollStructureCompatibility<'a>,
    direction: Option<DirectionCompatibility<'a>>,
}

#[derive(Debug, Serialize)]
struct RollStructureCompatibility<'a> {
    #[serde(rename = "organizing-axis")]
    organizing_axis: Option<StructureCompatibility<'a>>,
    #[serde(rename = "depth-strategy")]
    depth_strategy: Option<StructureCompatibility<'a>>,
    framing: Option<StructureCompatibility<'a>>,
}

#[derive(Debug, Serialize)]
struct StructureCompatibility<'a> {
    id: &'a str,
    name: &'a str,
    tier: &'a str,
    modes: &'a [String],
    thesis: &'a str,
    refuses: &'a str,
    laws: StructureLawsCompatibility<'a>,
    fits: &'a [String],
    avoid: &'a str,
    rating: i64,
}

#[derive(Debug, Serialize)]
struct StructureLawsCompatibility<'a> {
    grouping: &'a str,
    hierarchy: &'a str,
    actions: &'a str,
    empty: &'a str,
    motion: &'a str,
}

#[derive(Debug, Serialize)]
struct DirectionCompatibility<'a> {
    id: &'a str,
    name: &'a str,
    source: &'a str,
    strength: &'a str,
    verb: &'a str,
    modes: &'a [String],
    thesis: &'a str,
    laws: DirectionLawsCompatibility<'a>,
    spark: &'a str,
    #[serde(rename = "borrowSkeleton")]
    borrow_skeleton: &'a str,
    rating: i64,
}

#[derive(Debug, Serialize)]
struct DirectionLawsCompatibility<'a> {
    palette: &'a str,
    #[serde(rename = "type")]
    type_law: &'a str,
    topology: &'a str,
    controls: &'a str,
    motion: &'a str,
}

impl From<&Finding> for LintFindingCompatibility {
    fn from(finding: &Finding) -> Self {
        Self {
            rule: finding.rule.clone(),
            severity: finding.severity,
            confidence: finding.confidence.clone(),
            file: finding.file.clone(),
            line: finding.line.unwrap_or(0),
            message: finding.message.clone(),
            hint: finding.hint.clone(),
        }
    }
}

fn structure_compatibility(
    entry: Option<&considered_core::roll::DeckEntry>,
) -> Option<StructureCompatibility<'_>> {
    entry.map(|entry| StructureCompatibility {
        id: &entry.id,
        name: &entry.name,
        tier: &entry.tier,
        modes: &entry.modes,
        thesis: &entry.thesis,
        refuses: &entry.refuses,
        laws: StructureLawsCompatibility {
            grouping: law(entry, "grouping"),
            hierarchy: law(entry, "hierarchy"),
            actions: law(entry, "actions"),
            empty: law(entry, "empty"),
            motion: law(entry, "motion"),
        },
        fits: &entry.fits,
        avoid: &entry.avoid,
        rating: entry.rating,
    })
}

fn direction_compatibility(
    entry: Option<&considered_core::roll::DeckEntry>,
) -> Option<DirectionCompatibility<'_>> {
    entry.map(|entry| DirectionCompatibility {
        id: &entry.id,
        name: &entry.name,
        source: &entry.source,
        strength: &entry.strength,
        verb: &entry.verb,
        modes: &entry.modes,
        thesis: &entry.thesis,
        laws: DirectionLawsCompatibility {
            palette: law(entry, "palette"),
            type_law: law(entry, "type"),
            topology: law(entry, "topology"),
            controls: law(entry, "controls"),
            motion: law(entry, "motion"),
        },
        spark: &entry.spark,
        borrow_skeleton: &entry.borrow_skeleton,
        rating: entry.rating,
    })
}

fn law<'a>(entry: &'a considered_core::roll::DeckEntry, name: &str) -> &'a str {
    entry.laws.get(name).map(String::as_str).unwrap_or("")
}

#[derive(Debug, Serialize)]
struct ContractDetailsOutput {
    mode: Option<String>,
    questions: usize,
    zones: usize,
}

impl From<ContractDetails> for ContractDetailsOutput {
    fn from(d: ContractDetails) -> Self {
        ContractDetailsOutput {
            mode: d.mode,
            questions: d.questions,
            zones: d.zones,
        }
    }
}

/// Gate output shape.
#[derive(Debug, Serialize)]
struct GateOutput {
    reports: Vec<String>,
    review: Option<String>,
    findings: Vec<ReportFinding>,
    outcome: String,
    counts: GateCountsOutput,
    #[serde(rename = "heuristicCounts")]
    heuristic_counts: GateCountsOutput,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GateCountsOutput {
    #[serde(rename = "S1", skip_serializing_if = "Option::is_none")]
    s1: Option<usize>,
    #[serde(rename = "S2", skip_serializing_if = "Option::is_none")]
    s2: Option<usize>,
    #[serde(rename = "S3", skip_serializing_if = "Option::is_none")]
    s3: Option<usize>,
}

/// Compact output for any command.
#[derive(Debug, Serialize)]
struct CompactOutput {
    status: String,
    counts: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContractCachePayload {
    findings: Vec<Finding>,
    details: ContractDetails,
}

/// Verify output.
#[derive(Debug, Serialize)]
struct VerifyOutput {
    surface_id: String,
    checks: Vec<VerifyCheck>,
    all_pass: bool,
}

#[derive(Debug, Serialize)]
struct VerifyCheck {
    name: String,
    pass: bool,
    detail: String,
}

/// A checker report as read from JSON.
/// Findings in reports may omit optional fields like message/hint.
#[derive(Debug, Deserialize)]
struct ReportJson {
    #[serde(default)]
    findings: Vec<ReportFinding>,
}

/// A finding as read from a report JSON — more lenient than the full Finding.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct ReportFinding {
    #[serde(default)]
    rule: String,
    #[serde(default = "default_severity")]
    severity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence: Option<serde_json::Value>,
}

fn default_severity() -> String {
    "S2".to_string()
}

impl From<ReportFinding> for Finding {
    fn from(rf: ReportFinding) -> Self {
        let severity = match rf.severity.as_str() {
            "S1" => Severity::S1,
            "S2" => Severity::S2,
            "S3" => Severity::S3,
            "S4" => Severity::S4,
            _ => Severity::S2,
        };
        Finding {
            rule: rf.rule,
            severity,
            confidence: rf.confidence,
            message: rf.message.unwrap_or_default(),
            hint: rf.hint,
            file: rf.file,
            line: rf.line,
            evidence: rf.evidence,
        }
    }
}

/// A review JSON file (portable or legacy shape).
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ReviewJson {
    #[serde(default)]
    weightedScore: Option<f64>,
    #[serde(default)]
    dimensions: Option<HashMap<String, f64>>,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    gate: Option<LegacyGate>,
    #[serde(default)]
    freshContext: Option<bool>,
    #[serde(default)]
    reviewValidity: Option<ReviewValidityJson>,
}

#[derive(Debug, Deserialize)]
struct ReviewValidityJson {
    #[serde(default)]
    stale: bool,
}

#[derive(Debug, Deserialize)]
struct LegacyGate {
    #[serde(default)]
    score: Option<f64>,
    #[serde(default)]
    dimensions: Option<HashMap<String, f64>>,
    #[serde(default)]
    decision: Option<String>,
}

fn run_contract(
    file: &str,
    gate: bool,
    format: OutputFormat,
    severity_filter: &[Severity],
    path_filter: Option<&str>,
    max_findings: Option<usize>,
) -> i32 {
    let root = match find_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot locate project root (no assets/rules/ found).");
            return 2;
        }
    };

    let catalog = match load_rule_catalog(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot load rule manifests: {e}");
            return 2;
        }
    };

    let text = match fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Cannot read {file}: {e}");
            return 2;
        }
    };

    // Check cache.
    let cache_dir = root.join(".considered").join("cache");
    let corpus_hash = hash_corpus(&root.join("assets").join("rules"));
    let file_hash = hash_file_content(Path::new(file)).unwrap_or_default();
    let mut file_hashes = BTreeMap::new();
    file_hashes.insert(file.to_string(), file_hash);
    let cache_key = CacheKey {
        format_version: 2,
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        corpus_hash,
        command: "contract".to_string(),
        arguments: vec![file.to_string()],
        file_hashes,
    };

    let use_cache = matches!(format, OutputFormat::Compact);
    let cached_payload = use_cache
        .then(|| considered_core::cache::lookup(&cache_dir, &cache_key))
        .flatten()
        .and_then(|entry| serde_json::from_str::<ContractCachePayload>(&entry.findings_json).ok());
    let (findings, details, cached) = if let Some(payload) = cached_payload {
        (payload.findings, payload.details, true)
    } else {
        let (findings, details) = validate_contract(&text, &catalog, &root);
        if use_cache {
            let payload = ContractCachePayload {
                findings: findings.clone(),
                details: details.clone(),
            };
            if let Ok(json) = serde_json::to_string(&payload) {
                let _ = considered_core::cache::store(&cache_dir, &cache_key, &json);
            }
        }
        (findings, details, false)
    };

    // Apply filters.
    let filtered = apply_filters(&findings, severity_filter, path_filter, max_findings);
    let counts = summarize(&filtered);
    let counts_json = build_counts_json(&counts);

    let mode_str = details
        .mode
        .clone()
        .unwrap_or_else(|| "undeclared".to_string());
    let q_count = details.questions;
    let z_count = details.zones;

    // Write check artifact if surface directory exists.
    let artifact_path = matches!(format, OutputFormat::Compact)
        .then(|| try_write_check_artifact("contract", &filtered, &counts))
        .flatten();

    let output = ContractOutput {
        file: file.to_string(),
        checker: "contract",
        scope: "contract-internal",
        details: details.into(),
        counts: counts_json.clone(),
        findings: filtered.clone(),
        artifact_path: artifact_path.clone(),
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Compact => {
            let status = if filtered.is_empty() {
                "pass"
            } else {
                "findings"
            };
            let compact = CompactOutput {
                status: status.to_string(),
                counts: counts_json,
                report_path: artifact_path,
                next_action: if filtered.is_empty() {
                    None
                } else {
                    Some("fix contract findings".to_string())
                },
                cached: Some(cached),
            };
            println!("{}", serde_json::to_string(&compact).unwrap());
        }
        OutputFormat::Ndjson => {
            for f in &filtered {
                println!("{}", serde_json::to_string(f).unwrap());
            }
        }
        OutputFormat::Compatibility => {
            println!("\n  CONTRACT AUDIT  {file}");
            println!("  mode {mode_str}   questions {q_count}   zones {z_count}");
            if filtered.is_empty() {
                println!("\n  No findings in contract-internal checks. Run the independent review gate before shipping.\n");
            } else {
                for f in &filtered {
                    let rule_padded = format!("{:<12}", f.rule);
                    println!(
                        "  {rule_padded} {}  {}\n               {}",
                        f.severity,
                        f.message,
                        f.hint.as_deref().unwrap_or("")
                    );
                }
            }
            println!(
                "\n  {} blocking, {} major, {} minor\n",
                counts.get("S1").unwrap_or(&0),
                counts.get("S2").unwrap_or(&0),
                counts.get("S3").unwrap_or(&0),
            );
        }
    }

    if gate {
        let s1 = counts.get("S1").copied().unwrap_or(0);
        let s2 = counts.get("S2").copied().unwrap_or(0);
        let s3 = counts.get("S3").copied().unwrap_or(0);
        if s1 > 0 || s2 > 2 || s3 > 6 {
            1
        } else {
            0
        }
    } else if counts.get("S1").copied().unwrap_or(0) > 0 {
        1
    } else {
        0
    }
}

fn run_gate(reports: &[String], review_path: &str, format: OutputFormat) -> i32 {
    let root = match find_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot locate project root (no assets/rules/ found).");
            return 2;
        }
    };

    let policy = match load_gate_policy(&root) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Cannot load gate policy: {e}");
            return 2;
        }
    };

    let mut report_findings = Vec::new();
    for path in reports {
        let data = match fs::read_to_string(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Cannot read JSON from {path}: {e}");
                return 2;
            }
        };
        let report: ReportJson = match serde_json::from_str(&data) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Cannot read JSON from {path}: {e}");
                return 2;
            }
        };
        report_findings.extend(report.findings);
    }

    let review_data = match fs::read_to_string(review_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Cannot read JSON from {review_path}: {e}");
            return 2;
        }
    };
    let review_json: ReviewJson = match serde_json::from_str(&review_data) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Cannot read JSON from {review_path}: {e}");
            return 2;
        }
    };

    // Normalize review: accept portable shape and legacy gate shape.
    let normalized = considered_core::ReviewInput {
        weighted_score: review_json
            .weightedScore
            .or_else(|| review_json.gate.as_ref().and_then(|g| g.score)),
        dimensions: review_json
            .dimensions
            .or_else(|| review_json.gate.as_ref().and_then(|g| g.dimensions.clone())),
        outcome: review_json
            .outcome
            .or_else(|| review_json.gate.as_ref().and_then(|g| g.decision.clone())),
        stale: review_json.freshContext == Some(false)
            || review_json
                .reviewValidity
                .as_ref()
                .is_some_and(|validity| validity.stale),
    };

    let all_findings: Vec<Finding> = report_findings.iter().cloned().map(Finding::from).collect();
    let result = evaluate_gate(&all_findings, Some(&normalized), &policy, true);
    let outcome = result.outcome.clone();

    match format {
        OutputFormat::Json => {
            let output = GateOutput {
                reports: reports.to_vec(),
                review: Some(review_path.to_string()),
                findings: report_findings,
                outcome: result.outcome,
                counts: GateCountsOutput {
                    s1: result.counts.s1,
                    s2: result.counts.s2,
                    s3: result.counts.s3,
                },
                heuristic_counts: GateCountsOutput {
                    s1: result.heuristic_counts.s1,
                    s2: result.heuristic_counts.s2,
                    s3: result.heuristic_counts.s3,
                },
                reasons: result.reasons,
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Compact => {
            let mut counts_map = serde_json::Map::new();
            if let Some(s1) = result.counts.s1 {
                counts_map.insert("S1".into(), s1.into());
            }
            if let Some(s2) = result.counts.s2 {
                counts_map.insert("S2".into(), s2.into());
            }
            if let Some(s3) = result.counts.s3 {
                counts_map.insert("S3".into(), s3.into());
            }
            let compact = CompactOutput {
                status: result.outcome.to_lowercase(),
                counts: serde_json::Value::Object(counts_map),
                report_path: None,
                next_action: if outcome == "SHIP" {
                    None
                } else {
                    Some(format!("gate outcome: {}", outcome))
                },
                cached: None,
            };
            println!("{}", serde_json::to_string(&compact).unwrap());
        }
        OutputFormat::Compatibility => {
            println!("\n  CONSIDERED GATE  {}", outcome);
            println!(
                "  {} S1, {} S2, {} S3",
                result.counts.s1.unwrap_or(0),
                result.counts.s2.unwrap_or(0),
                result.counts.s3.unwrap_or(0),
            );
            let h = &result.heuristic_counts;
            let h_total = h.s1.unwrap_or(0) + h.s2.unwrap_or(0) + h.s3.unwrap_or(0);
            if h_total > 0 {
                println!(
                    "  reviewer leads: {} S1, {} S2, {} S3 heuristic",
                    h.s1.unwrap_or(0),
                    h.s2.unwrap_or(0),
                    h.s3.unwrap_or(0),
                );
            }
            for reason in &result.reasons {
                println!("  - {reason}");
            }
            println!();
        }
        OutputFormat::Ndjson => {
            // Gate doesn't really benefit from ndjson; fall back to json.
            let output = GateOutput {
                reports: reports.to_vec(),
                review: Some(review_path.to_string()),
                findings: report_findings,
                outcome: result.outcome,
                counts: GateCountsOutput {
                    s1: result.counts.s1,
                    s2: result.counts.s2,
                    s3: result.counts.s3,
                },
                heuristic_counts: GateCountsOutput {
                    s1: result.heuristic_counts.s1,
                    s2: result.heuristic_counts.s2,
                    s3: result.heuristic_counts.s3,
                },
                reasons: result.reasons,
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }

    if outcome == "SHIP" {
        0
    } else {
        1
    }
}

fn summarize(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for f in findings {
        *counts.entry(f.severity.to_string()).or_insert(0) += 1;
    }
    counts
}

fn build_counts_json(counts: &HashMap<String, usize>) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for sev in ["S1", "S2", "S3", "S4"] {
        if let Some(&n) = counts.get(sev) {
            if n > 0 {
                map.insert(sev.to_string(), serde_json::Value::Number(n.into()));
            }
        }
    }
    serde_json::Value::Object(map)
}

/// Try to write a check artifact if a surface directory exists.
fn try_write_check_artifact(
    command: &str,
    findings: &[Finding],
    _counts: &HashMap<String, usize>,
) -> Option<String> {
    let considered_dir = find_considered_dir()?;
    let surfaces = discover_surfaces(&considered_dir);
    // Use the first surface if there's exactly one.
    if surfaces.len() != 1 {
        return None;
    }
    let surface_id = &surfaces[0];
    let timestamp = iso_now();
    let findings_json = serde_json::to_string_pretty(&serde_json::json!({
        "command": command,
        "timestamp": timestamp,
        "findings": findings,
    }))
    .ok()?;
    let path = write_check_artifact(
        &considered_dir,
        surface_id,
        command,
        &timestamp,
        &findings_json,
    )
    .ok()?;
    Some(path.to_string_lossy().to_string())
}

fn iso_now() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;
    let time_part = secs % 86400;
    let hours = time_part / 3600;
    let minutes = (time_part % 3600) / 60;
    let seconds = time_part % 60;
    let (y, m, d) = days_to_ymd(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn main() {
    let cli = Cli::parse();
    if cli.protocol_version {
        println!("1.0.0");
        return;
    }

    let exit_code = match cli.command {
        None => 0,
        Some(Commands::Contract {
            file,
            json,
            gate,
            format,
            severity,
            path,
            max_findings,
        }) => {
            let fmt = resolve_format(json, format);
            let sev_filter = severity
                .as_ref()
                .map(|s| parse_severity_filter(s))
                .unwrap_or_default();
            run_contract(&file, gate, fmt, &sev_filter, path.as_deref(), max_findings)
        }
        Some(Commands::Gate {
            reports,
            review,
            json,
            format,
        }) => {
            if reports.is_empty() {
                eprintln!("Usage: considered-rs gate <report.json> [...report.json] --review <REVIEW.json> [--json]");
                process::exit(2);
            }
            let fmt = resolve_format(json, format);
            run_gate(&reports, &review, fmt)
        }
        Some(Commands::Roll {
            mode,
            key,
            gen,
            reroll,
            json,
        }) => run_roll(mode, key, gen, reroll, json),
        Some(Commands::Validate) => run_validate(),
        Some(Commands::ValidateSkill { json, root }) => run_validate_skill(json, root),
        Some(Commands::Inventory { path, json, format }) => {
            let fmt = resolve_format(json, format);
            run_inventory(path.as_deref().unwrap_or("."), fmt)
        }
        Some(Commands::Lint {
            path,
            json,
            enforce_heuristics,
            format,
            severity,
            path_filter,
            max_findings,
        }) => {
            let fmt = resolve_format(json, format);
            let sev_filter = severity
                .as_ref()
                .map(|s| parse_severity_filter(s))
                .unwrap_or_default();
            run_lint(
                path.as_deref().unwrap_or("."),
                fmt,
                enforce_heuristics,
                &sev_filter,
                path_filter.as_deref(),
                max_findings,
            )
        }
        Some(Commands::Status {
            surface,
            json,
            format,
        }) => {
            let fmt = resolve_format(json, format);
            run_status(surface.as_deref(), fmt)
        }
        Some(Commands::Verify { surface, json }) => run_verify(surface.as_deref(), json),
        Some(Commands::Context { experimental, json }) => run_context(experimental, json),
    };

    process::exit(exit_code);
}

fn run_roll(
    mode: Option<String>,
    key: Option<String>,
    gen: Option<u64>,
    reroll: bool,
    json: bool,
) -> i32 {
    use considered_core::roll::{
        draw_chained, generation_capacity, load_deck, new_key, render_human, RollOutput,
    };

    let root = match find_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot find project root (no assets/rules/ directory found).");
            return 2;
        }
    };

    if reroll && key.is_none() {
        eprintln!("considered roll: --reroll requires --key");
        eprintln!("Run \"considered roll --help\" for usage.");
        return 2;
    }
    if reroll && gen.is_some() {
        eprintln!("considered roll: --reroll cannot be combined with --gen");
        eprintln!("Run \"considered roll --help\" for usage.");
        return 2;
    }
    if let Some(ref m) = mode {
        if !["persuade", "operate", "analyze", "read", "experience"].contains(&m.as_str()) {
            eprintln!("considered roll: unknown mode \"{m}\"; expected one of: persuade, operate, analyze, read, experience");
            eprintln!("Run \"considered roll --help\" for usage.");
            return 2;
        }
    }

    let structures = match load_deck(&root, "structures") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let directions = match load_deck(&root, "directions") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };

    let capacity = generation_capacity(&structures, &directions, &mode);
    if capacity == 0 {
        eprintln!(
            "Deck has no complete hand for mode \"{}\".",
            mode.as_deref().unwrap_or("any")
        );
        return 3;
    }

    let actual_gen = if reroll { 1 } else { gen.unwrap_or(0) };
    if actual_gen >= capacity as u64 {
        eprintln!(
            "Deck exhausted at generation {actual_gen} for mode \"{}\".",
            mode.as_deref().unwrap_or("any")
        );
        eprintln!(
            "Available generations are 0 through {}. Start a fresh key instead.",
            capacity - 1
        );
        return 3;
    }

    let key_str = key.unwrap_or_else(new_key);
    let dealt = draw_chained(&structures, &directions, &mode, &key_str, actual_gen);

    let output = RollOutput {
        key: key_str,
        generation: actual_gen,
        mode,
        capacity,
        warnings: vec![],
        structure: dealt.structure,
        direction: dealt.direction,
    };

    if json {
        let compatibility = RollCompatibilityOutput {
            key: &output.key,
            generation: output.generation,
            mode: &output.mode,
            capacity: output.capacity,
            warnings: &output.warnings,
            structure: RollStructureCompatibility {
                organizing_axis: structure_compatibility(
                    output
                        .structure
                        .get("organizing-axis")
                        .and_then(Option::as_ref),
                ),
                depth_strategy: structure_compatibility(
                    output
                        .structure
                        .get("depth-strategy")
                        .and_then(Option::as_ref),
                ),
                framing: structure_compatibility(
                    output.structure.get("framing").and_then(Option::as_ref),
                ),
            },
            direction: direction_compatibility(output.direction.as_ref()),
        };
        println!("{}", serde_json::to_string_pretty(&compatibility).unwrap());
    } else {
        print!("{}", render_human(&output));
    }
    0
}

fn run_validate() -> i32 {
    use considered_core::validate::validate_assets;

    let root = match find_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot find project root (no assets/rules/ directory found).");
            return 2;
        }
    };

    let result = validate_assets(&root);
    if result.is_ok() {
        println!(
            "Asset validation passed: {} unique rules, two valid decks, and a gate policy at {}/{}.",
            result.rule_count, result.min_score as i64, result.max_score as i64
        );
        0
    } else {
        eprintln!("ASSET VALIDATION FAILED");
        for msg in &result.failures {
            eprintln!("- {msg}");
        }
        1
    }
}

fn run_validate_skill(json: bool, root: Option<PathBuf>) -> i32 {
    use considered_core::validate_skill::validate_skill;

    let root = match root.or_else(find_root) {
        Some(r) => r,
        None => {
            eprintln!("Cannot find project root (no assets/rules/ directory found).");
            return 2;
        }
    };

    let result = validate_skill(&root);

    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        let total = result.checks.len();
        let passed = result.checks.iter().filter(|c| c.passed).count();
        let failed = total - passed;

        if result.passed {
            println!("Skill validation passed ({}/{} checks OK)", passed, total);
        } else {
            println!(
                "Skill validation FAILED ({}/{} checks OK, {} failed)",
                passed, total, failed
            );
        }
        println!();

        for check in &result.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            println!("  [{}] {}", status, check.name);
            for msg in &check.messages {
                println!("         {}", msg);
            }
        }
    }

    result.exit_code()
}

fn run_inventory(target: &str, format: OutputFormat) -> i32 {
    use considered_scan::inventory::inventory;

    let path = Path::new(target);
    if !path.exists() {
        eprintln!("Cannot inspect {target}: No such file or directory");
        return 2;
    }

    match inventory(path) {
        Ok(report) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report).unwrap());
                }
                OutputFormat::Compact => {
                    let has_system = !report.tokens.is_empty()
                        || !report.components.is_empty()
                        || !report.configs.is_empty();
                    let status = if has_system {
                        "system-found"
                    } else {
                        "clean-slate"
                    };
                    let compact = serde_json::json!({
                        "status": status,
                        "files": report.files,
                        "tokens": report.tokens.len(),
                        "components": report.components.len(),
                        "configs": report.configs.len(),
                    });
                    println!("{}", serde_json::to_string(&compact).unwrap());
                }
                OutputFormat::Compatibility => {
                    print_inventory_human(&report, target);
                }
                OutputFormat::Ndjson => {
                    // Inventory doesn't benefit from ndjson; use json.
                    println!("{}", serde_json::to_string_pretty(&report).unwrap());
                }
            }
            0
        }
        Err(e) => {
            eprintln!("considered inventory: {e}");
            2
        }
    }
}

fn print_inventory_human(report: &considered_scan::inventory::InventoryReport, target: &str) {
    let has_system =
        !report.tokens.is_empty() || !report.components.is_empty() || !report.configs.is_empty();
    let scope = if report.target_kind == "file" {
        format!("in {target}")
    } else {
        format!("under {target}")
    };
    println!(
        "\n  EXISTING SYSTEM  {} source {} {scope}\n",
        report.files,
        if report.files == 1 { "file" } else { "files" }
    );

    if !has_system {
        println!("  No design system detected. You are defining one.");
        println!("  Establish tokens before components, and components before screens.\n");
        return;
    }

    if !report.configs.is_empty() {
        println!("  Config      {}", report.configs.join(", "));
    }
    if !report.docs.is_empty() {
        println!(
            "  Context     {}  (read these before designing)",
            report.docs.join(", ")
        );
    }

    if !report.tokens.is_empty() {
        println!("\n  Tokens      {} custom properties", report.tokens.len());
        let grouped = report.ordered_token_groups();
        for (key, list) in &grouped {
            let capped: Vec<String> = list.iter().take(4).cloned().collect();
            let suffix = if list.len() > 4 {
                format!(" ... +{} more", list.len() - 4)
            } else {
                String::new()
            };
            println!("    {:<9} {}{}", key, capped.join("  "), suffix);
        }
    }
    if !report.fonts.is_empty() {
        let capped: Vec<&String> = report.fonts.iter().take(4).collect();
        println!(
            "\n  Fonts       {}",
            capped
                .into_iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    if !report.spacing.is_empty() {
        let mut sorted: Vec<&String> = report.spacing.iter().collect();
        sorted.sort_by(|a, b| {
            a.parse::<f64>()
                .unwrap_or(0.0)
                .partial_cmp(&b.parse::<f64>().unwrap_or(0.0))
                .unwrap()
        });
        let capped: Vec<&String> = sorted.into_iter().take(12).collect();
        println!(
            "  Spacing     {}",
            capped
                .into_iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    if !report.radii.is_empty() {
        let capped: Vec<&String> = report.radii.iter().take(6).collect();
        println!(
            "  Radii       {}",
            capped
                .into_iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    if !report.components.is_empty() {
        let dirs = report
            .component_dirs()
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        println!("\n  Components  {} in {}", report.components.len(), dirs);
        for (name, variants) in report.components.iter().take(20) {
            let v = if variants.is_empty() {
                "(no variants detected)".to_string()
            } else {
                variants.join(", ")
            };
            println!("    {:<18} {}", name, v);
        }
        if report.components.len() > 20 {
            println!("    ... +{} more", report.components.len() - 20);
        }
    }
    println!("\n  Inherit this system. Do not invent a parallel one.");
    println!("  If the assigned direction conflicts with these tokens, the tokens win");
    println!("  unless the user explicitly asked for a redesign.\n");
}

fn run_lint(
    target: &str,
    format: OutputFormat,
    enforce: bool,
    severity_filter: &[Severity],
    path_filter: Option<&str>,
    max_findings: Option<usize>,
) -> i32 {
    use considered_scan::lint_source::{lint_source, should_enforce_fail};

    let path = Path::new(target);
    if !path.exists() {
        eprintln!("Cannot read {target}: No such file or directory");
        return 2;
    }

    match lint_source(path) {
        Ok(report) => {
            if report.findings.is_empty() && report.skipped.is_empty() && report.scanned == 0 {
                eprintln!("No supported source files found under {target}.");
                return 2;
            }

            // Apply filters.
            let filtered =
                apply_filters(&report.findings, severity_filter, path_filter, max_findings);
            let counts = summarize(&filtered);

            // Write check artifact if surface directory exists.
            let artifact_path = matches!(format, OutputFormat::Compact)
                .then(|| try_write_check_artifact("lint", &filtered, &counts))
                .flatten();

            match format {
                OutputFormat::Json => {
                    let json_report = LintCompatibilityOutput {
                        checker: report.checker.clone(),
                        scope: report.scope.clone(),
                        scanned: report.scanned,
                        skipped: report.skipped.clone(),
                        counts: build_counts_json(&counts),
                        findings: filtered
                            .iter()
                            .map(LintFindingCompatibility::from)
                            .collect(),
                    };
                    println!("{}", serde_json::to_string_pretty(&json_report).unwrap());
                }
                OutputFormat::Compact => {
                    let status = if filtered.is_empty() {
                        "pass"
                    } else {
                        "findings"
                    };
                    let compact = CompactOutput {
                        status: status.to_string(),
                        counts: build_counts_json(&counts),
                        report_path: artifact_path,
                        next_action: if filtered.is_empty() {
                            None
                        } else {
                            Some("review heuristic leads".to_string())
                        },
                        cached: None,
                    };
                    println!("{}", serde_json::to_string(&compact).unwrap());
                }
                OutputFormat::Ndjson => {
                    for f in &filtered {
                        println!("{}", serde_json::to_string(f).unwrap());
                    }
                }
                OutputFormat::Compatibility => {
                    print_lint_human_filtered(&report, &filtered, target);
                }
            }

            if enforce && should_enforce_fail(&report) {
                1
            } else {
                0
            }
        }
        Err(considered_core::EngineError::Usage(msg))
            if msg.starts_with("No supported source files found under") =>
        {
            // Node's lint-source.mjs prints this exact line with no command
            // prefix; mirror it verbatim rather than wrapping it as a usage
            // error.
            eprintln!("{msg}");
            2
        }
        Err(e) => {
            eprintln!("considered lint: {e}");
            2
        }
    }
}

fn print_lint_human_filtered(
    report: &considered_scan::lint_source::LintSourceReport,
    filtered: &[Finding],
    target: &str,
) {
    println!(
        "\n  SOURCE REVIEW LEADS  {} files under {target}",
        report.scanned
    );
    println!("  These are heuristics. Verify them in the independent review packet.\n");
    if filtered.is_empty() {
        println!("  No heuristic leads. This is not a visual or semantic pass.\n");
    } else {
        for f in filtered {
            let file = f.file.as_deref().unwrap_or("?");
            let loc = match f.line {
                Some(l) => format!("{file}:{l}"),
                None => file.to_string(),
            };
            println!(
                "  {:<10} {}  {loc}\n               {}\n               {}\n",
                f.rule,
                f.severity,
                f.message,
                f.hint.as_deref().unwrap_or("")
            );
        }
    }
    if !report.skipped.is_empty() {
        println!(
            "  {} unreadable path(s) were skipped.\n",
            report.skipped.len()
        );
    }
    let s1 = filtered
        .iter()
        .filter(|f| f.severity == Severity::S1)
        .count();
    let s2 = filtered
        .iter()
        .filter(|f| f.severity == Severity::S2)
        .count();
    let s3 = filtered
        .iter()
        .filter(|f| f.severity == Severity::S3)
        .count();
    println!("  {s1} S1, {s2} S2, {s3} S3 heuristic leads\n");
}

fn run_status(surface: Option<&str>, format: OutputFormat) -> i32 {
    let considered_dir = match find_considered_dir() {
        Some(d) => d,
        None => {
            match format {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "error": "no surface state found"
                        }))
                        .unwrap()
                    );
                }
                _ => {
                    println!("no surface state found");
                }
            }
            return 0;
        }
    };

    let surface_id = match surface {
        Some(s) => s.to_string(),
        None => {
            let surfaces = discover_surfaces(&considered_dir);
            if surfaces.is_empty() {
                match format {
                    OutputFormat::Json => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "error": "no surface state found"
                            }))
                            .unwrap()
                        );
                    }
                    _ => {
                        println!("no surface state found");
                    }
                }
                return 0;
            }
            if surfaces.len() > 1 {
                match format {
                    OutputFormat::Json => {
                        let mut statuses = Vec::new();
                        for sid in &surfaces {
                            if let Some(status) = compute_status(&considered_dir, sid) {
                                statuses.push(status);
                            }
                        }
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "surfaces": statuses,
                                "hint": "specify a surface ID to see details"
                            }))
                            .unwrap()
                        );
                    }
                    _ => {
                        println!("multiple surfaces found:");
                        for sid in &surfaces {
                            println!("  - {sid}");
                        }
                        println!("specify a surface ID to see details");
                    }
                }
                return 0;
            }
            surfaces[0].clone()
        }
    };

    match compute_status(&considered_dir, &surface_id) {
        Some(status) => {
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&status).unwrap());
                }
                OutputFormat::Compact => {
                    let compact = serde_json::json!({
                        "stage": status.stage.to_string(),
                        "review_stale": status.review_stale,
                        "next_action": status.next_action,
                    });
                    println!("{}", serde_json::to_string(&compact).unwrap());
                }
                OutputFormat::Compatibility => {
                    println!("\n  SURFACE STATUS  {}", status.surface_id);
                    println!("  stage:          {}", status.stage);
                    println!("  next action:    {}", status.next_action);
                    println!(
                        "  review stale:   {}",
                        if status.review_stale { "yes" } else { "no" }
                    );
                    if !status.counts.is_empty() {
                        let counts_str: Vec<String> = status
                            .counts
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect();
                        println!("  counts:         {}", counts_str.join(", "));
                    }
                    if let Some(ref ts) = status.run_timestamp {
                        println!("  last run:       {ts}");
                    }
                    if let Some(ref cd) = status.checks_dir {
                        println!("  checks:         {cd}");
                    }
                    println!();
                }
                OutputFormat::Ndjson => {
                    // Status doesn't benefit from ndjson; use json.
                    println!("{}", serde_json::to_string_pretty(&status).unwrap());
                }
            }
            0
        }
        None => {
            match format {
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "error": "no run records found",
                            "surface_id": surface_id
                        }))
                        .unwrap()
                    );
                }
                _ => {
                    println!("no run records found for surface '{surface_id}'");
                }
            }
            0
        }
    }
}

fn run_verify(surface: Option<&str>, json: bool) -> i32 {
    let considered_dir = match find_considered_dir() {
        Some(d) => d,
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "error": "no surface state found"
                    }))
                    .unwrap()
                );
            } else {
                println!("no surface state found");
            }
            return 0;
        }
    };

    let surface_id = match surface {
        Some(s) => s.to_string(),
        None => {
            let surfaces = discover_surfaces(&considered_dir);
            if surfaces.is_empty() {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "error": "no surface state found"
                        }))
                        .unwrap()
                    );
                } else {
                    println!("no surface state found");
                }
                return 0;
            }
            surfaces[0].clone()
        }
    };

    let mut checks = Vec::new();

    // Check 1: Artifact presence.
    let artifacts = check_artifacts(&considered_dir, &surface_id);
    let missing_artifacts: Vec<&str> = artifacts
        .iter()
        .filter(|(_, &present)| !present)
        .map(|(name, _)| name.as_str())
        .collect();
    checks.push(VerifyCheck {
        name: "artifact-presence".to_string(),
        pass: missing_artifacts.is_empty(),
        detail: if missing_artifacts.is_empty() {
            "all expected artifacts present".to_string()
        } else {
            format!("missing: {}", missing_artifacts.join(", "))
        },
    });

    let run = latest_run(&considered_dir, &surface_id);

    // Check 2: Run record validity and identity.
    if let Some(run) = &run {
        let valid = run.schema_version >= 1 && run.surface_id == surface_id;
        checks.push(VerifyCheck {
            name: "run-record-schema".to_string(),
            pass: valid,
            detail: if valid {
                format!("schema version {}; surface id matches", run.schema_version)
            } else {
                format!(
                    "schema version {}; record surface '{}' does not match '{}'",
                    run.schema_version, run.surface_id, surface_id
                )
            },
        });
    } else {
        checks.push(VerifyCheck {
            name: "run-record-schema".to_string(),
            pass: false,
            detail: "no run records found".to_string(),
        });
    }

    // Check 3: Review freshness.
    if let Some(run) = &run {
        let review_fresh = !run
            .review_validity
            .as_ref()
            .is_some_and(|review| review.stale)
            && run.review_fresh.unwrap_or(true);
        checks.push(VerifyCheck {
            name: "review-freshness".to_string(),
            pass: review_fresh,
            detail: if review_fresh {
                "review is current".to_string()
            } else {
                "review is stale (source/contract changed since review)".to_string()
            },
        });
    }

    // Check 4: Contract synchronization uses recorded content, not presence.
    let contract_path = considered_dir.join(&surface_id).join("CONTRACT.md");
    let recorded_contract_hash = run.as_ref().and_then(|record| {
        record
            .review_validity
            .as_ref()
            .and_then(|review| review.contract_hash.clone())
            .or_else(|| record.contract_hash.clone())
    });
    let actual_contract_hash = hash_file_content(&contract_path);
    let contract_matches = recorded_contract_hash.is_some()
        && actual_contract_hash.is_some()
        && recorded_contract_hash == actual_contract_hash;
    checks.push(VerifyCheck {
        name: "contract-sync".to_string(),
        pass: contract_matches,
        detail: match (recorded_contract_hash, actual_contract_hash) {
            (_, None) => "no CONTRACT.md found".to_string(),
            (None, Some(_)) => {
                "contract exists but the run record has no contract hash".to_string()
            }
            (Some(expected), Some(actual)) if expected == actual => {
                "contract hash matches the run record".to_string()
            }
            (Some(_), Some(_)) => "contract changed after the recorded review".to_string(),
        },
    });

    // Check 5: Every recorded artifact path is workspace-relative and every
    // supplied hash matches current content.
    if let Some(run) = &run {
        let project_root = considered_dir.parent().unwrap_or(&considered_dir);
        let mut failures = Vec::new();
        for (name, artifact) in &run.artifacts {
            let Some(path) = &artifact.path else {
                failures.push(format!("{name}: missing path"));
                continue;
            };
            let safe = match ArtifactPath::new(path.clone()) {
                Ok(path) => path,
                Err(_) => {
                    failures.push(format!("{name}: unsafe path"));
                    continue;
                }
            };
            let Some(expected) = &artifact.hash else {
                failures.push(format!("{name}: missing hash"));
                continue;
            };
            if hash_file_content(&project_root.join(safe.to_path_buf())).as_ref() != Some(expected)
            {
                failures.push(format!("{name}: hash mismatch"));
            }
        }
        checks.push(VerifyCheck {
            name: "artifact-integrity".to_string(),
            pass: failures.is_empty(),
            detail: if failures.is_empty() {
                format!("{} recorded artifact hash(es) match", run.artifacts.len())
            } else {
                failures.join(", ")
            },
        });
    }

    let all_pass = checks.iter().all(|c| c.pass);

    let output = VerifyOutput {
        surface_id: surface_id.clone(),
        checks,
        all_pass,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("\n  VERIFY  {surface_id}\n");
        for check in &output.checks {
            let icon = if check.pass { "✓" } else { "✗" };
            println!("  {icon}  {:<24} {}", check.name, check.detail);
        }
        println!();
        if all_pass {
            println!("  All checks passed.\n");
        } else {
            println!("  Some checks failed.\n");
        }
    }

    if all_pass {
        0
    } else {
        1
    }
}

fn run_context(experimental: bool, json: bool) -> i32 {
    if !experimental {
        eprintln!("considered context: this command is experimental.");
        eprintln!("Pass --experimental to enable it.");
        return 2;
    }

    let root = match find_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot find project root (no assets/rules/ directory found).");
            return 2;
        }
    };

    let manifest = build_manifest(&root);

    if json {
        println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
    } else {
        println!("\n  CONTEXT MANIFEST (experimental)\n");
        println!("  Engine version: {}", manifest.engine_version);
        println!("\n  Skills:");
        for skill in &manifest.skills {
            println!(
                "    {} — commands: {}",
                skill.path,
                skill.commands.join(", ")
            );
        }
        println!("\n  References:");
        for reference in &manifest.references {
            println!("    {} ({})", reference.name, reference.path);
        }
        println!("\n  Templates:");
        for template in &manifest.templates {
            println!("    {} ({})", template.name, template.path);
        }
        println!("\n  Commands:");
        for (name, cmd) in &manifest.commands {
            println!("    {:<14} {}", name, cmd.description);
        }
        println!();
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_package() {
        let version = env!("CARGO_PKG_VERSION");
        assert_eq!(version, "0.7.0");
    }

    #[test]
    fn cli_parses_no_args() {
        let cli = Cli::try_parse_from(["considered-rs"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_contract_subcommand() {
        let cli = Cli::try_parse_from(["considered-rs", "contract", "test.tsx"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_contract_with_flags() {
        let cli =
            Cli::try_parse_from(["considered-rs", "contract", "test.tsx", "--json", "--gate"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_contract_with_format() {
        let cli = Cli::try_parse_from([
            "considered-rs",
            "contract",
            "test.tsx",
            "--format",
            "compact",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_contract_with_severity_filter() {
        let cli = Cli::try_parse_from([
            "considered-rs",
            "contract",
            "test.tsx",
            "--severity",
            "S1,S2",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_lint_with_filters() {
        let cli = Cli::try_parse_from([
            "considered-rs",
            "lint",
            "--severity",
            "S1",
            "--path-filter",
            "src/",
            "--max-findings",
            "10",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_gate_subcommand() {
        let cli = Cli::try_parse_from([
            "considered-rs",
            "gate",
            "report.json",
            "--review",
            "review.json",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_status_subcommand() {
        let cli = Cli::try_parse_from(["considered-rs", "status"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_verify_subcommand() {
        let cli = Cli::try_parse_from(["considered-rs", "verify"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_context_requires_experimental() {
        let cli = Cli::try_parse_from(["considered-rs", "context", "--experimental"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_rejects_unknown_flags() {
        let result = Cli::try_parse_from(["considered-rs", "--bogus"]);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_format_json_flag_wins() {
        assert_eq!(resolve_format(true, None), OutputFormat::Json);
        assert_eq!(
            resolve_format(true, Some(OutputFormat::Compact)),
            OutputFormat::Json
        );
    }

    #[test]
    fn resolve_format_uses_format_when_no_json() {
        assert_eq!(
            resolve_format(false, Some(OutputFormat::Compact)),
            OutputFormat::Compact
        );
        assert_eq!(
            resolve_format(false, Some(OutputFormat::Ndjson)),
            OutputFormat::Ndjson
        );
        assert_eq!(resolve_format(false, None), OutputFormat::Compatibility);
    }

    #[test]
    fn parse_severity_filter_works() {
        let sevs = parse_severity_filter(&["S1".into(), "S3".into()]);
        assert_eq!(sevs, vec![Severity::S1, Severity::S3]);
    }

    #[test]
    fn parse_severity_filter_ignores_invalid() {
        let sevs = parse_severity_filter(&["S1".into(), "invalid".into()]);
        assert_eq!(sevs, vec![Severity::S1]);
    }

    #[test]
    fn apply_filters_by_severity() {
        let findings = vec![
            Finding {
                rule: "A".into(),
                severity: Severity::S1,
                confidence: None,
                message: "".into(),
                hint: None,
                file: Some("a.tsx".into()),
                line: None,
                evidence: None,
            },
            Finding {
                rule: "B".into(),
                severity: Severity::S3,
                confidence: None,
                message: "".into(),
                hint: None,
                file: Some("b.tsx".into()),
                line: None,
                evidence: None,
            },
        ];
        let filtered = apply_filters(&findings, &[Severity::S1], None, None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule, "A");
    }

    #[test]
    fn apply_filters_by_path() {
        let findings = vec![
            Finding {
                rule: "A".into(),
                severity: Severity::S1,
                confidence: None,
                message: "".into(),
                hint: None,
                file: Some("src/a.tsx".into()),
                line: None,
                evidence: None,
            },
            Finding {
                rule: "B".into(),
                severity: Severity::S1,
                confidence: None,
                message: "".into(),
                hint: None,
                file: Some("lib/b.tsx".into()),
                line: None,
                evidence: None,
            },
        ];
        let filtered = apply_filters(&findings, &[], Some("src/"), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule, "A");
    }

    #[test]
    fn apply_filters_max_findings() {
        let findings = vec![
            Finding {
                rule: "A".into(),
                severity: Severity::S1,
                confidence: None,
                message: "".into(),
                hint: None,
                file: None,
                line: None,
                evidence: None,
            },
            Finding {
                rule: "B".into(),
                severity: Severity::S2,
                confidence: None,
                message: "".into(),
                hint: None,
                file: None,
                line: None,
                evidence: None,
            },
            Finding {
                rule: "C".into(),
                severity: Severity::S3,
                confidence: None,
                message: "".into(),
                hint: None,
                file: None,
                line: None,
                evidence: None,
            },
        ];
        let filtered = apply_filters(&findings, &[], None, Some(2));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn build_counts_json_skips_zeros() {
        let mut counts = HashMap::new();
        counts.insert("S1".to_string(), 3);
        counts.insert("S2".to_string(), 0);
        let json = build_counts_json(&counts);
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("S1"));
        assert!(!obj.contains_key("S2"));
    }
}
