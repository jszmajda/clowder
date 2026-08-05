# Arrow: render

This leaf owns everything between a `GameState` snapshot and the cells on screen: how the frame divides into regions, what each region draws, how the layout responds to size, how numbers and names are

## Status

**PARTIAL** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 14 of 15 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/render/render-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/render/render-specs.md` — 15 specs, prefix `TUI-RENDER`

### Tests
- `crates/clowder-tui/tests/composed.rs`
- `crates/clowder-tui/tests/fmt.rs`
- `crates/clowder-tui/tests/frame.rs`
- `crates/clowder-tui/tests/layout.rs`

### Code
- `crates/clowder-tui/src/render/fmt.rs`
- `crates/clowder-tui/src/render/layout.rs`
- `crates/clowder-tui/src/render/mod.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-RENDER-001 – TUI-RENDER-015 | 14 | 0 | 1 |

**Summary:** 14 of 15 active specs implemented; gaps below.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

### Must Fix
1. `TUI-RENDER-012` not implemented
