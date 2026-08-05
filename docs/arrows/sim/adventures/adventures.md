# Arrow: adventures

Adventures are clowder's challenge modes, reimagined as *optional themed outings* the clowder chooses to try — the inspiration's self-imposed challenge runs, kept entirely opt-in and kept cozy. An Adv

## Status

**OK** — never audited (bootstrapped 2026-08-10 from existing LID docs; `audited_sha` null). 5 of 5 active specs implemented.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/adventures/adventures-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/adventures/adventures-specs.md` — 5 specs, prefix `SIM-ADV`

### Tests
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
| All | SIM-ADV-001 – SIM-ADV-005 | 5 | 0 | 0 |

**Summary:** 5 of 5 active specs implemented.

## Key Findings

Bootstrapped from existing LID docs without an audit pass. Coverage above is
read from the spec files' own markers, which record what the project believes;
they have not been verified against the code by this overlay.

## Work Required

None recorded — every active spec is marked implemented.
