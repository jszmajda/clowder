# SIM-MOVE-007

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall contribute Elder Wisdom to production stack layer 7 as +1% per point, limited_dr-capped at (2.0 × nine_lives_ratio), and to storage as +0.1% per point (linear); the cap and slope scale with the Nine Lives traditions.

## Annotated code

- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## What the blind runs found

Word overlap 10-15% — the lowest in three batches. All three readers described save migration: missing fields taking their defaults when an older save is loaded. The spec is about a production-stack contribution and its diminishing-returns cap.

## Recommendation

One of this spec's two citations sits on a struct field carrying a serde default attribute. The conspicuous behaviour there is migration, which belongs to the persistence segment. Move the citation to the effect-resolution code that actually applies the contribution and its cap.
