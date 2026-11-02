# Arrow: persistence

Persistence turns the deterministic sim into a game that survives closing the terminal: versioned human-readable saves, atomic writes, and offline catch-up that replays the *same* `tick()` the live ga

## Status

**PARTIAL** — last audited 2026-11-02 (commit `376d3e12c927`). 31 of 31 active specs implemented, each cited by a test; 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/persistence/persistence-design.md`

### EARS
- `docs/intent/persistence/persistence-specs.md` — 32 specs, prefix `SAVE`

### Tests
- `crates/clowder-core/tests/integration/achievements.rs`
- `crates/clowder-core/tests/integration/economy_depth.rs`
- `crates/clowder-core/tests/integration/frontier.rs`
- `crates/clowder-core/tests/integration/idea_unlocks.rs`
- `crates/clowder-core/tests/integration/legacy.rs`
- `crates/clowder-core/tests/integration/neighbour_offers.rs`
- `crates/clowder-core/tests/integration/persistence.rs`
- `crates/clowder-core/tests/integration/profiles.rs`
- `crates/clowder-core/tests/integration/society.rs`
- `crates/clowder-core/tests/integration/trade_returns.rs`
- `crates/clowder-core/tests/integration/wonder.rs`
- `crates/clowder-tui/tests/integration/persist.rs`

### Code
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/journal.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SAVE-001 – SAVE-032 | 31 | 1 | 0 |

**Summary:** 31 of 31 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-02. First full audit, made for issue #30, which added the v10 to v11 step (SAVE-031, with a committed v10 fixture): every spec read against its code and tests by a reader given only the arrow artifacts, its findings then checked by hand. It found floors running after the caps that read them, so a hand-edited wisdom below zero could zero every stock (fixed, tested, and the test checked by putting the old order back; SAVE-005), a version past 32 bits wrapping round to a known one (fixed, SAVE-002), and an owed count too large to add to overflowing (fixed, SAVE-005). Negative integer counters are still rejected rather than clamped (SAVE-032, deferred). The docs now say what the code does where they disagreed: the save's shape, what a backwards clock still replays (tested), what a migration may change, and which memorials a memorial inherits from. SAVE-009 and SAVE-010 are properties of the whole path, so no code decides them, and their tests are their pins. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SAVE-001, SAVE-011, SAVE-012, SAVE-016, the shell's tests, and SAVE-024's oracle were read more lightly.

## Work Required

- #32: the catch-up bound is only measured on a tiny meadow.
- #36: a hand-edited save with a negative counter is rejected (SAVE-032, deferred).
- #37: thin tests, shape annotations, and an open question on achievements across a set-down.
