# Arrow: prestige

Prestige in clowder is **The Great Meadow Move**: when a clowder has grown as large as its meadow can hold, it relocates to a bigger, brighter meadow — and everything it learned comes along. This is t

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 13 of 13 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/prestige/prestige-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/prestige/prestige-specs.md` — 13 specs, prefix `SIM-MOVE`

### Tests
- `crates/clowder-core/tests/frontier.rs`
- `crates/clowder-core/tests/legacy.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-MOVE-001 – SIM-MOVE-013 | 13 | 0 | 0 |

**Summary:** 13 of 13 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
