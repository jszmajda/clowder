# TUI-RENDER-008

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall apply an asymmetric dead-band at each breakpoint — entering a layout at the boundary and leaving it some columns below — so a resize that overshoots and settles does not flip the layout repeatedly.

## Annotated code

- `crates/clowder-tui/src/render/layout.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## What the blind runs found

All three readers recovered the hysteresis: a layout entered at its boundary is held while the previous frame was already in it.

## Recommendation

No action.
