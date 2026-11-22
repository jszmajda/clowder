# Arrow: mystic

The mystic leaf is the unicorn economy and everything glittering that follows from it — the inspiration's unicorn/ziggurat/alicorn/tears chain, reimagined so nothing is ever sacrificed. Unicorns hold 

## Status

**OK** — last audited 2026-11-21 (commit `625d7b8b8444`). 20 of 20 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/mystic/mystic-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/mystic/mystic-specs.md` — 20 specs, prefix `SIM-UNI`

### Tests
- `crates/clowder-core/tests/integration/blurb_truth.rs`
- `crates/clowder-core/tests/integration/blurbs.rs`
- `crates/clowder-core/tests/integration/draw_schedule.rs`
- `crates/clowder-core/tests/integration/idea_unlocks.rs`
- `crates/clowder-core/tests/integration/mystic_pacing.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`
- `crates/clowder-core/tests/integration/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/effects/index.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-UNI-001 – SIM-UNI-020 | 20 | 0 | 0 |

**Summary:** 20 of 20 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its rule, its tests, and the design by a reader given only the arrow artifacts, its findings then checked by hand. It found a spend with no room for the output (fixed, SIM-UNI-004), absolute claims and formulas contradicted by sibling specs (corrected), and clauses no test exercised (tested). What a Promise grants was narrowed in the design rather than built out, on Tenet 1 (Decisions & Alternatives). Issue #13 held every arch upgrade's figures to its rule (SIM-UNI-012), and the idea granting a free arch now says so (SIM-UNI-011). Issue #14 pinned the descent and shower rolls' place in the draw order (SIM-UNI-006). Issue #16 stated and pinned a shower's windfall and the Sky Arch's drift (SIM-UNI-005).

Audited 2026-11-02, for what issue #30 changed. The two arch upgrades that dream wait on Dream Tending (SIM-UNI-016), through one function the command and the availability query both read. An upgrade already owned goes on working, tested for the Memory Arch. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #32 changed. The code that sums this segment's effect contributions moved behind the per-key source lists (SIM-ENG-084), annotations with it; its specs did not change. Each contribution is held by a bit-for-bit comparison against a walk of every source, and a real late save replayed by the previous build and this one ended in the same bytes. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #15 changed. Three effect keys no content fed left the rules and their specs (SIM-UNI-001, SIM-UNI-003, SIM-UNI-007): the two the issue named, and the third with the same shape in this segment. Each read one in play, so no outcome moved. The starshard refine keeps its multiplier, which Arch upgrades feed.

Audited 2026-11-21, for what issue #53 changed. Parades now bring prisms in proportion to the Rainbow Arches built (SIM-UNI-001, SIM-UNI-017), the supply the ladder's prism column was sized against. Four ladder prices and the Sky Arch's drift moved to fit it (SIM-UNI-005), and three properties hold the result. The reference clowder buys each step within ten days and the whole ladder in three to seven weeks (SIM-UNI-018). The top of the ladder refines four echoes a day (SIM-UNI-019). No upgrade asks more stardust than four Star Basins store (SIM-UNI-020). The pacing is timed by a model that steps between Parades on the sim's own rates. A calibration test holds that model to a day of real ticks, and before the change it agreed with a release-build real-tick climb to within a day per step. On the old content the same model stalls the ladder at its fifth step, the failure the issue reported. The three properties are decided by the ladder's price table, which carries their annotation. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-21, for what issue #54 changed. A Parade's prisms are scaled by the Prism Sight ratio where that tradition is held (SIM-UNI-001, SIM-MOVE-015), and a test holds the prisms brightened and the rainbows unchanged. The pacing properties (SIM-UNI-018, SIM-UNI-019) are held for a reference clowder with no traditions, so they did not move. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
