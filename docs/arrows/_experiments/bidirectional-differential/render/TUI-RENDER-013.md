# TUI-RENDER-013

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall scale a number's displayed precision to its magnitude, showing no trailing decimals that imply precision the value does not carry.

## Annotated code

- `crates/clowder-tui/src/render/fmt.rs`

## What the blind runs found

All three readers recovered the significant-figure limit, the magnitude suffix, and the trimming of trailing zeros.

## Recommendation

No action.
