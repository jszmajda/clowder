# TUI-NAV-012

**Classification:** `INCONSISTENT-BLIND` · **Action:** `re-run` · **Audited:** 2026-08-10 · **Extraction:** 76 lines

## Spec as written

> When X is pressed on a focused row that is not a heavy action, the system shall apply the row's command repeatedly until the core rejects it or a shell-side iteration cap is reached.

## What the blind runs found

Readers described the reducer's top-level dispatch — resolve to one outcome, quit evaluated first — not the repeat-until-rejected behaviour this spec governs. Extraction was 76 lines, under the cap but still the whole reducer entry point.

## Recommendation

The 90-line cap is too generous for a large match-based function: the reader sees every arm and reconstructs the dispatch. Re-run with the citation scoped to the specific arm. Left out of the audited count.
