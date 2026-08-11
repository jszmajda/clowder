# TUI-RENDER-014

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Extraction:** 38 lines

## Spec as written

> When a name does not fit its column, the system shall shorten it with an explicit ellipsis so the player can see that text was cut, the full name remaining available in the inspector.

## What the blind runs found

Readers recovered the truncation and, notably, that width is measured in display cells rather than bytes — the property that keeps multi-byte names from being split. None recovered the ellipsis, which is what the spec is about.

## Recommendation

The ellipsis is the requirement; truncation is how. Readers describing the mechanism and missing the marker suggests the marker is incidental in the code. State it in the spec as the observable, and assert it in a test.
