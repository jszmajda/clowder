# Arrow: theme

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the coats the stores wear, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the game fully playable by someone who cannot distinguish any of it.

## Status

**PARTIAL** — last audited 2026-11-01 (commit `c5a6aacb2ccc`). 30 of 30 active specs implemented, each cited by a test; 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/theme/theme-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/theme/theme-specs.md` — 31 specs, prefix `TUI-THEME`

### Tests
- `crates/clowder-tui/tests/integration/achievements.rs`
- `crates/clowder-tui/tests/integration/act_results.rs`
- `crates/clowder-tui/tests/integration/armed_summary.rs`
- `crates/clowder-tui/tests/integration/chrome.rs`
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
| All | TUI-THEME-001 – TUI-THEME-031 | 30 | 1 | 0 |

**Summary:** 30 of 30 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-01. First full audit, by the same method as render's. Every colour value in the design matches the code, and the band and clearance rules were recomputed and hold. It found the header's season word disagreeing with the accent while a season is held (fixed, TUI-THEME-008), a readout's sentence naming a store without its coat (fixed, TUI-THEME-022), lists of what wears `attention` that left items out (corrected), and specs tested only against local tables, four of them now read off whole drawn frames (TUI-THEME-001, TUI-THEME-004, TUI-THEME-005, TUI-THEME-006). Issue #2 added a store's drawing (TUI-THEME-031). A reader's pass is no proof: specs 001, 004, 009, 010, 012 to 014, 016, and 024 to 026 were read more lightly than the rest.

## Work Required

- #19: a focused row's reversal (TUI-THEME-007, deferred).
- #20: three colours the design states that the frame does not draw.
- #21: tests that assert less than their spec says (TUI-THEME-003, TUI-THEME-010, TUI-THEME-011, TUI-THEME-012, TUI-THEME-015, TUI-THEME-019, TUI-THEME-022).
