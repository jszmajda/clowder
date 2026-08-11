# TUI-RENDER-011

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Extraction:** 37 lines

## Spec as written

> When a list is longer than its viewport, the system shall indicate that more rows exist beyond the visible range without spending a column on a scrollbar.

## What the blind runs found

Readers recovered the scrolled window that keeps the cursor visible — the sibling spec — rather than the indicator this spec requires. The overflow note is three lines inside a function whose bulk is windowing.

## Recommendation

Cite the note, not the window.
