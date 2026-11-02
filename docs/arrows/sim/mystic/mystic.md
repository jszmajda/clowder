# Arrow: mystic

The mystic leaf is the unicorn economy and everything glittering that follows from it — the inspiration's unicorn/ziggurat/alicorn/tears chain, reimagined so nothing is ever sacrificed. Unicorns hold 

## Status

**PARTIAL** — last audited 2026-11-01 (commit `5c5f7e08dc32`). 15 of 15 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/mystic/mystic-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/mystic/mystic-specs.md` — 15 specs, prefix `SIM-UNI`

### Tests
- `crates/clowder-core/tests/integration/blurb_truth.rs`
- `crates/clowder-core/tests/integration/blurbs.rs`
- `crates/clowder-core/tests/integration/draw_schedule.rs`
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
| All | SIM-UNI-001 – SIM-UNI-015 | 15 | 0 | 0 |

**Summary:** 15 of 15 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its rule, its tests, and the design by a reader given only the arrow artifacts, its findings then checked by hand. It found a spend with no room for the output (fixed, SIM-UNI-004), absolute claims and formulas contradicted by sibling specs (corrected), and clauses no test exercised (tested). What a Promise grants was narrowed in the design rather than built out, on Tenet 1 (Decisions & Alternatives). Issue #13 held every arch upgrade's figures to its rule (SIM-UNI-012), and the idea granting a free arch now says so (SIM-UNI-011). Issue #14 pinned the descent and shower rolls' place in the draw order (SIM-UNI-006). Issue #16 stated and pinned a shower's windfall and the Sky Arch's drift (SIM-UNI-005).

## Work Required

- #15: two effect keys no content feeds (SIM-UNI-001, SIM-UNI-003).
