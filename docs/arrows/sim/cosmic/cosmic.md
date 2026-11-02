# Arrow: cosmic

The cosmic leaf is clowder's late-game frontier: once the clowder has mastered its meadow, the boldest kittens begin gazing past the fence — to the Fence Top, the Old Oak, the rooftops, the hills, and

## Status

**PARTIAL** — last audited 2026-11-01 (commit `39585f101764`). 10 of 10 active specs implemented, each cited by a test; 1 deferred.

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
| All | SIM-SKY-001 – SIM-SKY-011 | 10 | 1 | 0 |

**Summary:** 10 of 10 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-01. First full audit: every spec read by hand against its code and tests, and the design's site table read row by row against the definitions and pinned by tests over all thirteen sites. It found a bonus the design states citing an unrelated spec (now SIM-SKY-009, the issue that opened this audit), one outpost left out of the outdoor channel (fixed, SIM-SKY-004), a forecast in the table no rule gives (removed), a storage figure missing from a row (added), and clauses no test exercised (tested, SIM-SKY-003, SIM-SKY-007). One designed bonus was never built (SIM-SKY-011, deferred). Hand-read by the agent that made these changes, with no second reader.

## Work Required

- #25: a late outpost's storage bonus was never built (SIM-SKY-011, deferred).
