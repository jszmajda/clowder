# Arrow: kittens

Kittens are the heart of the game and the reason the cozy invariants exist. This leaf owns the population lifecycle (arrival, wandering off, returning), kitten identity (names), job assignment mechani

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 30 of 30 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/kittens/kittens-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/kittens/kittens-specs.md` — 30 specs, prefix `SIM-KIT`

### Tests
- `crates/clowder-core/tests/kittens.rs`
- `crates/clowder-core/tests/legacy.rs`
- `crates/clowder-core/tests/society.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-KIT-001 – SIM-KIT-030 | 30 | 0 | 0 |

**Summary:** 30 of 30 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
