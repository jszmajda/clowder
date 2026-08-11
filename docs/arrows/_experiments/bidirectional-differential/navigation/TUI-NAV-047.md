# TUI-NAV-047

**Classification:** `INCONSISTENT-BLIND` · **Action:** `harness-fixed` · **Audited:** 2026-08-10 · **Extraction:** 3 lines

## Spec as written

> The system shall apply a dead-band to the inspector's layout breakpoint so a single-column resize wobble does not flip the layout between the persistent-pane and swap modes.

## What the blind runs found

Invalid. Readers were handed three lines of imports; two refused outright. My import filter was checking the block *before* comments were stripped, so doc-comment lines counted as logic and the block passed — then stripping removed them and left bare `use` declarations.

## Recommendation

Third harness fix, and the first that was a logic error rather than a threshold being wrong: I filtered the wrong side of a transform. Now judged after stripping. Both specs correctly skipped on re-check. Left out of the audited count.
