# TUI-RUN-014

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-annotation` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> Each iteration the system shall poll for input with a frame-budget timeout, apply any resulting commands, advance the simulation, autosave if due, and render one frame, in that order.

## Annotated code

- `crates/clowder-tui/src/runtime/mod.rs`

## What the blind runs found

Readers recovered input handling and command application. None recovered the *order* the spec fixes — poll, commands, ticks, autosave, render — which is its entire subject.

## Recommendation

The loop body genuinely carries five citations in one annotation (TUI-RUN-014, 015, 016, 017, 026). Five requirements in one body, and the ordering one is the least visible. Cite the phases individually where they occur in the function.
