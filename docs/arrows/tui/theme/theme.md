# Arrow: theme

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the ga

## Status

**PARTIAL** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 12 of 15 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/theme/theme-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/theme/theme-specs.md` — 15 specs, prefix `TUI-THEME`

### Tests
- `crates/clowder-tui/tests/theme.rs`

### Code
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/theme/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-THEME-001 – TUI-THEME-015 | 12 | 0 | 3 |

**Summary:** 12 of 15 active specs implemented; gaps below.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

### Must Fix
1. `TUI-THEME-008` not implemented
2. `TUI-THEME-011` not implemented
3. `TUI-THEME-015` not implemented
