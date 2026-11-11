# Arrow: navigation

This leaf owns how a human drives clowder from a keyboard: the key model, section navigation, the action model (select / act / repeat / bulk / confirm), the inspector, the glanceable chrome, discovera

## Status

**OK** — last audited 2026-11-11 (commit `9c1932d81d1c`). 213 of 213 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/navigation/navigation-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/navigation/navigation-specs.md` — 213 specs, prefix `TUI-NAV`

### Tests
- `crates/clowder-tui/tests/integration/act_prices.rs`
- `crates/clowder-tui/tests/integration/raze.rs`
- `crates/clowder-tui/tests/integration/achievements.rs`
- `crates/clowder-tui/tests/integration/act_results.rs`
- `crates/clowder-tui/tests/integration/adventures.rs`
- `crates/clowder-tui/tests/integration/armed_summary.rs`
- `crates/clowder-tui/tests/integration/bench.rs`
- `crates/clowder-tui/tests/integration/chrome.rs`
- `crates/clowder-tui/tests/integration/composed.rs`
- `crates/clowder-tui/tests/integration/count_prompt.rs`
- `crates/clowder-tui/tests/integration/frame.rs`
- `crates/clowder-tui/tests/integration/inspector.rs`
- `crates/clowder-tui/tests/integration/inspector_reasons.rs`
- `crates/clowder-tui/tests/integration/keys.rs`
- `crates/clowder-tui/tests/integration/kitten_card.rs`
- `crates/clowder-tui/tests/integration/loop_body.rs`
- `crates/clowder-tui/tests/integration/neighbour_offers.rs`
- `crates/clowder-tui/tests/integration/overlays.rs`
- `crates/clowder-tui/tests/integration/palette.rs`
- `crates/clowder-tui/tests/integration/pen_pals.rs`
- `crates/clowder-tui/tests/integration/pointer.rs`
- `crates/clowder-tui/tests/integration/readouts.rs`
- `crates/clowder-tui/tests/integration/report_queue.rs`
- `crates/clowder-tui/tests/integration/resize.rs`
- `crates/clowder-tui/tests/integration/rows.rs`
- `crates/clowder-tui/tests/integration/section_reveal.rs`
- `crates/clowder-tui/tests/integration/sidebar_links.rs`
- `crates/clowder-tui/tests/integration/sidebar_scroll.rs`
- `crates/clowder-tui/tests/integration/sources_hint.rs`
- `crates/clowder-tui/tests/integration/store_card.rs`
- `crates/clowder-tui/tests/integration/ticker_lights.rs`
- `crates/clowder-tui/tests/integration/transients.rs`
- `crates/clowder-tui/tests/integration/update.rs`
- `crates/clowder-tui/tests/integration/waking.rs`
- `crates/clowder-tui/tests/integration/new_rows.rs`
- `crates/clowder-tui/tests/integration/scroll.rs`

### Code
- `crates/clowder-tui/src/nav/action.rs`
- `crates/clowder-tui/src/nav/keys.rs`
- `crates/clowder-tui/src/nav/mnemonics.rs`
- `crates/clowder-tui/src/nav/mod.rs`
- `crates/clowder-tui/src/nav/palette.rs`
- `crates/clowder-tui/src/nav/pointer.rs`
- `crates/clowder-tui/src/nav/reports.rs`
- `crates/clowder-tui/src/nav/rows.rs`
- `crates/clowder-tui/src/nav/section.rs`
- `crates/clowder-tui/src/nav/update.rs`
- `crates/clowder-tui/src/nav/view.rs`
- `crates/clowder-tui/src/render/fmt.rs`
- `crates/clowder-tui/src/render/layout.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/render/widgets/chrome.rs`
- `crates/clowder-tui/src/render/widgets/inspector.rs`
- `crates/clowder-tui/src/render/widgets/overlays.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/runtime/wake.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-NAV-001 – TUI-NAV-213 | 213 | 0 | 0 |

**Summary:** 213 of 213 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Issue #2 added what a store's card opens with (TUI-NAV-197). The audit found the persistent inspector repeating an arming's summary (fixed, TUI-NAV-077) and a placement helper no frame calls (annotation removed, the rest filed). Issue #3 added the sidebar's store-link marks (TUI-NAV-198, TUI-NAV-199, TUI-NAV-200), with marks read off drawn frames over every row of every section. Issue #20 found flavour events reaching the screen as a single placeholder word, because no line ever read their template; they now read as written, for every template (TUI-NAV-201). Issue #10 moved the held-key and arming tests onto a clock they control, so they take no real time and no longer fail under load. `nav::rows` serves a frame's row lists through one scoped build, held by a guard tied to the state's borrow (#18); outside a frame, every call still builds afresh. The inspector's place is decided by `render::layout::compute` and the frame's narrow body alone, and the tests read it off whole frames, through the dead-band in both directions (#22). The header's kitten fact states the clowder's room and free slots from the core's own queries (TUI-NAV-202, #23), and the design's list of chips and width budget match the code again.

Audited 2026-11-02, for what issue #25 changed. It added a sentence for a new structure field (TUI-NAV-203), held word for word from the field and by the test that a statement is every field its definition sets. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #27 changed. An in-process gap past the absence cap became a third route to a retained report (TUI-NAV-027), whose banner says more than the cap (TUI-NAV-055), read off a drawn frame. Which reports get a banner stays judged from the journal alone. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #30 changed. No rule of this segment changed: its rows read the core's gates, so what they list moved with them, and one inspector fixture named the new idea. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #33 changed. The waking screen and the report's banner now also serve a catch-up begun in play (TUI-NAV-027, TUI-NAV-102, TUI-NAV-103), and the capped gap's own queueing went with its report. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #34 changed. A shy colony's pen-pal row states the courtesy gift's chance beside the nap's, read from the core's constant (TUI-NAV-172). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #37 changed. What the core answers as held before an absence now survives a quit mid-wake, so the chip and the `new` tag still tell what the absence earned, tested through the shell (TUI-NAV-174). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #38 changed. The Meadow's structures stand in one ordinary group per family (TUI-NAV-097, TUI-NAV-098, TUI-NAV-100), so `[`/`]` stop at each and a header stands only over a listed row (TUI-NAV-099). The list is no shorter, so what stands below it is still off-screen until the cursor or a group key reaches it. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #39 changed. A press on a row now also holds that section's list where the frame aimed at drew it (TUI-NAV-129), so the clicked row stays under the pointer; the wheel's rule is unchanged (TUI-NAV-130). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #44 changed. `X` on a recipe's make row is now one call that makes every batch the stocks cover (TUI-NAV-012); every other row keeps its run of calls and its cap. The census of commands a row must reach names the new command as having no row of its own (TUI-NAV-039). The shell-cap comments now say what holds on both kinds of row.

Audited 2026-11-04, for what issue #43 changed. `-` and `Backspace` arm taking a copy down from a structure's row (TUI-NAV-204), the armed row states the core's report (TUI-NAV-205), and the hint bar and help name it (TUI-NAV-206). The new command has no row of its own, as TUI-NAV-039 now says. Twenty-three mutations across this and the sim segments were each caught; the first run exposed test loops a broken count could hang, which are now bounded.

Audited 2026-11-04, for what issue #46 changed. An action row the core prices as a fixed act states the price in the inspector, with need and have (TUI-NAV-207). A test reads every action row against what applying its command takes, so a row that spends and says nothing fails. On screen there were eleven such rows, two more than the code scan the issue started from. Twelve mutations across this and the sim engine were each caught.

Audited 2026-11-11, for what issue #48 changed. A kitten's row states its next rank's price beneath the kitten's name and before its card (TUI-NAV-208), and the row keeps its one word. A refusal for short exp names it. Two older tests that read a kitten's inspector as the name alone now set the price aside, and the row-wide test that every row which spends says so now includes kittens.

Audited 2026-11-11, for what issue #49 changed. A reveal's line names the item and the tab that lists it (TUI-NAV-209). A row a tick reveals in play is marked until the cursor rests on it at a frame's end (TUI-NAV-211, TUI-NAV-212), and a reveal out of focus badges its section's tab (TUI-NAV-213). A test over every structure and idea holds that each revealed one is listed in its section (TUI-NAV-210). Tests drive real ticks through the loop, and one shows a catch-up's reveals mark nothing. The audit removed a per-section watermark that every visit wrote and nothing read; its comment described a chip count the design had dropped. Two follow-ups are deferred in the design and filed: reveals during an absence or a nap (#51), and items a gate brings into a known section (#52). Every spec marked implemented has a test citing it, and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-11, for what issue #50 changed. A click and a wheel step move only the cursor, and the list follows it by the render leaf's rule (TUI-NAV-129, TUI-NAV-130). The click no longer writes a line of its own, and a test now holds that a click changes no list's line. The design's wording on where a list stands after a click, and its Decisions row, follow the render leaf. Every spec marked implemented has a test citing it, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
