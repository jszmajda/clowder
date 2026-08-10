# TUI-NAV-026

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall render typed events into a dim ticker and, on e, a scrollable Journal overlay over the retained journal — the recent-event window plus every milestone and report the core has kept — which survives closing and reopening the game; no event shall flash, block, or expire unseen while it is retained.

## Annotated code

- `crates/clowder-tui/src/nav/update.rs`
- `crates/clowder-tui/src/nav/view.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/render/widgets/overlays.rs`

## What the blind runs found

Every blind reader led with an invariant the spec never states: the journal is presented **newest-first**. They also did not recover the spec's claim that the journal survives closing and reopening the game.

## Recommendation

The code carries an ordering guarantee that intent does not record — so nothing stops a later change reversing it, and no test would fail. Add the ordering to the spec. The survives-restart half is core-side (the journal is saved state) and is not visible in the annotated shell code, which is expected rather than wrong.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
Agreement *within* a direction and disagreement *between* them is the signal.
