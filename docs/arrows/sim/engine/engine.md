# Arrow: engine

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RN

## Status

**OK** — last audited 2026-11-02 (commit `376d3e12c927`). 82 of 82 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/engine/engine-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/engine/engine-specs.md` — 82 specs, prefix `SIM-ENG`

### Tests
- `crates/clowder-core/tests/integration/draw_schedule.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/engine.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/idea_unlocks.rs`
- `crates/clowder-core/tests/integration/journal.rs`
- `crates/clowder-core/tests/integration/legacy.rs`
- `crates/clowder-core/tests/integration/persistence.rs`
- `crates/clowder-core/tests/integration/queries_bench.rs`
- `crates/clowder-core/tests/integration/queries_flows.rs`
- `crates/clowder-core/tests/integration/queries_gating.rs`
- `crates/clowder-core/tests/integration/queries_rates.rs`
- `crates/clowder-core/tests/integration/queries_timers.rs`
- `crates/clowder-core/tests/integration/revealed_resources.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/sources.rs`
- `crates/clowder-core/tests/integration/store_links.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`
- `crates/clowder-core/tests/integration/unit_price.rs`
- `crates/clowder-core/tests/integration/wonder.rs`
- `crates/clowder-tui/tests/integration/update.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/journal.rs`
- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/rng.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`
- `crates/clowder-tui/src/nav/rows.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ENG-001 – SIM-ENG-083 | 82 | 0 | 0 |

**Summary:** 82 of 82 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-ENG-076 is a property of the content tables as a whole, so no code decides it, and its tests are its pins. Issue #3 added the store-links query (SIM-ENG-079 to SIM-ENG-083), read by a reader given only the arrow artifacts, its findings then checked by hand: items growing a store the first draft missed, a recipe not yet learned leaking through a year-close craft, and a pounce reporting goods it would not bring were all fixed and tested, and a fixed act's links are held to what applying it moves. The sources query now takes each source's goods from one function, and lists a loom it had left out. Issue #14 pinned SIM-ENG-018's draw order with a test that walks a twin of the generator through the schedule on thousands of seeds, confirmed by mutating the real order; the golden test stays ignored, for the different question of whether a build plays a save's future out the same.

Audited 2026-11-02, for what issue #25 changed. The effect sum gained one structure's share of a key every cap reads each tick, taken from its one holder behind the check for which keys carry a structure share. A scan for it, and then a check for it ahead of that one, each took the catch-up bound (SAVE-009) from about 4.8 s to about 5.0 s of its 5 s; read this way, alone and alternating against the previous build, it stands within about 1% of that build and passes. That bound's thin margin is #32. The audit cited SIM-ENG-083 on the line that decides it, which had no annotation in code. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #30 changed. Which idea opens the loom and the picnics moved in the availability and command-gate answers, and the pen pal gate now says what its command says, where a row could offer a letter that refused (SIM-ENG-045). The reveal scan reads a structure's gate inline, since a call there, once the gate gained a kind, took the catch-up bound (SAVE-009) about 13% over the previous build and past its limit; measured alone and alternating, it now stands level with that build. That bound's thin margin is #32. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
