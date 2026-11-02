# Arrow: curiosity

Curiosity is the game's research system: kittens investigate the world and their accumulated curiosity is spent on **Ideas** — one-time unlocks that open structures, jobs, and permanent modifiers. The

## Status

**PARTIAL** — last audited 2026-11-01 (commit `c799869e32f7`). 15 of 15 active specs implemented, each cited by a test; 1 deferred.

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
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/wonder.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-CUR-001 – SIM-CUR-016 | 15 | 1 | 0 |

**Summary:** 15 of 15 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code and tests, and the design's three tables read row by row, by a reader given only the arrow artifacts, its findings then checked by hand. All 59 idea, 35 refinement, and 8 rule rows match on every number. It found ideas whose named unlocks no rule reads (SIM-CUR-009, deferred), three blurbs leaving out figures their rules apply and one stating the wrong kind (fixed), a spec forbidding grants others require (SIM-CUR-006, corrected), and one claiming more permanence than a Move leaves (SIM-CUR-004, corrected). Issue #13 added SIM-CUR-016, holding every blurb's figures to their rule in both directions.

## Work Required

- #30: some ideas buy nothing, because no rule reads them (SIM-CUR-009, deferred).
- #31: thin table tests and stray annotations.
