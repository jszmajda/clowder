# TUI-RUN-001

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Extraction:** 44 lines

## Spec as written

> The system shall resolve the save path and attempt the load before entering raw mode or the alternate screen, so a startup failure reports itself on an ordinary terminal.

## What the blind runs found

Readers described entering raw mode and the alternate screen with cleanup on failure — the sibling spec. This spec's claim is an *ordering*: the save is resolved and loaded before any of that happens.

## Recommendation

Ordering between two phases cannot be seen from inside either one. The citation sits on the terminal setup; it belongs at the startup sequence where the load precedes it, and the existing child-process test that proves a failure prints on an ordinary terminal is the real evidence.
