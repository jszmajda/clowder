# TUI-RUN-029

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-11 · **Extraction:** 47 lines

## Spec as written

> The system shall acquire the lock before reading the save, and shall release it on every path that then refuses to start, so that two simultaneous launches cannot both pass the check and no refusal leaves a lock behind.

## What the blind runs found

Readers described the acquisition mechanism. The spec's claim is an *ordering* — the lock is taken before the save is read — plus a release-on-every-refusal guarantee that spans the startup sequence.

## Recommendation

Relational, like TUI-RUN-001: neither the lock nor the load contains the relation between them. The simultaneous-launch test is the only thing that observes it; cite there.
