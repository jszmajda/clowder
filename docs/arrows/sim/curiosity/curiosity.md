# Arrow: curiosity

Curiosity is the game's research system: kittens investigate the world and their accumulated curiosity is spent on **Ideas** — one-time unlocks that open structures, jobs, and permanent modifiers. The

## Status

**OK** — last audited 2026-11-04 (commit `c20801cda765`). 17 of 17 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/curiosity/curiosity-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/curiosity/curiosity-specs.md` — 17 specs, prefix `SIM-CUR`

### Tests
- `crates/clowder-core/tests/integration/blurb_truth.rs`
- `crates/clowder-core/tests/integration/curiosity.rs`
- `crates/clowder-core/tests/integration/curiosity_tables.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/idea_unlocks.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/effects/index.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-CUR-001 – SIM-CUR-017 | 17 | 0 | 0 |

**Summary:** 17 of 17 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code and tests, and the design's three tables read row by row, by a reader given only the arrow artifacts, its findings then checked by hand. All 59 idea, 35 refinement, and 8 rule rows match on every number. It found ideas whose named unlocks no rule reads (SIM-CUR-009, deferred), three blurbs leaving out figures their rules apply and one stating the wrong kind (fixed), a spec forbidding grants others require (SIM-CUR-006, corrected), and one claiming more permanence than a Move leaves (SIM-CUR-004, corrected). Issue #13 added SIM-CUR-016, holding every blurb's figures to their rule in both directions.

Audited 2026-11-02, for what issue #30 changed. A rule now reads each idea for each unlock its row names (SIM-CUR-009): Scout Picnicking, Dream Tending, and Hushfelt Seeking gate what they promise, and Dream Loomcraft opens the loom ahead of the idea that used to. Solar Devotion's row and blurb stopped claiming the ceremonies, which keep their own reason to wait, and Sun Salutations' row and blurb state them. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SIM-CUR-016 is a property of the tables as a whole, so no code decides it, and its tests are its pins. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #32 changed. The code that sums this segment's effect contributions moved behind the per-key source lists (SIM-ENG-084), annotations with it; its specs did not change. Each contribution is held by a bit-for-bit comparison against a walk of every source, and a real late save replayed by the previous build and this one ended in the same bytes. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #31 changed. The tables are held by `curiosity_tables.rs`, which reads the design's own rows: every idea's price, gate craft, and prerequisites (SIM-CUR-005); each structure, job, and recipe an idea opens, in its row and read by a rule, both ways round (SIM-CUR-009); every refinement's price, materials, gate, and effects, then bought under its gate, whole and once (SIM-CUR-010); and every idea with no box (SIM-CUR-001). The source order is held off the lists themselves (SIM-CUR-012). The question is pinned: a store's production share scales the stack's flows, and a flat add stays the figure its blurb gives (SIM-CUR-017). Annotations moved off the types that list outcomes onto the rules and tables that decide. Thirteen mutations of later rows, the design itself, and the rules were each caught.

## Work Required

None.
