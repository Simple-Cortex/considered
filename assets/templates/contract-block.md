# Contract block

`structure` writes one canonical raw contract to:

```text
.considered/<surface-id>/CONTRACT.md
```

`compose` copies the identical contract into a comment wrapper at the top of the built artifact when that file type and ownership make comments safe. The raw sidecar is mandatory for generated, binary, remote, or comment-hostile targets, and is the checker target when no wrapper exists.

Do not rename fields or change separators. `node <skill-dir>/bin/considered.mjs contract` (backed by `scripts/lint-contract.mjs`) parses the raw body and comment-wrapped forms. `P4` documents chrome outside the content hierarchy, so keep P4 chrome out of the zone element list.

## Canonical raw body

```text
CONSIDERED-CONTRACT v1
THESIS:     <the one organizing idea this surface owns, and the default it refuses>
DECISION:   <role> decides <what> within <time budget>; being wrong costs <consequence>
MODE:       persuade | operate | analyze | read | experience
QUESTIONS:
  Q1 [P0] <question in the user's voice> -> <ZoneId>
  Q2 [P1] <question> -> <ZoneId>
ZONES:
  <ZoneId> "<Heading in the user's words>" :: Q1,Q2 :: E1,E2,E3
HIERARCHY:
  P0 <element> - <why it beats every rival>
  P1 <element>, <element>, <element>
  P2 <element>, <element>
  P3 <deferred reference>
  P4 chrome: <nav>, <context filter>, <utility>
ACTIONS:
  S1 primary "<Verb object>" safe
  S1 secondary "<Verb object>" safe
  S1 overflow "<Verb object>" safe
  S4 tertiary "<Verb object>" reversible
DELETED:    <item> - <reason>; <item> - <reason>
ROLL:       <structure ids>, <direction id>, cns-<key>/<generation>
```

## Wrapper forms

Use one wrapper that matches the target: an HTML comment for `.html`, `.md`, `.vue`, `.svelte`, or `.astro`; a block comment for `.js`, `.ts`, `.jsx`, `.tsx`, or `.css`; a line comment for a line-comment-only source; and the sidecar alone — no wrapper — for generated, binary, remote, or comment-hostile targets. The body between wrapper markers must match `CONTRACT.md` byte for byte apart from line indentation introduced by the comment syntax.

### HTML, Markdown, Vue, Svelte, or Astro

```html
<!--
CONSIDERED-CONTRACT v1
THESIS:     ...
DECISION:   ...
MODE:       ...
QUESTIONS:
  Q1 [P0] ... -> A
ZONES:
  A "..." :: Q1 :: E1,E2,E3
HIERARCHY:
  P0 E1 - ...
  P1 E2, E3, E4
  P4 chrome: nav, filters
ACTIONS:
  S1 primary "Verb object" safe
DELETED:    item - reason
ROLL:       ids, direction, cns-<key>/0
-->
```

### JavaScript, TypeScript, JSX, TSX, CSS, or source with block comments

```text
/*
CONSIDERED-CONTRACT v1
THESIS:     ...
DECISION:   ...
MODE:       ...
QUESTIONS:
  Q1 [P0] ... -> A
ZONES:
  A "..." :: Q1 :: E1,E2,E3
HIERARCHY:
  P0 E1 - ...
  P1 E2, E3, E4
  P4 chrome: nav, filters
ACTIONS:
  S1 primary "Verb object" safe
DELETED:    item - reason
ROLL:       ids, direction, cns-<key>/0
*/
```

### Line-comment source

```text
# CONSIDERED-CONTRACT v1
# THESIS:     ...
# DECISION:   ...
# MODE:       ...
# QUESTIONS:
#   Q1 [P0] ... -> A
# ZONES:
#   A "..." :: Q1 :: E1,E2,E3
# HIERARCHY:
#   P0 E1 - ...
#   P1 E2, E3, E4
#   P4 chrome: nav, filters
# ACTIONS:
#   S1 primary "Verb object" safe
# DELETED:    item - reason
# ROLL:       ids, direction, cns-<key>/0
```

## Sidecar and verification

1. Write the raw body to `CONTRACT.md` before composing.
2. Add the matching wrapper to the target when safe. If not safe, record `contract placement: sidecar` in `STRUCTURE.md`.
3. After any contract change, compare the field values in both copies.
4. Run `node <skill-dir>/bin/considered.mjs contract <target>` for a wrapper, or target `.considered/<surface-id>/CONTRACT.md` for sidecar-only delivery.
