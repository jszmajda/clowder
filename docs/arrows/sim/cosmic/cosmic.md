# Arrow: cosmic

The cosmic leaf is clowder's late-game frontier: once the clowder has mastered its meadow, the boldest kittens begin gazing past the fence — to the Fence Top, the Old Oak, the rooftops, the hills, and

## Status

**OK** — last audited 2026-11-02 (commit `781d15ccba65`). 11 of 11 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/cosmic/cosmic-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/cosmic/cosmic-specs.md` — 11 specs, prefix `SIM-SKY`

### Tests
- `crates/clowder-core/tests/integration/frontier.rs`
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
| All | SIM-SKY-001 – SIM-SKY-011 | 11 | 0 | 0 |

**Summary:** 11 of 11 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read by hand against its code and tests, and the design's site table read row by row against the definitions and pinned by tests over all thirteen sites. It found a bonus the design states citing an unrelated spec (now SIM-SKY-009, the issue that opened this audit), one outpost left out of the outdoor channel (fixed, SIM-SKY-004), a forecast in the table no rule gives (removed), a storage figure missing from a row (added), and clauses no test exercised (tested, SIM-SKY-003, SIM-SKY-007). Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Issue #25 built the one designed bonus the first audit found missing (SIM-SKY-011), with a test over every store's cap, the cap with a multiplier of its own, and how it stacks. The audit cited SIM-SKY-010 on the table that holds it, which had no annotation in code. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
