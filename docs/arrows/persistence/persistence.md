# Arrow: persistence

Persistence turns the deterministic sim into a game that survives closing the terminal: versioned human-readable saves, atomic writes, and offline catch-up that replays the *same* `tick()` the live ga

## Status

**OK** — last audited 2026-11-11 (commit `8f62315c606c`). 41 of 41 active specs implemented, each cited by a test.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/persistence/persistence-design.md`

### EARS
- `docs/intent/persistence/persistence-specs.md` — 41 specs, prefix `SAVE`

### Tests
- `crates/clowder-core/tests/integration/raze.rs`
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
- `crates/clowder-tui/tests/integration/waking.rs`

### Code
- `crates/clowder-core/src/events.rs`
- `crates/clowder-core/src/journal.rs`
- `crates/clowder-core/src/save.rs`
- `crates/clowder-core/src/save/wide.rs`
- `crates/clowder-core/src/state.rs`
- `crates/clowder-tui/src/runtime/mod.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | SAVE-001 – SAVE-041 | 41 | 0 | 0 |

**Summary:** 41 of 41 active specs implemented.

## Key Findings

Audited 2026-11-02. First full audit, made for issue #30, which added the v10 to v11 step (SAVE-031, with a committed v10 fixture): every spec read against its code and tests by a reader given only the arrow artifacts, its findings then checked by hand. It found floors running after the caps that read them, so a hand-edited wisdom below zero could zero every stock (fixed, tested, and the test checked by putting the old order back; SAVE-005), a version past 32 bits wrapping round to a known one (fixed, SAVE-002), and an owed count too large to add to overflowing (fixed, SAVE-005). Negative integer counters are still rejected rather than clamped (SAVE-032, deferred). The docs now say what the code does where they disagreed: the save's shape, what a backwards clock still replays (tested), what a migration may change, and which memorials a memorial inherits from. SAVE-009 and SAVE-010 are properties of the whole path, so no code decides them, and their tests are their pins. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. SAVE-001, SAVE-011, SAVE-012, SAVE-016, the shell's tests, and SAVE-024's oracle were read more lightly.

Audited 2026-11-03, for what issue #32 changed. SAVE-009 now names the meadow and speed its bound holds at, and SAVE-033 bounds a late meadow, the design saying why a faster speed scales the wait rather than the bound. The late guard fails on the previous build at 424 s and passes on this one at about 28 s; the early guards stand at about 4 s of their 10 s, and every ignored guard passes run side by side. A real late save, both fixtures, and a larger late meadow, replayed by the previous build and this one, ended in the same bytes. SAVE-009 and SAVE-033 are properties of the whole path, so no code decides them, and their tests are their pins. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #36 changed. A load reads every integer through one reader, as the nearest value its field can hold (SAVE-032), which closes the deferred gap. A test walks every integer the format writes, at a negative, a fraction, and numbers past any field's range, and plays each meadow on with overflow checks on: that walk found housing, tinkerer seats, an away stamp, and the tick count overflowing at the top of their ranges, now given room (SAVE-035) or saturated. A negative Elder index is no Elder (SAVE-034), and required now names the fields every format version wrote, the walk holding both lists by name (SAVE-036). A real save loads byte for byte as it did under the previous build. An owed count read at its greatest is honoured rather than capped, so a one-call load of one would not return; the shell's wake stays paced and can be set down. Each test was checked by breaking what it guards. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #33 changed. A caller that stayed running across an absence can begin a catch-up for it (SAVE-037) and join a further one to it (SAVE-038), each ending byte for byte as a load of the same absence would. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #37 changed, which closes what the first audit left open. Its question has an answer: one tale does cover achievements, so a report lists what its replay earned and a wake set down keeps that as new (SAVE-039). Tests now hold the speed clamp (SAVE-005), the exact minimum a report is kept at (SAVE-015), older entries in a current save (SAVE-025), and a v8 save's inherited, kept, and unread looks (SAVE-029), where breaking the keeping of looks passed until a look differing from its draw was tried, a v8 binary's own always agreeing with it. Annotations left the types that enumerate outcomes for what decides, and SAVE-004's missing fields are held by #36's walk. SAVE-009, SAVE-010, and SAVE-033 are properties of the whole path, so no code decides them, and their tests are their pins. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #43 changed. Saves move to version 12 so a journal holding the new event reads as too new to an older binary rather than corrupt, and a v11 fixture holds that migrating changes no field (SAVE-040).

Audited 2026-11-11, for what issue #51 changed. A replay's report now lists the structures and ideas it revealed, in order. A set-down wake carries the list into the load that finishes it, and a nap's report lists its own through the same fold (SAVE-041). This is not a save version step, by the design's rule for a report that gains data: an older binary drops the detail unread. Tests cover a load, a set-down wake, and a nap. The release catch-up guards were run on the change against their bounds. Every spec marked implemented has a test citing it, by source scan. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
