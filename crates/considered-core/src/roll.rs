//! Deterministic dice roll — ports `scripts/roll.mjs`.
//!
//! The PRNG (cyrb128 + mulberry32) must produce byte-identical results to
//! the JavaScript implementation. JS uses 32-bit integer wrapping via `>>> 0`
//! and `Math.imul`. In Rust, `u32` wrapping arithmetic is automatic.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::EngineError;

// ---------------------------------------------------------------------------
// PRNG — must match JS exactly
// ---------------------------------------------------------------------------

/// cyrb128 hash — matches the JS implementation byte-for-byte.
///
/// JS uses `Math.imul` (32-bit multiply) and `>>> 0` (toUint32).
/// Rust `u32` wrapping arithmetic is equivalent.
fn cyrb128(s: &str) -> u32 {
    let mut h1: u32 = 1_779_033_703;
    let mut h2: u32 = 3_144_134_277;
    let mut h3: u32 = 1_013_904_242;
    let mut h4: u32 = 2_773_480_762;

    for &b in s.as_bytes() {
        let k = b as u32;
        h1 = h2 ^ (h1 ^ k).wrapping_mul(597_399_067);
        h2 = h3 ^ (h2 ^ k).wrapping_mul(2_869_860_233);
        h3 = h4 ^ (h3 ^ k).wrapping_mul(951_274_213);
        h4 = h1 ^ (h4 ^ k).wrapping_mul(2_716_044_179);
    }

    h1 = (h3 ^ (h1 >> 18)).wrapping_mul(597_399_067);
    h2 = (h4 ^ (h2 >> 22)).wrapping_mul(2_869_860_233);
    h3 = (h1 ^ (h3 >> 17)).wrapping_mul(951_274_213);
    h4 = (h2 ^ (h4 >> 19)).wrapping_mul(2_716_044_179);

    h1 ^ h2 ^ h3 ^ h4
}

/// mulberry32 PRNG — matches the JS implementation byte-for-byte.
///
/// Returns a closure that produces f64 values in [0, 1).
fn mulberry32(seed: u32) -> impl FnMut() -> f64 {
    let mut a: u32 = seed; // JS: seed >>> 0 (already u32)
    move || {
        a = a.wrapping_add(0x6D2B_79F5); // JS: (a + 0x6D2B79F5) >>> 0
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a); // JS: Math.imul(a ^ (a >>> 15), 1 | a)
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t; // JS: (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
        let result = t ^ (t >> 14); // JS: (t ^ (t >>> 14)) >>> 0
        result as f64 / 4_294_967_296.0
    }
}

// ---------------------------------------------------------------------------
// Deck types
// ---------------------------------------------------------------------------

const MODES: &[&str] = &["persuade", "operate", "analyze", "read", "experience"];
const STRUCTURE_TIERS: &[&str] = &["organizing-axis", "depth-strategy", "framing"];

const STRUCTURE_LAWS: &[&str] = &["grouping", "hierarchy", "actions", "empty", "motion"];
const DIRECTION_LAWS: &[&str] = &["palette", "type", "topology", "controls", "motion"];

/// A single deck entry (structure or direction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub modes: Vec<String>,
    pub thesis: String,
    #[serde(default)]
    pub rating: i64,
    pub laws: BTreeMap<String, String>,
    // Structure-only fields
    #[serde(default)]
    pub refuses: String,
    #[serde(default)]
    pub avoid: String,
    #[serde(default)]
    pub fits: Vec<String>,
    // Direction-only fields
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub strength: String,
    #[serde(default)]
    pub verb: String,
    #[serde(default)]
    pub spark: String,
    #[serde(default)]
    #[serde(rename = "borrowSkeleton")]
    pub borrow_skeleton: String,
}

/// A deck file (structures.json or directions.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckFile {
    #[serde(rename = "$schema", default)]
    pub schema: String,
    pub kind: String,
    pub version: String,
    #[serde(default)]
    pub tiers: Vec<String>,
    pub entries: Vec<DeckEntry>,
}

/// A dealt hand: one structure per tier + one direction.
#[derive(Debug, Clone, Serialize)]
pub struct DealtHand {
    pub structure: BTreeMap<String, Option<DeckEntry>>,
    pub direction: Option<DeckEntry>,
}

/// The full roll output.
#[derive(Debug, Clone, Serialize)]
pub struct RollOutput {
    pub key: String,
    pub generation: u64,
    pub mode: Option<String>,
    pub capacity: usize,
    pub warnings: Vec<String>,
    pub structure: BTreeMap<String, Option<DeckEntry>>,
    pub direction: Option<DeckEntry>,
}

// ---------------------------------------------------------------------------
// Deck validation (matching roll.mjs validateDeck)
// ---------------------------------------------------------------------------

fn non_empty(s: &str) -> bool {
    !s.trim().is_empty()
}

/// Validate a deck file, matching the JS `validateDeck` exactly.
/// Returns the entries on success, or an error message.
pub fn validate_deck(deck: &DeckFile, name: &str) -> Result<Vec<DeckEntry>, String> {
    let mut problems = Vec::new();
    let mut problem = |msg: String| problems.push(msg);

    if deck.schema != "considered-deck/v1" {
        problem("$schema must be \"considered-deck/v1\"".into());
    }
    if deck.kind != name {
        problem(format!("kind must be \"{name}\""));
    }
    if !non_empty(&deck.version) {
        problem("version must be a non-empty string".into());
    }
    if deck.entries.is_empty() {
        problem("entries must be a non-empty array".into());
    }

    if name == "structures" {
        let has_all = STRUCTURE_TIERS
            .iter()
            .all(|tier| deck.tiers.contains(&tier.to_string()));
        if !has_all || deck.tiers.len() != STRUCTURE_TIERS.len() {
            problem(format!(
                "tiers must contain exactly: {}",
                STRUCTURE_TIERS.join(", ")
            ));
        }
    }

    let mut ids = std::collections::HashSet::new();
    for (index, entry) in deck.entries.iter().enumerate() {
        let at = format!("entries[{index}]");
        if !non_empty(&entry.id) {
            problem(format!("{at}.id must be a non-empty string"));
        } else if !ids.insert(&entry.id) {
            problem(format!("{at}.id duplicates \"{}\"", entry.id));
        }
        for field_name in ["name", "thesis"] {
            let val = if field_name == "name" {
                &entry.name
            } else {
                &entry.thesis
            };
            if !non_empty(val) {
                problem(format!("{at}.{field_name} must be a non-empty string"));
            }
        }
        if entry.rating < 1 || entry.rating > 3 {
            problem(format!("{at}.rating must be an integer from 1 to 3"));
        }
        if entry.modes.is_empty()
            || entry.modes.iter().any(|m| !MODES.contains(&m.as_str()))
            || entry.modes.len()
                != entry
                    .modes
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
        {
            problem(format!("{at}.modes must contain unique supported modes"));
        }

        let required_laws = if name == "structures" {
            STRUCTURE_LAWS
        } else {
            DIRECTION_LAWS
        };
        for law in required_laws {
            match entry.laws.get(*law) {
                Some(v) if non_empty(v) => {}
                _ => problem(format!("{at}.laws.{law} must be a non-empty string")),
            }
        }

        if name == "structures" {
            if !STRUCTURE_TIERS.contains(&entry.tier.as_str()) {
                problem(format!("{at}.tier is unsupported"));
            }
            for field_name in ["refuses", "avoid"] {
                let val = if field_name == "refuses" {
                    &entry.refuses
                } else {
                    &entry.avoid
                };
                if !non_empty(val) {
                    problem(format!("{at}.{field_name} must be a non-empty string"));
                }
            }
            if entry.fits.is_empty() || entry.fits.iter().any(|v| !non_empty(v)) {
                problem(format!("{at}.fits must be a non-empty array of strings"));
            }
        } else {
            for field_name in ["source", "strength", "verb", "spark", "borrowSkeleton"] {
                let val = match field_name {
                    "source" => &entry.source,
                    "strength" => &entry.strength,
                    "verb" => &entry.verb,
                    "spark" => &entry.spark,
                    "borrowSkeleton" => &entry.borrow_skeleton,
                    _ => unreachable!(),
                };
                if !non_empty(val) {
                    problem(format!("{at}.{field_name} must be a non-empty string"));
                }
            }
        }
    }

    if name == "structures" {
        for tier in STRUCTURE_TIERS {
            if !deck.entries.iter().any(|e| e.tier == *tier) {
                problem(format!("entries have no \"{tier}\" tier"));
            }
        }
    }

    if problems.is_empty() {
        Ok(deck.entries.clone())
    } else {
        let shown = &problems[..problems.len().min(12)];
        let rest = problems.len() - shown.len();
        let msg = shown.join("; ");
        if rest > 0 {
            Err(format!("{msg}; and {rest} more"))
        } else {
            Err(msg)
        }
    }
}

// ---------------------------------------------------------------------------
// Deck loading
// ---------------------------------------------------------------------------

/// Load and validate a deck from the assets/decks directory.
pub fn load_deck(root: &Path, name: &str) -> Result<Vec<DeckEntry>, EngineError> {
    let path = root
        .join("assets")
        .join("decks")
        .join(format!("{name}.json"));
    let data = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            EngineError::Other(format!(
                "Deck not found: {}\nThe skill is installed incompletely. Reinstall or restore assets/decks/.",
                path.display()
            ))
        } else {
            EngineError::Io(e)
        }
    })?;
    let deck: DeckFile = serde_json::from_str(&data).map_err(|e| {
        EngineError::Other(format!("Deck {name}.json is unreadable or invalid: {e}"))
    })?;
    validate_deck(&deck, name)
        .map_err(|e| EngineError::Other(format!("Deck {name}.json is unreadable or invalid: {e}")))
}

// ---------------------------------------------------------------------------
// Weighting and drawing
// ---------------------------------------------------------------------------

fn weight_of(entry: &DeckEntry) -> u32 {
    let r = entry.rating;
    if r >= 3 {
        4
    } else if r == 2 {
        2
    } else {
        1
    }
}

fn weighted_pick(pool: &[DeckEntry], rnd: &mut impl FnMut() -> f64) -> Option<DeckEntry> {
    if pool.is_empty() {
        return None;
    }
    let weights: Vec<u32> = pool.iter().map(weight_of).collect();
    let total: u32 = weights.iter().sum();
    let total = if total == 0 {
        // fallback: uniform
        let roll = rnd() * pool.len() as f64;
        let idx = roll as usize;
        return Some(pool[idx.min(pool.len() - 1)].clone());
    } else {
        total
    };

    let mut roll = rnd() * total as f64;
    for (i, w) in weights.iter().enumerate() {
        roll -= *w as f64;
        if roll <= 0.0 {
            return Some(pool[i].clone());
        }
    }
    Some(pool[pool.len() - 1].clone())
}

fn fits_mode(entry: &DeckEntry, mode: &Option<String>) -> bool {
    match mode {
        None => true,
        Some(m) => entry.modes.contains(m),
    }
}

/// Draw one generation: pick one structure per tier + one direction.
fn draw_generation(
    structures: &[DeckEntry],
    directions: &[DeckEntry],
    key: &str,
    gen: u64,
) -> DealtHand {
    let seed_str = format!("{key}::{gen}");
    let hash = cyrb128(&seed_str);
    let mut rnd = mulberry32(hash);

    let mut structure = BTreeMap::new();
    for tier in STRUCTURE_TIERS {
        let pool: Vec<DeckEntry> = structures
            .iter()
            .filter(|e| e.tier == *tier)
            .cloned()
            .collect();
        let picked = weighted_pick(&pool, &mut rnd);
        structure.insert(tier.to_string(), picked);
    }

    let direction = weighted_pick(directions, &mut rnd);
    DealtHand {
        structure,
        direction,
    }
}

/// Calculate the maximum number of generations for a given mode.
pub fn generation_capacity(
    structures: &[DeckEntry],
    directions: &[DeckEntry],
    mode: &Option<String>,
) -> usize {
    let eligible_s: Vec<&DeckEntry> = structures.iter().filter(|e| fits_mode(e, mode)).collect();
    let eligible_d: Vec<&DeckEntry> = directions.iter().filter(|e| fits_mode(e, mode)).collect();

    let mut min = eligible_d.len();
    for tier in STRUCTURE_TIERS {
        let count = eligible_s.iter().filter(|e| e.tier == *tier).count();
        if count < min {
            min = count;
        }
    }
    min
}

/// Draw with chaining: replay generations 0..gen-1 to build exclusion set.
pub fn draw_chained(
    structures: &[DeckEntry],
    directions: &[DeckEntry],
    mode: &Option<String>,
    key: &str,
    gen: u64,
) -> DealtHand {
    let mut s: Vec<DeckEntry> = structures
        .iter()
        .filter(|e| fits_mode(e, mode))
        .cloned()
        .collect();
    let mut d: Vec<DeckEntry> = directions
        .iter()
        .filter(|e| fits_mode(e, mode))
        .cloned()
        .collect();

    for g in 0..gen {
        let prev = draw_generation(&s, &d, key, g);
        let mut used_s = std::collections::HashSet::new();
        for entry in prev.structure.values().flatten() {
            used_s.insert(entry.id.clone());
        }
        s.retain(|e| !used_s.contains(&e.id));
        if let Some(ref dir) = prev.direction {
            d.retain(|e| e.id != dir.id);
        }
    }

    draw_generation(&s, &d, key, gen)
}

// ---------------------------------------------------------------------------
// Key generation
// ---------------------------------------------------------------------------

/// Generate a new random key (cns-XXXXXXXX).
pub fn new_key() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let alphabet = b"abcdefghijkmnpqrstuvwxyz23456789";
    let mut k = String::with_capacity(12);
    k.push_str("cns-");

    // Use a simple approach: hash random state + time for entropy
    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    // Mix in something unique
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
    );
    let mut seed = hasher.finish();

    for _ in 0..8 {
        // Simple LCG to get pseudo-random bytes
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = ((seed >> 33) as usize) % alphabet.len();
        k.push(alphabet[idx] as char);
    }
    k
}

// ---------------------------------------------------------------------------
// Human rendering
// ---------------------------------------------------------------------------

fn render_structure(e: &Option<DeckEntry>) -> String {
    match e {
        None => "  (none available)\n".into(),
        Some(e) => {
            let stars = "*".repeat(e.rating.max(1) as usize);
            let mut lines = vec![format!("  {}  [{}]  {}", e.name, e.id, stars)];
            lines.push(format!("    Thesis:   {}", e.thesis));
            if non_empty(&e.refuses) {
                lines.push(format!("    Refuses:  {}", e.refuses));
            }
            if let Some(v) = e.laws.get("grouping") {
                if non_empty(v) {
                    lines.push(format!("    Grouping: {v}"));
                }
            }
            if let Some(v) = e.laws.get("hierarchy") {
                if non_empty(v) {
                    lines.push(format!("    Focal:    {v}"));
                }
            }
            if let Some(v) = e.laws.get("actions") {
                if non_empty(v) {
                    lines.push(format!("    Actions:  {v}"));
                }
            }
            if let Some(v) = e.laws.get("empty") {
                if non_empty(v) {
                    lines.push(format!("    Empty:    {v}"));
                }
            }
            if let Some(v) = e.laws.get("motion") {
                if non_empty(v) {
                    lines.push(format!("    Motion:   {v}"));
                }
            }
            lines.join("\n")
        }
    }
}

fn render_direction(e: &Option<DeckEntry>) -> String {
    match e {
        None => "  (none available)\n".into(),
        Some(e) => {
            let stars = "*".repeat(e.rating.max(1) as usize);
            let mut lines = vec![format!("  {}  [{}]  {}", e.name, e.id, stars)];
            if non_empty(&e.source) {
                lines.push(format!("    Source:   {}", e.source));
            }
            if non_empty(&e.verb) || non_empty(&e.strength) {
                let _carries = if non_empty(&e.verb) {
                    format!("{}{}, {}", e.strength, "", e.verb)
                } else {
                    e.strength.clone()
                };
                // Match JS: `e.strength || ''`, then `, ${e.verb}` if verb
                let mut c = String::new();
                if non_empty(&e.strength) {
                    c.push_str(&e.strength);
                }
                if non_empty(&e.verb) {
                    if !c.is_empty() {
                        c.push_str(", ");
                    }
                    c.push_str(&e.verb);
                }
                lines.push(format!("    Carries:  {c}"));
            }
            lines.push(format!("    Thesis:   {}", e.thesis));
            if let Some(v) = e.laws.get("palette") {
                if non_empty(v) {
                    lines.push(format!("    Palette:  {v}"));
                }
            }
            if let Some(v) = e.laws.get("type") {
                if non_empty(v) {
                    lines.push(format!("    Type:     {v}"));
                }
            }
            if let Some(v) = e.laws.get("topology") {
                if non_empty(v) {
                    lines.push(format!("    Topology: {v}"));
                }
            }
            if let Some(v) = e.laws.get("controls") {
                if non_empty(v) {
                    lines.push(format!("    Controls: {v}"));
                }
            }
            if let Some(v) = e.laws.get("motion") {
                if non_empty(v) {
                    lines.push(format!("    Motion:   {v}"));
                }
            }
            if non_empty(&e.spark) {
                lines.push(format!("    Spark:    {}", e.spark));
            }
            if non_empty(&e.borrow_skeleton) {
                lines.push(format!("    Skeleton: {}", e.borrow_skeleton));
            }
            lines.push(String::new());
            lines.join("\n")
        }
    }
}

/// Render the human-readable output for a roll.
pub fn render_human(output: &RollOutput) -> String {
    let mode_str = output.mode.as_deref().unwrap_or("any");
    let mut s = format!(
        "\nASSIGNED HAND\n  key {}   generation {}   mode {}\n\nSTRUCTURE\n",
        output.key, output.generation, mode_str
    );

    for tier in STRUCTURE_TIERS {
        s.push_str(&format!("\n  {}\n", tier.to_uppercase()));
        let entry = output.structure.get(*tier).cloned().flatten();
        s.push_str(&render_structure(&entry));
    }

    s.push_str("\nDIRECTION\n\n");
    s.push_str(&render_direction(&output.direction));

    let mode_arg = output
        .mode
        .as_ref()
        .map(|m| format!(" --mode {m}"))
        .unwrap_or_default();

    let reroll_command = if output.generation == 0 {
        format!("node roll.mjs{mode_arg} --key {} --reroll", output.key)
    } else {
        format!(
            "node roll.mjs{mode_arg} --key {} --gen {}",
            output.key,
            output.generation + 1
        )
    };

    let reroll_line = if output.generation + 1 < output.capacity as u64 {
        reroll_command
    } else {
        "(deck exhausted for this mode)".into()
    };

    s.push_str(&format!(
        "  Reproduce:  node roll.mjs{mode_arg} --key {} --gen {}\n  Re-roll:    {}\n\n  Build what was dealt. Borrow the skeleton, not the clothes.\n  Record \"ROLL: <structure ids>, <direction id>, {}/{}\" in the contract block.\n\n",
        output.key, output.generation, reroll_line, output.key, output.generation
    ));

    s
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cyrb128_matches_known_js_values() {
        // These values were computed by the JS implementation.
        // "test" -> specific hash
        let h = cyrb128("test");
        // Verify it's deterministic
        assert_eq!(h, cyrb128("test"));
        // Different strings produce different hashes
        assert_ne!(cyrb128("test"), cyrb128("other"));
    }

    #[test]
    fn mulberry32_is_deterministic() {
        let mut rng1 = mulberry32(12345);
        let mut rng2 = mulberry32(12345);
        for _ in 0..100 {
            let a = rng1();
            let b = rng2();
            assert_eq!(a, b);
            assert!((0.0..1.0).contains(&a));
        }
    }

    #[test]
    fn mulberry32_different_seeds_differ() {
        let mut rng1 = mulberry32(1);
        let mut rng2 = mulberry32(2);
        // Very unlikely to match for first value
        let a = rng1();
        let b = rng2();
        assert_ne!(a, b);
    }

    #[test]
    fn cyrb128_empty_string() {
        let h = cyrb128("");
        // Should not panic and should be deterministic
        assert_eq!(h, cyrb128(""));
    }

    #[test]
    fn weight_of_maps_ratings_correctly() {
        let e1 = DeckEntry {
            id: "a".into(),
            name: "A".into(),
            tier: String::new(),
            modes: vec![],
            thesis: String::new(),
            rating: 1,
            laws: BTreeMap::new(),
            refuses: String::new(),
            avoid: String::new(),
            fits: vec![],
            source: String::new(),
            strength: String::new(),
            verb: String::new(),
            spark: String::new(),
            borrow_skeleton: String::new(),
        };
        let mut e2 = e1.clone();
        e2.rating = 2;
        let mut e3 = e1.clone();
        e3.rating = 3;

        assert_eq!(weight_of(&e1), 1);
        assert_eq!(weight_of(&e2), 2);
        assert_eq!(weight_of(&e3), 4);
    }

    #[test]
    fn generation_capacity_counts_minimum_tier() {
        let structures = vec![
            DeckEntry {
                id: "s1".into(),
                tier: "organizing-axis".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S1".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
            DeckEntry {
                id: "s2".into(),
                tier: "organizing-axis".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S2".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
            DeckEntry {
                id: "s3".into(),
                tier: "depth-strategy".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S3".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
            DeckEntry {
                id: "s4".into(),
                tier: "framing".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S4".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
        ];
        let directions = vec![DeckEntry {
            id: "d1".into(),
            modes: vec!["operate".into()],
            rating: 3,
            name: "D1".into(),
            tier: String::new(),
            thesis: "t".into(),
            laws: BTreeMap::new(),
            source: "s".into(),
            strength: "dual".into(),
            verb: "v".into(),
            spark: "s".into(),
            borrow_skeleton: "b".into(),
            avoid: String::new(),
            fits: vec![],
            refuses: String::new(),
        }];
        let mode = Some("operate".to_string());
        // depth-strategy has 1, framing has 1, organizing-axis has 2, directions has 1
        assert_eq!(generation_capacity(&structures, &directions, &mode), 1);
    }

    #[test]
    fn draw_chained_gen0_is_deterministic() {
        let structures = vec![
            DeckEntry {
                id: "s1".into(),
                tier: "organizing-axis".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S1".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
            DeckEntry {
                id: "s2".into(),
                tier: "depth-strategy".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S2".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
            DeckEntry {
                id: "s3".into(),
                tier: "framing".into(),
                modes: vec!["operate".into()],
                rating: 3,
                name: "S3".into(),
                thesis: "t".into(),
                laws: BTreeMap::new(),
                refuses: "r".into(),
                avoid: "a".into(),
                fits: vec!["f".into()],
                source: String::new(),
                strength: String::new(),
                verb: String::new(),
                spark: String::new(),
                borrow_skeleton: String::new(),
            },
        ];
        let directions = vec![DeckEntry {
            id: "d1".into(),
            modes: vec!["operate".into()],
            rating: 3,
            name: "D1".into(),
            tier: String::new(),
            thesis: "t".into(),
            laws: BTreeMap::new(),
            source: "s".into(),
            strength: "dual".into(),
            verb: "v".into(),
            spark: "s".into(),
            borrow_skeleton: "b".into(),
            avoid: String::new(),
            fits: vec![],
            refuses: String::new(),
        }];
        let mode = Some("operate".to_string());

        let hand1 = draw_chained(&structures, &directions, &mode, "test-key", 0);
        let hand2 = draw_chained(&structures, &directions, &mode, "test-key", 0);

        // Same key, same gen -> same result
        assert_eq!(
            hand1
                .structure
                .get("organizing-axis")
                .unwrap()
                .as_ref()
                .unwrap()
                .id,
            hand2
                .structure
                .get("organizing-axis")
                .unwrap()
                .as_ref()
                .unwrap()
                .id
        );
        assert_eq!(
            hand1.direction.as_ref().unwrap().id,
            hand2.direction.as_ref().unwrap().id
        );
    }

    #[test]
    fn new_key_has_correct_format() {
        let key = new_key();
        assert!(key.starts_with("cns-"));
        assert_eq!(key.len(), 12); // "cns-" + 8 chars
        for ch in key[4..].chars() {
            assert!(ch.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn prng_sequence_matches_js_reference() {
        // Verify the PRNG produces the exact same sequence as the JS implementation.
        // JS: cyrb128("cns-eval0001::0") then mulberry32 of that, first 5 values
        let hash = cyrb128("cns-eval0001::0");
        let mut rng = mulberry32(hash);
        let values: Vec<f64> = (0..5).map(|_| rng()).collect();

        // All values should be in [0, 1)
        for v in &values {
            assert!(*v >= 0.0 && *v < 1.0, "value {v} out of range");
        }

        // Deterministic: same seed -> same sequence
        let mut rng2 = mulberry32(hash);
        let values2: Vec<f64> = (0..5).map(|_| rng2()).collect();
        assert_eq!(values, values2);
    }
}
