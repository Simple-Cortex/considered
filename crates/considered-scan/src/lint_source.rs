//! Port of `scripts/lint-source.mjs` — heuristic source audit.
//!
//! Walks a directory tree, applies regex-based style/markup/state checks to
//! code files, and returns structured findings. All findings carry
//! `confidence: "heuristic"`. The crate never prints or terminates.

#![allow(
    clippy::useless_format,
    clippy::unnecessary_literal_unwrap,
    clippy::needless_raw_string_hashes
)]

use considered_core::{EngineError, Finding, Severity};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

macro_rules! regex {
    ($pattern:literal) => {{
        static INSTANCE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new($pattern).expect("static regex must compile"));
        &*INSTANCE
    }};
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    ".next",
    ".git",
    "coverage",
    "out",
    ".turbo",
    "vendor",
];

const CODE_EXTENSIONS: &[&str] = &[
    ".css", ".scss", ".sass", ".less", ".jsx", ".tsx", ".js", ".ts", ".mjs", ".cjs", ".mts",
    ".cts", ".html", ".vue", ".svelte", ".astro", ".mdx",
];

const STYLE_EXTENSIONS: &[&str] = &[".css", ".scss", ".sass", ".less"];

const BANNED_LABELS: &[&str] = &[
    "submit",
    "ok",
    "yes",
    "no",
    "go",
    "continue",
    "manage",
    "update",
    "click here",
    "learn more",
    "read more",
];

const BANNED_HEADINGS: &[&str] = &[
    "overview",
    "metrics",
    "kpis",
    "charts",
    "tables",
    "other",
    "analytics",
    "data",
    "stats",
    "misc",
];

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Result of a source lint pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintSourceReport {
    pub checker: String,
    pub scope: String,
    pub scanned: usize,
    pub skipped: Vec<SkippedFile>,
    pub counts: BTreeMap<String, usize>,
    pub findings: Vec<Finding>,
}

/// A file that could not be read and was skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Budget
// ---------------------------------------------------------------------------

struct Budget {
    type_sizes: BTreeSet<String>,
    font_weights: BTreeSet<String>,
    text_colors: BTreeSet<String>,
    radius_values: BTreeSet<String>,
    shadow_levels: BTreeSet<String>,
    border_styles: BTreeSet<String>,
}

impl Budget {
    fn new() -> Self {
        Self {
            type_sizes: BTreeSet::new(),
            font_weights: BTreeSet::new(),
            text_colors: BTreeSet::new(),
            radius_values: BTreeSet::new(),
            shadow_levels: BTreeSet::new(),
            border_styles: BTreeSet::new(),
        }
    }

    fn note(set: &mut BTreeSet<String>, value: &str) {
        let token = value.trim().to_lowercase();
        if token.is_empty()
            || matches!(
                token.as_str(),
                "inherit" | "initial" | "unset" | "0" | "none"
            )
        {
            return;
        }
        set.insert(token);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let ext = format!(".{e}");
            CODE_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

fn is_style_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let ext = format!(".{e}");
            STYLE_EXTENSIONS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

fn line_of(text: &str, index: usize) -> u32 {
    text[..index.min(text.len())].matches('\n').count() as u32 + 1
}

/// Byte offset that is exactly `count` Unicode scalar values (code points)
/// before `byte_index`, clamped to the start of the text. Windows must be
/// measured in code points — not bytes, and not UTF-16 units — so this and
/// the Node port (which counts via `Array.from`) agree on where a
/// multi-byte run (em dashes, curly quotes, …) puts the window boundary.
fn code_points_before(text: &str, byte_index: usize, count: usize) -> usize {
    let prefix = &text[..byte_index];
    let char_starts: Vec<usize> = prefix.char_indices().map(|(i, _)| i).collect();
    if char_starts.len() <= count {
        0
    } else {
        char_starts[char_starts.len() - count]
    }
}

/// The byte-length prefix of `text` containing its first `count` code
/// points (or the whole text if shorter).
fn code_points_prefix(text: &str, count: usize) -> &str {
    match text.char_indices().nth(count) {
        Some((idx, _)) => &text[..idx],
        None => text,
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, EngineError> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        if e.depth() > 0 && name.starts_with('.') && name != ".storybook" {
            return false;
        }
        if e.depth() > 0 && e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
            return false;
        }
        true
    }) {
        let e = entry.map_err(|error| {
            EngineError::Usage(format!("Cannot traverse {}: {error}", root.display()))
        })?;
        if e.file_type().is_file() && is_code_file(e.path()) {
            files.push(e.into_path());
        }
    }

    files.sort();
    Ok(files)
}

fn severity_for_rule(rule: &str) -> Severity {
    match rule {
        "A11Y-04" | "A11Y-09" | "A11Y-11" | "STATE-01" | "ACT-06" => Severity::S1,
        "ACT-01" | "IA-03" => Severity::S2,
        "ACT-05" | "COMP-03" | "COMP-04" | "COMP-07" | "COMP-16" | "COMP-17" => Severity::S3,
        // HON-03's declared severity lives in assets/rules/guidance.json (S2);
        // this lint lead must emit the catalog severity like every other
        // rule, never a hand-picked override.
        _ => Severity::S2,
    }
}

fn add_finding(
    findings: &mut Vec<Finding>,
    rule: &str,
    file: &str,
    line: u32,
    message: &str,
    hint: &str,
) {
    findings.push(Finding {
        rule: rule.to_string(),
        severity: severity_for_rule(rule),
        confidence: Some("heuristic".to_string()),
        message: message.to_string(),
        hint: Some(hint.to_string()),
        file: Some(file.to_string()),
        line: if line == 0 { None } else { Some(line) },
        evidence: None,
    });
}

// ---------------------------------------------------------------------------
// Style checks
// ---------------------------------------------------------------------------

fn check_styles(text: &str, file: &str, budget: &mut Budget, findings: &mut Vec<Finding>) {
    let re_font_size = regex!(r"(?i)font-size\s*:\s*([^;{}]+)");
    let re_font_weight = regex!(r"(?i)font-weight\s*:\s*([^;{}]+)");
    let re_color = regex!(r"(?i)\bcolor\s*:\s*([^;{}]+)");
    let re_radius = regex!(r"(?i)border-radius\s*:\s*([^;{}]+)");
    let re_shadow = regex!(r"(?i)box-shadow\s*:\s*([^;{}]+)");
    let re_border_style =
        regex!(r"(?i)border(?:-(?:top|right|bottom|left))?-style\s*:\s*([^;{}]+)");
    let re_outline_remove = regex!(r"(?i)outline\s*:\s*(none|0)\s*[;}]");
    let re_spacing_decl = regex!(r"(?i)(?:margin|padding|gap)(?:-[a-z]+)?\s*:\s*([^;{}]+)");
    let re_px_val = regex!(r"(\d+(?:\.\d+)?)px");
    let re_focus_nearby =
        regex!(r"(?i):focus-visible|outline-offset|box-shadow[^;]*focus|focus-within");
    let re_keyframes = regex!(r"(?i)@keyframes|animation\s*:|transition\s*:");
    let re_reduced_motion = regex!(r"(?i)prefers-reduced-motion");
    let re_table_center =
        regex!(r#"(?i)\b(td|th|\.cell|\[role="cell"\])[^\{]*\{[^}]*text-align\s*:\s*center"#);
    let re_appearance_token =
        regex!(r"(?i)--(?:color-)?(red|green|blue|purple|orange|yellow|pink|teal)(?:-\d+)?\s*:");

    // Budget collection
    for cap in re_font_size.captures_iter(text) {
        Budget::note(&mut budget.type_sizes, &cap[1]);
    }
    for cap in re_font_weight.captures_iter(text) {
        Budget::note(&mut budget.font_weights, &cap[1]);
    }
    for cap in re_color.captures_iter(text) {
        let m = cap.get(0).unwrap();
        // Skip if preceded by background-, border-, or outline- (no look-behind in Rust regex)
        let before = &text[..m.start()];
        if before.ends_with("background-")
            || before.ends_with("border-")
            || before.ends_with("outline-")
        {
            continue;
        }
        Budget::note(&mut budget.text_colors, &cap[1]);
    }
    for cap in re_radius.captures_iter(text) {
        Budget::note(&mut budget.radius_values, &cap[1]);
    }
    for cap in re_shadow.captures_iter(text) {
        Budget::note(&mut budget.shadow_levels, &cap[1]);
    }
    for cap in re_border_style.captures_iter(text) {
        Budget::note(&mut budget.border_styles, &cap[1]);
    }

    // A11Y-04: focus outline removed
    for m in re_outline_remove.find_iter(text) {
        let start = m.start().saturating_sub(600);
        let end = (m.end() + 600).min(text.len());
        let near = &text[start..end];
        if !re_focus_nearby.is_match(near) {
            add_finding(
                findings,
                "A11Y-04",
                file,
                line_of(text, m.start()),
                "Focus outline appears to be removed without a nearby replacement.",
                "Verify a visible focus indicator on the affected selector.",
            );
        }
    }

    // COMP-03: off-scale spacing. Node collects these in a JS `Set`, whose
    // iteration order is insertion (first-seen) order, not lexicographic —
    // preserve that here with a Vec + membership check rather than a
    // BTreeSet. The remainder check also runs on the raw decimal value (not
    // truncated to an integer), so e.g. 4.5px and 8.25px are off-scale too.
    let mut off_scale: Vec<String> = Vec::new();
    for cap in re_spacing_decl.captures_iter(text) {
        for v in re_px_val.captures_iter(&cap[1]) {
            if let Ok(n) = v[1].parse::<f64>() {
                if n > 0.0 && n % 4.0 != 0.0 {
                    let token = format!("{}px", &v[1]);
                    if !off_scale.contains(&token) {
                        off_scale.push(token);
                    }
                }
            }
        }
    }
    if !off_scale.is_empty() {
        let sample: Vec<&str> = off_scale.iter().take(6).map(|s| s.as_str()).collect();
        add_finding(
            findings,
            "COMP-03",
            file,
            0,
            &format!("Off-scale spacing values: {}.", sample.join(", ")),
            "Confirm these are intentional tokens or snap them to the spacing scale.",
        );
    }

    // A11Y-09: motion without reduced-motion
    if re_keyframes.is_match(text) && !re_reduced_motion.is_match(text) {
        add_finding(
            findings,
            "A11Y-09",
            file,
            0,
            "Motion appears without a reduced-motion override in this file.",
            "Verify the global stylesheet or add prefers-reduced-motion handling.",
        );
    }

    // COMP-17: table cells center-aligned
    for m in re_table_center.find_iter(text) {
        add_finding(
            findings,
            "COMP-17",
            file,
            line_of(text, m.start()),
            "Table cells are center-aligned.",
            "Keep text left-aligned and numeric columns right-aligned.",
        );
    }

    // COMP-04: appearance-named tokens
    for cap in re_appearance_token.captures_iter(text) {
        let (Some(m), Some(name)) = (cap.get(0), cap.get(1)) else {
            continue;
        };
        add_finding(
            findings,
            "COMP-04",
            file,
            line_of(text, m.start()),
            &format!("Token --{} is named for appearance.", name.as_str()),
            "Use a semantic role such as --danger, --success, or --accent.",
        );
    }
}

fn check_budget(budget: &Budget, file: &str, findings: &mut Vec<Finding>) {
    let checks: &[(&BTreeSet<String>, usize, &str)] = &[
        (&budget.type_sizes, 5, "type sizes"),
        (&budget.font_weights, 3, "font weights"),
        (&budget.text_colors, 3, "text colors"),
        (&budget.radius_values, 2, "radius values"),
        (&budget.shadow_levels, 2, "shadow levels"),
        (&budget.border_styles, 2, "border styles"),
    ];

    for (set, max, label) in checks {
        if set.len() > *max {
            add_finding(
                findings,
                "COMP-16",
                file,
                0,
                &format!(
                    "{} distinct {} in one file (budget {}).",
                    set.len(),
                    label,
                    max
                ),
                "Review this view-level estimate and consolidate values that do not encode meaning.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Markup checks
// ---------------------------------------------------------------------------

fn check_markup(text: &str, file: &str, findings: &mut Vec<Finding>) {
    let re_text_between_tags = regex!(r">\s*([A-Za-z][A-Za-z ]{0,18})\s*<");
    let re_preceding_button = regex!(r"(?i)<(button|a|Button|Link)\b[\s\S]{0,240}$");
    let re_heading = regex!(r"(?i)<h[1-6][^>]*>\s*([^<]{2,40})<");
    let re_button_block = regex!(r"(?i)<button\b([^>]*)>([\s\S]{0,240}?)</button>");
    let re_strip_tags = regex!(r"<[^>]+>");
    let re_h1 = regex!(r"(?i)<h1\b");
    let re_emoji = regex!(r"[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]");
    let re_high_emphasis_variant =
        regex!(r##"(?i)(?:variant|appearance)\s*=\s*["'](primary|filled|solid|cta)["']"##);
    let re_high_emphasis_class = regex!(
        r##"(?i)class(?:Name)?\s*=\s*["'][^"']*\b(?:btn-primary|bg-(?:blue|indigo|violet|primary)-[5-7]00)\b"##
    );
    let re_glass_backdrop = regex!(r"(?i)backdrop-(?:blur|filter)");
    let re_glass_bg = regex!(r"(?i)bg-white/\d{1,2}|rgba\(255,\s*255,\s*255,\s*0?\.[0-3]");
    let re_gradient = regex!(
        r##"(?i)bg-gradient-to-[a-z]+[^"']*(?:purple|violet|fuchsia|indigo)[^"']*(?:pink|blue|cyan)"##
    );
    let re_outline_none_tw =
        regex!(r##"(?i)class(?:Name)?\s*=\s*["'][^"']*\boutline-none\b[^"']*["']"##);
    let re_focus_tw = regex!(r"focus(?:-visible)?:");
    let re_motion_tw = regex!(r"(?:animate-|transition-)");
    let re_motion_pref = regex!(r"motion-reduce:|motion-safe:|prefers-reduced-motion");

    // ACT-05: banned labels
    for m in re_text_between_tags.find_iter(text) {
        let label_text = m.as_str();
        // Extract the captured group
        if let Some(cap) = re_text_between_tags.captures(label_text) {
            let label = cap[1].trim().to_lowercase();
            if BANNED_LABELS.contains(&label.as_str()) {
                let preceding_start = m.start().saturating_sub(240);
                let preceding = &text[preceding_start..m.start()];
                if re_preceding_button.is_match(preceding) {
                    let raw_label = cap[1].trim();
                    add_finding(
                        findings,
                        "ACT-05",
                        file,
                        line_of(text, m.start()),
                        &format!("Action label \"{raw_label}\" may not name an outcome."),
                        "Use a concise verb plus object.",
                    );
                }
            }
        }
    }

    // IA-03: banned headings
    for m in re_heading.find_iter(text) {
        if let Some(cap) = re_heading.captures(m.as_str()) {
            let heading = cap[1].trim().to_lowercase();
            if BANNED_HEADINGS.contains(&heading.as_str()) {
                let raw = cap[1].trim();
                add_finding(
                    findings,
                    "IA-03",
                    file,
                    line_of(text, m.start()),
                    &format!("Heading \"{raw}\" is a container label."),
                    "Name the user question, object, or decision instead.",
                );
            }
        }
    }

    // ACT-06: icon-only button without aria-label
    for m in re_button_block.find_iter(text) {
        if let Some(cap) = re_button_block.captures(m.as_str()) {
            let attrs = &cap[1];
            let inner = &cap[2];
            let plain = re_strip_tags.replace_all(inner, " ").trim().to_string();
            let named = regex!(r"(?i)aria-label|aria-labelledby").is_match(attrs)
                || regex!(r"(?i)aria-label").is_match(inner);
            let icon = regex!(r"<(svg|Icon|[A-Z][A-Za-z]*Icon)\b").is_match(inner);
            if icon && plain.len() < 2 && !named {
                add_finding(
                    findings,
                    "ACT-06",
                    file,
                    line_of(text, m.start()),
                    "Icon-only button may have no accessible name.",
                    "Add aria-label and a visible tooltip, or use a text label.",
                );
            }
        }
    }

    // A11Y-11: multiple h1
    let h1_matches: Vec<_> = re_h1.find_iter(text).collect();
    if h1_matches.len() > 1 {
        add_finding(
            findings,
            "A11Y-11",
            file,
            line_of(text, h1_matches[1].start()),
            &format!(
                "{} h1 elements appear in one source file.",
                h1_matches.len()
            ),
            "Verify each is not rendered in the same view.",
        );
    }

    // COMP-07: emoji
    let emoji_matches: Vec<_> = re_emoji.find_iter(text).collect();
    if emoji_matches.len() > 2 {
        add_finding(
            findings,
            "COMP-07",
            file,
            0,
            &format!("{} emoji appear in UI source.", emoji_matches.len()),
            "Use a coherent icon system when these represent controls or status.",
        );
    }

    // ACT-01: high-emphasis action treatments
    let filled_count = re_high_emphasis_variant.find_iter(text).count()
        + re_high_emphasis_class.find_iter(text).count();
    if filled_count > 2 {
        add_finding(
            findings,
            "ACT-01",
            file,
            0,
            &format!(
                "{} high-emphasis action treatments appear in one source file.",
                filled_count
            ),
            "Verify they do not share a viewport; demote competing actions.",
        );
    }

    // COMP-07: glass treatment
    if re_glass_backdrop.is_match(text) && re_glass_bg.is_match(text) {
        add_finding(
            findings,
            "COMP-07",
            file,
            0,
            "Glass treatment detected.",
            "Keep it only if the assigned direction and accessibility requirements justify it.",
        );
    }

    // COMP-07: purple-blue gradient
    if re_gradient.is_match(text) {
        add_finding(
            findings,
            "COMP-07",
            file,
            0,
            "Purple-blue gradient treatment detected.",
            "Verify it is intentional rather than a default AI aesthetic.",
        );
    }

    // A11Y-04: tailwind outline-none without focus
    for m in re_outline_none_tw.find_iter(text) {
        if !re_focus_tw.is_match(m.as_str()) {
            add_finding(
                findings,
                "A11Y-04",
                file,
                line_of(text, m.start()),
                "Tailwind outline-none appears without a focus utility.",
                "Add a focus-visible ring or verify a component-level replacement.",
            );
        }
    }

    // A11Y-09: tailwind motion without preference
    if re_motion_tw.is_match(text) && !re_motion_pref.is_match(text) {
        add_finding(
            findings,
            "A11Y-09",
            file,
            0,
            "Tailwind-like motion utility appears without a motion preference utility.",
            "Verify the global motion policy or add motion-reduce behavior.",
        );
    }
}

// ---------------------------------------------------------------------------
// State checks
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Instance-data checks (HON-03)
// ---------------------------------------------------------------------------

/// HON-03 (craft.md / guidance.json): sample data instantiates, never
/// asserts. Detect instance-data patterns (emails, card numbers, invoice or
/// period labels, recency phrases, name+email pairs, current-value
/// settings) that are not marked illustrative anywhere nearby. These are
/// leads for a reviewer against the brief's fact list, not proof — the
/// checker cannot know what the brief supplied, only that the text reads as
/// a specific real-world instance. Port of `checkInstanceData` in
/// `scripts/lint-source.mjs`; keep both in lockstep.
fn check_instance_data(text: &str, file: &str, findings: &mut Vec<Finding>) {
    static RE_EMAIL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b").unwrap());
    // The final alternative catches the raw-data-field shape a card's last
    // four digits usually take before a template renders it into "Card
    // ending in 4242" text (e.g. `paymentMethodLast4: "4242"`), which the
    // phrase patterns never see because the phrase is assembled at render
    // time. The key name is anchored to card/last-four vocabulary (never a
    // bare "card" or "wildcard" substring) and the value must be a quoted
    // 4-digit string, so `cardWidth = 1280` and `wildcardTimeout = 3000`
    // never match.
    static RE_CARD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?i)(?:Card\s+)?ending in \d{4}|\u{2022}{4}\s?\d{4}|\*{4}\s?\d{4}|last 4 \d{4}|\b(?:\w*last_?4|lastFour|card_?number)\b\s*[:=]\s*["']\d{4}["']"#,
        )
        .unwrap()
    });
    static RE_INVOICE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)(?:Invoice\s*[\u{2014}\u{2013}-]?\s*)?(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4}",
        )
        .unwrap()
    });
    static RE_RECENCY: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)last used \d+ (?:minutes?|hours?|days?|weeks?|months?|years?) ago|\d+ (?:days?|hours?|months?) ago",
        )
        .unwrap()
    });
    // Timezone matches are anchored to actual IANA area names so import
    // specifiers like "Components/Button" never qualify. The bare locale
    // label form ("Time (ET)") was dropped: it fired on ordinary prose too
    // often to carry as a lead.
    static RE_CURRENT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"\b(?:America|Europe|Asia|Africa|Australia|Pacific|Atlantic|Indian|Antarctic|Etc|US)/[A-Z][A-Za-z_]+\b|\b\d{2}:\d{2}\s*[\u{2013}-]\s*\d{2}:\d{2}\b|\bMM/DD/YYYY\b|\bDD/MM/YYYY\b|\bYYYY-MM-DD\b",
        )
        .unwrap()
    });
    static RE_NAME_PAIR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\b[A-Z][a-zA-Z]+ [A-Z][a-zA-Z]+\b").unwrap());
    static RE_ILLUSTRATIVE_FILE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)illustrative data").unwrap());
    static RE_SUPPRESS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)illustrative|sample|example|placeholder|data-illustrative|aria-label="illustrative"#)
            .unwrap()
    });
    static RE_INVOICE_PREFIX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^invoice").unwrap());
    static RE_INVOICE_CONTEXT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)invoice|receipt|billed|statement").unwrap());

    let file_illustrative = RE_ILLUSTRATIVE_FILE.is_match(code_points_prefix(text, 400));

    let is_suppressed = |index: usize| -> bool {
        if file_illustrative {
            return true;
        }
        let start = code_points_before(text, index, 200);
        RE_SUPPRESS.is_match(&text[start..index])
    };

    let mut emails: Vec<(usize, String)> = Vec::new();
    for m in RE_EMAIL.find_iter(text) {
        let value = m.as_str();
        let domain = value.split('@').nth(1).unwrap_or("").to_lowercase();
        if domain.contains("example.") {
            continue;
        }
        if is_suppressed(m.start()) {
            continue;
        }
        emails.push((m.start(), value.to_string()));
    }

    let card_values: Vec<String> = RE_CARD
        .find_iter(text)
        .filter(|m| !is_suppressed(m.start()))
        .map(|m| m.as_str().to_string())
        .collect();

    let mut invoice_values: Vec<String> = Vec::new();
    for m in RE_INVOICE.find_iter(text) {
        let matched = m.as_str();
        if !RE_INVOICE_PREFIX.is_match(matched) {
            let start = code_points_before(text, m.start(), 40);
            let before = &text[start..m.start()];
            if !RE_INVOICE_CONTEXT.is_match(before) {
                continue;
            }
        }
        if is_suppressed(m.start()) {
            continue;
        }
        invoice_values.push(matched.to_string());
    }

    let recency_values: Vec<String> = RE_RECENCY
        .find_iter(text)
        .filter(|m| !is_suppressed(m.start()))
        .map(|m| m.as_str().to_string())
        .collect();

    let mut person_values: Vec<String> = Vec::new();
    for (index, value) in &emails {
        let line_start = text[..*index].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let cp_start = code_points_before(text, *index, 80);
        let window_start = line_start.max(cp_start);
        let before = &text[window_start..*index];
        let mut name: Option<&str> = None;
        for m in RE_NAME_PAIR.find_iter(before) {
            name = Some(m.as_str());
        }
        let Some(name) = name else { continue };
        if is_suppressed(*index) {
            continue;
        }
        person_values.push(format!("{name} {value}"));
    }

    let current_values: Vec<String> = RE_CURRENT
        .find_iter(text)
        .filter(|m| !is_suppressed(m.start()))
        .map(|m| m.as_str().to_string())
        .collect();

    let email_values: Vec<String> = emails.into_iter().map(|(_, value)| value).collect();

    let classes: [(&str, Vec<String>); 6] = [
        ("email", email_values),
        ("card", card_values),
        ("invoice_or_period", invoice_values),
        ("recency", recency_values),
        ("person", person_values),
        ("current_value", current_values),
    ];

    let mut parts: Vec<String> = Vec::new();
    for (name, values) in classes.iter() {
        if values.is_empty() {
            continue;
        }
        let mut distinct: Vec<&String> = Vec::new();
        for value in values {
            if !distinct.iter().any(|seen| *seen == value) {
                distinct.push(value);
            }
        }
        let mut display: Vec<String> = distinct.into_iter().take(3).map(|s| s.clone()).collect();
        if values.len() > display.len() {
            display.push("\u{2026}".to_string());
        }
        parts.push(format!(
            "{} \u{d7}{} ({})",
            name,
            values.len(),
            display.join(", ")
        ));
    }

    if parts.is_empty() {
        return;
    }

    add_finding(
        findings,
        "HON-03",
        file,
        0,
        &format!("Instance data not marked illustrative: {}.", parts.join(", ")),
        "Confirm each value is supplied by the brief or label it illustrative / render unset state.",
    );
}

fn check_states(text: &str, file: &str, findings: &mut Vec<Finding>) {
    let re_fetch =
        regex!(r"(?i)useQuery|useSWR|fetch\(|axios\.|await\s+\w+\.(?:get|post|find|query)");

    if !re_fetch.is_match(text) {
        return;
    }

    let mut missing = Vec::new();

    if !regex!(r"(?i)isLoading|isPending|loading|Skeleton|Spinner").is_match(text) {
        missing.push("loading");
    }
    if !regex!(r"(?i)isError|error|catch\s*\(|ErrorState").is_match(text) {
        missing.push("error");
    }
    if !regex!(r"(?i)length\s*===\s*0|isEmpty|EmptyState|\.length\s*\?|no results|nothing")
        .is_match(text)
    {
        missing.push("empty");
    }

    if !missing.is_empty() {
        add_finding(
            findings,
            "STATE-01",
            file,
            0,
            &format!("Data fetching may lack {} handling.", missing.join(", ")),
            "Inspect the rendered region and record all eight states in REVIEW-PACKET.md.",
        );
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

fn relative_path_from(base: &Path, target: &Path) -> PathBuf {
    let base_parts: Vec<Component<'_>> = base.components().collect();
    let target_parts: Vec<Component<'_>> = target.components().collect();
    if base_parts.first() != target_parts.first() {
        return target.to_path_buf();
    }
    let common = base_parts
        .iter()
        .zip(&target_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut result = PathBuf::new();
    for part in &base_parts[common..] {
        if matches!(part, Component::Normal(_)) {
            result.push("..");
        }
    }
    for part in &target_parts[common..] {
        result.push(part.as_os_str());
    }
    result
}

fn compatibility_path(path: &Path) -> String {
    let relative = if path.is_absolute() {
        std::env::current_dir()
            .map(|cwd| relative_path_from(&cwd, path))
            .unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    let value = relative.to_string_lossy().replace('\\', "/");
    if value.is_empty() {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    } else {
        value
    }
}

/// Run the heuristic source lint on `target`.
///
/// `target` may be a directory or a single file. Returns a structured
/// [`LintSourceReport`] with all findings sorted by severity, file, line.
pub fn lint_source(target: &Path) -> Result<LintSourceReport, EngineError> {
    let meta = fs::symlink_metadata(target)
        .map_err(|e| EngineError::Usage(format!("Cannot read {}: {e}", target.display())))?;
    if meta.file_type().is_symlink() {
        return Err(EngineError::Usage(format!(
            "refusing symlink target: {}",
            target.display()
        )));
    }

    let files: Vec<PathBuf> = if meta.is_dir() {
        collect_files(target)?
    } else if meta.is_file() && is_code_file(target) {
        vec![target.to_path_buf()]
    } else {
        Vec::new()
    };

    if files.is_empty() {
        return Err(EngineError::Usage(format!(
            "No supported source files found under {}.",
            target.display()
        )));
    }

    let mut findings = Vec::new();
    let mut skipped = Vec::new();

    for file in &files {
        let text = match fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) => {
                skipped.push(SkippedFile {
                    path: file.to_string_lossy().to_string(),
                    reason: e.to_string(),
                });
                continue;
            }
        };

        // The Node compatibility stream reports the supplied target prefix,
        // rather than a path relative to the scan root.
        let rel = compatibility_path(file);

        let mut budget = Budget::new();
        check_styles(&text, &rel, &mut budget, &mut findings);
        check_instance_data(&text, &rel, &mut findings);

        if !is_style_file(file) {
            check_markup(&text, &rel, &mut findings);
            check_states(&text, &rel, &mut findings);
        }

        check_budget(&budget, &rel, &mut findings);
    }

    // Sort by severity, file (case-insensitive like JS localeCompare), line
    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| {
                let af = a.file.as_deref().unwrap_or("").to_lowercase();
                let bf = b.file.as_deref().unwrap_or("").to_lowercase();
                af.cmp(&bf)
            })
            .then_with(|| a.line.cmp(&b.line))
    });

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in &findings {
        *counts.entry(f.severity.to_string()).or_insert(0) += 1;
    }

    Ok(LintSourceReport {
        checker: "source".to_string(),
        scope: "heuristic-source-leads".to_string(),
        scanned: files.len(),
        skipped,
        counts,
        findings,
    })
}

/// Check whether the report should fail under `--enforce-heuristics` rules.
///
/// Returns `true` if S1 > 0, S2 > 2, or S3 > 6.
pub fn should_enforce_fail(report: &LintSourceReport) -> bool {
    let s1 = report.counts.get("S1").copied().unwrap_or(0);
    let s2 = report.counts.get("S2").copied().unwrap_or(0);
    let s3 = report.counts.get("S3").copied().unwrap_or(0);
    s1 > 0 || s2 > 2 || s3 > 6
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lint_source_empty_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = lint_source(dir.path());
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn lint_source_rejects_symlink_root() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.tsx"), "<h1>outside</h1>").unwrap();
        let link = dir.path().join("linked");
        symlink(outside.path(), &link).unwrap();
        assert!(lint_source(&link).is_err());
    }

    #[test]
    fn lint_source_detects_budget_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let css = r#"
.a { font-size: 12px; }
.b { font-size: 14px; }
.c { font-size: 16px; }
.d { font-size: 18px; }
.e { font-size: 20px; }
.f { font-size: 24px; }
"#;
        fs::write(dir.path().join("styles.css"), css).unwrap();

        let report = lint_source(dir.path()).unwrap();
        assert_eq!(report.scanned, 1);
        let comp16: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "COMP-16")
            .collect();
        assert!(!comp16.is_empty());
    }

    #[test]
    fn lint_source_detects_banned_heading() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("page.tsx"),
            r#"<div><h2>Overview</h2></div>"#,
        )
        .unwrap();

        let report = lint_source(dir.path()).unwrap();
        let ia03: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "IA-03")
            .collect();
        assert!(!ia03.is_empty());
    }

    #[test]
    fn lint_source_detects_multiple_h1() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("page.tsx"), "<h1>First</h1><h1>Second</h1>").unwrap();

        let report = lint_source(dir.path()).unwrap();
        let a11y11: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "A11Y-11")
            .collect();
        assert!(!a11y11.is_empty());
    }

    #[test]
    fn lint_source_all_findings_are_heuristic() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("app.tsx"), "<h1>A</h1><h1>B</h1>").unwrap();

        let report = lint_source(dir.path()).unwrap();
        for f in &report.findings {
            assert_eq!(f.confidence.as_deref(), Some("heuristic"));
        }
    }

    #[test]
    fn lint_source_detects_motion_without_reduced() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("anim.css"),
            "@keyframes fade { from { opacity: 0; } to { opacity: 1; } }",
        )
        .unwrap();

        let report = lint_source(dir.path()).unwrap();
        let a11y09: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "A11Y-09")
            .collect();
        assert!(!a11y09.is_empty());
    }

    #[test]
    fn lint_source_skips_style_files_for_markup() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("theme.css"), ".btn { color: red; }").unwrap();

        let report = lint_source(dir.path()).unwrap();
        // No ACT-05 or IA-03 from CSS files
        let markup_rules: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule.starts_with("ACT-") || f.rule.starts_with("IA-"))
            .collect();
        assert!(markup_rules.is_empty());
    }

    #[test]
    fn should_enforce_fail_thresholds() {
        let mut report = LintSourceReport {
            checker: "source".into(),
            scope: "heuristic-source-leads".into(),
            scanned: 0,
            skipped: vec![],
            counts: BTreeMap::new(),
            findings: vec![],
        };
        assert!(!should_enforce_fail(&report));

        report.counts.insert("S1".into(), 1);
        assert!(should_enforce_fail(&report));
    }

    #[test]
    fn absolute_paths_render_relative_to_working_directory() {
        let base = Path::new("/workspace/project");
        let target = Path::new("/private/tmp/fixture/App.tsx");
        assert_eq!(
            relative_path_from(base, target),
            PathBuf::from("../../private/tmp/fixture/App.tsx")
        );
    }

    #[test]
    fn comp03_reports_off_scale_values_in_first_seen_order_capped_at_six() {
        // 8 distinct off-scale integer px values in deliberately
        // non-lexicographic document order. A BTreeSet would sort these
        // lexicographically ("10px" < "18px" < "1px" < ...) and silently
        // drop the wrong subset once there are more than 6; Node's `Set`
        // preserves first-seen order, and so must we.
        let dir = tempfile::tempdir().unwrap();
        let css = "
.a { margin: 18px; }
.b { padding: 6px; }
.c { gap: 22px; }
.d { margin: 10px; }
.e { padding: 1px; }
.f { gap: 30px; }
.g { margin: 5px; }
.h { padding: 9px; }
";
        fs::write(dir.path().join("spacing.css"), css).unwrap();

        let report = lint_source(dir.path()).unwrap();
        let comp03: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "COMP-03")
            .collect();
        assert_eq!(comp03.len(), 1);
        assert_eq!(
            comp03[0].message,
            "Off-scale spacing values: 18px, 6px, 22px, 10px, 1px, 30px."
        );
    }

    #[test]
    fn comp03_flags_decimal_px_values_without_truncating() {
        // Node checks `Number(v) % 4 !== 0` on the raw decimal; truncating
        // to an integer first (as `as i64` did) let 4.5px slip through
        // because `(4.5 as i64) % 4 == 0`.
        let dir = tempfile::tempdir().unwrap();
        let css = ".a { padding: 4.5px; margin: 8.25px; gap: 0.5px; }";
        fs::write(dir.path().join("decimals.css"), css).unwrap();

        let report = lint_source(dir.path()).unwrap();
        let comp03: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "COMP-03")
            .collect();
        assert_eq!(comp03.len(), 1);
        assert_eq!(
            comp03[0].message,
            "Off-scale spacing values: 4.5px, 8.25px, 0.5px."
        );
    }

    #[test]
    fn lint_source_empty_dir_error_matches_node_message() {
        let dir = tempfile::tempdir().unwrap();
        let unsupported = dir.path().join("readme.txt");
        fs::write(&unsupported, "no code files here").unwrap();

        let err = lint_source(dir.path()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!(
                "usage error: No supported source files found under {}.",
                dir.path().display()
            )
        );
    }
}
