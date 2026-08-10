# TUI-RENDER-005

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 10 lines

## Spec as written

> Between 80 and 99 columns the system shall draw a sidebar and a list, the inspector appearing only in place of the list when the player calls for it.

## Annotated code

- `crates/clowder-tui/src/render/layout.rs`

## What the blind runs found

Re-run under the capped extractor — 10 lines shown instead of 252. **The finding survives.** Readers still described the three-way layout classification rather than this spec's particular width band, at 43-50% overlap.

## Recommendation

So the earlier result was right for the wrong reason, and the correction was still worth making: the citation sits on the classification that covers all three bands, not on the branch this spec governs. Cite the branch.
