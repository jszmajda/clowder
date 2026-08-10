# SAVE-006

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When loading with a caller-supplied now_unix, the system shall compute elapsed = max(0, now_unix − saved_at_unix), cap it at 365 days, and replay elapsed × 4 × speed ticks (speed = the save's persisted game-speed) through the core's replay entry point — the same simulation step as live play in every respect but journaling, which is history-keeping rather than a game rule.

## Annotated code

- `crates/clowder-core/src/save.rs`

## What the blind runs found

Every reader described version refusal, corruption handling, and numeric sanitisation. None described offline catch-up — which is the entire subject of this spec.

## Recommendation

The citation sits on the load function, which does four things; catch-up is the last and least conspicuous. The dominant behaviour a reader sees belongs to SAVE-002, SAVE-004 and SAVE-005. Cite the replay loop specifically rather than the whole function.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
