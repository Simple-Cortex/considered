//! Durable, local cache for indexed design-system components.
//!
//! The index remains small so ordinary searches do not need to read component
//! documents. Detail documents are immutable and addressed by their SHA-256.

use crate::cache::{hash_file_content, sha256_hex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component as PathComponent, Path, PathBuf};
use thiserror::Error;

const INDEX_NAME: &str = "index.json";
const MAX_INDEX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DETAIL_BYTES: u64 = 4 * 1024 * 1024;
/// Names listed in `coverage.oversized`; the rest are only counted.
const MAX_OVERSIZED_NAMES: usize = 10;

/// Errors returned by design-system cache operations.
#[derive(Debug, Error)]
pub enum DesignSystemError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid design-system JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid design-system catalog: {0}")]
    Invalid(String),
    #[error("requested response budget is too small for its envelope")]
    BudgetTooSmall,
}

/// Search and response-shaping options for [`query`].
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Free-text search over component names and purposes.
    pub query: String,
    /// Exact component names to include; empty means no name filter.
    pub names: Vec<String>,
    /// Top-level fields to return. Empty selects catalog metadata fields.
    pub fields: Vec<String>,
    /// Maximum number of results.
    pub limit: usize,
    /// Maximum serialized response size in bytes. Zero means no budget.
    pub max_bytes: usize,
    /// Include the full selected detail document.
    pub details: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    name: String,
    resolved_version: String,
    source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Catalog {
    schema_version: u64,
    identity: Identity,
    #[serde(default)]
    indexed_at: String,
    #[serde(default)]
    coverage: Value,
    #[serde(default)]
    source_fingerprints: Vec<Fingerprint>,
    components: Vec<Component>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Component {
    name: String,
    #[serde(default)]
    purpose: String,
    detail_path: String,
    #[serde(default)]
    coverage_status: String,
    #[serde(default)]
    freshness: Value,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Fingerprint {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    fingerprint: String,
    #[serde(default)]
    observed_at: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// Import a normalized catalog and its referenced component documents.
///
/// # Errors
/// Returns an error for invalid schema, identities, paths, JSON, or I/O.
pub fn import_cache(source_index: &Path, workspace: &Path) -> Result<Value, DesignSystemError> {
    let source_index = fs::canonicalize(source_index)?;
    let source_root = source_index
        .parent()
        .ok_or_else(|| DesignSystemError::Invalid("catalog has no parent directory".into()))?;
    let raw = read_bounded(&source_index, MAX_INDEX_BYTES)?;
    let mut catalog: Catalog = serde_json::from_slice(&raw)?;
    validate_catalog(&catalog)?;
    normalize_catalog_fingerprints(&mut catalog);

    let slug = safe_slug(&catalog.identity.name);
    let version = safe_slug(&catalog.identity.resolved_version);
    fs::create_dir_all(workspace)?;
    let workspace_root = fs::canonicalize(workspace)?;
    let cache_dir = workspace_root
        .join(".considered/cache/design-systems")
        .join(slug)
        .join(version);
    ensure_workspace_directory(&workspace_root, &cache_dir)?;
    let details_dir = cache_dir.join("details");
    ensure_workspace_directory(&workspace_root, &details_dir)?;

    let index_path = cache_dir.join(INDEX_NAME);
    let existing = load_existing_catalog(&index_path, &catalog.identity)?;
    let incoming_names: HashSet<String> = catalog
        .components
        .iter()
        .map(|item| item.name.clone())
        .collect();
    if let Some(previous) = &existing {
        if catalog.coverage.get("status").and_then(Value::as_str) != Some("complete") {
            for previous_component in &previous.components {
                if !incoming_names.contains(&previous_component.name) {
                    let mut preserved = previous_component.clone();
                    mark_freshness_unknown(&mut preserved);
                    catalog.components.push(preserved);
                }
            }
        }
        merge_catalog_fingerprints(&mut catalog, previous);
    }

    let mut imported = 0usize;
    let mut reused = 0usize;
    for component in &mut catalog.components {
        if !incoming_names.contains(&component.name) {
            continue;
        }
        let invalidate_cached = component
            .extra
            .remove("invalidate_cached_detail")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if invalidate_cached {
            clear_freshness_evidence(component);
        }
        let previous = existing.as_ref().and_then(|old| {
            old.components
                .iter()
                .find(|item| item.name == component.name)
        });
        if component.detail_path.is_empty() {
            if let Some(previous) = previous.filter(|_| !invalidate_cached) {
                preserve_cached_detail(component, previous);
                continue;
            }
            if matches!(component.coverage_status.as_str(), "partial" | "unreviewed") {
                continue;
            }
            return Err(DesignSystemError::Invalid(format!(
                "complete component {} has no detail path",
                component.name
            )));
        }
        let src = match safe_join(source_root, &component.detail_path) {
            Ok(path) => path,
            Err(DesignSystemError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound
                    && matches!(component.coverage_status.as_str(), "partial" | "unreviewed") =>
            {
                if let Some(previous) = previous.filter(|_| !invalidate_cached) {
                    preserve_cached_detail(component, previous);
                } else {
                    component.detail_path.clear();
                }
                continue;
            }
            Err(error) => return Err(error),
        };
        let bytes = read_bounded(&src, MAX_DETAIL_BYTES)?;
        let detail: Value = serde_json::from_slice(&bytes)?;
        validate_detail(&detail, &catalog.identity, &component.name)?;
        merge_detail_fingerprints(component, &detail);
        let digest = content_digest(&bytes);
        let relative = format!("details/{digest}.json");
        let target = details_dir.join(format!("{digest}.json"));
        if target.exists() {
            let existing = safe_join(&cache_dir, &relative)?;
            if content_digest(&read_bounded(&existing, MAX_DETAIL_BYTES)?) != digest {
                return Err(DesignSystemError::Invalid(format!(
                    "cached detail digest mismatch for {}",
                    component.name
                )));
            }
            reused += 1;
        } else {
            atomic_write_new(&target, &bytes)?;
            imported += 1;
        }
        component.detail_path = relative;
    }

    normalize_catalog_fingerprints(&mut catalog);
    let index_bytes = serde_json::to_vec_pretty(&catalog)?;
    let index_exists = match fs::symlink_metadata(&index_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(DesignSystemError::Invalid(
                "cache index may not be a symlink".into(),
            ));
        }
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if index_exists && read_bounded(&index_path, MAX_INDEX_BYTES)? == index_bytes {
        return Ok(
            json!({"index": index_path, "identity": catalog.identity, "components": catalog.components.len(), "imported": imported, "reused": reused, "unchanged": true}),
        );
    }
    atomic_replace(&index_path, &index_bytes)?;
    prune_unreferenced_details(&details_dir, &index_path, &catalog);
    Ok(
        json!({"index": index_path, "identity": catalog.identity, "components": catalog.components.len(), "imported": imported, "reused": reused, "unchanged": false}),
    )
}

/// Search the compact index, loading detail files only for selected results.
///
/// # Errors
/// Returns an error for invalid indexes, unsafe paths, unreadable details, or
/// a response budget too small to hold the required JSON envelope.
pub fn query(index: &Path, options: &QueryOptions) -> Result<Value, DesignSystemError> {
    let (catalog, root, _) = load_catalog(index)?;
    let fields = validate_fields(&options.fields)?;
    let query = options.query.to_lowercase();
    let name_filter: HashSet<&str> = options.names.iter().map(String::as_str).collect();
    let mut candidates: Vec<(&Component, u8)> = catalog
        .components
        .iter()
        .filter_map(|component| {
            if !name_filter.is_empty() && !name_filter.contains(component.name.as_str()) {
                return None;
            }
            let name = component.name.to_lowercase();
            let purpose = component.purpose.to_lowercase();
            let rank = if !query.is_empty() && name == query {
                0
            } else if !query.is_empty() && name.starts_with(&query) {
                1
            } else if query.is_empty() || name.contains(&query) || purpose.contains(&query) {
                2
            } else {
                return None;
            };
            Some((component, rank))
        })
        .collect();
    candidates.sort_by(|(a, ar), (b, br)| ar.cmp(br).then_with(|| a.name.cmp(&b.name)));

    let total_matches = catalog
        .components
        .iter()
        .filter(|c| {
            (name_filter.is_empty() || name_filter.contains(c.name.as_str()))
                && (query.is_empty()
                    || c.name.to_lowercase().contains(&query)
                    || c.purpose.to_lowercase().contains(&query))
        })
        .count();

    let limit = if options.limit == 0 {
        candidates.len()
    } else {
        options.limit
    };
    let mut results: Vec<Value> = Vec::new();
    let mut reads: Vec<bool> = Vec::new();
    let mut oversized: Vec<String> = Vec::new();
    let mut oversized_count = 0usize;
    let mut stopped = false;
    let mut items_bytes = 0usize;
    let mut detail_files_read = 0usize;
    for (component, _) in candidates.into_iter().take(limit) {
        let metadata_fields = [
            "name",
            "purpose",
            "detail_path",
            "coverage_status",
            "freshness",
        ];
        let selected: Vec<&str> = if fields.is_empty() {
            vec!["name", "purpose", "coverage_status", "freshness"]
        } else {
            fields.iter().map(String::as_str).collect()
        };
        let needs_detail = options.details
            || selected
                .iter()
                .any(|field| !metadata_fields.contains(field));
        let mut did_read = false;
        let detail = if needs_detail && !component.detail_path.is_empty() {
            match read_detail(&root, &component.detail_path) {
                Ok(bytes) => {
                    let detail: Value = serde_json::from_slice(&bytes)?;
                    validate_detail(&detail, &catalog.identity, &component.name)?;
                    did_read = true;
                    Some(detail)
                }
                Err(DesignSystemError::Io(error))
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    None
                }
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        let mut item = Map::new();
        let metadata = serde_json::to_value(component)?;
        let metadata_map = metadata
            .as_object()
            .ok_or_else(|| DesignSystemError::Invalid("component is not an object".into()))?;
        for field in &selected {
            if let Some(value) = metadata_map.get(*field) {
                item.insert((*field).into(), value.clone());
            } else if let Some(value) = detail.as_ref().and_then(|doc| doc.get(*field)) {
                item.insert((*field).into(), value.clone());
            }
        }
        if needs_detail && detail.is_none() {
            item.insert("detail_missing".into(), json!(true));
        }
        if options.details {
            if let Some(detail) = detail {
                item.insert("detail".into(), detail);
            } else {
                item.insert("detail_missing".into(), json!(true));
            }
        }
        let item = Value::Object(item);
        if options.max_bytes > 0 {
            let item_len = serde_json::to_vec(&item)?.len();
            // Oversized means: the envelope plus this item alone cannot fit.
            let alone = estimated_len(
                &catalog,
                total_matches,
                1,
                usize::from(did_read),
                0,
                item_len,
            )?;
            if alone > options.max_bytes {
                oversized_count += 1;
                if oversized.len() < MAX_OVERSIZED_NAMES {
                    oversized.push(component.name.clone());
                }
                continue;
            }
            let with_item = estimated_len(
                &catalog,
                total_matches,
                results.len() + 1,
                detail_files_read + usize::from(did_read),
                oversized_count,
                items_bytes + item_len,
            )?;
            if with_item > options.max_bytes {
                // It fits alone but not after earlier results: stop here.
                stopped = true;
                break;
            }
            items_bytes += item_len;
        }
        results.push(item);
        reads.push(did_read);
        detail_files_read += usize::from(did_read);
    }
    let mut response = query_response(
        &catalog,
        total_matches,
        &results,
        detail_files_read,
        results.len() < total_matches || oversized_count > 0 || stopped,
        &oversized,
        oversized_count,
    );
    while options.max_bytes > 0
        && serde_json::to_vec(&response)?.len().saturating_add(1) > options.max_bytes
    {
        // The bounded oversized report never costs results: shed its names first.
        if !oversized.is_empty() {
            oversized.clear();
        } else if results.pop().is_some() {
            detail_files_read -= usize::from(reads.pop().unwrap_or(false));
        } else {
            return Err(DesignSystemError::BudgetTooSmall);
        }
        response = query_response(
            &catalog,
            total_matches,
            &results,
            detail_files_read,
            true,
            &oversized,
            oversized_count,
        );
    }
    Ok(response)
}

/// Serialized size (with trailing newline) of a response holding `returned`
/// results whose JSON bytes sum to `items_bytes`, without serializing them.
fn estimated_len(
    catalog: &Catalog,
    total_matches: usize,
    returned: usize,
    detail_files_read: usize,
    oversized_count: usize,
    items_bytes: usize,
) -> Result<usize, DesignSystemError> {
    let mut envelope = query_response(
        catalog,
        total_matches,
        &[],
        detail_files_read,
        true,
        &[],
        oversized_count,
    );
    envelope["coverage"]["returned"] = json!(returned);
    let base = serde_json::to_vec(&envelope)?.len();
    Ok(base
        .saturating_add(items_bytes)
        .saturating_add(returned.saturating_sub(1))
        .saturating_add(1))
}

fn query_response(
    catalog: &Catalog,
    total_matches: usize,
    results: &[Value],
    detail_files_read: usize,
    truncated: bool,
    oversized: &[String],
    oversized_count: usize,
) -> Value {
    let mut value = json!({
        "identity": catalog.identity,
        "catalog_coverage": catalog.coverage,
        "results": results,
        "coverage": {
            "total_matches": total_matches,
            "returned": results.len(),
            "truncated": truncated,
            "detail_files_read": detail_files_read,
        }
    });
    if oversized_count > 0 {
        if !oversized.is_empty() {
            value["coverage"]["oversized"] = json!(oversized);
        }
        value["coverage"]["oversized_count"] = json!(oversized_count);
    }
    value
}

/// Compare recorded local source fingerprints with files under `workspace`.
/// URL or missing fingerprints are reported as unknown and never current.
///
/// # Errors
/// Returns an error when the cached index is invalid or cannot be read.
pub fn freshness(index: &Path, workspace: &Path) -> Result<Value, DesignSystemError> {
    let (catalog, _, _) = load_catalog(index)?;
    let mut states = Vec::new();
    let mut fingerprint_cache: BTreeMap<String, Option<String>> = BTreeMap::new();
    for component in &catalog.components {
        let mut evidence = Vec::new();
        let mut fingerprints: Vec<Value> = catalog
            .source_fingerprints
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<_, _>>()?;
        if let Some(values) = component
            .freshness
            .get("source_fingerprints")
            .and_then(Value::as_array)
        {
            fingerprints.extend(values.iter().cloned());
        }
        for fp in &mut fingerprints {
            normalize_fingerprint_value(fp);
        }
        fingerprints.sort_by_key(|fp| {
            (
                fp.get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                fp.get("fingerprint")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            )
        });
        fingerprints.dedup();
        for fp in &fingerprints {
            let path = fp.get("path").and_then(Value::as_str).unwrap_or_default();
            let expected = fp
                .get("fingerprint")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let actual = if path.is_empty() || path.contains("://") || expected.is_empty() {
                None
            } else if let Some(actual) = fingerprint_cache.get(path) {
                actual.clone()
            } else {
                let actual = safe_join(workspace, path)
                    .ok()
                    .and_then(|p| hash_file_content(&p));
                fingerprint_cache.insert(path.to_string(), actual.clone());
                actual
            };
            let status = match actual.as_deref() {
                Some(value) if normalized_hash(value) == normalized_hash(expected) => "current",
                Some(_) => "stale",
                None => "unknown",
            };
            evidence.push(json!({"path":path,"expected":expected,"actual":actual,"status":status}));
        }
        let state = if evidence.iter().any(|e| e["status"] == "stale") {
            "stale"
        } else if evidence.is_empty() || evidence.iter().any(|e| e["status"] == "unknown") {
            "unknown"
        } else {
            "current"
        };
        states.push(json!({"name":component.name,"status":state,"sources":evidence}));
    }
    let resolved_package = if catalog.identity.source.starts_with("npm") {
        let package_path = format!("node_modules/{}/package.json", catalog.identity.name);
        safe_join(workspace, &package_path)
            .ok()
            .and_then(|path| read_bounded(&path, 1024 * 1024).ok())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|value| {
                value
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
    } else {
        None
    };
    let package_status = catalog
        .identity
        .source
        .starts_with("npm")
        .then(|| match resolved_package.as_deref() {
            Some(version) if version == catalog.identity.resolved_version => "current",
            Some(_) => "stale",
            None => "unknown",
        });
    if let Some(package_status) = package_status {
        for state in &mut states {
            match package_status {
                "stale" => state["status"] = json!("stale"),
                "unknown" if state["status"] == "current" => state["status"] = json!("unknown"),
                _ => {}
            }
        }
    }
    let statuses = states
        .iter()
        .filter(|v| v["status"] == "stale")
        .map(|v| v["name"].clone())
        .collect::<Vec<_>>();
    Ok(
        json!({"identity":catalog.identity,"installed_version":resolved_package,"identity_status":package_status.unwrap_or("not_applicable"),"components":states,"affected":statuses,"summary":{"current":states.iter().filter(|v| v["status"]=="current").count(),"stale":states.iter().filter(|v| v["status"]=="stale").count(),"unknown":states.iter().filter(|v| v["status"]=="unknown").count()}}),
    )
}

/// Write a stable, full-fidelity snapshot of the exact named components.
///
/// # Errors
/// Returns an error if a requested name is missing or any detail is invalid.
pub fn snapshot(
    index: &Path,
    names: &[String],
    output_dir: &Path,
) -> Result<Value, DesignSystemError> {
    let (catalog, root, index_digest) = load_catalog(index)?;
    let mut details = Vec::with_capacity(names.len());
    let mut aggregate_bytes = 0u64;
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name.as_str()) {
            return Err(DesignSystemError::Invalid(format!(
                "duplicate requested component: {name}"
            )));
        }
        let component = catalog
            .components
            .iter()
            .find(|item| item.name == *name)
            .ok_or_else(|| DesignSystemError::Invalid(format!("unknown component: {name}")))?;
        if component.detail_path.is_empty() {
            return Err(DesignSystemError::Invalid(format!(
                "component {name} has no cached detail"
            )));
        }
        let bytes = read_detail(&root, &component.detail_path)?;
        aggregate_bytes = aggregate_bytes.saturating_add(bytes.len() as u64);
        if aggregate_bytes > 32 * MAX_DETAIL_BYTES {
            return Err(DesignSystemError::Invalid(
                "snapshot exceeds aggregate detail safety limit".into(),
            ));
        }
        let detail: Value = serde_json::from_slice(&bytes)?;
        validate_detail(&detail, &catalog.identity, name)?;
        details.push(json!({"name":name,"detail":detail}));
    }
    let payload = json!({"schema_version":1,"identity":catalog.identity,"components":details,"provenance":{"index_sha256":index_digest,"files":details.len()}});
    let bytes = serde_json::to_vec_pretty(&payload)?;
    let digest = content_digest(&bytes);
    fs::create_dir_all(output_dir)?;
    let path = output_dir.join(format!("snapshot-{digest}.json"));
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(DesignSystemError::Invalid(
                "snapshot path may not be a symlink".into(),
            ));
        }
        Ok(_) => {
            if read_bounded(
                &path,
                (MAX_DETAIL_BYTES * names.len() as u64).max(MAX_DETAIL_BYTES),
            )? != bytes
            {
                return Err(DesignSystemError::Invalid(
                    "snapshot digest collision or modified snapshot".into(),
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            atomic_write_new(&path, &bytes)?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(json!({"path":path,"sha256":digest,"components":names}))
}

fn validate_catalog(catalog: &Catalog) -> Result<(), DesignSystemError> {
    if catalog.schema_version != 1 {
        return Err(DesignSystemError::Invalid(
            "schema_version must be 1".into(),
        ));
    }
    for (label, value) in [
        ("identity.name", catalog.identity.name.as_str()),
        (
            "identity.resolved_version",
            catalog.identity.resolved_version.as_str(),
        ),
        ("identity.source", catalog.identity.source.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(DesignSystemError::Invalid(format!(
                "{label} must be non-empty"
            )));
        }
    }
    validate_identity_name(&catalog.identity.name)?;
    if catalog
        .identity
        .resolved_version
        .chars()
        .any(char::is_control)
    {
        return Err(DesignSystemError::Invalid("unsafe resolved version".into()));
    }
    let mut names = HashSet::new();
    for component in &catalog.components {
        if component.name.trim().is_empty() || !names.insert(component.name.as_str()) {
            return Err(DesignSystemError::Invalid(format!(
                "empty or duplicate component name: {}",
                component.name
            )));
        }
        if !matches!(
            component.coverage_status.as_str(),
            "complete" | "partial" | "unreviewed"
        ) {
            return Err(DesignSystemError::Invalid(format!(
                "invalid coverage_status for {}",
                component.name
            )));
        }
        if component.detail_path.is_empty()
            && !matches!(component.coverage_status.as_str(), "partial" | "unreviewed")
        {
            return Err(DesignSystemError::Invalid(format!(
                "missing detail path for reviewed component {}",
                component.name
            )));
        }
        if !component.detail_path.is_empty() {
            validate_relative(&component.detail_path)?;
        }
    }
    Ok(())
}

fn load_existing_catalog(
    index_path: &Path,
    identity: &Identity,
) -> Result<Option<Catalog>, DesignSystemError> {
    match fs::symlink_metadata(index_path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(DesignSystemError::Invalid(
                "cache index may not be a symlink".into(),
            ));
        }
        Ok(_) => {}
    }
    let bytes = read_bounded(index_path, MAX_INDEX_BYTES)?;
    let catalog: Catalog = serde_json::from_slice(&bytes)?;
    validate_catalog(&catalog)?;
    if &catalog.identity != identity {
        return Err(DesignSystemError::Invalid(
            "existing cache identity differs from requested source identity".into(),
        ));
    }
    let root = index_path
        .parent()
        .ok_or_else(|| DesignSystemError::Invalid("cache index has no parent".into()))?;
    for component in &catalog.components {
        if component.detail_path.is_empty() {
            continue;
        }
        let relative = Path::new(&component.detail_path);
        let stem = relative.file_stem().and_then(|value| value.to_str());
        if relative.parent() != Some(Path::new("details"))
            || !stem.is_some_and(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err(DesignSystemError::Invalid(format!(
                "cached component {} does not reference a content-addressed detail",
                component.name
            )));
        }
        let bytes = read_detail(root, &component.detail_path)?;
        let detail: Value = serde_json::from_slice(&bytes)?;
        validate_detail(&detail, identity, &component.name)?;
    }
    Ok(Some(catalog))
}

/// Delete content-addressed detail files no longer referenced by the index.
/// Best effort; only regular files directly inside `details_dir` are touched
/// (symlinks are never followed or removed through).
fn prune_unreferenced_details(details_dir: &Path, index_path: &Path, catalog: &Catalog) {
    let mut referenced: HashSet<String> = HashSet::new();
    add_detail_references(&mut referenced, catalog);
    // Union the live index as well: a concurrent import may have replaced it.
    if let Some(live) = read_bounded(index_path, MAX_INDEX_BYTES)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Catalog>(&bytes).ok())
    {
        add_detail_references(&mut referenced, &live);
    }
    let Ok(entries) = fs::read_dir(details_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let stem = name.strip_suffix(".json").unwrap_or("");
        let is_cas = stem.len() == 64 && stem.bytes().all(|b| b.is_ascii_hexdigit());
        if is_cas && !referenced.contains(&name) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Collect `details/<file>` references by path components, so non-canonical
/// spellings such as `details//<hash>.json` count as references.
fn add_detail_references(referenced: &mut HashSet<String>, catalog: &Catalog) {
    for component in &catalog.components {
        let path = Path::new(&component.detail_path);
        if path.parent() == Some(Path::new("details")) {
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                referenced.insert(name.to_owned());
            }
        }
    }
}

fn mark_freshness_unknown(component: &mut Component) {
    if !component.freshness.is_object() {
        component.freshness = json!({});
    }
    if let Some(freshness) = component.freshness.as_object_mut() {
        freshness.insert("status".into(), json!("unknown"));
        freshness
            .entry("source_fingerprints")
            .or_insert_with(|| json!([]));
    }
}

fn clear_freshness_evidence(component: &mut Component) {
    if !component.freshness.is_object() {
        component.freshness = json!({});
    }
    if let Some(freshness) = component.freshness.as_object_mut() {
        freshness.insert("status".into(), json!("unknown"));
        freshness.insert("source_fingerprints".into(), json!([]));
    }
}

fn preserve_cached_detail(component: &mut Component, previous: &Component) {
    component.detail_path.clone_from(&previous.detail_path);
    if !component.freshness.is_object() {
        component.freshness = json!({});
    }
    let previous_fingerprints = previous
        .freshness
        .get("source_fingerprints")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(freshness) = component.freshness.as_object_mut() {
        let fingerprints = freshness
            .entry("source_fingerprints")
            .or_insert_with(|| json!([]));
        if !fingerprints.is_array() {
            *fingerprints = json!([]);
        }
        if let Some(values) = fingerprints.as_array_mut() {
            for fingerprint in previous_fingerprints {
                if !values.contains(&fingerprint) {
                    values.push(fingerprint);
                }
            }
        }
    }
    mark_freshness_unknown(component);
}

fn merge_catalog_fingerprints(incoming: &mut Catalog, previous: &Catalog) {
    if incoming.coverage.get("status").and_then(Value::as_str) == Some("complete") {
        return;
    }
    for fingerprint in &previous.source_fingerprints {
        if let Ok(value) = serde_json::to_value(fingerprint) {
            let already_present = incoming
                .source_fingerprints
                .iter()
                .filter_map(|entry| serde_json::to_value(entry).ok())
                .any(|entry| entry == value);
            if !already_present {
                incoming.source_fingerprints.push(fingerprint.clone());
            }
        }
    }
}

fn validate_detail(
    detail: &Value,
    identity: &Identity,
    component: &str,
) -> Result<(), DesignSystemError> {
    if detail.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err(DesignSystemError::Invalid(format!(
            "detail for {component} has unsupported schema"
        )));
    }
    let detail_identity = detail.get("identity").ok_or_else(|| {
        DesignSystemError::Invalid(format!("detail for {component} has no identity"))
    })?;
    if detail_identity.get("design_system").and_then(Value::as_str) != Some(identity.name.as_str())
        || detail_identity
            .get("resolved_version")
            .and_then(Value::as_str)
            != Some(identity.resolved_version.as_str())
        || detail_identity.get("component").and_then(Value::as_str) != Some(component)
    {
        return Err(DesignSystemError::Invalid(format!(
            "detail identity mismatch for {component}"
        )));
    }
    Ok(())
}

/// Normalize a fingerprint path by stripping leading `./`. Everything else
/// (absolute, `..`, URLs) is left as-is: freshness reports those `unknown`
/// and never reads outside the project root.
fn normalize_fingerprint_path(path: &str) -> &str {
    let mut trimmed = path;
    while let Some(rest) = trimmed.strip_prefix("./") {
        trimmed = rest;
    }
    trimmed
}

fn normalize_fingerprint_value(value: &mut Value) {
    if let Some(path) = value.get("path").and_then(Value::as_str) {
        let normalized = normalize_fingerprint_path(path).to_owned();
        value["path"] = json!(normalized);
    }
}

/// Normalize and de-duplicate every fingerprint path in the catalog, including
/// entries merged or preserved from a previous index.
fn normalize_catalog_fingerprints(catalog: &mut Catalog) {
    let mut seen: Vec<Value> = Vec::new();
    let mut unique = Vec::new();
    for mut fingerprint in std::mem::take(&mut catalog.source_fingerprints) {
        fingerprint.path = normalize_fingerprint_path(&fingerprint.path).to_owned();
        let Ok(value) = serde_json::to_value(&fingerprint) else {
            unique.push(fingerprint);
            continue;
        };
        if !seen.contains(&value) {
            seen.push(value);
            unique.push(fingerprint);
        }
    }
    catalog.source_fingerprints = unique;
    for component in &mut catalog.components {
        if let Some(values) = component
            .freshness
            .get_mut("source_fingerprints")
            .and_then(Value::as_array_mut)
        {
            let mut unique: Vec<Value> = Vec::new();
            for mut value in std::mem::take(values) {
                normalize_fingerprint_value(&mut value);
                if !unique.contains(&value) {
                    unique.push(value);
                }
            }
            *values = unique;
        }
    }
}

fn merge_detail_fingerprints(component: &mut Component, detail: &Value) {
    let Some(sources) = detail
        .pointer("/provenance/sources")
        .and_then(Value::as_array)
    else {
        return;
    };
    let mut sources = sources.clone();
    for source in &mut sources {
        normalize_fingerprint_value(source);
    }
    if !component.freshness.is_object() {
        component.freshness = json!({"status":"unknown","checked_at":""});
    }
    let Some(freshness) = component.freshness.as_object_mut() else {
        return;
    };
    let fingerprints = freshness
        .entry("source_fingerprints")
        .or_insert_with(|| json!([]));
    if !fingerprints.is_array() {
        *fingerprints = json!([]);
    }
    if let Some(values) = fingerprints.as_array_mut() {
        for source in sources {
            if !values.contains(&source) {
                values.push(source);
            }
        }
    }
}

fn validate_fields(fields: &[String]) -> Result<Vec<String>, DesignSystemError> {
    let allowed = [
        "name",
        "purpose",
        "detail_path",
        "coverage_status",
        "freshness",
        "constraints",
        "project_overrides",
        "usage",
        "api",
        "accessibility",
        "provenance",
    ];
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for field in fields {
        if !allowed.contains(&field.as_str()) {
            return Err(DesignSystemError::Invalid(format!(
                "unsupported projection field: {field}"
            )));
        }
        if seen.insert(field.as_str()) {
            result.push(field.clone());
        }
    }
    Ok(result)
}

fn load_catalog(index: &Path) -> Result<(Catalog, PathBuf, String), DesignSystemError> {
    let canonical = fs::canonicalize(index)?;
    let root = canonical
        .parent()
        .ok_or_else(|| DesignSystemError::Invalid("index has no parent".into()))?
        .to_path_buf();
    let index_bytes = read_bounded(&canonical, MAX_INDEX_BYTES)?;
    let digest = content_digest(&index_bytes);
    let catalog: Catalog = serde_json::from_slice(&index_bytes)?;
    validate_catalog(&catalog)?;
    Ok((catalog, root, digest))
}

fn ensure_workspace_directory(
    workspace_root: &Path,
    target: &Path,
) -> Result<(), DesignSystemError> {
    let relative = target
        .strip_prefix(workspace_root)
        .map_err(|_| DesignSystemError::Invalid("cache path escapes workspace".into()))?;
    let mut current = workspace_root.to_path_buf();
    for part in relative.components() {
        let PathComponent::Normal(name) = part else {
            return Err(DesignSystemError::Invalid(
                "unsafe cache directory path".into(),
            ));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(DesignSystemError::Invalid(
                    "cache directories may not be symlinks".into(),
                ))
            }
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                return Err(DesignSystemError::Invalid(
                    "cache path component is not a directory".into(),
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match fs::create_dir(&current) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error.into()),
                }
                let meta = fs::symlink_metadata(&current)?;
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err(DesignSystemError::Invalid(
                        "cache directory changed during creation".into(),
                    ));
                }
            }
            Err(error) => return Err(error.into()),
        }
        let resolved = fs::canonicalize(&current)?;
        if !resolved.starts_with(workspace_root) {
            return Err(DesignSystemError::Invalid(
                "cache directory escapes workspace through a symlink".into(),
            ));
        }
    }
    Ok(())
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, DesignSystemError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(DesignSystemError::Invalid("expected a regular file".into()));
    }
    if metadata.len() > limit {
        return Err(DesignSystemError::Invalid(format!(
            "file exceeds {} byte safety limit",
            limit
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(DesignSystemError::Invalid(format!(
            "file exceeds {} byte safety limit",
            limit
        )));
    }
    Ok(bytes)
}

fn validate_relative(path: &str) -> Result<(), DesignSystemError> {
    let parsed = Path::new(path);
    if parsed.as_os_str().is_empty()
        || parsed.is_absolute()
        || parsed
            .components()
            .any(|part| !matches!(part, PathComponent::Normal(_)))
    {
        return Err(DesignSystemError::Invalid(format!(
            "unsafe relative path: {path}"
        )));
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, DesignSystemError> {
    validate_relative(relative)?;
    let root = fs::canonicalize(root)?;
    let joined = root.join(relative);
    let canonical = fs::canonicalize(&joined)?;
    if !canonical.starts_with(&root) {
        return Err(DesignSystemError::Invalid(format!(
            "path escapes its root: {relative}"
        )));
    }
    Ok(canonical)
}

fn read_detail(root: &Path, relative: &str) -> Result<Vec<u8>, DesignSystemError> {
    let path = safe_join(root, relative)?;
    let bytes = read_bounded(&path, MAX_DETAIL_BYTES)?;
    let relative_path = Path::new(relative);
    if relative_path.parent() == Some(Path::new("details")) {
        if let Some(stem) = relative_path.file_stem().and_then(|value| value.to_str()) {
            if stem.len() == 64 && stem.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let actual = content_digest(&bytes);
                if actual != stem.to_ascii_lowercase() {
                    return Err(DesignSystemError::Invalid(format!(
                        "content-addressed detail digest mismatch: {relative}"
                    )));
                }
            }
        }
    }
    Ok(bytes)
}

fn safe_slug(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let digest = content_digest(value.as_bytes());
    format!("{}-{}", slug.trim_matches('-'), &digest[..12])
}

fn validate_identity_name(value: &str) -> Result<(), DesignSystemError> {
    if value.is_empty()
        || value.contains('\\')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(DesignSystemError::Invalid(format!(
            "unsafe design-system identity name: {value}"
        )));
    }
    Ok(())
}

fn normalized_hash(value: &str) -> &str {
    value.strip_prefix("sha256:").unwrap_or(value)
}

fn content_digest(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

fn unique_temp(path: &Path) -> Result<(PathBuf, fs::File), DesignSystemError> {
    let parent = path
        .parent()
        .ok_or_else(|| DesignSystemError::Invalid("target has no parent".into()))?;
    for _ in 0..32 {
        let id = TEMP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let candidate = parent.join(format!(".cache-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(DesignSystemError::Invalid(
        "could not allocate unique temporary file".into(),
    ))
}

static TEMP_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), DesignSystemError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let (temp, mut file) = unique_temp(path)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::hard_link(&temp, path)?;
        fs::remove_file(&temp)?;
        Ok::<(), std::io::Error>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            let existing = read_bounded(path, bytes.len() as u64)?;
            if existing == bytes {
                return Ok(());
            }
            return Err(DesignSystemError::Invalid(
                "existing content-addressed file has different contents".into(),
            ));
        }
        return Err(error.into());
    }
    Ok(())
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), DesignSystemError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let (temp, mut file) = unique_temp(path)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn fixture(dir: &Path) -> (PathBuf, PathBuf) {
        let source = dir.join("source");
        fs::create_dir_all(source.join("components")).unwrap();
        let detail = json!({"schema_version":1,"identity":{"design_system":"demo","resolved_version":"1.0","component":"Button"},"purpose":"press things","constraints":[],"project_overrides":[],"usage":{"use_for":[],"avoid_when":[],"examples":[]},"api":{},"accessibility":{},"provenance":{"sources":[],"indexed_at":"","freshness":{"status":"unknown","checked_at":""}}});
        fs::write(
            source.join("components/button.json"),
            serde_json::to_vec(&detail).unwrap(),
        )
        .unwrap();
        let catalog = json!({"schema_version":1,"identity":{"name":"demo","resolved_version":"1.0","source":"npm"},"components":[{"name":"Button","purpose":"press things","detail_path":"components/button.json","coverage_status":"complete","freshness":{"status":"unknown"}}]});
        let index = source.join("index.json");
        fs::write(&index, serde_json::to_vec(&catalog).unwrap()).unwrap();
        (index, dir.join("workspace"))
    }

    #[test]
    fn import_reuses_content_and_query_returns_detail_on_demand() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let first = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(first["index"].as_str().unwrap());
        let second = import_cache(&source, &workspace).unwrap();
        assert_eq!(second["reused"], 1);
        let found = query(
            &index,
            &QueryOptions {
                query: "but".into(),
                details: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found["results"][0]["name"], "Button");
        assert_eq!(
            found["results"][0]["detail"]["identity"]["component"],
            "Button"
        );
        let metadata = query(&index, &QueryOptions::default()).unwrap();
        assert_eq!(metadata["coverage"]["detail_files_read"], 0);
    }

    #[test]
    fn query_and_snapshot_reject_tampered_content_addressed_detail() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let catalog: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        let detail_path = index
            .parent()
            .unwrap()
            .join(catalog["components"][0]["detail_path"].as_str().unwrap());
        let mut detail: Value = serde_json::from_slice(&fs::read(&detail_path).unwrap()).unwrap();
        detail["purpose"] = json!("tampered but identity still matches");
        fs::write(&detail_path, serde_json::to_vec(&detail).unwrap()).unwrap();
        assert!(query(
            &index,
            &QueryOptions {
                details: true,
                ..Default::default()
            }
        )
        .is_err());
        assert!(snapshot(
            &index,
            &["Button".to_string()],
            &dir.path().join("snapshots")
        )
        .is_err());
    }

    #[test]
    fn import_rejects_traversal_and_mismatched_identity() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let mut data: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        data["components"][0]["detail_path"] = json!("../outside.json");
        fs::write(&source, serde_json::to_vec(&data).unwrap()).unwrap();
        assert!(import_cache(&source, &workspace).is_err());
        data["components"][0]["detail_path"] = json!("components/button.json");
        fs::write(&source, serde_json::to_vec(&data).unwrap()).unwrap();
        let detail_path = source.parent().unwrap().join("components/button.json");
        let mut detail: Value = serde_json::from_slice(&fs::read(&detail_path).unwrap()).unwrap();
        detail["identity"]["component"] = json!("Other");
        fs::write(&detail_path, serde_json::to_vec(&detail).unwrap()).unwrap();
        assert!(import_cache(&source, &workspace).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn query_rejects_symlink_escape_and_budgets_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let mut catalog: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        let external = dir.path().join("external.json");
        fs::write(&external, "{}").unwrap();
        let escape = index.parent().unwrap().join("details/escape.json");
        symlink(external, &escape).unwrap();
        catalog["components"][0]["detail_path"] = json!("details/escape.json");
        fs::write(&index, serde_json::to_vec(&catalog).unwrap()).unwrap();
        assert!(query(
            &index,
            &QueryOptions {
                details: true,
                ..Default::default()
            }
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn import_rejects_cache_directory_symlink_before_writing_outside_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        fs::create_dir_all(&workspace).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, workspace.join(".considered")).unwrap();
        assert!(import_cache(&source, &workspace).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn import_rejects_details_directory_symlink_before_writing_outside_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let cache_parent = workspace
            .join(".considered/cache/design-systems")
            .join(safe_slug("demo"))
            .join(safe_slug("1.0"));
        fs::create_dir_all(&cache_parent).unwrap();
        let outside = dir.path().join("outside-details");
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, cache_parent.join("details")).unwrap();
        assert!(import_cache(&source, &workspace).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }

    #[test]
    fn partial_component_without_detail_stays_queryable_and_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        catalog["components"][0]["coverage_status"] = json!("unreviewed");
        catalog["components"][0]["detail_path"] = json!("");
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        assert_eq!(
            query(&index, &QueryOptions::default()).unwrap()["results"][0]["name"],
            "Button"
        );
        assert_eq!(
            freshness(&index, &workspace).unwrap()["components"][0]["status"],
            "unknown"
        );
    }

    #[test]
    fn partial_import_retains_unmentioned_and_missing_details_until_complete_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let source_root = source.parent().unwrap();
        let mut button_detail: Value =
            serde_json::from_slice(&fs::read(source_root.join("components/button.json")).unwrap())
                .unwrap();
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        catalog["coverage"]["status"] = json!("complete");
        let mut card_detail = button_detail.clone();
        card_detail["identity"]["component"] = json!("Card");
        card_detail["purpose"] = json!("original card details");
        fs::write(
            source_root.join("components/card.json"),
            serde_json::to_vec(&card_detail).unwrap(),
        )
        .unwrap();
        catalog["components"].as_array_mut().unwrap().push(json!({
            "name":"Card",
            "purpose":"card summary",
            "detail_path":"components/card.json",
            "coverage_status":"complete",
            "freshness":{"status":"unknown"}
        }));
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let initial: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        let old_button_detail_path = initial["components"][0]["detail_path"].clone();

        catalog["coverage"]["status"] = json!("partial");
        catalog["components"] = json!([{
            "name":"Button",
            "purpose":"new button summary",
            "detail_path":"",
            "coverage_status":"partial",
            "freshness":{"status":"current"}
        }]);
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        import_cache(&source, &workspace).unwrap();
        let partial: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        assert_eq!(partial["components"].as_array().unwrap().len(), 2);
        let button = partial["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == "Button")
            .unwrap();
        assert_eq!(button["detail_path"], old_button_detail_path);
        assert_eq!(button["freshness"]["status"], "unknown");
        let selected = query(
            &index,
            &QueryOptions {
                names: vec!["Button".into()],
                details: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected["results"][0]["detail"]["purpose"], "press things");

        button_detail["purpose"] = json!("new authoritative button detail");
        fs::write(
            source_root.join("components/button.json"),
            serde_json::to_vec(&button_detail).unwrap(),
        )
        .unwrap();
        catalog["coverage"]["status"] = json!("complete");
        catalog["components"] = json!([{
            "name":"Button",
            "purpose":"final button summary",
            "detail_path":"components/button.json",
            "coverage_status":"complete",
            "freshness":{"status":"unknown"}
        }]);
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        import_cache(&source, &workspace).unwrap();
        let complete: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        assert_eq!(complete["components"].as_array().unwrap().len(), 1);
        let selected = query(
            &index,
            &QueryOptions {
                details: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            selected["results"][0]["detail"]["purpose"],
            "new authoritative button detail"
        );
    }

    #[test]
    fn import_refuses_to_borrow_cache_across_source_identity_change() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let first = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(first["index"].as_str().unwrap());
        let original = fs::read(&index).unwrap();
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        catalog["identity"]["source"] = json!("git");
        catalog["coverage"]["status"] = json!("partial");
        catalog["components"][0]["detail_path"] = json!("");
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        assert!(import_cache(&source, &workspace).is_err());
        assert_eq!(fs::read(index).unwrap(), original);
    }

    #[test]
    fn snapshot_is_content_addressed_and_stable() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let names = vec!["Button".to_string()];
        let out = dir.path().join("snapshots");
        let first = snapshot(&index, &names, &out).unwrap();
        let second = snapshot(&index, &names, &out).unwrap();
        assert_eq!(first["sha256"], second["sha256"]);
        let snapshot_bytes = fs::read(first["path"].as_str().unwrap()).unwrap();
        assert_eq!(first["sha256"], sha256_hex(&snapshot_bytes));
        assert_eq!(
            snapshot_bytes,
            fs::read(second["path"].as_str().unwrap()).unwrap()
        );
    }

    #[test]
    fn freshness_identity_mismatch_marks_components_stale() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let package = workspace.join("node_modules/demo/package.json");
        fs::create_dir_all(package.parent().unwrap()).unwrap();
        fs::write(&package, r#"{"name":"demo","version":"1.0"}"#).unwrap();
        let current = freshness(&index, &workspace).unwrap();
        assert_eq!(current["identity_status"], "current");
        assert_eq!(current["components"][0]["status"], "unknown");
        fs::write(package, r#"{"name":"demo","version":"2.0"}"#).unwrap();
        let report = freshness(&index, &workspace).unwrap();
        assert_eq!(report["identity_status"], "stale");
        assert_eq!(report["components"][0]["status"], "stale");
    }

    #[test]
    fn freshness_marks_changed_local_evidence_stale() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let evidence_path = workspace.join("src/button.rs");
        fs::create_dir_all(evidence_path.parent().unwrap()).unwrap();
        fs::write(&evidence_path, "old").unwrap();
        let fingerprint = hash_file_content(&evidence_path).unwrap();
        let detail_path = source.parent().unwrap().join("components/button.json");
        let mut detail: Value = serde_json::from_slice(&fs::read(&detail_path).unwrap()).unwrap();
        detail["provenance"]["sources"] = json!([{"kind":"library","path":"src/button.rs","fingerprint":fingerprint,"observed_at":""}]);
        fs::write(&detail_path, serde_json::to_vec(&detail).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let cache_index = PathBuf::from(imported["index"].as_str().unwrap());
        let cached: Value = serde_json::from_slice(&fs::read(cache_index).unwrap()).unwrap();
        assert_eq!(
            cached["components"][0]["freshness"]["source_fingerprints"][0]["path"],
            "src/button.rs"
        );
        fs::write(evidence_path, "changed").unwrap();
        let report = freshness(Path::new(imported["index"].as_str().unwrap()), &workspace).unwrap();
        assert_eq!(report["components"][0]["status"], "stale");
    }

    #[test]
    fn exact_budget_edge_keeps_the_results_that_still_fit() {
        let dir = tempfile::tempdir().unwrap();
        let (source, _) = fixture(dir.path());
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        let mut components = Vec::new();
        for name in ["Button", "Dialog", "Input"] {
            let mut component = catalog["components"][0].clone();
            component["name"] = json!(name);
            components.push(component);
        }
        catalog["components"] = json!(components);
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        let full = query(&source, &QueryOptions::default()).unwrap();
        assert_eq!(full["coverage"]["returned"], 3);
        // `false` occupies one byte more than the probe's `true` value.
        let budget = serde_json::to_vec(&full).unwrap().len();
        let bounded = query(
            &source,
            &QueryOptions {
                max_bytes: budget,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(bounded["coverage"]["returned"], 2);
        assert_eq!(bounded["coverage"]["truncated"], true);
        assert!(serde_json::to_vec(&bounded).unwrap().len() < budget);
    }

    #[test]
    fn query_returns_budget_error_when_envelope_cannot_fit() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let error = query(
            &index,
            &QueryOptions {
                max_bytes: 1,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(error, DesignSystemError::BudgetTooSmall));
    }

    /// Write a multi-component catalog with one detail file per component.
    /// `usage_pad` maps a component name to extra bytes of `usage` text.
    fn multi_fixture(dir: &Path, names: &[&str], usage_pad: &[(&str, usize)]) -> PathBuf {
        let source = dir.join("multi-source");
        fs::create_dir_all(source.join("components")).unwrap();
        let mut components = Vec::new();
        for name in names {
            let pad = usage_pad
                .iter()
                .find(|(n, _)| n == name)
                .map_or(0, |(_, size)| *size);
            let detail = json!({"schema_version":1,"identity":{"design_system":"demo","resolved_version":"1.0","component":name},"purpose":"p","constraints":[],"project_overrides":[],"usage":{"use_for":["x".repeat(pad)],"avoid_when":[],"examples":[]},"api":{},"accessibility":{},"provenance":{"sources":[],"indexed_at":"","freshness":{"status":"unknown","checked_at":""}}});
            let rel = format!("components/{}.json", name.to_lowercase());
            fs::write(source.join(&rel), serde_json::to_vec(&detail).unwrap()).unwrap();
            components.push(json!({"name":name,"purpose":"p","detail_path":rel,"coverage_status":"complete","freshness":{"status":"unknown"}}));
        }
        let catalog = json!({"schema_version":1,"identity":{"name":"demo","resolved_version":"1.0","source":"npm"},"coverage":{"status":"complete"},"components":components});
        let index = source.join("index.json");
        fs::write(&index, serde_json::to_vec(&catalog).unwrap()).unwrap();
        index
    }

    #[test]
    fn query_skips_oversized_items_and_reports_them() {
        let dir = tempfile::tempdir().unwrap();
        let source = multi_fixture(dir.path(), &["Big", "Small"], &[("Big", 9000)]);
        let workspace = dir.path().join("workspace");
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let found = query(
            &index,
            &QueryOptions {
                fields: vec!["name".into(), "usage".into()],
                max_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found["coverage"]["returned"], 1);
        assert_eq!(found["results"][0]["name"], "Small");
        assert_eq!(found["coverage"]["truncated"], true);
        assert_eq!(found["coverage"]["oversized"], json!(["Big"]));
        // Only the detail that survived the budget counts as read.
        assert_eq!(found["coverage"]["detail_files_read"], 1);
        assert!(found["coverage"].get("available").is_none());
        assert!(found["coverage"].get("skipped_corrupt").is_none());
        let none = query(
            &index,
            &QueryOptions {
                names: vec!["Big".into()],
                details: true,
                max_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(none["coverage"]["returned"], 0);
        assert_eq!(none["coverage"]["oversized"], json!(["Big"]));
        assert_eq!(none["coverage"]["detail_files_read"], 0);
    }

    #[test]
    fn snapshot_digest_is_invariant_to_index_path_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let parent = index.parent().unwrap();
        let alt = parent.join("details").join("..").join("index.json");
        let names = vec!["Button".to_string()];
        let a = snapshot(&index, &names, &dir.path().join("out-a")).unwrap();
        let b = snapshot(&alt, &names, &dir.path().join("out-b")).unwrap();
        assert_eq!(a["sha256"], b["sha256"]);
    }

    #[test]
    fn fingerprint_paths_with_leading_dot_slash_are_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let evidence = workspace.join("src/button.rs");
        fs::create_dir_all(evidence.parent().unwrap()).unwrap();
        fs::write(&evidence, "body").unwrap();
        let fingerprint = hash_file_content(&evidence).unwrap();
        let detail_path = source.parent().unwrap().join("components/button.json");
        let mut detail: Value = serde_json::from_slice(&fs::read(&detail_path).unwrap()).unwrap();
        detail["provenance"]["sources"] = json!([{"kind":"library","path":"./src/button.rs","fingerprint":fingerprint,"observed_at":""}]);
        fs::write(&detail_path, serde_json::to_vec(&detail).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let report = freshness(Path::new(imported["index"].as_str().unwrap()), &workspace).unwrap();
        assert_eq!(report["components"][0]["sources"][0]["status"], "current");
        assert_eq!(
            report["components"][0]["sources"][0]["path"],
            "src/button.rs"
        );

        let mut bad = detail.clone();
        bad["provenance"]["sources"][0]["path"] = json!("../outside.rs");
        fs::write(&detail_path, serde_json::to_vec(&bad).unwrap()).unwrap();
        // Unsafe paths are tolerated at import and reported unknown, never read.
        let imported = import_cache(&source, &workspace).unwrap();
        let report = freshness(Path::new(imported["index"].as_str().unwrap()), &workspace).unwrap();
        assert_eq!(report["components"][0]["sources"][0]["status"], "unknown");
        assert_eq!(
            report["components"][0]["sources"][0]["path"],
            "../outside.rs"
        );
        assert_eq!(report["components"][0]["status"], "unknown");
        let mut abs = detail.clone();
        abs["provenance"]["sources"][0]["path"] = json!(evidence.to_str().unwrap());
        fs::write(&detail_path, serde_json::to_vec(&abs).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let report = freshness(Path::new(imported["index"].as_str().unwrap()), &workspace).unwrap();
        assert_eq!(report["components"][0]["sources"][0]["status"], "unknown");
    }

    #[test]
    fn freshness_dedups_dot_slash_and_plain_fingerprints() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let evidence = workspace.join("a");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(&evidence, "body").unwrap();
        let fp = hash_file_content(&evidence).unwrap();
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        catalog["source_fingerprints"] = json!([{"kind":"library","path":"./a","fingerprint":fp,"observed_at":""},{"kind":"library","path":"a","fingerprint":fp,"observed_at":""}]);
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        // Hand-edit the cached index back to un-normalized, duplicate spellings.
        let mut cached: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        cached["source_fingerprints"] = json!([{"kind":"library","path":"./a","fingerprint":fp,"observed_at":""},{"kind":"library","path":"a","fingerprint":fp,"observed_at":""}]);
        fs::write(&index, serde_json::to_vec(&cached).unwrap()).unwrap();
        let report = freshness(&index, &workspace).unwrap();
        assert_eq!(
            report["components"][0]["sources"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    fn partial_import_normalizes_fingerprints_carried_from_previous_index() {
        let dir = tempfile::tempdir().unwrap();
        let source = multi_fixture(dir.path(), &["Alpha", "Beta"], &[]);
        let workspace = dir.path().join("workspace");
        let first = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(first["index"].as_str().unwrap());
        let mut cached: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        let old = json!({"kind":"library","path":"./a","fingerprint":"sha256:x","observed_at":""});
        cached["source_fingerprints"] = json!([old.clone()]);
        cached["components"][0]["freshness"]["source_fingerprints"] = json!([old]);
        fs::write(&index, serde_json::to_vec_pretty(&cached).unwrap()).unwrap();
        let mut src: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        src["coverage"] = json!({"status":"partial"});
        src["source_fingerprints"] =
            json!([{"kind":"library","path":"a","fingerprint":"sha256:x","observed_at":""}]);
        src["components"] = json!([src["components"][1].clone()]);
        fs::write(&source, serde_json::to_vec(&src).unwrap()).unwrap();
        import_cache(&source, &workspace).unwrap();
        let after: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        assert_eq!(after["source_fingerprints"].as_array().unwrap().len(), 1);
        assert_eq!(after["source_fingerprints"][0]["path"], "a");
        let alpha = after["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "Alpha")
            .unwrap();
        assert_eq!(alpha["freshness"]["source_fingerprints"][0]["path"], "a");
    }

    #[test]
    fn snapshot_names_component_without_cached_detail() {
        let dir = tempfile::tempdir().unwrap();
        let (source, workspace) = fixture(dir.path());
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        catalog["components"] = json!([{"name":"Ghost","purpose":"p","detail_path":"","coverage_status":"partial","freshness":{"status":"unknown"}}]);
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let error = snapshot(&index, &["Ghost".to_string()], &dir.path().join("out"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("component Ghost has no cached detail"),
            "{error}"
        );
    }

    #[test]
    fn complete_import_prunes_details_of_omitted_components() {
        let dir = tempfile::tempdir().unwrap();
        let source = multi_fixture(dir.path(), &["Alpha", "Beta"], &[]);
        let workspace = dir.path().join("workspace");
        let first = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(first["index"].as_str().unwrap());
        let details = index.parent().unwrap().join("details");
        assert_eq!(fs::read_dir(&details).unwrap().count(), 2);
        let mut catalog: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        catalog["components"] = json!([catalog["components"][0].clone()]);
        fs::write(&source, serde_json::to_vec(&catalog).unwrap()).unwrap();
        import_cache(&source, &workspace).unwrap();
        assert_eq!(fs::read_dir(&details).unwrap().count(), 1);
        let found = query(
            &index,
            &QueryOptions {
                details: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found["coverage"]["returned"], 1);
        assert_eq!(found["coverage"]["detail_files_read"], 1);
    }

    #[test]
    fn many_oversized_items_stay_bounded_and_truncate_instead_of_budget_error() {
        let dir = tempfile::tempdir().unwrap();
        let names: Vec<String> = (0..400).map(|i| format!("Component{i:04}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let pads: Vec<(&str, usize)> = refs.iter().map(|n| (*n, 300usize)).collect();
        let source = multi_fixture(dir.path(), &refs, &pads);
        let workspace = dir.path().join("workspace");
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let found = query(
            &index,
            &QueryOptions {
                fields: vec!["name".into(), "usage".into()],
                max_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(found["coverage"]["returned"].as_u64().unwrap() > 0);
        assert_eq!(found["coverage"]["truncated"], true);
        assert!(found["coverage"].get("oversized").is_none());
        assert!(serde_json::to_vec(&found).unwrap().len() < 4096);
    }

    #[test]
    fn oversized_report_is_bounded_and_counted() {
        let dir = tempfile::tempdir().unwrap();
        let names: Vec<String> = (0..30).map(|i| format!("Big{i:02}")).collect();
        let mut all: Vec<&str> = names.iter().map(String::as_str).collect();
        all.push("Zsmall");
        let pads: Vec<(&str, usize)> = names.iter().map(|n| (n.as_str(), 9000usize)).collect();
        let source = multi_fixture(dir.path(), &all, &pads);
        let workspace = dir.path().join("workspace");
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let found = query(
            &index,
            &QueryOptions {
                fields: vec!["name".into(), "usage".into()],
                max_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found["results"][0]["name"], "Zsmall");
        assert_eq!(found["coverage"]["oversized"].as_array().unwrap().len(), 10);
        assert_eq!(found["coverage"]["oversized_count"], 30);
        assert_eq!(found["coverage"]["truncated"], true);
    }

    #[test]
    fn item_that_fits_alone_after_a_full_page_is_not_labelled_oversized() {
        let dir = tempfile::tempdir().unwrap();
        let source = multi_fixture(
            dir.path(),
            &["A", "B", "C"],
            &[("A", 1500), ("B", 1500), ("C", 1500)],
        );
        let workspace = dir.path().join("workspace");
        let imported = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(imported["index"].as_str().unwrap());
        let found = query(
            &index,
            &QueryOptions {
                fields: vec!["name".into(), "usage".into()],
                max_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found["coverage"]["returned"], 2);
        assert_eq!(found["coverage"]["truncated"], true);
        assert!(found["coverage"].get("oversized").is_none());
        assert!(found["coverage"].get("oversized_count").is_none());
    }

    #[test]
    fn prune_keeps_details_referenced_by_non_canonical_paths() {
        let dir = tempfile::tempdir().unwrap();
        let source = multi_fixture(dir.path(), &["Alpha", "Beta"], &[]);
        let workspace = dir.path().join("workspace");
        let first = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(first["index"].as_str().unwrap());
        let details = index.parent().unwrap().join("details");
        let mut cat: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
        let p = cat["components"][0]["detail_path"]
            .as_str()
            .unwrap()
            .replace("details/", "details//");
        cat["components"][0]["detail_path"] = json!(p);
        fs::write(&index, serde_json::to_vec_pretty(&cat).unwrap()).unwrap();
        let mut src: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        src["coverage"] = json!({"status":"partial"});
        src["components"] = json!([src["components"][1].clone()]);
        fs::write(&source, serde_json::to_vec(&src).unwrap()).unwrap();
        import_cache(&source, &workspace).unwrap();
        assert_eq!(fs::read_dir(&details).unwrap().count(), 2);
        snapshot(&index, &["Alpha".to_string()], &dir.path().join("out")).unwrap();
    }

    #[test]
    fn partial_import_with_prune_preserves_details_of_preserved_components() {
        let dir = tempfile::tempdir().unwrap();
        let source = multi_fixture(dir.path(), &["Alpha", "Beta"], &[]);
        let workspace = dir.path().join("workspace");
        let first = import_cache(&source, &workspace).unwrap();
        let index = PathBuf::from(first["index"].as_str().unwrap());
        let details = index.parent().unwrap().join("details");
        let mut src: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
        src["coverage"] = json!({"status":"partial"});
        src["components"] = json!([src["components"][1].clone()]);
        fs::write(&source, serde_json::to_vec(&src).unwrap()).unwrap();
        import_cache(&source, &workspace).unwrap();
        assert_eq!(fs::read_dir(&details).unwrap().count(), 2);
        snapshot(&index, &["Alpha".to_string()], &dir.path().join("out")).unwrap();
    }
}
