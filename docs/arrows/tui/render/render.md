# Arrow: render

This leaf owns everything between a `GameState` snapshot and the cells on screen: how the frame divides into regions, what each region draws, how the layout responds to size, how numbers and names are

## Status

**OK** — last audited 2026-11-11 (commit `a414659672bc`). 38 of 38 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/render/render-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/render/render-specs.md` — 38 specs, prefix `TUI-RENDER`

### Tests
- `crates/clowder-tui/tests/integration/act_prices.rs`
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
- `crates/clowder-tui/tests/integration/new_rows.rs`
- `crates/clowder-tui/tests/integration/pointer.rs`
- `crates/clowder-tui/tests/integration/readouts.rs`
- `crates/clowder-tui/tests/integration/resize.rs`
- `crates/clowder-tui/tests/integration/sidebar_links.rs`
- `crates/clowder-tui/tests/integration/sidebar_scroll.rs`
- `crates/clowder-tui/tests/integration/store_card.rs`

### Code
- `crates/clowder-tui/src/nav/pointer.rs`
- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/nav/view.rs`
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
| All | TUI-RENDER-001 – TUI-RENDER-038 | 38 | 0 | 0 |

**Summary:** 38 of 38 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code, its tests, and the design by a reader given only the arrow artifacts, its findings then checked by hand. It found an overflow note clipped with the cursor mid-list (fixed, TUI-RENDER-011), a rate that is not a number signed as a drain (fixed, TUI-RENDER-012), ties judged two ways in the compact form (fixed and pinned, TUI-RENDER-017), specs that left out what a frame really reads (corrected, TUI-RENDER-001), an inspector minimum the dead-band undercuts (stated and tested, TUI-RENDER-008), and annotations on shapes and helpers (moved). A reader's pass is no proof: specs 002 to 007, 010, 013, 014, and 023 were read more lightly than the rest. Issue #3 placed the store-link mark in the column every store keeps free (TUI-RENDER-034, TUI-RENDER-019). Issue #12 moved the sidebar to one all-stores rate query a frame (TUI-RENDER-035), held by a timing test on a large roster, where one query outweighs the drawing; on a long-running save that took a game from about 7.7% of a core to about 4.4%. Issue #10 routed the frame's reads of the clock through the runtime's one function. A frame builds each section's row list at most once and drops the lists when it ends (TUI-RENDER-015, #18); on a copy of a long-running save in a 120×40 pane with no keys pressed, that moved the game from about 6.8% of a core to about 5.2%. The inspector widget keeps no placement rule of its own; the layout and the frame hold the only one (#22). The header rule's sweep covers the header's fifth width step (#23). How often frames run is the runtime leaf's decision, and the Frame Budget now points there (#24).

Audited 2026-11-03, for what issue #39 changed. A click now holds its section's list (TUI-RENDER-036), the pointer record carrying the line the frame began on. The held line is only where each frame's derivation starts (TUI-RENDER-009), held to keeping the cursor drawn across every list, pane, cursor, and held line down to a pane of one line. Clicks inside the drawn list move nothing, an edge row moves the list by one, and later steps move it no further than the lines they pass, a group header counted. A list no click has held scrolls as it did, its tests unchanged. Each test was checked by breaking what it guards. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #45 changed. A count prompt's count keeps its place and its layout (TUI-RENDER-021), its text and every fit unchanged, and is only drawn apart (TUI-THEME-034). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #21 changed. The clause cutting the bench's seats could not be reached: the place fits the seats at six digits each, which the test now pins, far past a meadow's figures. It left the spec, the design, and the code (TUI-RENDER-028).

Audited 2026-11-04, for what issue #46 changed. The cost field draws a fixed act's price as it draws any other (TUI-RENDER-020), and the Sky list now gives the field, since rows it lists state that heading.

Audited 2026-11-11, for what issue #49 changed. A new row draws `★` in its cursor column and moves no other character (TUI-RENDER-037), which a test checks by comparing the row with and without the mark. An overflow note counts the new rows past its end (TUI-RENDER-038). The References now also list the two navigation files that hold a clicked list's line (TUI-RENDER-036), which the last regeneration had missed. Every spec marked implemented has a test citing it, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
