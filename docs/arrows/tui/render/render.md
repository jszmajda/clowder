# Arrow: render

This leaf owns everything between a `GameState` snapshot and the cells on screen: how the frame divides into regions, what each region draws, how the layout responds to size, how numbers and names are

## Status

**PARTIAL** — last audited 2026-11-01 (commit `3fb11a5d7099`). 33 of 33 active specs implemented, each cited by a test; 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/render/render-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/render/render-specs.md` — 34 specs, prefix `TUI-RENDER`

### Tests
- `crates/clowder-tui/tests/integration/armed_summary.rs`
- `crates/clowder-tui/tests/integration/bench.rs`
- `crates/clowder-tui/tests/integration/chrome.rs`
- `crates/clowder-tui/tests/integration/composed.rs`
- `crates/clowder-tui/tests/integration/count_prompt.rs`
- `crates/clowder-tui/tests/integration/fmt.rs`
- `crates/clowder-tui/tests/integration/frame.rs`
- `crates/clowder-tui/tests/integration/inspector.rs`
- `crates/clowder-tui/tests/integration/kitten_card.rs`
- `crates/clowder-tui/tests/integration/layout.rs`
- `crates/clowder-tui/tests/integration/pointer.rs`
- `crates/clowder-tui/tests/integration/readouts.rs`
- `crates/clowder-tui/tests/integration/resize.rs`
- `crates/clowder-tui/tests/integration/sidebar_links.rs`
- `crates/clowder-tui/tests/integration/sidebar_scroll.rs`
- `crates/clowder-tui/tests/integration/store_card.rs`

### Code
- `crates/clowder-tui/src/render/fmt.rs`
- `crates/clowder-tui/src/render/layout.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/render/pointer.rs`
- `crates/clowder-tui/src/render/portrait.rs`
- `crates/clowder-tui/src/render/store_art.rs`
- `crates/clowder-tui/src/render/widgets/chrome.rs`
- `crates/clowder-tui/src/render/widgets/inspector.rs`
- `crates/clowder-tui/src/render/widgets/overlays.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-RENDER-001 – TUI-RENDER-034 | 33 | 1 | 0 |

**Summary:** 33 of 33 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code, its tests, and the design by a reader given only the arrow artifacts, its findings then checked by hand. It found an overflow note clipped with the cursor mid-list (fixed, TUI-RENDER-011), a rate that is not a number signed as a drain (fixed, TUI-RENDER-012), ties judged two ways in the compact form (fixed and pinned, TUI-RENDER-017), specs that left out what a frame really reads (corrected, TUI-RENDER-001), an inspector minimum the dead-band undercuts (stated and tested, TUI-RENDER-008), and annotations on shapes and helpers (moved). A reader's pass is no proof: specs 002 to 007, 010, 013, 014, and 023 were read more lightly than the rest. Issue #3 placed the store-link mark in the column every store keeps free (TUI-RENDER-034, TUI-RENDER-019).

## Work Required

- #18: the frame budget's unmet part (TUI-RENDER-015, deferred).
- #21: the last clause of TUI-RENDER-028 is untested and may be unreachable.
