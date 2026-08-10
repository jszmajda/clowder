# TUI-NAV-003

**Classification:** `BIDIRECTIONAL-DRIFT` · **Action:** `reconcile-EARS-or-code` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When Space (or i) is pressed on a focused row, the system shall toggle the inspector for that row.

## Annotated code

- `crates/clowder-tui/src/nav/keys.rs`
- `crates/clowder-tui/src/nav/update.rs`

## What the blind runs found

The spec says "toggle the inspector **for that row**". Every blind reader described a single global preference — "a persistent preference flipped on each press". The code holds one boolean for the whole view.

## Recommendation

Either the spec means a per-row inspection state that was never built, or "for that row" is loose phrasing for "showing the focused row" and the spec should say so. This is a genuine fork about intent and needs deciding, not patching. A-runs split 1-vs-2 on whether to build per-row state, which is itself evidence the sentence admits both readings.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
Agreement *within* a direction and disagreement *between* them is the signal.
