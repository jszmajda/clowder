# Arrow: engine

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RN

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 45 of 45 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/engine/engine-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/engine/engine-specs.md` — 45 specs, prefix `SIM-ENG`

### Tests
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/engine.rs`
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/journal.rs`
- `crates/clowder-core/tests/legacy.rs`
- `crates/clowder-core/tests/persistence.rs`
- `crates/clowder-core/tests/queries_gating.rs`
- `crates/clowder-core/tests/queries_rates.rs`
- `crates/clowder-core/tests/society.rs`
- `crates/clowder-core/tests/wonder.rs`
- `crates/clowder-tui/tests/update.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/journal.rs`
- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/rng.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`
- `crates/clowder-tui/src/nav/rows.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ENG-001 – SIM-ENG-046 | 45 | 0 | 0 |

**Summary:** 45 of 45 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
