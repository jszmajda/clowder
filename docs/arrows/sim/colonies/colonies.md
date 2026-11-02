# Arrow: colonies

Colonies are clowder's trade and diplomacy: eight neighbouring animal colonies the clowder sends caravans to, exchanging surplus for goods it can't make yet. This is the inspiration's race/trade/embas

## Status

**PARTIAL** — last audited 2026-11-02 (commit `376d3e12c927`). 13 of 13 active specs implemented, each cited by a test; 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/colonies/colonies-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/colonies/colonies-specs.md` — 14 specs, prefix `SIM-TRADE`

### Tests
- `crates/clowder-core/tests/integration/idea_unlocks.rs`
- `crates/clowder-core/tests/integration/neighbour_offers.rs`
- `crates/clowder-core/tests/integration/sources.rs`
- `crates/clowder-core/tests/integration/trade_returns.rs`
- `crates/clowder-core/tests/integration/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/state.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-TRADE-001 – SIM-TRADE-014 | 13 | 1 | 0 |

**Summary:** 13 of 13 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-02. First full audit, made for issue #30: every spec read against its code, its tests, and the design's table by a reader given only the arrow artifacts, its findings then checked by hand. The table matches on every number. It found a picnic trying the first tier ahead of colonies whose prerequisites were met, against the spec and design (fixed, with positive cases for all three and the find rate; SIM-TRADE-007), a pen pal row listed before its command would act (fixed, SIM-TRADE-006, SIM-ENG-045), and a nap's courtesy gift the spec marked implemented and no rule gives (split out, SIM-TRADE-014, deferred). It also pinned what the docs left unstated: the fuzz, the pen pal's base price, the global standing ratio's formula (now tested for shy colonies too), and what an unreached colony reports, and it removed a contradiction between never undiscovering and a Move forgetting. A trade's report states what came home though a full store may trim it on the next clamp; the design says the amounts are the stock moves the trade made, so that stands. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-TRADE-011, the save half of SIM-TRADE-008, and the whale half of SIM-TRADE-009 were read more lightly.

## Work Required

- #34: a napping colony's courtesy gift was never built (SIM-TRADE-014, deferred).
- #35: thin table and multiplier tests.
