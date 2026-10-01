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

- [ ] **TUI-RUN-014**: Once the game is awake — from the start of a new game, from before the first frame for a load whose first bound finishes its catch-up (TUI-RUN-034), and otherwise from the end of its waking (TUI-RUN-038) — each iteration the system shall poll for input with a frame-budget timeout, apply any resulting commands, advance the simulation, autosave if due, and render one frame, in that order.
- [x] **TUI-RUN-015**: The system shall apply a frame's commands before running that frame's ticks, so a key pressed during the preceding wait takes effect at the time it was pressed rather than after the elapsed interval.
- [x] **TUI-RUN-016**: The system shall consume at most a bounded number of input events per frame and shall coalesce queued resize events to the most recent, so a flood of input cannot starve the simulation or the renderer.
- [x] **TUI-RUN-017**: The system shall advance the simulation regardless of render state — including a terminal too small to draw and a frame with nothing to redraw — so time never depends on what is on screen.

## Time & Ticks

- [x] **TUI-RUN-018**: The system shall convert elapsed real time into ticks as elapsed seconds × the core's ticks-per-second × the current speed, running only whole ticks and banking the fractional remainder across frames so no partial tick is lost or double-counted.
- [x] **TUI-RUN-019**: The system shall count ticks owed as integers rather than floating-point, so a large backlog cannot lose whole ticks to precision.
- [x] **TUI-RUN-020**: When the speed changes, the system shall apply the new speed only to time elapsing after the change and shall not rescale already-banked time.
- [x] **TUI-RUN-021**: The system shall drive ordinary accumulation from a monotonic clock, so a wall-clock correction can neither rewind the simulation nor inject elapsed time it did not observe.
- [x] **TUI-RUN-022**: When wall-clock elapsed exceeds monotonic elapsed — the machine having been suspended while the process ran — the system shall convert the excess into ticks through the same accumulator, so a suspended machine costs the player no game time. No threshold gates the conversion: an excess too small to reach one would be time the player is owed and silently loses, and forward clock drift credited as absence errs in the direction the meadow should err.
- [x] **TUI-RUN-023**: When wall-clock elapsed falls short of monotonic elapsed, the system shall keep the monotonic reading and re-base its checkpoint silently; the simulation shall never move backwards.
- [x] **TUI-RUN-024**: The system shall cap an in-process time gap at the same limit the load path applies and flag it the same way, so an absence spanning a suspend and an absence spanning a restart are indistinguishable to the player.
- [x] **TUI-RUN-025**: The system shall run at most a bounded number of backlogged ticks per frame and shall bank the remainder for following frames, so a long catch-up never blocks the loop and never discards owed ticks. The bound is a count rather than a slice of wall time: a time budget would have to read a clock inside the drain loop, and would buy a different number of ticks on a fast machine than on a slow one, making how far the meadow advances a property of the hardware. The accumulator cannot know what a tick costs, so the caller chooses the count.
- [D] **TUI-RUN-032**: The system shall bank the real-time span an unrun backlog represents as it accrues, rather than deriving it from the backlog and the current speed. Deferred: this existed to make back-dating safe across a speed change, and back-dating is gone — the banked ticks are carried as a count (SAVE-017, TUI-RUN-027), so nothing in production reads the span. Kept rather than deleted because a future consumer wanting a true elapsed span would need exactly this, and the reasoning is worth not rediscovering.
- [x] **TUI-RUN-030**: The system shall distinguish the sub-tick fractional remainder from the whole-tick backlog not yet run, and shall convert every interval to ticks at the speed in force when that interval elapsed — so both quantities are held already-converted and a speed change re-rates neither. Banking un-converted time would let a later speed change silently re-price a span the player already lived at another pace.
- [x] **TUI-RUN-031**: When shutdown begins while a tick backlog is outstanding, the system shall save and exit promptly rather than draining the backlog first, the saved count of banked ticks carrying them forward so the next load runs them.

## Saving

- [ ] **TUI-RUN-026**: The system shall autosave on a wall-clock interval once the game is awake (TUI-RUN-014), never while waking (TUI-RUN-037), and shall save on every clean shutdown, one taken while waking included, using the core's atomic write.
- [x] **TUI-RUN-027**: The system shall record a save's banked-but-unrun ticks as an explicit count beside the present timestamp, so a save taken mid-catch-up owes exactly the ticks it banked — neither lost nor counted twice, and never re-priced by a speed set after they were earned.
- [x] **TUI-RUN-028**: When a save fails to write, the system shall report it once, continue playing, and retry at the next cadence, rather than ending the session or retrying continuously.

## Waking Up

- [x] **TUI-RUN-033**: When a load owes catch-up, the system shall replay it after entering raw mode and the alternate screen rather than before, advancing the core's catch-up (SAVE-018) by at most a bounded count of ticks per frame — a count rather than a slice of wall time, for the reasons TUI-RUN-025 gives, and its own count rather than live play's — and shall draw its first frame having replayed at most one such bound, so a long absence shows itself waking rather than holding a terminal that has not yet appeared.
- [x] **TUI-RUN-034**: Where the first bound of a load's catch-up finishes it — a short absence, a backwards clock, or none — the system shall open the game directly, drawing no waking screen; otherwise it shall draw the waking screen (TUI-NAV-102) on every frame until the catch-up finishes.
- [x] **TUI-RUN-035**: While waking, the system shall poll for input without waiting out the frame budget, so the replay runs at the machine's pace with frames as its only pauses.
- [x] **TUI-RUN-036**: While waking, the system shall run no live tick, and shall observe the real time passing through the ordinary accumulator at the save's speed — suspend excess and the absence cap included — banking it to run as live ticks from the first frame of play, after every replayed tick, so the ticks run in the order they were earned and the wait costs no game time.
- [x] **TUI-RUN-037**: While waking, the system shall take no autosave, leaving the save on disk owing everything the wake has not finished, so a crash or kill mid-wake loses nothing and the next load replays from that save.
- [x] **TUI-RUN-038**: When an advance finishes a load's catch-up, the system shall run no live tick in that frame, and shall take the opening (TUI-NAV-024) and queue the load's retained report (TUI-NAV-027) against the caught-up state before the first frame of play, so the opening is taken against the state a one-call load of the same save at the same now_unix produces, bit for bit, and the live ticks banked while waking run only after it (TUI-RUN-036).
- [x] **TUI-RUN-039**: When shutdown begins while waking — a quit key or a termination signal — the system shall set the catch-up down (SAVE-020) and save promptly rather than finishing the wake, the save's owed count carrying the catch-up's remaining ticks together with any live ticks banked while waking, and the save carrying the report so far the set-down returns (SAVE-021), so a later load wakes through the rest and tells the absence once (SAVE-022).
