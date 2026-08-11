# TUI-NAV-017

**Classification:** `B-ONLY-DRIFT` · **Action:** `reconcile-EARS` · **Audited:** 2026-08-10 · **Extraction:** 23 lines

## Spec as written

> Where the terminal is at least about 100 columns wide, the system shall show the inspector as a persistent pane that live-updates for the focused row; on an 80-column terminal, Space shall swap the inspector in place of the list (and Space or Esc returns), never as a popup that stops the render or sim.

## What the blind runs found

Readers recovered the persistent pane above the breakpoint precisely, including that a width equal to the breakpoint counts as wide. None recovered the narrow-terminal swap, which is the spec's other half.

## Recommendation

Compound across two layouts that live in two places. Split.
