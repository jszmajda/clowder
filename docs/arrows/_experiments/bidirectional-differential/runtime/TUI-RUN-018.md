# TUI-RUN-018

**Classification:** `INCONSISTENT-BLIND` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall convert elapsed real time into ticks as elapsed seconds × the core's ticks-per-second × the current speed, running only whole ticks and banking the fractional remainder across frames so no partial tick is lost or double-counted.

## Annotated code

- `crates/clowder-tui/src/runtime/clock.rs`

## What the blind runs found

Readers led with the clock-choice rule — accrue from the larger of monotonic and wall — rather than the conversion this spec describes. One reader did reach the whole-tick conversion; the other two did not.

## Recommendation

The function they read carries five citations at once (TUI-RUN-018, 021, 022, 023, 024). No reader can attribute behaviour to one of five requirements sharing a body. This is over-subscription rather than misplacement, and it is why this spec and TUI-RUN-024 produced near-identical reconstructions.
