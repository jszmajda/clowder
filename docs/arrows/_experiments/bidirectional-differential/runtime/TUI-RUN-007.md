# TUI-RUN-007

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When the lock is held by a process that is still running, the system shall refuse to start, name the holding process, and state how to override.

## Annotated code

- `crates/clowder-tui/src/runtime/lock.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## What the blind runs found

All three readers described the *save-loading* failure policy — unreadable, newer-version, corrupt. None described the session lock refusing a second launch, which is the whole of this spec. Overlap 20-27%.

## Recommendation

The citation sits on the startup function, whose conspicuous behaviour is the load policy. Cite the lock refusal branch specifically.
