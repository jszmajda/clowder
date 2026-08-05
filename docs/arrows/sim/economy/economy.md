# Arrow: economy

The economy is the material world of the game: what exists, what it costs, what produces what. One leaf, five facets — `RES` (resources), `BLD` (structures), `CRAFT` (crafting), `NRG` (energy), `MESS`

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 44 of 44 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/economy/economy-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/economy/economy-specs.md` — 44 specs, prefix `SIM-ECO`

### Tests
- `crates/clowder-core/tests/economy.rs`
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/society.rs`
- `crates/clowder-core/tests/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ECO-001 – SIM-ECO-044 | 44 | 0 | 0 |

**Summary:** 44 of 44 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
