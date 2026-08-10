# TUI-RUN-027

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall record a save's banked-but-unrun ticks as an explicit count beside the present timestamp, so a save taken mid-catch-up owes exactly the ticks it banked — neither lost nor counted twice, and never re-priced by a speed set after they were earned.

## Annotated code

- `crates/clowder-tui/src/runtime/clock.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## What the blind runs found

All three readers recovered the requirement *and its purpose*: state plus timestamp plus the outstanding backlog, 'so that ticks still owed are carried across'.

## Recommendation

No action. Worth noting why this one is clean: it is the spec rewritten after the save-seam bug, where the mechanism was changed to carry an explicit count rather than smuggle it through a back-dated timestamp. The clearer mechanism made the intent recoverable from the code.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
