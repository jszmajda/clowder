# Arrow: navigation

This leaf owns how a human drives clowder from a keyboard: the key model, section navigation, the action model (select / act / repeat / bulk / confirm), the inspector, the glanceable chrome, discovera

## Status

**OK** — last audited 2026-10-29 (git SHA `8443eda1698c`). 180 of 180 active specs implemented, each cited by a test and annotated at its entry point.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/navigation/navigation-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/navigation/navigation-specs.md` — 180 specs, prefix `TUI-NAV`

### Tests
- `crates/clowder-tui/tests/achievements.rs`
- `crates/clowder-tui/tests/act_results.rs`
- `crates/clowder-tui/tests/adventures.rs`
- `crates/clowder-tui/tests/armed_summary.rs`
- `crates/clowder-tui/tests/bench.rs`
- `crates/clowder-tui/tests/chrome.rs`
- `crates/clowder-tui/tests/composed.rs`
- `crates/clowder-tui/tests/count_prompt.rs`
- `crates/clowder-tui/tests/frame.rs`
- `crates/clowder-tui/tests/inspector.rs`
- `crates/clowder-tui/tests/inspector_reasons.rs`
- `crates/clowder-tui/tests/keys.rs`
- `crates/clowder-tui/tests/kitten_card.rs`
- `crates/clowder-tui/tests/loop_body.rs`
- `crates/clowder-tui/tests/neighbour_offers.rs`
- `crates/clowder-tui/tests/overlays.rs`
- `crates/clowder-tui/tests/palette.rs`
- `crates/clowder-tui/tests/pen_pals.rs`
- `crates/clowder-tui/tests/pointer.rs`
- `crates/clowder-tui/tests/readouts.rs`
- `crates/clowder-tui/tests/report_queue.rs`
- `crates/clowder-tui/tests/resize.rs`
- `crates/clowder-tui/tests/rows.rs`
- `crates/clowder-tui/tests/section_reveal.rs`
- `crates/clowder-tui/tests/sidebar_scroll.rs`
- `crates/clowder-tui/tests/sources_hint.rs`
- `crates/clowder-tui/tests/ticker_lights.rs`
- `crates/clowder-tui/tests/transients.rs`
- `crates/clowder-tui/tests/update.rs`
- `crates/clowder-tui/tests/waking.rs`

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
| All | TUI-NAV-001 – TUI-NAV-180 | 180 | 0 | 0 |

**Summary:** 180 of 180 active specs implemented.

## Key Findings

Audited 2026-10-29 against the source: every spec marked implemented has a test citing it, no `@spec` in the code or tests names a spec that does not exist, and no spec ID is defined twice.

## Work Required

None.
