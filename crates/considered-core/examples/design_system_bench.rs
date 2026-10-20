//! Synthetic cache workload. Run with:
//! `cargo run --release -p considered-core --example design_system_bench`.
//! Timings describe this local workload, not documentation fetching or LLM time.

use considered_core::design_system::{freshness, import_cache, query, QueryOptions};
use serde_json::json;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("source");
    fs::create_dir_all(source.join("components"))?;
    let mut components = Vec::with_capacity(1_000);
    let mut detail_bytes = 0usize;
    for number in 0..1_000 {
        let name = format!("Component{number:04}");
        let detail_path = format!("components/{name}.json");
        let detail = json!({
            "schema_version": 1,
            "identity": {"design_system":"bench-ui", "resolved_version":"1.0.0", "component":name},
            "purpose": "Inspect a record while retaining list context",
            "usage": {"use_for":["record inspection"], "avoid_when":[], "examples":[]},
            "api": {"import":"bench-ui", "props":["open", "onClose"]},
            "constraints": ["Requires a labelled trigger"],
            "project_overrides": [],
            "accessibility": {"requirements":["Provide an accessible title"]},
            "provenance": {"sources":[{"kind":"documentation", "path":"https://example.invalid/bench-ui", "fingerprint":"", "observed_at":""}]},
        });
        let bytes = serde_json::to_vec(&detail)?;
        detail_bytes += bytes.len();
        fs::write(source.join(&detail_path), bytes)?;
        components.push(json!({"name":name,"purpose":"Inspect a record while retaining list context", "detail_path":detail_path,"coverage_status":"complete", "freshness":{"status":"unknown","checked_at":"","source_fingerprints":[]}}));
    }
    let input = source.join("index.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "schema_version":1, "identity":{"name":"bench-ui","resolved_version":"1.0.0","source":"synthetic fixture"},
            "indexed_at":"", "source_fingerprints":[],
            "coverage":{"status":"complete","scope":"1000 synthetic exports","evidence":[]},
            "components":components,
        }))?,
    )?;
    let workspace = temporary.path().join("workspace");
    let start = Instant::now();
    let imported = import_cache(&input, &workspace)?;
    let cold_us = start.elapsed().as_micros();
    let index = imported["index"]
        .as_str()
        .ok_or("missing imported index path")?;
    let start = Instant::now();
    let reused = import_cache(&input, &workspace)?;
    let repeat_us = start.elapsed().as_micros();
    let options = QueryOptions {
        query: "record".into(),
        names: vec![],
        fields: vec![],
        limit: 5,
        max_bytes: 4096,
        details: false,
    };
    let mut timings = Vec::with_capacity(50);
    let mut output_bytes = 0usize;
    let mut detail_files_read = json!(null);
    for _ in 0..50 {
        let start = Instant::now();
        let result = query(std::path::Path::new(index), &options)?;
        output_bytes = serde_json::to_vec(std::hint::black_box(&result))?.len() + 1;
        detail_files_read = result["coverage"]["detail_files_read"].clone();
        timings.push(start.elapsed().as_micros());
    }
    timings.sort_unstable();
    let start = Instant::now();
    let fresh = freshness(std::path::Path::new(index), &workspace)?;
    let freshness_us = start.elapsed().as_micros();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "workload":"1000 synthetic components; 50 catalog queries; release build recommended",
            "cold_import_us":cold_us, "repeat_import_us":repeat_us,
            "repeat_import_report":reused,
            "query_median_us":timings[25], "query_p95_us":timings[47],
            "full_detail_bytes":detail_bytes, "bounded_query_bytes":output_bytes,
        "query_detail_files_read":detail_files_read,
        "freshness_us":freshness_us, "freshness_summary":fresh["summary"],
            "max_query_bytes":4096,
        }))?
    );
    Ok(())
}
