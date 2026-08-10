# TUI-RUN-008

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When the lock's recorded process is no longer running — or its id now belongs to a process with a different start time — the system shall treat the lock as stale, clear it, and continue starting.

## Annotated code

- `crates/clowder-tui/src/runtime/lock.rs`

## What the blind runs found

Highest agreement in the audit so far — 61-67% overlap, all three recovering exclusive creation, the recorded process id and start time, and the staleness decision built on both. Under the 70% threshold that would suggest a stripping leak.

## Recommendation

No action. Notable as the cleanest result yet from a shell segment.
