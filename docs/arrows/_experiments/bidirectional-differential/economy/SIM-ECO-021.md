# SIM-ECO-021

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> If pooled demand exceeds the available catnip stock, then the system shall set the stock to zero and mark the tick as a shortfall; a stock exactly covering demand is not a shortfall.

## Annotated code

- `crates/clowder-core/src/tick.rs`

## What the blind runs found

All three readers recovered the per-tick consumption rule including its guard conditions and the population subset it applies to. Word overlap 60-67% with the spec, well under the leak threshold.

## Recommendation

No action.
