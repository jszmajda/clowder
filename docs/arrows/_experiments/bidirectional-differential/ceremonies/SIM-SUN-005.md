# SIM-SUN-005

**Classification:** `UNANNOTATABLE` · **Action:** `signpost` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall define exactly the eleven Sunbeam Rites of the ceremonies design's ladder — warmth+shinies costs, devotion gates, and effects — each unlockable once devotion's persisted high-water has ever reached its gate (so a Reverie that resets live devotion never re-locks a rite) and buyable via BuyRite (Locked before the gate, InsufficientResources when unaffordable).

## Annotated code

- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/state.rs`

## What the blind runs found

Readers described the purchase command's mechanics; the spec defines a *content table* — the ladder's entries, their costs, gates and effects. Word overlap 18-23%, the lowest coherent-looking result in the batch.

## Recommendation

A table-definition spec is not behaviourally recoverable: the table is data, and no reader of the code that consumes it can reconstruct what the table must contain. This is a whole class in the sim leaves — every spec of the form 'shall define exactly the N items of the design's table'. Such specs are verified by a test asserting the table against the design, not by a differential. Recommend excluding this class from future differential scope and auditing it with a table-vs-design check instead.
