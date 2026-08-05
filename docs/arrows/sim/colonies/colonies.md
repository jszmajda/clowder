# Arrow: colonies

Colonies are clowder's trade and diplomacy: eight neighbouring animal colonies the clowder sends caravans to, exchanging surplus for goods it can't make yet. This is the inspiration's race/trade/embas

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 8 of 8 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/colonies/colonies-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/colonies/colonies-specs.md` — 8 specs, prefix `SIM-TRADE`

### Tests
- `crates/clowder-core/tests/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/state.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-TRADE-001 – SIM-TRADE-008 | 8 | 0 | 0 |

**Summary:** 8 of 8 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
