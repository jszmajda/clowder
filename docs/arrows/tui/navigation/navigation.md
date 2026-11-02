# Arrow: navigation

This leaf owns how a human drives clowder from a keyboard: the key model, section navigation, the action model (select / act / repeat / bulk / confirm), the inspector, the glanceable chrome, discovera

## Status

**PARTIAL** — last audited 2026-11-01 (commit `f36f4f91aa8e`). 201 of 201 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/navigation/navigation-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/navigation/navigation-specs.md` — 201 specs, prefix `TUI-NAV`

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
| All | TUI-NAV-001 – TUI-NAV-201 | 201 | 0 | 0 |

**Summary:** 201 of 201 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Issue #2 added what a store's card opens with (TUI-NAV-197). The audit found the persistent inspector repeating an arming's summary (fixed, TUI-NAV-077) and a placement helper no frame calls (annotation removed, the rest filed). Issue #3 added the sidebar's store-link marks (TUI-NAV-198, TUI-NAV-199, TUI-NAV-200), with marks read off drawn frames over every row of every section. Issue #20 found flavour events reaching the screen as a single placeholder word, because no line ever read their template; they now read as written, for every template (TUI-NAV-201). Issue #10 moved the held-key and arming tests onto a clock they control, so they take no real time and no longer fail under load.

## Work Required

- #22: the inspector's placement helper is called by no frame, and some tests still exercise it (TUI-NAV-017, TUI-NAV-029).
