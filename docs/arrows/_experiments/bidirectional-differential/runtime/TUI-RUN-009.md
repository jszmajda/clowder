# TUI-RUN-009

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-11 · **Extraction:** 47 lines

## Spec as written

> When the override is given, the system shall clear the lock and start even if its holder is live.

## What the blind runs found

Readers described acquisition and held-detection rather than the override this spec governs. Same 47-line body as two sibling specs.

## Recommendation

Three specs cite one function, and all three reconstructions are the same composite. Cite the force branch.
