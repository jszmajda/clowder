# TUI-RUN-002

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 14 lines

## Spec as written

> On startup the system shall enter raw mode and the alternate screen, and shall restore both on every exit path — normal quit, error return, panic, and termination signal.

## Annotated code

- `crates/clowder-tui/src/runtime/term.rs`

## What the blind runs found

All three readers recovered restoration on every exit path including panic unwind, one at 63% overlap — the highest single reconstruction in the audit.

## Recommendation

No action. The entering half is less recovered than the restoring half, which is fair: the guard's whole purpose is the undo.
