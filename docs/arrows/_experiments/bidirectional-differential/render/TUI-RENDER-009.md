# TUI-RENDER-009

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall derive the list's scroll position each frame from the cursor's position, keeping the cursor visible with a margin above and below, rather than storing a scroll offset.

## Annotated code

- `crates/clowder-tui/src/render/mod.rs`

## What the blind runs found

All three readers recovered the derived scroll window and the margin held around the cursor; one named the margin exactly.

## Recommendation

No action. The clause none recovered — that the position is *derived each frame rather than stored* — is a statement about what the code does not do, and is correctly invisible.
