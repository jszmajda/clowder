# Arrow: dreamtime

Dreamtime is clowder's time layer, reimagined as *napping*: a well-rested clowder can drift into a long, warm dream and wake to find the meadow has quietly advanced. This is the inspiration's Time/Chr

## Status

**OK** — last audited 2026-11-01 (commit `5c5f7e08dc32`). 11 of 11 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/dreamtime/dreamtime-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/dreamtime/dreamtime-specs.md` — 11 specs, prefix `SIM-DREAM`

### Tests
- `crates/clowder-core/tests/integration/blurb_truth.rs`
- `crates/clowder-core/tests/integration/blurbs.rs`
- `crates/clowder-core/tests/integration/bulktime.rs`
- `crates/clowder-core/tests/integration/draw_schedule.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/structure_fields.rs`

### Code
- `crates/clowder-core/src/command.rs`
- `crates/clowder-core/src/content.rs`
- `crates/clowder-core/src/effects.rs`
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/journal.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-core/src/tick.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SIM-DREAM-001 – SIM-DREAM-013 | 11 | 0 | 0 |

**Summary:** 11 of 11 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit, by the same route as the mystic leaf. It found a timed pause one tick short (fixed, SIM-DREAM-004) and a blurb stating a relative change for a rule that adds an absolute one (blurb corrected to the rule, which the spec and design agree with). Design and spec disagreements were reconciled, and untested clauses were tested. SIM-DREAM-005 is a negative requirement annotated at the one spend it permits. Issue #13 held every dream loom's figures to its rule (SIM-DREAM-012). Issue #14 pinned the déjà-vu roll's place in the draw order (SIM-DREAM-004). Issue #16 stated the nap curve, heat, cooling, and reverie band in the design and specs, and held them with literal tests (SIM-DREAM-001 to SIM-DREAM-004).

## Work Required

None.
