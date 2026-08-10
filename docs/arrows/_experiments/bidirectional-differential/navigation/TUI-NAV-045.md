# TUI-NAV-045

**Classification:** `BIDIRECTIONAL-DRIFT` · **Action:** `reconcile-code` · **Audited:** 2026-08-10 · **Runs:** 3 per direction

## Spec as written

> When the hint bar's context keys exceed the available width, the system shall drop the least-relevant keys first and always retain the help key last, the complete key map remaining available through the help overlay.

## Annotated code

- `crates/clowder-tui/src/render/widgets/chrome.rs`

## What the blind runs found

All three readers described choosing hints appropriate to the current context. None described the overflow behaviour — dropping least-relevant keys first while retaining the help key — which is the entire subject of this spec.

## Recommendation

The context selection is implemented and the width-overflow handling appears not to be. Worth confirming directly: if the hint bar never drops keys, a narrow terminal will overflow or truncate arbitrarily, and the help key is not guaranteed to survive.
