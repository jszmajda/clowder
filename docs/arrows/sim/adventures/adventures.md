# Arrow: adventures

Adventures are clowder's challenge modes, reimagined as *optional themed outings* the clowder chooses to try — the inspiration's self-imposed challenge runs, kept entirely opt-in and kept cozy. An Adv

## Status

**OK** — last audited 2026-10-29 (git SHA `8443eda1698c`). 8 of 8 active specs implemented, each cited by a test and annotated at its entry point.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/adventures/adventures-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/adventures/adventures-specs.md` — 8 specs, prefix `SIM-ADV`

### Tests
- `crates/clowder-core/tests/blurbs.rs`
- `crates/clowder-core/tests/economy_depth.rs`
- `crates/clowder-core/tests/legacy.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ADV-001 – SIM-ADV-008 | 8 | 0 | 0 |

**Summary:** 8 of 8 active specs implemented.

## Key Findings

Audited 2026-10-29 against the source: every spec marked implemented has a test citing it, no `@spec` in the code or tests names a spec that does not exist, and no spec ID is defined twice.

## Work Required

None.
