# Arrow: render

This leaf owns everything between a `GameState` snapshot and the cells on screen: how the frame divides into regions, what each region draws, how the layout responds to size, how numbers and names are

## Status

**PARTIAL** — last audited 2026-11-02 (commit `fbf98a4799fb`). 35 of 35 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/render/render-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/render/render-specs.md` — 35 specs, prefix `TUI-RENDER`

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
- `crates/clowder-tui/src/nav/rows.rs`
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
| All | TUI-RENDER-001 – TUI-RENDER-035 | 35 | 0 | 0 |

**Summary:** 35 of 35 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code, its tests, and the design by a reader given only the arrow artifacts, its findings then checked by hand. It found an overflow note clipped with the cursor mid-list (fixed, TUI-RENDER-011), a rate that is not a number signed as a drain (fixed, TUI-RENDER-012), ties judged two ways in the compact form (fixed and pinned, TUI-RENDER-017), specs that left out what a frame really reads (corrected, TUI-RENDER-001), an inspector minimum the dead-band undercuts (stated and tested, TUI-RENDER-008), and annotations on shapes and helpers (moved). A reader's pass is no proof: specs 002 to 007, 010, 013, 014, and 023 were read more lightly than the rest. Issue #3 placed the store-link mark in the column every store keeps free (TUI-RENDER-034, TUI-RENDER-019). Issue #12 moved the sidebar to one all-stores rate query a frame (TUI-RENDER-035), held by a timing test on a large roster, where one query outweighs the drawing; on a long-running save that took a game from about 7.7% of a core to about 4.4%. Issue #10 routed the frame's reads of the clock through the runtime's one function. A frame builds each section's row list at most once and drops the lists when it ends (TUI-RENDER-015, #18); on a copy of a long-running save in a 120×40 pane with no keys pressed, that moved the game from about 6.8% of a core to about 5.2%. The inspector widget keeps no placement rule of its own; the layout and the frame hold the only one (#22). The header rule's sweep covers the header's fifth width step (#23). How often frames run is the runtime leaf's decision, and the Frame Budget now points there (#24).

## Work Required

- #21: the last clause of TUI-RENDER-028 is untested and may be unreachable.
