# Arrow: ceremonies

Ceremonies are clowder's faith system, reimagined as warmth and devotion rather than worship and sacrifice. Kittens gather in sunpuddles for Sunbeam Ceremonies; their quiet devotion accrues, is offere

## Status

**OK** — last audited 2026-11-01 (commit `c799869e32f7`). 10 of 10 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/sim/ceremonies/ceremonies-design.md`
- `docs/intent/sim/sim-design.md` (parent sub-HLD)

### EARS
- `docs/intent/sim/ceremonies/ceremonies-specs.md` — 10 specs, prefix `SIM-SUN`

### Tests
- `crates/clowder-core/tests/integration/blurb_truth.rs`
- `crates/clowder-core/tests/integration/blurbs.rs`
- `crates/clowder-core/tests/integration/legacy.rs`
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
| All | SIM-SUN-001 – SIM-SUN-010 | 10 | 0 | 0 |

**Summary:** 10 of 10 active specs implemented.

## Key Findings

Audited 2026-11-01. First full audit, by the same route as the mystic leaf. It found a shortfall naming an unrelated resource (fixed, SIM-SUN-008), an absolute claim its sibling spec contradicted (corrected, SIM-SUN-003), a formula missing a term and a factor (corrected, SIM-SUN-004), and clauses no test exercised (tested). Issue #17 named the flat term the cap formula had left out (SIM-SUN-004), fed from the cosmic and adventures leaves. Issue #13 held every rite's figures to its rule (SIM-SUN-009).

## Work Required

None.
