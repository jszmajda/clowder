# Arrow: prestige

Prestige in clowder is **The Great Meadow Move**: when a clowder has grown as large as its meadow can hold, it relocates to a bigger, brighter meadow — and everything it learned comes along. This is t

## Status

**OK** — last audited 2026-11-01 (commit `7ad9e6ec6caa`). 16 of 16 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/prestige/prestige-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/prestige/prestige-specs.md` — 16 specs, prefix `SIM-MOVE`

### Tests
- `crates/clowder-core/tests/integration/achievements.rs`
- `crates/clowder-core/tests/integration/blurbs.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/legacy.rs`
- `crates/clowder-core/tests/integration/profiles.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`
- `crates/clowder-core/tests/integration/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-MOVE-001 – SIM-MOVE-016 | 16 | 0 | 0 |

**Summary:** 16 of 16 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-MOVE-014 is a property of the content tables as a whole, so no code decides it, and its tests are its pins.

## Work Required

None.
