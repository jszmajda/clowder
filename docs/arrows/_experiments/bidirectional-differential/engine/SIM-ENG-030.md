# SIM-ENG-030

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall expose a per-resource net rate at current state, per game second, computed by running the tick pipeline's own definition of every per-tick stock-moving phase — excluding the day-boundary, random-event, and clamp phases — never from a separate formula that could diverge from it.

## Annotated code

- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/tick.rs`

## What the blind runs found

Readers recovered the derivation precisely — one resource-phase tick on a cloned state, including the conditionally-active middle phase, scaled to a per-second rate.

## Recommendation

No action on the mechanism. The clause a reader cannot recover is the prohibition — 'never from a separate formula that could diverge' — which is rationale, not behaviour, and is correctly unrecoverable from code. It belongs in the design doc, where it already is.

## Method note

Three sessions received only the spec text and wrote a naive implementation; three
received only the code with comments, spec ids and annotations stripped, and
reconstructed the requirement. Neither side saw the other's input or this project.
