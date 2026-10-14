# Arrow: theme

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the coats the stores wear, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the game fully playable by someone who cannot distinguish any of it.

## Status

**IMPLEMENTED** per the spec file's own markers — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 21 of 21 active specs marked implemented; the coats (TUI-THEME-016 – 021) landed 2026-10-14 with tests across the theme, chrome, frame, rows, inspector, overlays, and armed-summary test files.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/theme/theme-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/theme/theme-specs.md` — 21 specs, prefix `TUI-THEME`

### Tests
- `crates/clowder-tui/tests/theme.rs`
- `crates/clowder-tui/tests/chrome.rs`
- `crates/clowder-tui/tests/frame.rs`
- `crates/clowder-tui/tests/rows.rs`
- `crates/clowder-tui/tests/inspector.rs`
- `crates/clowder-tui/tests/overlays.rs`
- `crates/clowder-tui/tests/armed_summary.rs`

### Code
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/render/widgets/chrome.rs`
- `crates/clowder-tui/src/render/widgets/inspector.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/theme/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-THEME-001 – TUI-THEME-021 | 21 | 0 | 0 |

**Summary:** 21 of 21 active specs marked implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None open from the markers; an audit pass against the code has still never run.
