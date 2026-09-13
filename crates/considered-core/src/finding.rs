//! Finding types: severity, individual findings, and aggregate counts.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Severity levels for findings, ordered from most to least severe.
///
/// Serialized as `"S1"`, `"S2"`, `"S3"`, `"S4"` to match the existing
/// JavaScript contract exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    S1,
    S2,
    S3,
    S4,
}

impl Severity {
    /// All variants in descending severity order.
    pub fn all() -> &'static [Severity] {
        &[Severity::S1, Severity::S2, Severity::S3, Severity::S4]
    }
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S1" => Ok(Severity::S1),
            "S2" => Ok(Severity::S2),
            "S3" => Ok(Severity::S3),
            "S4" => Ok(Severity::S4),
            _ => Err(format!("invalid severity: {s}")),
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::S1 => write!(f, "S1"),
            Severity::S2 => write!(f, "S2"),
            Severity::S3 => write!(f, "S3"),
            Severity::S4 => write!(f, "S4"),
        }
    }
}

/// A single finding from a lint or validation pass.
///
/// Fields match the JavaScript contract: rule ID, severity, confidence,
/// message, and optional file/line/hint/evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub evidence: Option<serde_json::Value>,
}

/// Aggregate counts of findings by severity.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    #[serde(rename = "S1")]
    pub s1: usize,
    #[serde(rename = "S2")]
    pub s2: usize,
    #[serde(rename = "S3")]
    pub s3: usize,
    #[serde(rename = "S4")]
    pub s4: usize,
}

impl Counts {
    /// Count findings from a slice, grouping by severity.
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut counts = Counts::default();
        for f in findings {
            match f.severity {
                Severity::S1 => counts.s1 += 1,
                Severity::S2 => counts.s2 += 1,
                Severity::S3 => counts.s3 += 1,
                Severity::S4 => counts.s4 += 1,
            }
        }
        counts
    }

    /// Total number of findings across all severities.
    pub fn total(&self) -> usize {
        self.s1 + self.s2 + self.s3 + self.s4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_serializes_as_s1_through_s4() {
        assert_eq!(serde_json::to_string(&Severity::S1).unwrap(), "\"S1\"");
        assert_eq!(serde_json::to_string(&Severity::S4).unwrap(), "\"S4\"");
    }

    #[test]
    fn severity_deserializes_from_string() {
        let s: Severity = serde_json::from_str("\"S2\"").unwrap();
        assert_eq!(s, Severity::S2);
    }

    #[test]
    fn severity_from_str_rejects_invalid() {
        assert!(Severity::from_str("S5").is_err());
        assert!(Severity::from_str("").is_err());
        assert!(Severity::from_str("critical").is_err());
    }

    #[test]
    fn severity_ordering_is_s1_most_severe() {
        assert!(Severity::S1 < Severity::S2);
        assert!(Severity::S3 > Severity::S2);
    }

    #[test]
    fn finding_round_trips_through_json() {
        let f = Finding {
            rule: "CONTRACT-01".into(),
            severity: Severity::S1,
            confidence: Some("high".into()),
            message: "Missing contract block".into(),
            hint: Some("Add a CONSIDERED-CONTRACT block".into()),
            file: Some("src/App.tsx".into()),
            line: Some(10),
            evidence: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn finding_omits_none_fields() {
        let f = Finding {
            rule: "IA-03".into(),
            severity: Severity::S2,
            confidence: None,
            message: "test".into(),
            hint: None,
            file: None,
            line: None,
            evidence: None,
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("confidence"));
        assert!(!json.contains("hint"));
        assert!(!json.contains("file"));
    }

    #[test]
    fn counts_from_findings_groups_correctly() {
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
                severity: Severity::S1,
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
        let counts = Counts::from_findings(&findings);
        assert_eq!(counts.s1, 2);
        assert_eq!(counts.s2, 0);
        assert_eq!(counts.s3, 1);
        assert_eq!(counts.s4, 0);
        assert_eq!(counts.total(), 3);
    }

    #[test]
    fn finding_ordering_is_deterministic() {
        let mut findings = vec![
            Finding {
                rule: "B".into(),
                severity: Severity::S2,
                confidence: None,
                message: "".into(),
                hint: None,
                file: Some("z.tsx".into()),
                line: Some(1),
                evidence: None,
            },
            Finding {
                rule: "A".into(),
                severity: Severity::S1,
                confidence: None,
                message: "".into(),
                hint: None,
                file: Some("a.tsx".into()),
                line: Some(1),
                evidence: None,
            },
        ];
        findings.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then_with(|| a.line.cmp(&b.line))
                .then_with(|| a.rule.cmp(&b.rule))
        });
        assert_eq!(findings[0].rule, "A");
        assert_eq!(findings[1].rule, "B");
    }
}
