# Arrow: achievements

Achievements are a scrapbook of what a clowder has done: a table of them, noticing when one is earned, and a record of what was earned and when, read by no rule of the simulation.

## Status

**PARTIAL** — last audited 2026-11-03 (commit `2cddde1e39c7`). 6 of 6 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/achievements/achievements-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/achievements/achievements-specs.md` — 6 specs, prefix `SIM-ACH`

### Tests
- `crates/clowder-core/tests/integration/achievements.rs`

### Code
- `crates/clowder-core/src/achievements.rs`
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/state.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ACH-001 – SIM-ACH-006 | 6 | 0 | 0 |

**Summary:** 6 of 6 active specs implemented.

## Key Findings

Audited 2026-11-03. First full audit, made for issue #32, which touched the design's pointer to the catch-up bounds and found this leaf with no page in the overlay: every spec read against its code and tests by a reader given only the arrow artifacts, its findings then checked by hand. Table names, order, and secret flags match the design by script, and no rule reads the record outside the catch-up's as-loaded answer (SIM-ACH-003). The docs now say what the code does where they disagreed: which kitten readings wait for an arrival or a new year (SIM-ACH-002), table row 16 naming the roster it reads, and an achievement earned during a replay being told by the record rather than the journal. SIM-ACH-001's hints were read more lightly.

## Work Required

- #42: most of the table's conditions have no test (SIM-ACH-006), nor does noticing a kitten-reading condition at a day close (SIM-ACH-002).
