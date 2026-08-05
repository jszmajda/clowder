# Arrow: mystic

The mystic leaf is the unicorn economy and everything glittering that follows from it — the inspiration's unicorn/ziggurat/alicorn/tears chain, reimagined so nothing is ever sacrificed. Unicorns hold 

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 11 of 11 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/mystic/mystic-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/mystic/mystic-specs.md` — 11 specs, prefix `SIM-UNI`

### Tests
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
| All | SIM-UNI-001 – SIM-UNI-011 | 11 | 0 | 0 |

**Summary:** 11 of 11 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
