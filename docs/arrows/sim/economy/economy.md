# Arrow: economy

The economy is the material world of the game: what exists, what it costs, what produces what. One leaf, five facets — `RES` (resources), `BLD` (structures), `CRAFT` (crafting), `NRG` (energy), `MESS`

## Status

**OK** — last audited 2026-10-29 (git SHA `8443eda1698c`). 52 of 52 active specs implemented, each cited by a test and, save the table properties noted below, annotated at its entry point.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/economy/economy-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/economy/economy-specs.md` — 52 specs, prefix `SIM-ECO`

### Tests
- `crates/clowder-core/tests/economy.rs`
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/queries_bench.rs`
- `crates/clowder-core/tests/society.rs`
- `crates/clowder-core/tests/wonder.rs`
- `crates/clowder-tui/tests/bench.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ECO-001 – SIM-ECO-052 | 52 | 0 | 0 |

**Summary:** 52 of 52 active specs implemented.

## Key Findings

Audited 2026-10-29 against the source: every spec marked implemented has a test citing it, no `@spec` in the code or tests names a spec that does not exist, and no spec ID is defined twice. SIM-ECO-024 and SIM-ECO-047 are properties of the content tables as a whole, so no code decides them, and their tests are their pins.

## Work Required

None.
