# Arrow: prestige

Prestige in clowder is **The Great Meadow Move**: when a clowder has grown as large as its meadow can hold, it relocates to a bigger, brighter meadow — and everything it learned comes along. This is t

## Status

**OK** — last audited 2026-11-03 (commit `2cddde1e39c7`). 16 of 16 active specs implemented, each cited by a test.

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
- `crates/clowder-core/tests/integration/blurb_truth.rs`
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

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-MOVE-014 is a property of the content tables as a whole, so no code decides it, and its tests are its pins. Issue #13 held every tradition's figures to its rule (SIM-MOVE-014).

Audited 2026-11-03, for what issue #32 changed. The code that sums this segment's effect contributions moved behind the per-key source lists (SIM-ENG-084), annotations with it; its specs did not change. Each contribution is held by a bit-for-bit comparison against a walk of every source, and a real late save replayed by the previous build and this one ended in the same bytes. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
