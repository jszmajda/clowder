# Arrow: navigation

This leaf owns how a human drives clowder from a keyboard: the key model, section navigation, the action model (select / act / repeat / bulk / confirm), the inspector, the glanceable chrome, discovera

## Status

**OK** — last audited 2026-11-03 (commit `e1a0720b63e5`). 203 of 203 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/navigation/navigation-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/navigation/navigation-specs.md` — 203 specs, prefix `TUI-NAV`

### Tests
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
| All | TUI-NAV-001 – TUI-NAV-203 | 203 | 0 | 0 |

**Summary:** 203 of 203 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Issue #2 added what a store's card opens with (TUI-NAV-197). The audit found the persistent inspector repeating an arming's summary (fixed, TUI-NAV-077) and a placement helper no frame calls (annotation removed, the rest filed). Issue #3 added the sidebar's store-link marks (TUI-NAV-198, TUI-NAV-199, TUI-NAV-200), with marks read off drawn frames over every row of every section. Issue #20 found flavour events reaching the screen as a single placeholder word, because no line ever read their template; they now read as written, for every template (TUI-NAV-201). Issue #10 moved the held-key and arming tests onto a clock they control, so they take no real time and no longer fail under load. `nav::rows` serves a frame's row lists through one scoped build, held by a guard tied to the state's borrow (#18); outside a frame, every call still builds afresh. The inspector's place is decided by `render::layout::compute` and the frame's narrow body alone, and the tests read it off whole frames, through the dead-band in both directions (#22). The header's kitten fact states the clowder's room and free slots from the core's own queries (TUI-NAV-202, #23), and the design's list of chips and width budget match the code again.

Audited 2026-11-02, for what issue #25 changed. It added a sentence for a new structure field (TUI-NAV-203), held word for word from the field and by the test that a statement is every field its definition sets. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #27 changed. An in-process gap past the absence cap became a third route to a retained report (TUI-NAV-027), whose banner says more than the cap (TUI-NAV-055), read off a drawn frame. Which reports get a banner stays judged from the journal alone. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #30 changed. No rule of this segment changed: its rows read the core's gates, so what they list moved with them, and one inspector fixture named the new idea. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #33 changed. The waking screen and the report's banner now also serve a catch-up begun in play (TUI-NAV-027, TUI-NAV-102, TUI-NAV-103), and the capped gap's own queueing went with its report. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #34 changed. A shy colony's pen-pal row states the courtesy gift's chance beside the nap's, read from the core's constant (TUI-NAV-172). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #37 changed. What the core answers as held before an absence now survives a quit mid-wake, so the chip and the `new` tag still tell what the absence earned, tested through the shell (TUI-NAV-174). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
