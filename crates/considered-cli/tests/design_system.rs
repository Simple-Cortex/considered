use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_considered-rs"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run considered-rs")
}

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().expect("parent directory")).expect("create parent");
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize fixture"),
    )
    .expect("write fixture");
}

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    workspace: PathBuf,
    source_index: PathBuf,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().expect("temporary fixture directory");
    let root = temp.path().join("prepared");
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).expect("create workspace");
    let names = ["Button", "Dialog", "Input"];
    let components = names
        .iter()
        .map(|name| {
            let detail_path = format!("components/{}.json", name.to_lowercase());
            write(
                &root.join(&detail_path),
                &json!({
                    "schema_version": 1,
                    "identity": {
                        "design_system": "demo",
                        "resolved_version": "1.0.0",
                        "component": name
                    },
                    "purpose": format!("{name} purpose"),
                    "constraints": [],
                    "project_overrides": [],
                    "usage": {"use_for": [], "avoid_when": [], "examples": []},
                    "api": {"props": []},
                    "accessibility": {"requirements": []},
                    "provenance": {"sources": [], "indexed_at": "fixture", "freshness": {"status": "unknown"}}
                }),
            );
            json!({
                "name": name,
                "purpose": format!("{name} purpose"),
                "detail_path": detail_path,
                "coverage_status": "partial",
                "freshness": {"status": "unknown"}
            })
        })
        .collect::<Vec<_>>();
    let source_index = root.join("index.json");
    write(
        &source_index,
        &json!({
            "schema_version": 1,
            "identity": {"name": "demo", "resolved_version": "1.0.0", "source": "fixture"},
            "indexed_at": "fixture",
            "coverage": {"status": "partial", "scope": "test", "evidence": []},
            "source_fingerprints": [],
            "components": components
        }),
    );
    Fixture {
        _temp: temp,
        root,
        workspace,
        source_index,
    }
}

fn imported_index(f: &Fixture) -> PathBuf {
    let output = run(
        &[
            "design-system",
            "import",
            f.source_index.to_str().expect("index path"),
            "--workspace",
            f.workspace.to_str().expect("workspace path"),
        ],
        &f.root,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    let response: Value = serde_json::from_slice(&output.stdout).expect("import JSON");
    PathBuf::from(response["index"].as_str().expect("index path in response"))
}

#[test]
fn import_query_freshness_and_snapshot_work_end_to_end() {
    let f = fixture();
    let index = imported_index(&f);

    let query = run(
        &[
            "design-system",
            "query",
            index.to_str().expect("index path"),
            "--component",
            "Button",
            "--fields",
            "name,purpose,api",
            "--details",
        ],
        &f.root,
    );
    assert_eq!(
        query.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&query.stderr)
    );
    assert_eq!(
        query.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    let response: Value = serde_json::from_slice(&query.stdout).expect("query JSON");
    assert_eq!(response["results"][0]["name"], "Button");
    assert_eq!(
        response["results"][0]["detail"]["identity"]["component"],
        "Button"
    );

    let freshness = run(
        &[
            "design-system",
            "freshness",
            index.to_str().expect("index path"),
            "--workspace",
            f.workspace.to_str().expect("workspace path"),
        ],
        &f.root,
    );
    assert_eq!(
        freshness.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&freshness.stderr)
    );
    let response: Value = serde_json::from_slice(&freshness.stdout).expect("freshness JSON");
    assert_eq!(response["summary"]["unknown"], 3);

    let output_dir = f.workspace.join("snapshots");
    let snapshot = run(
        &[
            "design-system",
            "snapshot",
            index.to_str().expect("index path"),
            "--component",
            "Button",
            "--component",
            "Input",
            "--output-dir",
            output_dir.to_str().expect("output directory"),
        ],
        &f.root,
    );
    assert_eq!(
        snapshot.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&snapshot.stderr)
    );
    let response: Value = serde_json::from_slice(&snapshot.stdout).expect("snapshot JSON");
    let snapshot_path = PathBuf::from(response["path"].as_str().expect("snapshot path"));
    let written: Value = serde_json::from_slice(&fs::read(snapshot_path).expect("read snapshot"))
        .expect("snapshot file JSON");
    assert_eq!(
        written["components"].as_array().expect("components").len(),
        2
    );
}

#[test]
fn query_respects_byte_budget_and_errors_exit_two() {
    let f = fixture();
    let index = imported_index(&f);
    let budgeted = run(
        &[
            "design-system",
            "query",
            index.to_str().expect("index path"),
            "--limit",
            "3",
            "--max-bytes",
            "512",
        ],
        &f.root,
    );
    assert_eq!(
        budgeted.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&budgeted.stderr)
    );
    let response: Value = serde_json::from_slice(&budgeted.stdout).expect("budgeted query JSON");
    assert!(
        budgeted.stdout.len() <= 512,
        "response exceeded its byte budget"
    );
    assert_eq!(response["coverage"]["truncated"], true);

    let too_small = run(
        &[
            "design-system",
            "query",
            index.to_str().expect("index path"),
            "--max-bytes",
            "1",
        ],
        &f.root,
    );
    assert_eq!(too_small.status.code(), Some(2));
    assert!(too_small.stdout.is_empty());

    let missing = run(
        &[
            "design-system",
            "query",
            f.workspace
                .join("missing.json")
                .to_str()
                .expect("missing index path"),
        ],
        &f.root,
    );
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());

    let unknown = run(
        &[
            "design-system",
            "snapshot",
            index.to_str().expect("index path"),
            "--component",
            "Unknown",
            "--output-dir",
            f.workspace
                .join("bad-snapshot")
                .to_str()
                .expect("output path"),
        ],
        &f.root,
    );
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
}

#[test]
fn query_skips_oversized_component_and_keeps_later_fits() {
    let f = fixture();
    let dialog = f.root.join("components/button.json");
    let mut detail: Value =
        serde_json::from_slice(&fs::read(&dialog).expect("read detail")).expect("detail JSON");
    detail["usage"]["use_for"] = json!(["x".repeat(9000)]);
    write(&dialog, &detail);
    let index = imported_index(&f);
    let output = run(
        &[
            "design-system",
            "query",
            index.to_str().expect("index path"),
            "--fields",
            "name,usage",
            "--max-bytes",
            "4096",
        ],
        &f.root,
    );
    assert_eq!(output.status.code(), Some(0));
    let response: Value = serde_json::from_slice(&output.stdout).expect("query JSON");
    assert_eq!(response["coverage"]["returned"], 2);
    assert_eq!(response["coverage"]["truncated"], true);
    assert_eq!(response["coverage"]["oversized"], json!(["Button"]));
    assert_eq!(response["coverage"]["detail_files_read"], 2);
}
