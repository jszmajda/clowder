# Arrow: engine

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RN

## Status

**OK** — last audited 2026-10-29 (git SHA `8443eda1698c`). 75 of 75 active specs implemented, each cited by a test and, save the table property noted below, annotated at its entry point.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/engine/engine-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/engine/engine-specs.md` — 75 specs, prefix `SIM-ENG`

### Tests
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/engine.rs`
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/journal.rs`
- `crates/clowder-core/tests/legacy.rs`
- `crates/clowder-core/tests/persistence.rs`
- `crates/clowder-core/tests/queries_bench.rs`
- `crates/clowder-core/tests/queries_flows.rs`
- `crates/clowder-core/tests/queries_gating.rs`
- `crates/clowder-core/tests/queries_rates.rs`
- `crates/clowder-core/tests/queries_timers.rs`
- `crates/clowder-core/tests/revealed_resources.rs`
- `crates/clowder-core/tests/society.rs`
- `crates/clowder-core/tests/sources.rs`
- `crates/clowder-core/tests/unit_price.rs`
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
| All | SIM-ENG-001 – SIM-ENG-076 | 75 | 0 | 0 |

**Summary:** 75 of 75 active specs implemented.

## Key Findings

Audited 2026-10-29 against the source: every spec marked implemented has a test citing it, no `@spec` in the code or tests names a spec that does not exist, and no spec ID is defined twice. SIM-ENG-076 is a property of the content tables as a whole, so no code decides it, and its tests are its pins.

## Work Required

None.
