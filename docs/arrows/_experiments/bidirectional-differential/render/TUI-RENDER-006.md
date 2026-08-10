# TUI-RENDER-006

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction · **Extraction:** 10 lines

## Spec as written

> At 100 columns and wider the system shall draw the sidebar, the list, and the inspector side by side, the inspector taking the remaining width.

## Annotated code

- `crates/clowder-tui/src/render/layout.rs`

## What the blind runs found

Same shape as its sibling and from the same region — readers described the three-way classification, not the wide-layout behaviour this spec names.

## Recommendation

Two specs citing one classification site, each recovering the other's subject. Cite each band's branch.
