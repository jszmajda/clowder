# Arrow: economy

The economy is the material world of the game: what exists, what it costs, what produces what. One leaf, five facets — `RES` (resources), `BLD` (structures), `CRAFT` (crafting), `NRG` (energy), `MESS`

## Status

**OK** — last audited 2026-11-01 (commit `7ad9e6ec6caa`). 60 of 60 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/economy/economy-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/economy/economy-specs.md` — 60 specs, prefix `SIM-ECO`

### Tests
- `crates/clowder-core/tests/integration/economy.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/queries_bench.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`
- `crates/clowder-core/tests/integration/wonder.rs`
- `crates/clowder-tui/tests/integration/bench.rs`

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
| All | SIM-ECO-001 – SIM-ECO-060 | 60 | 0 | 0 |

**Summary:** 60 of 60 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. The structure table was read row by row against the definitions: one effect it listed had never reached the rules, and now does (SIM-ECO-053). Rule-held structure behaviour now lives in definition fields, and no rule branches on a structure's id to find a rate. SIM-ECO-024 and SIM-ECO-047 are properties of the content tables as a whole, so no code decides them and their tests are their pins.

## Work Required

None.
