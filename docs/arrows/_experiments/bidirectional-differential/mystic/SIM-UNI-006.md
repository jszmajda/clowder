# SIM-UNI-006

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> At each day boundary, the system shall roll a prism descent with probability equal to the summed prism-descent effects (0.02%/day per Rainbow Arch structure, plus Arch-upgrade bonuses), adding one prism on success, in the fixed random-event draw order.

## Annotated code

- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/tick.rs`

## What the blind runs found

All three readers recovered the daily roll, the aggregate probability, and the ownership condition that enables it.

## Recommendation

No action.
