# TUI-RUN-021

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> The system shall drive ordinary accumulation from a monotonic clock, so a wall-clock correction can neither rewind the simulation nor inject elapsed time it did not observe.

## Annotated code

- `crates/clowder-tui/src/runtime/clock.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## What the blind runs found

Readers described taking the greater of the two clocks — which is the suspend rule, a sibling spec — rather than this one's claim that the monotonic clock drives accumulation so a wall correction cannot rewind or inject.

## Recommendation

Same annotation as TUI-RUN-018/022/023/024: one line citing five specs above one body. Confirmed by reading the source directly. This is the audit's clearest instance of over-subscription and the strongest case for splitting an annotation.
