# Arrow: adventures

Adventures are clowder's challenge modes, reimagined as *optional themed outings* the clowder chooses to try — the inspiration's self-imposed challenge runs, kept entirely opt-in and kept cozy. An Adv

## Status

**OK** — last audited 2026-11-03 (commit `2cddde1e39c7`). 8 of 8 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/adventures/adventures-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/adventures/adventures-specs.md` — 8 specs, prefix `SIM-ADV`

### Tests
- `crates/clowder-core/tests/integration/blurbs.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/legacy.rs`

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

Audited 2026-11-03, for what issue #32 changed. The code that sums this segment's effect contributions moved behind the per-key source lists (SIM-ENG-084), annotations with it; its specs did not change. Each contribution is held by a bit-for-bit comparison against a walk of every source, and a real late save replayed by the previous build and this one ended in the same bytes. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
