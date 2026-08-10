# TUI-RUN-016

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 32 lines

## Spec as written

> The system shall consume at most a bounded number of input events per frame and shall coalesce queued resize events to the most recent, so a flood of input cannot starve the simulation or the renderer.

## Annotated code

- `crates/clowder-tui/src/runtime/mod.rs`

## What the blind runs found

Readers recovered both halves — the per-frame bound on events consumed, and the coalescing of resizes to the most recent.

## Recommendation

No action. Notable as a compound spec that reads cleanly anyway, because both clauses live in one place and neither is an absence.
