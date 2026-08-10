# TUI-THEME-007

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall render reverse-video emphasis by inverting the terminal's own foreground and background pair rather than a palette colour, so emphasis works in either polarity.

## Annotated code

- `crates/clowder-tui/src/render/mod.rs`

## What the blind runs found

Readers recovered reverse-video emphasis applied to the focused row, including that it swaps the terminal's own pair.

## Recommendation

No action on the mechanism. The clause 'rather than a palette colour, so emphasis works in either polarity' is rationale and correctly unrecoverable from code.
