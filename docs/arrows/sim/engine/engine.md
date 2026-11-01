# Arrow: engine

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RN

## Status

**OK** — last audited 2026-11-01 (commit `7ad9e6ec6caa`). 77 of 77 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/engine/engine-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/engine/engine-specs.md` — 77 specs, prefix `SIM-ENG`

### Tests
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/engine.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/journal.rs`
- `crates/clowder-core/tests/integration/legacy.rs`
- `crates/clowder-core/tests/integration/persistence.rs`
- `crates/clowder-core/tests/integration/queries_bench.rs`
- `crates/clowder-core/tests/integration/queries_flows.rs`
- `crates/clowder-core/tests/integration/queries_gating.rs`
- `crates/clowder-core/tests/integration/queries_rates.rs`
- `crates/clowder-core/tests/integration/queries_timers.rs`
- `crates/clowder-core/tests/integration/revealed_resources.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/sources.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`
- `crates/clowder-core/tests/integration/unit_price.rs`
- `crates/clowder-core/tests/integration/wonder.rs`
- `crates/clowder-tui/tests/integration/update.rs`

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
| All | SIM-ENG-001 – SIM-ENG-078 | 77 | 0 | 0 |

**Summary:** 77 of 77 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-ENG-076 is a property of the content tables as a whole, so no code decides it, and its tests are its pins. SIM-ENG-018's order is correct by reading but pinned by no running test past the early rolls (#14).

## Work Required

- #14: a running test of the draw order.
