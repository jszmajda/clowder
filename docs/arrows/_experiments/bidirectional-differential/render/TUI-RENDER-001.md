# TUI-RENDER-001

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Extraction:** 113 lines

## Spec as written

> The system shall derive every frame as a pure function of the game-state snapshot, the shell's view state, and the terminal size, caching nothing between frames and mirroring no value the core owns.

## What the blind runs found

Readers recovered that everything shown derives from current state. None recovered the clause the thesis actually rests on: that *nothing is cached between frames*.

## Recommendation

The leaf's central claim is an absence. A frame that derives from state looks identical to one that derives from state *and* keeps a cache — the difference is what the code does not have. This is the most consequential instance of the absence class in the project, because purity is what makes layout testable at exact sizes. It is already asserted by the frame-purity tests; the spec should cite them rather than the drawing code.
