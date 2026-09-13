//! Surface state: run records, stages, and `.considered/` directory reading.
//!
//! Reads `.considered/<surface-id>/runs/` to find the latest run record
//! and reports current stage, next action, staleness, and finding counts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Stages of a surface's lifecycle, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Init,
    Structure,
    Contract,
    Source,
    Review,
    Gate,
    Complete,
}

impl Stage {
    /// All stages in lifecycle order.
    pub fn all() -> &'static [Stage] {
        &[
            Stage::Init,
            Stage::Structure,
            Stage::Contract,
            Stage::Source,
            Stage::Review,
            Stage::Gate,
            Stage::Complete,
        ]
    }

    /// The next stage in the lifecycle, or None if complete.
    pub fn next(self) -> Option<Stage> {
        let all = Self::all();
        let pos = all.iter().position(|&s| s == self)?;
        all.get(pos + 1).copied()
    }

    /// Human-readable next action description.
    pub fn next_action(self) -> &'static str {
        match self.next() {
            Some(Stage::Structure) => "run `considered structure` to define the surface frame",
            Some(Stage::Contract) => "run `considered contract` to validate the contract block",
            Some(Stage::Source) => "run `considered lint` to audit source files",
            Some(Stage::Review) => "run the independent review and save REVIEW.json",
            Some(Stage::Gate) => "run `considered gate` to evaluate the ship decision",
            Some(Stage::Complete) => "surface is complete",
            None => "surface is complete",
            Some(Stage::Init) => "unreachable",
        }
    }
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stage::Init => write!(f, "init"),
            Stage::Structure => write!(f, "structure"),
            Stage::Contract => write!(f, "contract"),
            Stage::Source => write!(f, "source"),
            Stage::Review => write!(f, "review"),
            Stage::Gate => write!(f, "gate"),
            Stage::Complete => write!(f, "complete"),
        }
    }
}

impl std::str::FromStr for Stage {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "init" => Ok(Stage::Init),
            "structure" => Ok(Stage::Structure),
            "contract" => Ok(Stage::Contract),
            "source" => Ok(Stage::Source),
            "review" => Ok(Stage::Review),
            "gate" => Ok(Stage::Gate),
            "complete" => Ok(Stage::Complete),
            _ => Err(format!("unknown stage: {s}")),
        }
    }
}

/// A run record stored in `.considered/<surface-id>/runs/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    /// Schema version for forward compatibility.
    #[serde(
        rename = "schemaVersion",
        alias = "schema_version",
        default = "default_schema_version",
        deserialize_with = "deserialize_schema_version",
        serialize_with = "serialize_schema_version"
    )]
    pub schema_version: u32,
    /// Unique run identifier in the public schema.
    #[serde(rename = "runId", default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Surface identifier.
    #[serde(rename = "surfaceId", alias = "surface_id")]
    pub surface_id: String,
    /// Current stage at time of recording.
    pub stage: Stage,
    /// ISO-8601 timestamp of the run.
    #[serde(rename = "createdAt", alias = "timestamp")]
    pub timestamp: String,
    /// Finding counts from the latest checks.
    #[serde(default)]
    pub counts: BTreeMap<String, usize>,
    /// Hash of source files at time of lint.
    #[serde(default)]
    #[serde(rename = "sourceHashes", alias = "source_hashes")]
    pub source_hashes: BTreeMap<String, String>,
    /// Hash of contract block at time of contract check.
    #[serde(default)]
    #[serde(rename = "contractHash", alias = "contract_hash")]
    pub contract_hash: Option<String>,
    /// Whether the review is fresh (not stale).
    #[serde(default)]
    #[serde(rename = "reviewFresh", alias = "review_fresh")]
    pub review_fresh: Option<bool>,
    /// Review validity from considered-run-record/v1.
    #[serde(rename = "reviewValidity", default)]
    pub review_validity: Option<ReviewValidity>,
    /// Versioned artifact links and hashes from considered-run-record/v1.
    #[serde(default)]
    pub artifacts: BTreeMap<String, ArtifactRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewValidity {
    #[serde(rename = "reviewId", default)]
    pub review_id: Option<String>,
    #[serde(rename = "sourceHash", default)]
    pub source_hash: Option<String>,
    #[serde(rename = "contractHash", default)]
    pub contract_hash: Option<String>,
    #[serde(default)]
    pub stale: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArtifactRecord {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
}

fn default_schema_version() -> u32 {
    1
}

fn deserialize_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(number) => number
            .as_u64()
            .and_then(|number| u32::try_from(number).ok())
            .ok_or_else(|| serde::de::Error::custom("schemaVersion must be a positive integer")),
        serde_json::Value::String(version) => version
            .split('.')
            .next()
            .and_then(|major| major.parse::<u32>().ok())
            .ok_or_else(|| {
                serde::de::Error::custom("schemaVersion must begin with a numeric major version")
            }),
        _ => Err(serde::de::Error::custom(
            "schemaVersion must be a version string or integer",
        )),
    }
}

fn serialize_schema_version<S>(version: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{version}.0.0"))
}

/// Status report for a surface.
#[derive(Debug, Clone, Serialize)]
pub struct SurfaceStatus {
    pub surface_id: String,
    pub stage: Stage,
    pub next_action: String,
    pub review_stale: bool,
    pub counts: BTreeMap<String, usize>,
    pub run_timestamp: Option<String>,
    pub checks_dir: Option<String>,
}

/// Find all surface IDs under `.considered/`.
pub fn discover_surfaces(considered_dir: &Path) -> Vec<String> {
    let mut surfaces = Vec::new();
    if let Ok(entries) = fs::read_dir(considered_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip non-surface directories.
                if name == "cache" || name == "checks" || name.starts_with('.') {
                    continue;
                }
                surfaces.push(name);
            }
        }
    }
    surfaces.sort();
    surfaces
}

/// Read the latest run record for a surface.
pub fn latest_run(considered_dir: &Path, surface_id: &str) -> Option<RunRecord> {
    let runs_dir = considered_dir.join(surface_id).join("runs");
    if !runs_dir.exists() {
        return None;
    }

    let mut records: Vec<(String, RunRecord)> = Vec::new();
    if let Ok(entries) = fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<RunRecord>(&data) {
                        let filename = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        records.push((filename, record));
                    }
                }
            }
        }
    }

    // Sort by timestamp (filename) descending, take latest.
    records.sort_by(|a, b| b.0.cmp(&a.0));
    records.into_iter().next().map(|(_, r)| r)
}

/// List all run records for a surface, sorted newest first.
pub fn all_runs(considered_dir: &Path, surface_id: &str) -> Vec<RunRecord> {
    let runs_dir = considered_dir.join(surface_id).join("runs");
    if !runs_dir.exists() {
        return Vec::new();
    }

    let mut records = Vec::new();
    if let Ok(entries) = fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<RunRecord>(&data) {
                        records.push(record);
                    }
                }
            }
        }
    }
    records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    records
}

/// Compute the status for a surface.
pub fn compute_status(considered_dir: &Path, surface_id: &str) -> Option<SurfaceStatus> {
    let run = latest_run(considered_dir, surface_id)?;
    let checks_path = considered_dir.join(surface_id).join("checks");
    let checks_dir = if checks_path.exists() {
        Some(checks_path.to_string_lossy().to_string())
    } else {
        None
    };

    let review_stale = run
        .review_validity
        .as_ref()
        .is_some_and(|review| review.stale)
        || run.review_fresh == Some(false);

    Some(SurfaceStatus {
        surface_id: surface_id.to_string(),
        stage: run.stage,
        next_action: run.stage.next_action().to_string(),
        review_stale,
        counts: run.counts,
        run_timestamp: Some(run.timestamp),
        checks_dir,
    })
}

/// Check whether artifact files exist for a surface.
pub fn check_artifacts(considered_dir: &Path, surface_id: &str) -> BTreeMap<String, bool> {
    let surface_dir = considered_dir.join(surface_id);
    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        "STRUCTURE.md".to_string(),
        surface_dir.join("STRUCTURE.md").exists(),
    );
    artifacts.insert(
        "FRAME.md".to_string(),
        surface_dir.join("FRAME.md").exists(),
    );
    artifacts.insert(
        "CONTRACT.md".to_string(),
        surface_dir.join("CONTRACT.md").exists(),
    );
    artifacts.insert(
        "REVIEW.json".to_string(),
        surface_dir.join("REVIEW.json").exists(),
    );
    artifacts.insert(
        "REVIEW-PACKET.md".to_string(),
        surface_dir.join("REVIEW-PACKET.md").exists(),
    );
    artifacts.insert(
        "DESIGN.md".to_string(),
        surface_dir.join("DESIGN.md").exists(),
    );
    artifacts
}

/// Write a run record to the surface's runs directory.
pub fn write_run_record(considered_dir: &Path, record: &RunRecord) -> std::io::Result<PathBuf> {
    let runs_dir = considered_dir.join(&record.surface_id).join("runs");
    fs::create_dir_all(&runs_dir)?;
    let filename = format!("{}.json", record.timestamp.replace([':', '.'], "-"));
    let path = runs_dir.join(&filename);
    let data = serde_json::to_string_pretty(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(&path, data)?;
    Ok(path)
}

/// Write findings JSON to the checks directory.
pub fn write_check_artifact(
    considered_dir: &Path,
    surface_id: &str,
    command: &str,
    timestamp: &str,
    findings_json: &str,
) -> std::io::Result<PathBuf> {
    let checks_dir = considered_dir.join(surface_id).join("checks");
    fs::create_dir_all(&checks_dir)?;
    let safe_ts = timestamp.replace([':', '.'], "-");
    let filename = format!("{command}-{safe_ts}.json");
    let path = checks_dir.join(&filename);
    fs::write(&path, findings_json)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_ordering() {
        assert!(Stage::Init < Stage::Structure);
        assert!(Stage::Gate < Stage::Complete);
    }

    #[test]
    fn stage_next_advances() {
        assert_eq!(Stage::Init.next(), Some(Stage::Structure));
        assert_eq!(Stage::Gate.next(), Some(Stage::Complete));
        assert_eq!(Stage::Complete.next(), None);
    }

    #[test]
    fn stage_display_and_parse() {
        for stage in Stage::all() {
            let s = stage.to_string();
            let parsed: Stage = s.parse().unwrap();
            assert_eq!(parsed, *stage);
        }
    }

    #[test]
    fn discover_surfaces_skips_cache() {
        let dir = tempfile::tempdir().unwrap();
        let considered = dir.path().join(".considered");
        fs::create_dir_all(considered.join("cache")).unwrap();
        fs::create_dir_all(considered.join("my-surface")).unwrap();
        fs::create_dir_all(considered.join(".hidden")).unwrap();

        let surfaces = discover_surfaces(&considered);
        assert_eq!(surfaces, vec!["my-surface"]);
    }

    #[test]
    fn latest_run_returns_newest() {
        let dir = tempfile::tempdir().unwrap();
        let considered = dir.path().join(".considered");
        let runs_dir = considered.join("surf").join("runs");
        fs::create_dir_all(&runs_dir).unwrap();

        let r1 = RunRecord {
            schema_version: 1,
            run_id: Some("run-1".into()),
            surface_id: "surf".into(),
            stage: Stage::Init,
            timestamp: "2026-01-01T00-00-00Z".into(),
            counts: BTreeMap::new(),
            source_hashes: BTreeMap::new(),
            contract_hash: None,
            review_fresh: None,
            review_validity: None,
            artifacts: BTreeMap::new(),
        };
        let r2 = RunRecord {
            schema_version: 1,
            run_id: Some("run-2".into()),
            surface_id: "surf".into(),
            stage: Stage::Contract,
            timestamp: "2026-01-02T00-00-00Z".into(),
            counts: BTreeMap::new(),
            source_hashes: BTreeMap::new(),
            contract_hash: None,
            review_fresh: None,
            review_validity: None,
            artifacts: BTreeMap::new(),
        };
        write_run_record(&considered, &r1).unwrap();
        write_run_record(&considered, &r2).unwrap();

        let latest = latest_run(&considered, "surf").unwrap();
        assert_eq!(latest.stage, Stage::Contract);
    }

    #[test]
    fn check_artifacts_reports_missing() {
        let dir = tempfile::tempdir().unwrap();
        let considered = dir.path().join(".considered");
        let surface = considered.join("test");
        fs::create_dir_all(&surface).unwrap();
        fs::write(surface.join("STRUCTURE.md"), "# test").unwrap();

        let artifacts = check_artifacts(&considered, "test");
        assert!(artifacts["STRUCTURE.md"]);
        assert!(!artifacts["FRAME.md"]);
    }

    #[test]
    fn write_check_artifact_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let considered = dir.path().join(".considered");
        let path = write_check_artifact(
            &considered,
            "surf",
            "lint",
            "2026-01-01T00-00-00Z",
            r#"{"findings":[]}"#,
        )
        .unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, r#"{"findings":[]}"#);
    }

    #[test]
    fn public_run_record_schema_drives_status_staleness() {
        let json = r#"{
          "schemaVersion":"1.0.0",
          "runId":"run-public",
          "surfaceId":"checkout",
          "stage":"review",
          "createdAt":"2026-09-11T00:00:00Z",
          "reviewValidity":{"reviewId":"review-1","stale":true}
        }"#;
        let record: RunRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.schema_version, 1);
        assert_eq!(record.surface_id, "checkout");
        assert!(record.review_validity.unwrap().stale);
    }
}
