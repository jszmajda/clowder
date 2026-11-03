# Arrow: persistence

Persistence turns the deterministic sim into a game that survives closing the terminal: versioned human-readable saves, atomic writes, and offline catch-up that replays the *same* `tick()` the live ga

## Status

**PARTIAL** — last audited 2026-11-03 (commit `2cddde1e39c7`). 32 of 32 active specs implemented, each cited by a test; 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/persistence/persistence-design.md`

### EARS
- `docs/intent/persistence/persistence-specs.md` — 33 specs, prefix `SAVE`

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
| All | SAVE-001 – SAVE-033 | 32 | 1 | 0 |

**Summary:** 32 of 32 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-02. First full audit, made for issue #30, which added the v10 to v11 step (SAVE-031, with a committed v10 fixture): every spec read against its code and tests by a reader given only the arrow artifacts, its findings then checked by hand. It found floors running after the caps that read them, so a hand-edited wisdom below zero could zero every stock (fixed, tested, and the test checked by putting the old order back; SAVE-005), a version past 32 bits wrapping round to a known one (fixed, SAVE-002), and an owed count too large to add to overflowing (fixed, SAVE-005). Negative integer counters are still rejected rather than clamped (SAVE-032, deferred). The docs now say what the code does where they disagreed: the save's shape, what a backwards clock still replays (tested), what a migration may change, and which memorials a memorial inherits from. SAVE-009 and SAVE-010 are properties of the whole path, so no code decides them, and their tests are their pins. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SAVE-001, SAVE-011, SAVE-012, SAVE-016, the shell's tests, and SAVE-024's oracle were read more lightly.

Audited 2026-11-03, for what issue #32 changed. SAVE-009 now names the meadow and speed its bound holds at, and SAVE-033 bounds a late meadow, the design saying why a faster speed scales the wait rather than the bound. The late guard fails on the previous build at 424 s and passes on this one at about 28 s; the early guards stand at about 4 s of their 10 s, and every ignored guard passes run side by side. A real late save, both fixtures, and a larger late meadow, replayed by the previous build and this one, ended in the same bytes. SAVE-009 and SAVE-033 are properties of the whole path, so no code decides them, and their tests are their pins. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

- #36: a hand-edited save with a negative counter is rejected (SAVE-032, deferred).
- #37: thin tests, shape annotations, and an open question on achievements across a set-down.
