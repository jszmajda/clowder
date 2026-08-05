# Arrow: expeditions

Pouncing expeditions are the active-play burst: the player spends stored pep on squads that scamper beyond the meadow and come back with luxuries, glitter, and — rarely — a unicorn who decides to foll

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 6 of 6 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/expeditions/expeditions-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/expeditions/expeditions-specs.md` — 6 specs, prefix `SIM-EXP`

### Tests
- `crates/clowder-core/tests/society.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/rng.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-EXP-001 – SIM-EXP-006 | 6 | 0 | 0 |

**Summary:** 6 of 6 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
