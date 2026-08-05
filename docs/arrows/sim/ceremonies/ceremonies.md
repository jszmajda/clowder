# Arrow: ceremonies

Ceremonies are clowder's faith system, reimagined as warmth and devotion rather than worship and sacrifice. Kittens gather in sunpuddles for Sunbeam Ceremonies; their quiet devotion accrues, is offere

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 8 of 8 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/ceremonies/ceremonies-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/ceremonies/ceremonies-specs.md` — 8 specs, prefix `SIM-SUN`

### Tests
- `crates/clowder-core/tests/legacy.rs`
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
| All | SIM-SUN-001 – SIM-SUN-008 | 8 | 0 | 0 |

**Summary:** 8 of 8 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
