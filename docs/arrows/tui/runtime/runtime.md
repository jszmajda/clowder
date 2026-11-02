# Arrow: runtime

This leaf owns the boundary between the operating system and the pure core: terminal lifecycle, the event loop, the conversion of real time into ticks, autosave cadence, save-path resolution, and the 

## Status

**PARTIAL** — last audited 2026-11-02 (commit `fbf98a4799fb`). 50 of 50 active specs implemented, each cited by a test; 7 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/runtime/runtime-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/runtime/runtime-specs.md` — 57 specs, prefix `TUI-RUN`

### Tests
- `crates/clowder-tui/tests/integration/clock.rs`
- `crates/clowder-tui/tests/integration/held_clock.rs`
- `crates/clowder-tui/tests/integration/lock.rs`
- `crates/clowder-tui/tests/integration/loop_body.rs`
- `crates/clowder-tui/tests/integration/loop_order.rs`
- `crates/clowder-tui/tests/integration/persist.rs`
- `crates/clowder-tui/tests/integration/session.rs`
- `crates/clowder-tui/tests/integration/terminal_lost.rs`
- `crates/clowder-tui/tests/integration/waking.rs`
- `crates/clowder-tui/tests/term.rs`

### Code
- `crates/clowder-core/src/save.rs`
- `crates/clowder-tui/src/runtime/clock.rs`
- `crates/clowder-tui/src/runtime/lock.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/runtime/now.rs`
- `crates/clowder-tui/src/runtime/persist.rs`
- `crates/clowder-tui/src/runtime/term.rs`
- `crates/clowder-tui/src/runtime/wake.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-RUN-001 – TUI-RUN-068 | 50 | 7 | 0 |

**Summary:** 50 of 50 active specs implemented; 7 deferred.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code, its tests, and the design by a reader given only the arrow artifacts, its most serious findings then checked by hand. It found a lock that can be cleared under a live holder, time away going uncounted around a resume, a damaged save that can be overwritten, and a suspend tolerance in the design that the spec and code deny (corrected). Issue #10 routed every monotonic read through one holdable function (TUI-RUN-066, TUI-RUN-067). Specs 001 to 004, 010, 011, 013, 017, 060 to 063, and 065 were read more lightly than the rest. A frame of play waits one tick at normal speed (TUI-RUN-068, #24): on one save copy in a 120×40 pane with no keys pressed, about 5.3% of a core became about 1.6%, with keys answered as quickly as before. The design records why a redraw-on-change check was not taken.

## Work Required

- #26: a session's lock can be cleared while its holder runs (TUI-RUN-007, deferred).
- #27: time away uncounted or mispriced around a resume (TUI-RUN-022, TUI-RUN-024, TUI-RUN-036, deferred).
- #28: a damaged save can be overwritten; a fallback path unstated (TUI-RUN-005, TUI-RUN-012, deferred).
- #29: thin tests, stray annotations, and open questions.
