# Arrow: kittens

Kittens are the heart of the game and the reason the cozy invariants exist. This leaf owns the population lifecycle (arrival, wandering off, returning), kitten identity (names), job assignment mechani

## Status

**PARTIAL** — last audited 2026-11-03 (commit `2cddde1e39c7`). 42 of 42 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/kittens/kittens-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/kittens/kittens-specs.md` — 42 specs, prefix `SIM-KIT`

### Tests
- `crates/clowder-core/tests/integration/kittens.rs`
- `crates/clowder-core/tests/integration/legacy.rs`
- `crates/clowder-core/tests/integration/profiles.rs`
- `crates/clowder-core/tests/integration/society.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/profile.rs`
- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-KIT-001 – SIM-KIT-042 | 42 | 0 | 0 |

**Summary:** 42 of 42 active specs implemented.

## Key Findings

Audited 2026-11-03. First full audit, made for issue #32, which changed the job production pass and happiness: every spec read against its code and tests by a reader given only the arrow artifacts, its findings then checked by hand. Both changes matched their specs, but nothing measured a job's yield with a worker away, workers of different skill, the Elder beside another worker, or a ratio on one job beside another, nor happiness with a kitten away. Each is now tested and each test checked by breaking the line it guards, which is how the ratio case was found. The facts query's reading of the master-skill effect is now stated and tested (SIM-KIT-041). The docs now say what the code does where they disagreed: a Move as the one way off the roster (SIM-KIT-001, SIM-KIT-022, SIM-KIT-027), what a name is unique among (SIM-KIT-002), the arrival chance's last term (SIM-KIT-004), which traits' consumers exist (SIM-KIT-024), a jamboree's end event (SIM-KIT-025), the version 8 redraw (SIM-KIT-034), the tier above which a Tinkerer slows, the happiness formula's legacy term, the struct, and two finished items still deferred. SIM-KIT-028, SIM-KIT-029, SIM-KIT-035 to SIM-KIT-038, SIM-KIT-042's table content, SIM-KIT-026's and SIM-KIT-040's tests, profile migration, and the shell's tests were read more lightly.

## Work Required

- #40: one timed effect runs one tick short (SIM-KIT-025).
- #41: thin tests and shape annotations; SIM-KIT-001 and SIM-KIT-033 cite nothing in source.
