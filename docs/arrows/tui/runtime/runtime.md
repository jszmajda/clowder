# Arrow: runtime

This leaf owns the boundary between the operating system and the pure core: terminal lifecycle, the event loop, the conversion of real time into ticks, autosave cadence, save-path resolution, and the 

## Status

**OK** — last audited 2026-11-11 (commit `9c1932d81d1c`). 68 of 68 active specs implemented, each cited by a test; 1 deferred.

## References

### HLD
- `docs/high-level-design.md`

### LLD
- `docs/intent/tui/runtime/runtime-design.md`
- `docs/intent/tui/tui-design.md` (parent sub-HLD)

### EARS
- `docs/intent/tui/runtime/runtime-specs.md` — 69 specs, prefix `TUI-RUN`

### Tests
- `crates/clowder-tui/tests/integration/clock.rs`
- `crates/clowder-tui/tests/integration/held_clock.rs`
- `crates/clowder-tui/tests/integration/lock.rs`
- `crates/clowder-tui/tests/integration/loop_body.rs`
- `crates/clowder-tui/tests/integration/loop_order.rs`
- `crates/clowder-tui/tests/integration/persist.rs`
- `crates/clowder-tui/tests/integration/report_queue.rs`
- `crates/clowder-tui/tests/integration/session.rs`
- `crates/clowder-tui/tests/integration/terminal_lost.rs`
- `crates/clowder-tui/tests/integration/waking.rs`
- `crates/clowder-tui/tests/term.rs`

### Code
- `crates/clowder-core/src/save.rs`
- `crates/clowder-tui/src/runtime/clock.rs`
- `crates/clowder-tui/src/runtime/lock.rs`
- `crates/clowder-tui/src/runtime/mod.rs`
- `crates/clowder-tui/src/runtime/now.rs`
- `crates/clowder-tui/src/runtime/persist.rs`
- `crates/clowder-tui/src/runtime/term.rs`
- `crates/clowder-tui/src/runtime/wake.rs`

## Spec Coverage

| Category | Spec IDs | Implemented | Deferred | Gaps |
|----------|----------|-------------|----------|------|
| All | TUI-RUN-001 – TUI-RUN-080 | 68 | 1 | 0 |

**Summary:** 68 of 68 active specs implemented; 1 deferred.

## Key Findings

Audited 2026-11-01. First full audit: every spec read against its code, its tests, and the design by a reader given only the arrow artifacts, its most serious findings then checked by hand. It found a lock that can be cleared under a live holder, time away going uncounted around a resume, a damaged save that can be overwritten, and a suspend tolerance in the design that the spec and code deny (corrected). Issue #10 routed every monotonic read through one holdable function (TUI-RUN-066, TUI-RUN-067). Specs 001 to 004, 010, 011, 013, 017, 060 to 063, and 065 were read more lightly than the rest. A frame of play waits one tick at normal speed (TUI-RUN-068, #24): on one save copy in a 120×40 pane with no keys pressed, about 5.3% of a core became about 1.6%, with keys answered as quickly as before. The design records why a redraw-on-change check was not taken.

Audited 2026-11-02, for what issue #26 changed. A holder now reads as running from any time zone, since start times are recorded and compared in UTC, with a lock recorded in local time still counting in its own zone (TUI-RUN-069, TUI-RUN-070). A session releases only a lock that still names it (TUI-RUN-071), and a lock that cannot be removed stops the launch (TUI-RUN-072). The zone and read-only cases run the built game as a child. The audit moved two annotations off the types they sat on, onto what decides (TUI-RUN-006, TUI-RUN-007), added one where the refusal is worded, and cited TUI-RUN-063 on what turns mouse reporting on, which had none. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #27 changed. A frame reads its clocks after its wait, each reading starting where the last ended across a wake and on into play (TUI-RUN-073, TUI-RUN-036), and every way out takes one last reading before it saves (TUI-RUN-074), each ending tested with the machine suspended just before it and the last reading checked by removing it. A reading past the absence cap retains a truncated report (TUI-RUN-024), with no gains, its ticks not having run; #33 holds what a suspend inside the cap shows. A wall clock a test can hold made the suspend testable (TUI-RUN-075). Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-02, for what issue #28 changed. A save is decoded strictly, so a bad byte makes it corrupt rather than letting it parse and be overwritten (TUI-RUN-012), and the whole of setting a damaged save aside, telling the player, and starting anew is now tested on a real terminal, for a bad byte and for an unparseable file. A fallback save path is stated before the terminal is taken (TUI-RUN-005). That branch cannot be reached by a real launch on a machine whose user has a home directory, so its choice of path and its statement are tested as functions, and the one line calling them is read rather than run. TUI-RUN-032 stays deferred on purpose, its reason in its spec. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-03, for what issue #33 changed. A reading spanning the core's minimum reportable absence now wakes the running game as a load does, keeping the player's place (TUI-RUN-076 to TUI-RUN-078), a second long gap joins the first (TUI-RUN-079), and a session ending on one saves it as a wake set down (TUI-RUN-080). This replaces the report a capped gap retained with no gains, and the hours a capped gap took to drain at live play's bound. A long gap in play and a load of the same absence were shown to end in the same bytes, and each new test was checked by breaking what it guards, eleven ways. Every spec marked implemented has a test citing it and no `@spec` names a spec that does not exist, by source scan. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-04, for what issue #29 changed. Two behaviours moved: a burst of resizes while waking applies once (TUI-RUN-016), and a quit key ends its waking frame before the advance, so the finishing frame is still a quit while waking (TUI-RUN-039). The open questions are pinned in the specs and design: a wall clock stepped forward is credited as a suspend (TUI-RUN-021, TUI-RUN-022); autosave keeps to the monotonic clock (TUI-RUN-026); two launches are kept apart wherever the filesystem grants the advisory lock (TUI-RUN-029). The last question, what an in-process absence owes a report, was already settled by TUI-RUN-024 and TUI-RUN-076. Tests now reach the once-gate and cadence of a failed save (TUI-RUN-028), a backlog left undrained (TUI-RUN-031), and the override and the lock's order through the game itself (TUI-RUN-009, TUI-RUN-029). The speed-key item the issue named was already held by `session.rs`, and the override text by `lock.rs`. Annotation ranges are gone and every spec is cited by name; annotations moved off an unused type, since deleted, and onto what decides, and a test of the rejected back-dating scheme went with them. Thirteen mutations were each caught. TUI-RUN-032 stays deferred as its own text explains.

Audited 2026-11-11, for what issue #49 changed. The loop now hands the events of the ticks it runs in play to navigation, which marks what they reveal (navigation leaf: New rows). The loop's step 3 in the design says so. No runtime spec changed, and a catch-up's replay still hands navigation nothing. TUI-RUN-032 was reviewed: it is implemented and tested, and its own note keeps it deferred because nothing reads it, so nothing is left open. Hand-read by the agent that made these changes, with no second reader.

Audited 2026-11-11, for what issue #50 changed. After each frame it draws, the loop keeps the line the frame's list began on, read from the pointer record (render leaf: The List Viewport). Step 5 of the loop in the design says so. No runtime spec changed. Hand-read by the agent that made these changes, with no second reader.

## Work Required

None.
