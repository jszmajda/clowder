# TUI-NAV-032

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Extraction:** 6 lines

## Spec as written

> When entering a section with no remembered item (or whose remembered item is no longer revealed), the system shall place the cursor on the first Available row, else the first row.

## What the blind runs found

Readers described row identity and display text, not the landing rule this spec governs. Extraction was six lines — the identity type, not the function that chooses where the cursor lands.

## Recommendation

The citation sits on the key type rather than on the entry function that applies the rule. Move it to where the landing decision is made.
