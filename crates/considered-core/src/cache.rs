//! Content-hash caching for deterministic command results.
//!
//! Cache key: engine version + rule corpus hash + command + arguments + file content hashes.
//! Cache value: serialized findings JSON.
//! Cache location: `.considered/cache/`.
//! A cache hit is observable via `"cached": true` in structured metadata.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A cache entry stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key components for verification.
    pub key: CacheKey,
    /// Whether this entry was a cache hit when loaded.
    #[serde(default)]
    pub cached: bool,
    /// Serialized findings JSON.
    pub findings_json: String,
    /// Digest of the serialized payload, used to reject corrupted entries.
    pub payload_hash: String,
    /// Timestamp (ISO-8601) when this entry was written.
    pub created_at: String,
}

/// Components that form the cache key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    /// Cache key serialization format. Bumping this invalidates old entries.
    #[serde(default)]
    pub format_version: u8,
    pub engine_version: String,
    pub corpus_hash: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub file_hashes: BTreeMap<String, String>,
}

impl CacheKey {
    /// Compute a deterministic hex digest for this key.
    pub fn digest(&self) -> String {
        // SHA-256 prevents practical collisions from selecting a stale result.
        let mut data = String::new();
        data.push_str(&self.format_version.to_string());
        data.push('|');
        data.push_str(&self.engine_version);
        data.push('|');
        data.push_str(&self.corpus_hash);
        data.push('|');
        data.push_str(&self.command);
        data.push('|');
        for arg in &self.arguments {
            data.push_str(arg);
            data.push('|');
        }
        for (path, hash) in &self.file_hashes {
            data.push_str(path);
            data.push('=');
            data.push_str(hash);
            data.push('|');
        }
        sha256_hex(data.as_bytes())
    }
}

/// Compute a collision-resistant content hash for a file.
pub fn hash_file_content(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    Some(sha256_hex(&data))
}

/// Minimal SHA-256 implementation, kept local to avoid expanding the runtime
/// dependency surface solely for cache filenames.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut bytes = data.to_vec();
    bytes.push(0x80);
    while bytes.len() % 64 != 56 {
        bytes.push(0);
    }
    bytes.extend_from_slice(&bit_len.to_be_bytes());
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in bytes.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes(word.try_into().unwrap_or([0; 4]));
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (value, add) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *value = value.wrapping_add(add);
        }
    }
    use std::fmt::Write;
    let mut output = String::with_capacity(64);
    for value in h {
        let _ = write!(output, "{value:08x}");
    }
    output
}

/// Compute a corpus hash from a directory of rule files.
pub fn hash_corpus(rules_dir: &Path) -> String {
    let mut combined = String::new();
    if let Ok(entries) = fs::read_dir(rules_dir) {
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        paths.sort();
        for path in paths {
            if let Ok(data) = fs::read(&path) {
                combined.push_str(&path.file_name().unwrap_or_default().to_string_lossy());
                combined.push('=');
                combined.push_str(&sha256_hex(&data));
                combined.push('|');
            }
        }
    }
    sha256_hex(combined.as_bytes())
}

/// Look up a cached result. Returns the entry if found and key matches.
pub fn lookup(cache_dir: &Path, key: &CacheKey) -> Option<CacheEntry> {
    let digest = key.digest();
    let path = cache_dir.join(format!("{digest}.json"));
    let data = fs::read_to_string(&path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&data).ok()?;
    // Verify key matches exactly.
    if entry.key == *key && entry.payload_hash == sha256_hex(entry.findings_json.as_bytes()) {
        Some(entry)
    } else {
        None
    }
}

/// Store a result in the cache.
pub fn store(cache_dir: &Path, key: &CacheKey, findings_json: &str) -> std::io::Result<()> {
    fs::create_dir_all(cache_dir)?;
    let digest = key.digest();
    let path = cache_dir.join(format!("{digest}.json"));
    let entry = CacheEntry {
        key: key.clone(),
        cached: false,
        findings_json: findings_json.to_string(),
        payload_hash: sha256_hex(findings_json.as_bytes()),
        created_at: iso_now(),
    };
    let data = serde_json::to_string_pretty(&entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    // Write-then-rename keeps readers from observing a truncated JSON document
    // after a crash or an interrupted concurrent writer.
    let temp = cache_dir.join(format!(".{digest}.{}.tmp", std::process::id()));
    fs::write(&temp, data)?;
    if let Err(error) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

/// Simple ISO-8601 timestamp (UTC).
fn iso_now() -> String {
    // Use a monotonic-ish approach: seconds since epoch via SystemTime.
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Format as basic ISO-8601 (good enough for cache timestamps).
    let days = secs / 86400;
    let time_part = secs % 86400;
    let hours = time_part / 3600;
    let minutes = (time_part % 3600) / 60;
    let seconds = time_part % 60;

    // Convert days since epoch to Y-M-D (simplified).
    let (y, m, d) = days_to_ymd(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_ymd(mut days: i64) -> (i64, i64, i64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_digest_is_deterministic() {
        let key = CacheKey {
            format_version: 1,
            engine_version: "0.6.0".into(),
            corpus_hash: "abc123".into(),
            command: "lint".into(),
            arguments: vec!["src/".into()],
            file_hashes: BTreeMap::new(),
        };
        let d1 = key.digest();
        let d2 = key.digest();
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
    }

    #[test]
    fn different_keys_produce_different_digests() {
        let key1 = CacheKey {
            format_version: 1,
            engine_version: "0.6.0".into(),
            corpus_hash: "abc".into(),
            command: "lint".into(),
            arguments: vec![],
            file_hashes: BTreeMap::new(),
        };
        let key2 = CacheKey {
            format_version: 1,
            engine_version: "0.6.0".into(),
            corpus_hash: "def".into(),
            command: "lint".into(),
            arguments: vec![],
            file_hashes: BTreeMap::new(),
        };
        assert_ne!(key1.digest(), key2.digest());
    }

    #[test]
    fn store_and_lookup_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let key = CacheKey {
            format_version: 1,
            engine_version: "0.6.0".into(),
            corpus_hash: "test".into(),
            command: "lint".into(),
            arguments: vec![".".into()],
            file_hashes: BTreeMap::new(),
        };
        store(&cache_dir, &key, r#"{"findings":[]}"#).unwrap();
        let entry = lookup(&cache_dir, &key).unwrap();
        assert_eq!(entry.findings_json, r#"{"findings":[]}"#);
        assert!(!entry.cached);
    }

    #[test]
    fn lookup_returns_none_for_missing_key() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let key = CacheKey {
            format_version: 1,
            engine_version: "0.6.0".into(),
            corpus_hash: "missing".into(),
            command: "lint".into(),
            arguments: vec![],
            file_hashes: BTreeMap::new(),
        };
        assert!(lookup(&cache_dir, &key).is_none());
    }

    #[test]
    fn lookup_rejects_corrupted_payload() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join("cache");
        let key = CacheKey {
            format_version: 1,
            engine_version: "0.6.0".into(),
            corpus_hash: "test".into(),
            command: "contract".into(),
            arguments: vec![],
            file_hashes: BTreeMap::new(),
        };
        store(&cache_dir, &key, r#"{"findings":[]}"#).unwrap();
        let path = cache_dir.join(format!("{}.json", key.digest()));
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        value["findings_json"] = serde_json::Value::String(r#"{"findings":[1]}"#.into());
        fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
        assert!(lookup(&cache_dir, &key).is_none());
    }

    #[test]
    fn sha256_matches_standard_vector() {
        assert_eq!(
            sha256_hex(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
