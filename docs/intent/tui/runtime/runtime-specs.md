# Runtime Specs (TUI-RUN)

## Startup & Terminal Lifecycle

- [x] **TUI-RUN-001**: The system shall resolve the save path and attempt the load before entering raw mode or the alternate screen, so a startup failure reports itself on an ordinary terminal.
- [x] **TUI-RUN-002**: On startup the system shall enter raw mode and the alternate screen, and shall restore both on every exit path — normal quit, error return, panic, and termination signal.
- [x] **TUI-RUN-003**: When the process panics, the system shall restore the terminal before the panic message is printed, so the message lands on a usable screen.
- [x] **TUI-RUN-004**: When SIGTERM or SIGHUP is received, the system shall record it and let the event loop take its ordinary shutdown path — saving and restoring the terminal — rather than performing that work inside the signal handler.
- [x] **TUI-RUN-005**: Where no platform data directory resolves, the system shall fall back to a save path beside the working directory and state where the save landed, rather than refusing to start.

## One Session Per Save

- [x] **TUI-RUN-006**: On startup the system shall acquire a lock beside the save file recording the owning process's id and start time, and shall release it on every exit path.
- [x] **TUI-RUN-007**: When the lock is held by a process that is still running, the system shall refuse to start, name the holding process, and state how to override.
- [x] **TUI-RUN-008**: When the lock's recorded process is no longer running — or its id now belongs to a process with a different start time — the system shall treat the lock as stale, clear it, and continue starting.
- [x] **TUI-RUN-009**: When the override is given, the system shall clear the lock and start even if its holder is live.
- [x] **TUI-RUN-029**: The system shall acquire the lock before reading the save, and shall release it on every path that then refuses to start, so that two simultaneous launches cannot both pass the check and no refusal leaves a lock behind.

## Load Failure Policy

- [x] **TUI-RUN-010**: When no save file exists, the system shall start a new game without reporting an error.
- [x] **TUI-RUN-011**: When the save was written by a newer version, the system shall refuse to start and explain plainly, and shall not modify or overwrite the file.
- [x] **TUI-RUN-012**: When the save file is present but unparseable, the system shall move it aside under a suffixed name, tell the player where it went, and then start a new game; it shall never overwrite the unparseable file in place.
- [x] **TUI-RUN-013**: When the save file cannot be read at all — a permission error, a path that is not a file, or an I/O failure — the system shall stop with a plain explanation and leave the file untouched.

## The Event Loop

- [x] **TUI-RUN-014**: Each iteration the system shall poll for input with a frame-budget timeout, apply any resulting commands, advance the simulation, autosave if due, and render one frame, in that order.
- [ ] **TUI-RUN-015**: The system shall apply a frame's commands before running that frame's ticks, so a key pressed during the preceding wait takes effect at the time it was pressed rather than after the elapsed interval.
- [ ] **TUI-RUN-016**: The system shall consume at most a bounded number of input events per frame and shall coalesce queued resize events to the most recent, so a flood of input cannot starve the simulation or the renderer.
- [x] **TUI-RUN-017**: The system shall advance the simulation regardless of render state — including a terminal too small to draw and a frame with nothing to redraw — so time never depends on what is on screen.

## Time & Ticks

- [x] **TUI-RUN-018**: The system shall convert elapsed real time into ticks as elapsed seconds × the core's ticks-per-second × the current speed, running only whole ticks and banking the fractional remainder across frames so no partial tick is lost or double-counted.
- [x] **TUI-RUN-019**: The system shall count ticks owed as integers rather than floating-point, so a large backlog cannot lose whole ticks to precision.
- [x] **TUI-RUN-020**: When the speed changes, the system shall apply the new speed only to time elapsing after the change and shall not rescale already-banked time.
- [x] **TUI-RUN-021**: The system shall drive ordinary accumulation from a monotonic clock, so a wall-clock correction can neither rewind the simulation nor inject elapsed time it did not observe.
- [x] **TUI-RUN-022**: When wall-clock elapsed exceeds monotonic elapsed by more than a small tolerance — the machine having been suspended while the process ran — the system shall convert the excess into ticks through the same accumulator, so a suspended machine costs the player no game time.
- [x] **TUI-RUN-023**: When wall-clock elapsed falls short of monotonic elapsed, the system shall keep the monotonic reading and re-base its checkpoint silently; the simulation shall never move backwards.
- [x] **TUI-RUN-024**: The system shall cap an in-process time gap at the same limit the load path applies and flag it the same way, so an absence spanning a suspend and an absence spanning a restart are indistinguishable to the player.
- [x] **TUI-RUN-025**: The system shall spend at most a bounded slice of wall time per frame running a tick backlog and shall bank the remainder for following frames, so a long catch-up never blocks the loop and never discards owed ticks.
- [D] **TUI-RUN-032**: The system shall bank the real-time span an unrun backlog represents as it accrues, rather than deriving it from the backlog and the current speed. Deferred: this existed to make back-dating safe across a speed change, and back-dating is gone — the banked ticks are carried as a count (SAVE-017, TUI-RUN-027), so nothing in production reads the span. Kept rather than deleted because a future consumer wanting a true elapsed span would need exactly this, and the reasoning is worth not rediscovering.
- [x] **TUI-RUN-030**: The system shall distinguish the sub-tick fractional remainder from the whole-tick backlog not yet run, and shall convert every interval to ticks at the speed in force when that interval elapsed — so both quantities are held already-converted and a speed change re-rates neither. Banking un-converted time would let a later speed change silently re-price a span the player already lived at another pace.
- [x] **TUI-RUN-031**: When shutdown begins while a tick backlog is outstanding, the system shall save and exit promptly rather than draining the backlog first, the saved count of banked ticks carrying them forward so the next load runs them.

## Saving

- [x] **TUI-RUN-026**: The system shall autosave on a wall-clock interval and on every clean shutdown, using the core's atomic write.
- [x] **TUI-RUN-027**: The system shall record a save's banked-but-unrun ticks as an explicit count beside the present timestamp, so a save taken mid-catch-up owes exactly the ticks it banked — neither lost nor counted twice, and never re-priced by a speed set after they were earned.
- [x] **TUI-RUN-028**: When a save fails to write, the system shall report it once, continue playing, and retry at the next cadence, rather than ending the session or retrying continuously.
