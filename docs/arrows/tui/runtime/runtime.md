# Arrow: runtime

This leaf owns the boundary between the operating system and the pure core: terminal lifecycle, the event loop, the conversion of real time into ticks, autosave cadence, save-path resolution, and the 

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 31 of 31 active specs implemented, 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/runtime/runtime-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/runtime/runtime-specs.md` — 32 specs, prefix `TUI-RUN`

### Tests
- `crates/clowder-tui/tests/clock.rs`
- `crates/clowder-tui/tests/lock.rs`
- `crates/clowder-tui/tests/loop_body.rs`
- `crates/clowder-tui/tests/loop_order.rs`
- `crates/clowder-tui/tests/persist.rs`
- `crates/clowder-tui/tests/term.rs`

### Code
- `crates/clowder-tui/src/runtime/clock.rs`
- `crates/clowder-tui/src/runtime/lock.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/runtime/persist.rs`
- `crates/clowder-tui/src/runtime/term.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-RUN-001 – TUI-RUN-032 | 31 | 1 | 0 |

**Summary:** 31 of 31 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
