# Arrow: prestige

Prestige in clowder is **The Great Meadow Move**: when a clowder has grown as large as its meadow can hold, it relocates to a bigger, brighter meadow — and everything it learned comes along. This is t

## Status

**OK** — last audited 2026-10-29 (git SHA `8443eda1698c`). 15 of 15 active specs implemented, each cited by a test and, save the table property noted below, annotated at its entry point.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/prestige/prestige-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/prestige/prestige-specs.md` — 15 specs, prefix `SIM-MOVE`

### Tests
- `crates/clowder-core/tests/achievements.rs`
- `crates/clowder-core/tests/blurbs.rs`
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/legacy.rs`
- `crates/clowder-core/tests/profiles.rs`
- `crates/clowder-core/tests/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-MOVE-001 – SIM-MOVE-015 | 15 | 0 | 0 |

**Summary:** 15 of 15 active specs implemented.

## Key Findings

Audited 2026-10-29 against the source: every spec marked implemented has a test citing it, no `@spec` in the code or tests names a spec that does not exist, and no spec ID is defined twice. SIM-MOVE-014 is a property of the content tables as a whole, so no code decides it, and its tests are its pins.

## Work Required

None.
