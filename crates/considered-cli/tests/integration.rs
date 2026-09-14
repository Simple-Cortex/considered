//! Integration tests for the considered-rs CLI.
//!
//! These tests run the binary against the existing test fixtures and verify
//! that the output matches the Node.js implementation.

use std::process::Command;

fn project_root() -> String {
    env!("CARGO_MANIFEST_DIR").replace("/crates/considered-cli", "")
}

#[test]
fn contract_valid_operate_has_no_findings() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/valid-operate.tsx",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "valid contract should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["findings"].as_array().unwrap().len(), 0);
    assert_eq!(json["details"]["mode"], "operate");
    assert_eq!(json["details"]["questions"], 5);
    assert_eq!(json["details"]["zones"], 3);
}

#[test]
fn contract_missing_block_produces_s1() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/missing-contract.tsx",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["findings"].as_array().unwrap().len(), 1);
    assert_eq!(json["findings"][0]["rule"], "CONTRACT-01");
    assert_eq!(json["findings"][0]["severity"], "S1");
}

#[test]
fn contract_invalid_operate_produces_many_findings() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    // Verify counts match the Node.js output exactly.
    assert_eq!(json["counts"]["S1"], 5);
    assert_eq!(json["counts"]["S2"], 10);
    assert_eq!(json["counts"]["S3"], 5);

    // Verify specific rules fired.
    let rules: Vec<&str> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"CONTRACT-01"));
    assert!(rules.contains(&"IA-09"));
    assert!(rules.contains(&"IA-06"));
    assert!(rules.contains(&"HIER-05"));
    assert!(rules.contains(&"HIER-06"));
    assert!(rules.contains(&"ACT-08"));
    assert!(rules.contains(&"LOOP-04"));
    assert!(rules.contains(&"ROLL-01"));
}

#[test]
fn gate_ship_with_clean_review() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "gate",
            "tests/fixtures/gate/contract.json",
            "tests/fixtures/gate/source.json",
            "--review",
            "tests/fixtures/gate/review-ship.json",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success(), "SHIP gate should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["outcome"], "SHIP");
    assert!(json["reasons"].as_array().unwrap().is_empty());
    // Heuristic S1 should be counted but not gating.
    assert_eq!(json["heuristicCounts"]["S1"], 1);
}

#[test]
fn gate_revise_with_low_score() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "gate",
            "tests/fixtures/gate/contract.json",
            "tests/fixtures/gate/source.json",
            "--review",
            "tests/fixtures/gate/review-revise.json",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["outcome"], "REVISE");
    let reasons: Vec<&str> = json["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    assert!(reasons.iter().any(|r| r.contains("below 120")));
    assert!(reasons.iter().any(|r| r.contains("below the floor")));
    assert!(reasons.iter().any(|r| r.contains("outcome is REVISE")));
}

#[test]
fn gate_no_outcome_returns_review_required() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "gate",
            "tests/fixtures/gate/contract.json",
            "--review",
            "tests/fixtures/gate/review-no-outcome.json",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["outcome"], "REVIEW-REQUIRED");
    assert_eq!(json["reasons"][0], "Independent review states no outcome.");
}

#[test]
fn gate_unknown_outcome_returns_review_required() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "gate",
            "tests/fixtures/gate/contract.json",
            "--review",
            "tests/fixtures/gate/review-unknown-outcome.json",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["outcome"], "REVIEW-REQUIRED");
    let reason = json["reasons"][0].as_str().unwrap();
    assert!(reason.contains("looks good to me"));
    assert!(reason.contains("not one of"));
}

#[test]
fn contract_gate_flag_uses_gate_thresholds() {
    let root = project_root();
    // Valid contract with --gate should exit 0 (no findings).
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/valid-operate.tsx",
            "--gate",
        ])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());

    // Invalid contract with --gate should exit 1 (many findings).
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--gate",
        ])
        .output()
        .expect("failed to run binary");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn contract_human_output_format() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args(["contract", "tests/fixtures/contracts/valid-operate.tsx"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CONTRACT AUDIT"));
    assert!(stdout.contains("mode operate"));
    assert!(stdout.contains("questions 5"));
    assert!(stdout.contains("zones 3"));
    assert!(stdout.contains("No findings"));
}

#[test]
fn gate_human_output_format() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "gate",
            "tests/fixtures/gate/contract.json",
            "--review",
            "tests/fixtures/gate/review-ship.json",
        ])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CONSIDERED GATE"));
    assert!(stdout.contains("SHIP"));
}

#[test]
fn usage_error_exit_code_2() {
    let root = project_root();
    // No file argument for contract.
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args(["contract"])
        .output()
        .expect("failed to run binary");
    // clap exits with 2 for missing required args.
    assert_eq!(output.status.code(), Some(2));
}

// ---------------------------------------------------------------------------
// Milestone 4 integration tests
// ---------------------------------------------------------------------------

#[test]
fn contract_compact_format_is_smaller_than_json() {
    let root = project_root();
    // Get JSON output size.
    let json_output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run binary");
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let json_len = json_stdout.len();

    // Get compact output size.
    let compact_output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--format",
            "compact",
        ])
        .output()
        .expect("failed to run binary");
    let compact_stdout = String::from_utf8_lossy(&compact_output.stdout);
    let compact_len = compact_stdout.len();

    // Compact must be at least 60% smaller.
    assert!(
        compact_len < json_len * 4 / 10,
        "compact ({compact_len}) should be < 40% of json ({json_len})"
    );

    // Compact output should be valid JSON with status and counts.
    let compact_json: serde_json::Value =
        serde_json::from_str(&compact_stdout).expect("valid JSON");
    assert!(compact_json["status"].is_string());
    assert!(compact_json["counts"].is_object());
}

#[test]
fn contract_ndjson_outputs_one_finding_per_line() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--format",
            "ndjson",
        ])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "ndjson should produce output");

    // Each line should be valid JSON with a rule field.
    for line in &lines {
        let finding: serde_json::Value =
            serde_json::from_str(line).expect("each line is valid JSON");
        assert!(finding["rule"].is_string(), "each finding has a rule");
        assert!(
            finding["severity"].is_string(),
            "each finding has a severity"
        );
    }
}

#[test]
fn contract_severity_filter_works() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--json",
            "--severity",
            "S1",
        ])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let findings = json["findings"].as_array().unwrap();
    for f in findings {
        assert_eq!(f["severity"], "S1", "only S1 findings should appear");
    }
    assert!(!findings.is_empty(), "should have S1 findings");
}

#[test]
fn contract_max_findings_limits_output() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/invalid-operate.tsx",
            "--json",
            "--max-findings",
            "3",
        ])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let findings = json["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 3, "should have at most 3 findings");
}

#[test]
fn lint_compact_format_works() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "lint",
            "tests/fixtures/source/violations",
            "--format",
            "compact",
        ])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(json["status"].is_string());
    assert!(json["counts"].is_object());
}

#[test]
fn lint_severity_filter_works() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "lint",
            "tests/fixtures/source/violations",
            "--json",
            "--severity",
            "S2",
        ])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let findings = json["findings"].as_array().unwrap();
    for f in findings {
        assert_eq!(f["severity"], "S2");
    }
}

#[test]
fn status_no_considered_dir_reports_no_state() {
    // Run from a temp dir with no .considered directory.
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(tmp.path())
        .args(["status", "--json"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["error"], "no surface state found");
}

#[test]
fn status_human_output_no_state() {
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(tmp.path())
        .args(["status"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no surface state found"));
}

#[test]
fn status_reads_public_run_record_and_reports_stale_review() {
    let tmp = tempfile::tempdir().unwrap();
    let runs = tmp.path().join(".considered").join("checkout").join("runs");
    std::fs::create_dir_all(&runs).unwrap();
    let run = serde_json::json!({
        "schemaVersion": "1.0.0",
        "runId": "run-1",
        "surfaceId": "checkout",
        "stage": "review",
        "createdAt": "2026-09-11T00:00:00Z",
        "reviewValidity": {"reviewId": "review-1", "stale": true}
    });
    std::fs::write(
        runs.join("2026-09-11T00-00-00Z.json"),
        serde_json::to_string_pretty(&run).unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(tmp.path())
        .args(["status", "--json"])
        .output()
        .expect("failed to run binary");
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["surface_id"], "checkout");
    assert_eq!(json["review_stale"], true);
    assert!(json["next_action"].as_str().unwrap().contains("gate"));
}

#[test]
fn verify_no_considered_dir_reports_no_state() {
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(tmp.path())
        .args(["verify", "--json"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["error"], "no surface state found");
}

#[test]
fn verify_with_surface_reports_checks() {
    let tmp = tempfile::tempdir().unwrap();
    let considered = tmp.path().join(".considered");
    std::fs::create_dir_all(considered.join("test-surface").join("runs")).unwrap();

    // Write a run record.
    let run = serde_json::json!({
        "schema_version": 1,
        "surface_id": "test-surface",
        "stage": "contract",
        "timestamp": "2026-01-01T00-00-00Z",
        "counts": {},
        "source_hashes": {},
    });
    std::fs::write(
        considered
            .join("test-surface")
            .join("runs")
            .join("2026-01-01T00-00-00Z.json"),
        serde_json::to_string_pretty(&run).unwrap(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(tmp.path())
        .args(["verify", "--json"])
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(json["checks"].is_array());
    assert!(json["surface_id"].as_str() == Some("test-surface"));
    // Some checks should fail (missing artifacts).
    assert_eq!(json["all_pass"], false);
}

#[test]
fn verify_compares_contract_content_to_public_run_record_hash() {
    let tmp = tempfile::tempdir().unwrap();
    let surface = tmp.path().join(".considered").join("checkout");
    let runs = surface.join("runs");
    std::fs::create_dir_all(&runs).unwrap();
    std::fs::write(surface.join("CONTRACT.md"), "hello").unwrap();
    let run = serde_json::json!({
        "schemaVersion": "1.0.0",
        "runId": "run-1",
        "surfaceId": "checkout",
        "stage": "review",
        "createdAt": "2026-09-11T00:00:00Z",
        "reviewValidity": {
            "contractHash": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "stale": false
        }
    });
    std::fs::write(
        runs.join("run-1.json"),
        serde_json::to_string(&run).unwrap(),
    )
    .unwrap();

    let execute = || {
        Command::new(env!("CARGO_BIN_EXE_considered-rs"))
            .current_dir(tmp.path())
            .args(["verify", "checkout", "--json"])
            .output()
            .unwrap()
    };
    let matching: serde_json::Value = serde_json::from_slice(&execute().stdout).unwrap();
    let matching_check = matching["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "contract-sync")
        .unwrap();
    assert_eq!(matching_check["pass"], true);

    std::fs::write(surface.join("CONTRACT.md"), "changed").unwrap();
    let changed: serde_json::Value = serde_json::from_slice(&execute().stdout).unwrap();
    let changed_check = changed["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "contract-sync")
        .unwrap();
    assert_eq!(changed_check["pass"], false);
    assert!(changed_check["detail"]
        .as_str()
        .unwrap()
        .contains("changed"));
}

#[test]
fn context_requires_experimental_flag() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args(["context"])
        .output()
        .expect("failed to run binary");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("experimental"));
}

#[test]
fn context_with_experimental_flag_works() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args(["context", "--experimental", "--json"])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(json["engine_version"].is_string());
    assert!(json["commands"].is_object());
    assert!(json["skills"].is_array());
    assert!(json["templates"].is_array());
}

#[test]
fn gate_compact_format_works() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "gate",
            "tests/fixtures/gate/contract.json",
            "--review",
            "tests/fixtures/gate/review-ship.json",
            "--format",
            "compact",
        ])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["status"], "ship");
    assert!(json["counts"].is_object());
}

#[test]
fn inventory_compact_format_works() {
    let root = project_root();
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "inventory",
            "tests/fixtures/projects/system",
            "--format",
            "compact",
        ])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(json["status"].is_string());
    assert!(json["files"].is_number());
}

#[test]
fn backward_compat_json_flag_still_works() {
    let root = project_root();
    // --json should still work as before (equivalent to --format json).
    let output = Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .current_dir(&root)
        .args([
            "contract",
            "tests/fixtures/contracts/valid-operate.tsx",
            "--json",
        ])
        .output()
        .expect("failed to run binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(json["findings"].as_array().unwrap().len(), 0);
}
