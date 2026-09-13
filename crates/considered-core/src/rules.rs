//! Rule catalog and gate policy loading.
//!
//! Loads rule manifests from `assets/rules/` and the gate policy from
//! `assets/rules/gate.json`. Provides `evaluate_gate` which matches the
//! JavaScript `evaluateGate` function exactly.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::finding::{Finding, Severity};
use crate::EngineError;

/// A single rule entry from a manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEntry {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub check: String,
    pub fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A rule manifest (contract.json, source.json, render.json, guidance.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleManifest {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub description: String,
    pub rules: Vec<RuleEntry>,
    #[serde(default)]
    pub version: String,
    /// Mode-to-module-budget mapping (only present in contract manifest).
    #[serde(default, rename = "modeModuleBudget")]
    pub mode_module_budget: Option<HashMap<String, (usize, usize)>>,
    /// Declared rule count from the manifest (for validation).
    #[serde(default, rename = "ruleCount")]
    pub rule_count: usize,
}

/// The rule catalog: all loaded manifests and a by-id lookup.
#[derive(Debug, Clone)]
pub struct RuleCatalog {
    pub by_id: HashMap<String, CatalogEntry>,
    pub manifests: HashMap<String, RuleManifest>,
}

impl RuleCatalog {
    pub fn by_id_len(&self) -> usize {
        self.by_id.len()
    }

    pub fn by_id_keys(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(|s| s.as_str())
    }

    pub fn has_rule(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }
}

/// A rule entry enriched with its manifest kind.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub id: String,
    pub severity: String,
    pub kind: String,
}

/// Gate policy loaded from gate.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatePolicy {
    #[serde(default)]
    pub kind: String,
    #[serde(default, rename = "findingLimits")]
    pub finding_limits: FindingLimits,
    #[serde(default)]
    pub review: ReviewPolicy,
}

/// Finding count limits for the gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingLimits {
    #[serde(default, rename = "S1")]
    pub s1: usize,
    #[serde(default, rename = "S2")]
    pub s2: usize,
    #[serde(default, rename = "S3")]
    pub s3: usize,
}

/// Review-related gate policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewPolicy {
    #[serde(default, rename = "maxScore")]
    pub max_score: f64,
    #[serde(default, rename = "minimumScore")]
    pub minimum_score: f64,
    #[serde(default, rename = "criticalDimensions")]
    pub critical_dimensions: Vec<String>,
    #[serde(default, rename = "dimensionFloor")]
    pub dimension_floor: f64,
    #[serde(default, rename = "shipVerdict")]
    pub ship_verdict: String,
    #[serde(default, rename = "recognizedVerdicts")]
    pub recognized_verdicts: Vec<String>,
}

/// Result of gate evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub outcome: String,
    pub counts: GateCounts,
    #[serde(rename = "heuristicCounts")]
    pub heuristic_counts: GateCounts,
    pub reasons: Vec<String>,
}

/// Counts for gate output (only S1/S2/S3 are used).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateCounts {
    #[serde(rename = "S1", skip_serializing_if = "Option::is_none")]
    pub s1: Option<usize>,
    #[serde(rename = "S2", skip_serializing_if = "Option::is_none")]
    pub s2: Option<usize>,
    #[serde(rename = "S3", skip_serializing_if = "Option::is_none")]
    pub s3: Option<usize>,
}

const KINDS: &[&str] = &["contract", "source", "render", "guidance"];
const EXECUTABLE_KINDS: &[&str] = &["contract", "source", "render"];

/// Load the rule catalog from the project root.
pub fn load_rule_catalog(root: &Path) -> Result<RuleCatalog, EngineError> {
    let rule_dir = root.join("assets").join("rules");
    let mut by_id = HashMap::new();
    let mut manifests = HashMap::new();

    for kind in KINDS {
        let file = rule_dir.join(format!("{kind}.json"));
        let data = fs::read_to_string(&file).map_err(EngineError::Io)?;
        let manifest: RuleManifest = serde_json::from_str(&data).map_err(EngineError::Json)?;

        if manifest.kind != *kind {
            return Err(EngineError::Other(format!("Invalid {kind} rule manifest.")));
        }
        if manifest.rules.is_empty() && *kind != "guidance" {
            return Err(EngineError::Other(format!("Invalid {kind} rule manifest.")));
        }

        for rule in &manifest.rules {
            if rule.id.is_empty() || rule.severity.is_empty() {
                return Err(EngineError::Other(format!(
                    "{kind} manifest has a rule missing id or severity."
                )));
            }
            if EXECUTABLE_KINDS.contains(kind) && rule.checker.is_none() {
                return Err(EngineError::Other(format!(
                    "{kind} rule {} has no checker.",
                    rule.id
                )));
            }
            if by_id.contains_key(&rule.id) {
                return Err(EngineError::Other(format!(
                    "Duplicate rule id {} across manifests.",
                    rule.id
                )));
            }
            by_id.insert(
                rule.id.clone(),
                CatalogEntry {
                    id: rule.id.clone(),
                    severity: rule.severity.clone(),
                    kind: kind.to_string(),
                },
            );
        }

        manifests.insert(kind.to_string(), manifest);
    }

    Ok(RuleCatalog { by_id, manifests })
}

/// Load the gate policy from the project root.
pub fn load_gate_policy(root: &Path) -> Result<GatePolicy, EngineError> {
    let file = root.join("assets").join("rules").join("gate.json");
    let data = fs::read_to_string(&file).map_err(EngineError::Io)?;
    let policy: GatePolicy = serde_json::from_str(&data).map_err(EngineError::Json)?;

    if policy.kind != "gate" {
        return Err(EngineError::Other("Invalid gate policy manifest.".into()));
    }
    if policy.review.critical_dimensions.is_empty() {
        return Err(EngineError::Other(
            "Gate policy has no critical dimensions.".into(),
        ));
    }
    if !policy
        .review
        .recognized_verdicts
        .contains(&policy.review.ship_verdict)
    {
        return Err(EngineError::Other(
            "Gate policy ship verdict is not among the recognized verdicts.".into(),
        ));
    }

    Ok(policy)
}

/// Look up the severity for a rule id from the catalog.
pub fn severity_for(catalog: &RuleCatalog, id: &str) -> String {
    catalog
        .by_id
        .get(id)
        .map(|e| e.severity.clone())
        .unwrap_or_else(|| "S2".to_string())
}

/// Parse a severity string into a Severity enum.
pub fn parse_severity(s: &str) -> Severity {
    match s {
        "S1" => Severity::S1,
        "S2" => Severity::S2,
        "S3" => Severity::S3,
        "S4" => Severity::S4,
        _ => Severity::S2,
    }
}

/// Summarize findings into counts by severity.
pub fn summarize_findings(findings: &[Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for f in findings {
        let key = f.severity.to_string();
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Normalized review input for gate evaluation.
#[derive(Debug, Clone, Default)]
pub struct ReviewInput {
    pub weighted_score: Option<f64>,
    pub dimensions: Option<HashMap<String, f64>>,
    pub outcome: Option<String>,
    pub stale: bool,
}

/// Evaluate the gate, matching the JS `evaluateGate` exactly.
pub fn evaluate_gate(
    findings: &[Finding],
    review: Option<&ReviewInput>,
    policy: &GatePolicy,
    require_review: bool,
) -> GateResult {
    let limits = &policy.finding_limits;
    let rules = &policy.review;

    // Filter out heuristic findings for gating counts.
    let gating_findings: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.confidence.as_deref() != Some("heuristic"))
        .collect();
    let heuristic_findings: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.confidence.as_deref() == Some("heuristic"))
        .collect();

    let gating_counts = summarize_counts(&gating_findings);
    let heuristic_counts = summarize_counts(&heuristic_findings);

    let s1_count = gating_counts.get("S1").copied().unwrap_or(0);
    let s2_count = gating_counts.get("S2").copied().unwrap_or(0);
    let s3_count = gating_counts.get("S3").copied().unwrap_or(0);

    let blockers = s1_count > limits.s1;
    let majors = s2_count > limits.s2;
    let minors = s3_count > limits.s3;

    let mut reasons = Vec::new();
    if blockers {
        reasons.push(format!("{s1_count} S1 finding(s)"));
    }
    if majors {
        reasons.push(format!("{s2_count} S2 findings (limit {})", limits.s2));
    }
    if minors {
        reasons.push(format!("{s3_count} S3 findings (limit {})", limits.s3));
    }

    if require_review && review.is_none() {
        return GateResult {
            outcome: "REVIEW-REQUIRED".into(),
            counts: to_gate_counts(&gating_counts),
            heuristic_counts: to_gate_counts(&heuristic_counts),
            reasons: vec!["Independent review result is missing.".into()],
        };
    }

    if let Some(rev) = review {
        if rev.stale {
            return GateResult {
                outcome: "REVISE".into(),
                counts: to_gate_counts(&gating_counts),
                heuristic_counts: to_gate_counts(&heuristic_counts),
                reasons: vec!["Independent review is stale; rerun it against the current source and contract.".into()],
            };
        }
        let verdict = rev.outcome.as_ref().map(|o| o.trim().to_uppercase());

        let verdict = match verdict {
            Some(v) if !v.is_empty() => Some(v),
            _ => None,
        };

        if verdict.is_none() {
            return GateResult {
                outcome: "REVIEW-REQUIRED".into(),
                counts: to_gate_counts(&gating_counts),
                heuristic_counts: to_gate_counts(&heuristic_counts),
                reasons: vec!["Independent review states no outcome.".into()],
            };
        }

        let verdict = verdict.unwrap();

        if !rules.recognized_verdicts.contains(&verdict) {
            let raw_outcome = rev.outcome.as_deref().unwrap_or("");
            return GateResult {
                outcome: "REVIEW-REQUIRED".into(),
                counts: to_gate_counts(&gating_counts),
                heuristic_counts: to_gate_counts(&heuristic_counts),
                reasons: vec![format!(
                    "Reviewer outcome \"{}\" is not one of {}.",
                    raw_outcome,
                    rules.recognized_verdicts.join(", ")
                )],
            };
        }

        let score = rev.weighted_score;
        let dims = rev.dimensions.as_ref();

        let missing_dimension = rules.critical_dimensions.iter().find(|key| {
            dims.and_then(|d| d.get(key.as_str()))
                .map(|v| !v.is_finite())
                .unwrap_or(true)
        });

        let low_dimension = rules.critical_dimensions.iter().find(|key| {
            dims.and_then(|d| d.get(key.as_str()))
                .map(|v| v.is_finite() && *v < rules.dimension_floor)
                .unwrap_or(false)
        });

        match score {
            None => reasons.push("Reviewer weightedScore is missing.".into()),
            Some(s) if !s.is_finite() => reasons.push("Reviewer weightedScore is missing.".into()),
            Some(s) if s < rules.minimum_score => {
                reasons.push(format!(
                    "Reviewer score {s}/{} is below {}.",
                    rules.max_score as i64, rules.minimum_score as i64
                ));
            }
            _ => {}
        }

        if let Some(missing) = missing_dimension {
            reasons.push(format!("Reviewer {missing} score is missing."));
        } else if let Some(low) = low_dimension {
            reasons.push(format!(
                "{low} is below the floor of {}.",
                rules.dimension_floor as i64
            ));
        }

        if verdict != rules.ship_verdict {
            let raw = rev.outcome.as_deref().unwrap_or("");
            reasons.push(format!("Reviewer outcome is {raw}."));
        }
    }

    if blockers || majors || minors || !reasons.is_empty() {
        GateResult {
            outcome: "REVISE".into(),
            counts: to_gate_counts(&gating_counts),
            heuristic_counts: to_gate_counts(&heuristic_counts),
            reasons,
        }
    } else {
        GateResult {
            outcome: "SHIP".into(),
            counts: to_gate_counts(&gating_counts),
            heuristic_counts: to_gate_counts(&heuristic_counts),
            reasons: vec![],
        }
    }
}

fn summarize_counts(findings: &[&Finding]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for f in findings {
        let key = f.severity.to_string();
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn to_gate_counts(counts: &HashMap<String, usize>) -> GateCounts {
    GateCounts {
        s1: counts.get("S1").copied().filter(|&v| v > 0),
        s2: counts.get("S2").copied().filter(|&v| v > 0),
        s3: counts.get("S3").copied().filter(|&v| v > 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_for_returns_declared_severity() {
        let catalog = RuleCatalog {
            by_id: HashMap::from([(
                "TEST-01".into(),
                CatalogEntry {
                    id: "TEST-01".into(),
                    severity: "S1".into(),
                    kind: "contract".into(),
                },
            )]),
            manifests: HashMap::new(),
        };
        assert_eq!(severity_for(&catalog, "TEST-01"), "S1");
    }

    #[test]
    fn severity_for_returns_s2_fallback() {
        let catalog = RuleCatalog {
            by_id: HashMap::new(),
            manifests: HashMap::new(),
        };
        assert_eq!(severity_for(&catalog, "MISSING"), "S2");
    }

    #[test]
    fn evaluate_gate_missing_review_returns_review_required() {
        let policy = GatePolicy {
            kind: "gate".into(),
            finding_limits: FindingLimits {
                s1: 0,
                s2: 2,
                s3: 6,
            },
            review: ReviewPolicy {
                max_score: 140.0,
                minimum_score: 120.0,
                critical_dimensions: vec!["D1".into()],
                dimension_floor: 3.0,
                ship_verdict: "SHIP".into(),
                recognized_verdicts: vec!["SHIP".into(), "REVISE".into()],
            },
        };
        let result = evaluate_gate(&[], None, &policy, true);
        assert_eq!(result.outcome, "REVIEW-REQUIRED");
    }

    #[test]
    fn evaluate_gate_ship_with_clean_review() {
        let policy = GatePolicy {
            kind: "gate".into(),
            finding_limits: FindingLimits {
                s1: 0,
                s2: 2,
                s3: 6,
            },
            review: ReviewPolicy {
                max_score: 140.0,
                minimum_score: 120.0,
                critical_dimensions: vec!["D1".into()],
                dimension_floor: 3.0,
                ship_verdict: "SHIP".into(),
                recognized_verdicts: vec!["SHIP".into(), "REVISE".into()],
            },
        };
        let review = ReviewInput {
            weighted_score: Some(126.0),
            dimensions: Some(HashMap::from([("D1".into(), 4.0)])),
            outcome: Some("SHIP".into()),
            stale: false,
        };
        let result = evaluate_gate(&[], Some(&review), &policy, true);
        assert_eq!(result.outcome, "SHIP");
    }

    #[test]
    fn evaluate_gate_heuristic_findings_are_excluded() {
        let policy = GatePolicy {
            kind: "gate".into(),
            finding_limits: FindingLimits {
                s1: 0,
                s2: 2,
                s3: 6,
            },
            review: ReviewPolicy {
                max_score: 140.0,
                minimum_score: 120.0,
                critical_dimensions: vec!["D1".into()],
                dimension_floor: 3.0,
                ship_verdict: "SHIP".into(),
                recognized_verdicts: vec!["SHIP".into(), "REVISE".into()],
            },
        };
        let findings = vec![Finding {
            rule: "A11Y-04".into(),
            severity: Severity::S1,
            confidence: Some("heuristic".into()),
            message: "test".into(),
            hint: None,
            file: None,
            line: None,
            evidence: None,
        }];
        let review = ReviewInput {
            weighted_score: Some(126.0),
            dimensions: Some(HashMap::from([("D1".into(), 4.0)])),
            outcome: Some("SHIP".into()),
            stale: false,
        };
        let result = evaluate_gate(&findings, Some(&review), &policy, true);
        assert_eq!(result.outcome, "SHIP");
    }

    #[test]
    fn evaluate_gate_rejects_stale_review() {
        let policy = GatePolicy {
            kind: "gate".into(),
            finding_limits: FindingLimits::default(),
            review: ReviewPolicy::default(),
        };
        let review = ReviewInput {
            stale: true,
            ..ReviewInput::default()
        };
        let result = evaluate_gate(&[], Some(&review), &policy, true);
        assert_eq!(result.outcome, "REVISE");
        assert!(result.reasons[0].contains("stale"));
    }
}
