# SIM-KIT-015

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> At each day close, the system shall update exactly one famine clock: a day that ended in shortfall increments famine_days and resets recovery_days; a day that did not increments recovery_days and resets famine_days (both saturating).

## Annotated code

- `crates/clowder-core/src/tick.rs`

## What the blind runs found

All three readers recovered the day-boundary counter behaviour and the branch that decides which counter moves. Consistent within direction.

## Recommendation

No action. Lower word overlap than SIM-ECO-021 reflects different vocabulary, not a different requirement.
