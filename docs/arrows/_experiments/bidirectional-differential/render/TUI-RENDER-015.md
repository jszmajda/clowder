# TUI-RENDER-015

**Classification:** `BD-COHERENT` · **Action:** `none` · **Audited:** 2026-08-10 · **Extraction:** 29 lines

## Spec as written

> The system shall bound each frame's drawing work by what is visible rather than by what exists, so a frame's cost does not grow with the size of the game's content tables.

## What the blind runs found

Readers recovered that only a window of rows is rendered, which is exactly the mechanism by which frame cost is bounded by what is visible.

## Recommendation

No action. The spec's cost framing and the code's windowing are the same thing seen from two sides, and both were legible.
