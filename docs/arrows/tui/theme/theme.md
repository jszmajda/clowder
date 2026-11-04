# Arrow: theme

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the coats the stores wear, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the game fully playable by someone who cannot distinguish any of it.

## Status

**OK** — last audited 2026-11-04 (commit `8ea2ab82bb22`). 34 of 34 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/theme/theme-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/theme/theme-specs.md` — 34 specs, prefix `TUI-THEME`

### Tests
- `crates/clowder-tui/tests/integration/achievements.rs`
- `crates/clowder-tui/tests/integration/act_results.rs`
- `crates/clowder-tui/tests/integration/armed_summary.rs`
- `crates/clowder-tui/tests/integration/chrome.rs`
- `crates/clowder-tui/tests/integration/count_prompt.rs`
- `crates/clowder-tui/tests/integration/frame.rs`
- `crates/clowder-tui/tests/integration/inspector.rs`
- `crates/clowder-tui/tests/integration/kitten_card.rs`
- `crates/clowder-tui/tests/integration/overlays.rs`
- `crates/clowder-tui/tests/integration/readouts.rs`
- `crates/clowder-tui/tests/integration/rows.rs`
- `crates/clowder-tui/tests/integration/sidebar_scroll.rs`
- `crates/clowder-tui/tests/integration/sources_hint.rs`
- `crates/clowder-tui/tests/integration/store_card.rs`
- `crates/clowder-tui/tests/integration/theme_frames.rs`
- `crates/clowder-tui/tests/integration/ticker_lights.rs`
- `crates/clowder-tui/tests/theme.rs`

### Code
- `crates/clowder-tui/src/nav/mod.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/render/portrait.rs`
- `crates/clowder-tui/src/render/store_art.rs`
- `crates/clowder-tui/src/render/widgets/chrome.rs`
- `crates/clowder-tui/src/render/widgets/inspector.rs`
- `crates/clowder-tui/src/render/widgets/overlays.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/theme/fur.rs`
- `crates/clowder-tui/src/theme/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-THEME-001 – TUI-THEME-034 | 34 | 0 | 0 |

**Summary:** 34 of 34 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit, by the same method as render's. Every colour value in the design matches the code, and the band and clearance rules were recomputed and hold. It found the header's season word disagreeing with the accent while a season is held (fixed, TUI-THEME-008), a readout's sentence naming a store without its coat (fixed, TUI-THEME-022), lists of what wears `attention` that left items out (corrected), and specs tested only against local tables, four of them now read off whole drawn frames (TUI-THEME-001, TUI-THEME-004, TUI-THEME-005, TUI-THEME-006). Issue #2 added a store's drawing (TUI-THEME-031). A reader's pass is no proof: specs 001, 004, 009, 010, 012 to 014, 016, and 024 to 026 were read more lightly than the rest. Issue #19 settled how a focused row takes the reversal: it keeps its state colour as a bar, and TUI-THEME-006 and TUI-THEME-007 now say so, read off drawn frames at each state. Issue #20 drew what the design promised: the star's colour on badges and event lines (TUI-THEME-032), flavour's dim in the Journal (TUI-THEME-033), and a coat on event words that name a store (TUI-THEME-024).

Audited 2026-11-04, for what issue #45 changed. A count prompt's count stands out of the bar in the terminal's own pair (TUI-THEME-034), the one run a reversed list row leaves out (TUI-THEME-007), so it needs no colour of its own, claims no state, and clears every bar in either polarity. It is tested cell by cell on every row a prompt opens on, and each test was checked by breaking what it guards; how it looks has not been judged by eye. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #21 changed. The tests that asserted less than their specs now read drawn frames. Colour by state rather than item is read off every content row of every section, and a store's coat across its stock, cap, and rate (TUI-THEME-003). A frame with its colour removed still says each state (TUI-THEME-010). The chip sweeps cover all six chips at every width (TUI-THEME-011, TUI-THEME-015). The notice is counted, so saying it twice fails (TUI-THEME-012). Coats are held on an open prompt's preview and a unit-price line (TUI-THEME-019), and on every kind of description that carries a blurb (TUI-THEME-022). Each was broken once in a scratch copy; eighteen mutations were caught, one only after the state-by-item fixture was widened to meadows with everything learned, and two more on later items confirmed that. The mood spec gained its annotation (TUI-THEME-005).

## Work Required

None.
