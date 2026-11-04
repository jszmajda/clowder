# Arrow: economy

The economy is the material world of the game: what exists, what it costs, what produces what. One leaf, five facets — `RES` (resources), `BLD` (structures), `CRAFT` (crafting), `NRG` (energy), `MESS`

## Status

**OK** — last audited 2026-11-04 (commit `ab33fd4fc7c9`). 66 of 66 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/economy/economy-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/economy/economy-specs.md` — 66 specs, prefix `SIM-ECO`

### Tests
- `crates/clowder-core/tests/integration/raze.rs`
- `crates/clowder-core/tests/integration/craft_all.rs`
- `crates/clowder-core/tests/integration/economy.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/queries_bench.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/store_descriptions.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`
- `crates/clowder-core/tests/integration/wonder.rs`
- `crates/clowder-tui/tests/integration/bench.rs`
- `crates/clowder-tui/tests/integration/frame.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/effects/index.rs`
- `crates/clowder-core/src/gating.rs`
- `crates/clowder-core/src/query.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-ECO-001 – SIM-ECO-066 | 66 | 0 | 0 |

**Summary:** 66 of 66 active specs implemented.

## Key Findings

Audited 2026-11-01. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. The structure table was read row by row against the definitions: one effect it listed had never reached the rules, and now does (SIM-ECO-053). Rule-held structure behaviour now lives in definition fields, and no rule branches on a structure's id to find a rate. SIM-ECO-024 and SIM-ECO-047 are properties of the content tables as a whole, so no code decides them and their tests are their pins. Issue #2 gave every store a description (SIM-ECO-061, SIM-ECO-062), held to its limits by tests over the whole table.

Audited 2026-11-03, for what issue #32 changed. The code that sums this segment's effect contributions moved behind the per-key source lists (SIM-ENG-084), annotations with it; its specs did not change. Each contribution is held by a bit-for-bit comparison against a walk of every source, and a real late save replayed by the previous build and this one ended in the same bytes. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader. Converters and fuel fractions now ask the throttle over demands worked out as they are read, allocating nothing, with the same fraction.

Audited 2026-11-03, for what issue #38 changed. Every structure now has one family (SIM-ECO-063), through a rule exhaustive over the structures. The table is pinned as the design states it, and what a family's name promises is held against each structure's own fields, so a change to what a structure does asks again where it belongs. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #44 changed. A recipe can now be made as many times as the stocks cover in one application (SIM-ECO-064), with the count worked out from the scarcest input. The tests hold it to what batch-by-batch calls leave, with each input in turn scarce and a craft ratio standing, and pin the recipe shape the count rests on: no input named twice, none the recipe's own output. Twelve mutations of the count, the spend, the yield, the refusals, and the row's call were each caught. Above roughly 2^53 a whole count of batches can round short of the stock by a sliver of its size, so a second press may still make a little; that is a rounding error of the stock, and no spec promises otherwise at that height.

Audited 2026-11-04, for what issue #43 changed. A copy of a structure can be taken down, returning a quarter of what it would cost to build back, up to each store's cap with the copy gone (SIM-ECO-065), and nobody is turned out (SIM-ECO-066). Tests hold that for every structure, and that building one and taking it down never gains anything.

## Work Required

None.
