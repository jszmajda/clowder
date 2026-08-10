# TUI-RENDER-004

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> Where the terminal is narrower than 80 columns or shorter than 24 rows, the system shall draw only the too-small card, while the simulation continues to advance.

## Annotated code

- `crates/clowder-tui/src/render/layout.rs`
- `crates/clowder-tui/src/render/mod.rs`

## What the blind runs found

Readers recovered the threshold and the suppression of every normal region, one of them almost verbatim. None recovered 'while the simulation continues to advance'.

## Recommendation

Compound, and the missing clause lives in a different leaf — the loop's ordering, not the layout's. Split it, and let the runtime spec that already guarantees ticking regardless of render state carry that half.
