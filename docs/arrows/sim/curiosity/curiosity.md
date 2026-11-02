# Arrow: curiosity

Curiosity is the game's research system: kittens investigate the world and their accumulated curiosity is spent on **Ideas** — one-time unlocks that open structures, jobs, and permanent modifiers. The

## Status

**PARTIAL** — last audited 2026-11-02 (commit `376d3e12c927`). 16 of 16 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/curiosity/curiosity-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/curiosity/curiosity-specs.md` — 16 specs, prefix `SIM-CUR`

### Tests
- `crates/clowder-core/tests/integration/blurb_truth.rs`
- `crates/clowder-core/tests/integration/curiosity.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/idea_unlocks.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-CUR-001 – SIM-CUR-016 | 16 | 0 | 0 |

**Summary:** 16 of 16 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code and tests, and the design's three tables read row by row, by a reader given only the arrow artifacts, its findings then checked by hand. All 59 idea, 35 refinement, and 8 rule rows match on every number. It found ideas whose named unlocks no rule reads (SIM-CUR-009, deferred), three blurbs leaving out figures their rules apply and one stating the wrong kind (fixed), a spec forbidding grants others require (SIM-CUR-006, corrected), and one claiming more permanence than a Move leaves (SIM-CUR-004, corrected). Issue #13 added SIM-CUR-016, holding every blurb's figures to their rule in both directions.

Audited 2026-11-02, for what issue #30 changed. A rule now reads each idea for each unlock its row names (SIM-CUR-009): Scout Picnicking, Dream Tending, and Hushfelt Seeking gate what they promise, and Dream Loomcraft opens the loom ahead of the idea that used to. Solar Devotion's row and blurb stopped claiming the ceremonies, which keep their own reason to wait, and Sun Salutations' row and blurb state them. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-CUR-016 is a property of the tables as a whole, so no code decides it, and its tests are its pins. Hand-read by the agent that made these changes, with no second reader.

## Work Required

- #31: thin table tests and stray annotations.
