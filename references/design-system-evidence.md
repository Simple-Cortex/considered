# Design system evidence

Use this reference when an audit or recommendation may involve an existing component library. Keep reusable library facts distinct from project-specific overrides and design judgment.

## Discover and identify

Inspect the target project’s installed packages, imports, local component wrappers, themes/tokens, and available docs. `inventory` can help locate component directories and docs; it does not prove a component is available or suitable.

Cache under `.considered/cache/design-systems/<package-slug>-<12-hex-hash>/<version>-<12-hex-hash>/index.json` (for example `acme-ui-ec8a6799207c/1.2.3-c47f5b18b8a4/index.json`; the hash suffixes are appended by the importer, so use the `index` path it returns). The Rust surface discovery skips this cache directory. Use the package’s resolved lockfile or installed package identity, not a manifest’s version range. For unknown or unversioned packages, identify the source revision or fingerprint instead. Include fingerprints for local wrappers and theme/token sources that affect component behavior.

Use evidence in this order: target source and checked-in docs; official docs/API for the resolved version; connected component/design-system MCP; secondary references checked against stronger sources when possible. Record source paths or URLs and version/revision/fingerprint for consequential claims. A latest-version page or similarly named library is not evidence of the target’s installed behavior.

## Keep a small catalog; load details on demand

The index is a broad, cheap catalog for discovery: component names, short purpose, detail-file path, and explicit coverage metadata. Do not crawl or copy the full library catalog during each audit. Create or refresh component detail files only for components needed by the current audit or recommendation. The catalog entry should point to a file conforming to `assets/templates/DESIGN-SYSTEM-COMPONENT.json`; the index should follow `assets/templates/DESIGN-SYSTEM-INDEX.json`.

The index shape includes `schema_version`, package/version identity, source and fingerprints, catalog entries (`name`, `purpose`, `detail_path`), and coverage. A detail file includes `schema_version`, package/version identity, component name, usage, API/variants and constraints, accessibility requirements, and provenance. Mark unverified or undocumented fields unknown; do not infer them from a component name.

A missing catalog entry does not prove a component is unavailable unless coverage is complete and verified for the relevant exports. State what coverage includes and what remains unknown.

Copy the templates as scaffolding and replace their blank entries with evidence. Catalog coverage is `complete`, `partial`, or `unknown`; component coverage is `complete`, `partial`, or `unreviewed`; freshness is `current`, `stale`, or `unknown`. Source kinds include `library`, `local_wrapper`, `theme`, `design_token`, and `documentation`. Keep detail paths relative to the index directory (normally `components/<component-name>.json`). An unreviewed catalog entry may point to a detail file that has not been created yet. Keep the catalog broad enough to discover unused alternatives, and add details only when needed.

## Check freshness economically

On reuse, cheaply compare resolved package identity and available local source fingerprints. Refresh only missing or changed catalog data, and fetch only the detail entries needed for the current task. Do not run a full crawl per audit. If documentation has no reliable change signal, record that freshness is unknown; a timestamp alone does not establish validity. Preserve prior evidence until a refresh is complete, and retain provenance for each claim.

## Import and use a prepared cache

The host gathers and normalizes source evidence, including the broad catalog and any component detail documents needed for the task. It then passes that prepared index to the Rust companion. The Rust commands provide deterministic cache persistence, bounded retrieval, workspace freshness checks, and component snapshots; they do not crawl remote documentation, discover package catalogs, or replace host-side evidence collection.

```text
considered-rs design-system import <source-index.json> [--workspace .]
considered-rs design-system query <index.json> [--query <text>] [--component <name> ...] \
  [--fields purpose,api,accessibility] [--limit 10] [--max-bytes 8192] [--details]
considered-rs design-system freshness <index.json> [--workspace .]
considered-rs design-system snapshot <index.json> --component <name> ... --output-dir <directory>
```

`import` accepts normalized evidence prepared by the host and writes it to the workspace cache location. `query` returns a compact single-line JSON response subject to `--max-bytes` in UTF-8 bytes, including the CLI's trailing newline; `--max-bytes 0` disables the bound. Use repeated `--component` values for specific entries and `--details` when their detail records are needed. `--details` embeds full records and may need a larger `--max-bytes`; an item is skipped and named in `coverage.oversized` (first 10 names, plus `coverage.oversized_count`) only when it cannot fit even alone; once the budget is full, results stop and `coverage.truncated` is true. `freshness` checks recorded local fingerprints and, for npm identities, the package version under `node_modules`; other package-resolution hosts should supply the resolved identity and relevant fingerprints in the prepared evidence. `snapshot` writes the selected detail records to the requested directory. Successful commands emit one JSON object on stdout; input and processing errors are plain text on stderr and exit with code `2`.

Partial or unknown catalog imports merge into the existing cache for the same package, resolved version, and source. Previously indexed components and missing detail records are retained with unknown freshness. A catalog marked `coverage.status: complete` is authoritative for membership and can remove omitted components. To discard a matching component's old detail without supplying a replacement, set `invalidate_cached_detail: true` on its incoming partial/unreviewed entry. Imports refuse to reuse a cache with a different source identity.

Freshness checks use fingerprint summaries materialized during import, so they do not reread every component detail. A shared local file is hashed once per check. Unknown documentation freshness remains unknown even when the installed package version matches.

Hosts that route commands through the native companion may expose the same operations as `considered design-system ...`, when that companion is configured. A cache import does not establish that evidence is correct: retain source provenance, version identity, coverage, and uncertainty as described above.

## Use evidence in decisions

For a component-related recommendation, connect the surface need to verified component purpose, API/constraints, and accessibility behavior, then explain the fit. Separate stable library facts from project overrides (including local wrappers and themes) and from recommendation rationale. Never cache build/recommendation rationale as reviewer evidence. Use the target’s actual resolved API in implementation advice; label unavailable or inconclusive evidence and resulting recommendations provisional. General design judgment may supplement library evidence, but should remain clearly distinct from it.

In audit output, cite the repository path or documentation URL and version/revision near consequential claims. Mention system implications only when they change the diagnosis or recommended fix.
