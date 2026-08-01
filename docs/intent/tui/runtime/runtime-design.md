---
parent: tui
prefix: TUI-RUN
---

# Runtime (terminal lifecycle, the loop, and time)

## Context and Design Philosophy

This leaf owns the boundary between the operating system and the pure core: terminal lifecycle, the event loop, the conversion of real time into ticks, autosave cadence, save-path resolution, and the failure policy for a save that will not load. It is the only place in clowder that reads a clock, touches the filesystem, or handles a signal.

It owns no game rules (those are `clowder-core`), no key semantics (the `navigation` leaf decides what a key *means*; runtime only delivers it), and no drawing (the `render` leaf). Its whole job is to keep a deterministic simulation fed with the right number of ticks and to leave the player's terminal exactly as it found it, on every exit path including the ugly ones.

Two properties drive nearly every decision here. **The game lives in a tmux pane for weeks** — so the process will be suspended, resumed, resized, disowned, and killed by things that are not the player pressing `q`, and none of those may cost game time or leave a wrecked terminal. **Absence is never worse than presence** (HLD tenet 2) — so any real-world interval the player was away, whether the process was running or not, must arrive in the sim as the same ticks it would have accrued attended.

## The Loop

```
startup
  resolve save path
  load save (or start a new game)          ← before raw mode, so failures print normally
  enter raw mode + alternate screen
  install panic hook, drop guard, signal handlers

loop
  1. poll for input with a frame-budget timeout
  2. drain the ready events; hand keys and resizes to the navigation layer,
     which returns commands to apply to the core
  3. advance the sim: convert elapsed real time × speed into whole ticks
  4. autosave if the cadence has elapsed
  5. render one frame from the current snapshot

shutdown (any path)
  save · restore terminal
```

Single-threaded and synchronous, per the shell sub-HLD. Nothing here needs concurrency: the sim is sequential, the save is a few tens of kilobytes of JSON, and a poll-with-timeout loop is the most trivially correct shape available.

Step 3 runs **regardless of render state**. A terminal too small to draw, a pane the player cannot see, a frame that had nothing to redraw — none of them stop the clock. The sim keeps advancing and the render layer catches up whenever it can (TUI-NAV-029 states this from the UX side; here it is a property of loop ordering).

**Commands land before the frame's ticks.** A key pressed during the previous frame's wait is chronologically older than the time that wait represents, so applying it before advancing is the honest ordering — and it means a keypress never has to wait out a catch-up burst to take effect. The alternative, ticking first, would make a command issued at a fast speed land some ticks later than the player pressed it.

**Input is drained but bounded.** A paste, a held key, or a drag-resize can deliver events faster than frames retire them. Each frame consumes at most a bounded number of input events so a flood can never starve the sim or the renderer, and queued resize events coalesce to the last one — only the final size matters, and recomputing layout for each intermediate width is wasted work.

## Time: Two Clocks, One Simulation

The core exposes `TICKS_PER_SECOND` and takes wall-clock time only as a caller-supplied `now_unix` on load. Everything else about time is this leaf's.

**Ticks owed** accumulate as `elapsed_seconds × TICKS_PER_SECOND × speed`, with the fractional remainder banked across frames so no partial tick is ever lost or double-counted. Speed changes apply from the moment they happen and never retroactively rescale already-banked time — banked time was earned at the old rate.

Two quantities are banked, and conflating them is the easy mistake: a **sub-tick remainder**, a fraction of one tick, and a **whole-tick backlog** not yet run. Both are held *already converted* — every interval becomes ticks at the speed in force when that interval elapsed, at the moment it is observed.

That is what makes the rule simple enough to keep: **nothing banked is ever re-rated.** The tempting alternative, banking the remainder as raw seconds, quietly breaks it — the remainder would convert at whatever speed happened to be set when it was next folded, so changing speed would retroactively re-price a span the player already lived at another pace. The error is under one tick, which is exactly why it would never be noticed and never be fixed.

**Two clocks, because neither alone is correct:**

- A **monotonic** clock drives ordinary frame-to-frame accumulation. It cannot run backwards, so an NTP correction or a daylight-saving shift can never rewind the sim or inject an hour of phantom production.
- A **wall clock** checkpoint runs alongside it, because on every platform we target the monotonic clock *stops while the machine is suspended*. A laptop closed for eight hours with clowder still running would otherwise wake up having advanced a few seconds — silently punishing the player for walking away, which is precisely what tenet 2 forbids.

Each frame compares the two. When wall-clock elapsed exceeds monotonic elapsed by more than a small tolerance, the excess is real absence the monotonic clock did not see; it is converted to ticks through the same accumulator. When wall-clock elapsed is *less* (a backwards correction), the monotonic reading wins and the checkpoint re-bases silently. The sim only ever moves forward, and only ever through `tick()`.

**One absence rule.** `load_string` caps catch-up at 365 days and sets `truncated` on the report. An in-process gap uses the same cap and the same flag, so a two-year suspend and a two-year closed save behave identically. There is one rule for absence; whether the process happened to be running across it is not something the player should be able to feel.

**Catch-up is paced, never dropped.** A long gap can owe millions of ticks, and running them all in one frame is indistinguishable from a hang. Each frame spends a bounded slice of wall time running ticks and banks the rest, so a big catch-up drains over several frames while the UI stays alive and can say so. The tick count is exact; only its spreading over frames is approximate. Ticks owed are counted in integers, never floats, so a large backlog cannot lose precision.

**A save carries its banked ticks as a count.** Because ticks can be banked, the state in memory may be behind real time — mid-catch-up, deliberately. The timestamp is simply the present, and the ticks earned but not yet run are written beside it.

The tempting alternative is to back-date the timestamp so the interval re-derives them, and it is wrong in a way that takes a while to see: the reload reconstructs ticks by multiplying an interval by whatever speed is persisted, so a backlog earned at one speed and saved at another is re-priced by the ratio between them. Earn fast, save slow, and most of it evaporates; the other way round, it multiplies. What the player earned is theirs, so it is stored as what it is.

## Terminal Lifecycle

Raw mode and the alternate screen are entered once at startup and must be undone on **every** exit path. Three mechanisms, because each covers a path the others miss:

| Mechanism | Covers |
|---|---|
| Drop guard on a lifecycle value | Normal return and error propagation out of `main` |
| Panic hook | A panic anywhere, restoring *before* the default hook prints, so the message lands on a usable terminal |
| Signal handlers (SIGTERM, SIGHUP) | `tmux kill-session`, logout, shutdown — the process ends without unwinding at all |

Signal handlers do no work in the handler itself; they set a flag the loop observes, and the loop takes the ordinary shutdown path. This matters more here than in a typical program: a tool that lives in a long-running pane will be killed by the pane far more often than it will be quit deliberately, and the failure mode — an unsaved game plus a terminal left in raw mode — is the worst outcome the shell can produce.

Ctrl-C is not a signal here. In raw mode the terminal delivers it as an ordinary key event, so it is handled as a key by the navigation layer (TUI-NAV-016), which is why that spec can promise it saves first.

## Saving

**Cadence.** Autosave runs on a wall-clock interval and on every clean shutdown. The write itself is the core's atomic temp-file-plus-rename, so a save interrupted mid-write never corrupts the previous one.

**Shutdown does not wait for the sim to catch up.** A quit or a signal arriving mid-backlog saves and exits promptly rather than draining first — a player pressing `q` should not watch the game refuse to close while it works through a week of absence. Back-dating makes this free: the unrun interval is carried in the timestamp and simply runs at the next load. Shutdown is the same operation whether the sim is current or a month behind.

**Failure is not fatal.** A failed autosave (full disk, revoked permissions, a data directory that vanished) reports itself once and the game keeps running — the player keeps playing, and the next cadence retries. Killing a healthy session because a disk was briefly full would destroy more progress than the failed write did.

**Save path.** `default_save_path()` returns an `Option`; a platform with no resolvable data directory falls back to a path beside the working directory, and the game says where it landed rather than refusing to start.

## One Session Per Save

A player who lives in tmux is well placed to start clowder twice by accident — a second pane, a forgotten session, a shell history recall. Two sessions autosaving the same path is silent progress loss: both hold a divergent state in memory and the later write simply erases the other's afternoon.

Startup therefore takes a lock beside the save file, holding the owning process's id and start time. The lock is taken **before the save is read** — it is the mutual-exclusion primitive, and acquiring it after the load leaves a window in which two launches both read the same save and both believe they are alone. Every path that then refuses to start releases it, so a refusal never leaves a lock for the next launch to inherit. A second launch that finds a **live** holder refuses to start, says which process holds the game, and names the override — one session per save, enforced rather than hoped for.

A **stale** lock is cleared automatically and play continues. A lock outliving its process means clowder crashed or was killed, and the player's response to reopening their game should not be a lockout over a file their previous session failed to clean up. Recording the start time alongside the process id is what makes "stale" decidable: an id alone can be recycled by an unrelated process, and refusing to start because some other program inherited a number would be the same lockout by a different route. The explicit override force-clears even a live lock, for the case the checks get wrong.

The lock releases on every exit path, through the same three mechanisms that restore the terminal.

## Load Failure Policy

The core reports three distinct load failures and deliberately never rewrites the file itself, leaving the policy here.

| Failure | Response |
|---|---|
| `NotFound` | Start a new game. This is an ordinary first run, not an error. |
| `TooNew` | Refuse to start, restore the terminal, and explain plainly. A newer clowder wrote this; guessing forward would corrupt a save that is currently fine. |
| `Corrupt` | Never overwrite. Move the file aside under a suffixed name and start fresh only after telling the player exactly where their old save went. |

A save that cannot be *read* is not a save that is corrupt. A permission error, a path that is a directory, or a transient I/O failure means the file's contents are unknown — and moving aside or replacing a file we were merely unable to open would destroy a perfectly good save. Those failures stop startup with a plain explanation, leaving the file untouched.

The `Corrupt` policy is the one that earns its keep. Saves are human-readable JSON specifically so a broken one can be hand-rescued (HLD decision 6); a shell that overwrites it with a fresh game destroys the only artifact that made that promise real. Moving it aside costs a rename and preserves every option.

## Core API Dependencies (a cascade into `clowder-core`)

One read-only addition, for the same reason the navigation leaf needs its own: the rule belongs core-side, and duplicating it shell-side is how the two drift.

1. **The catch-up cap** — `CATCHUP_CAP_SECONDS` is private to the persistence module, but TUI-RUN-024 requires an in-process gap to be capped at exactly the same limit the load path applies. A shell-side copy of the number is a second source of truth for the one rule that must be identical on both paths; export it instead.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Clock source | Monotonic for accumulation, wall-clock checkpoint for suspend detection | Wall clock alone; monotonic alone | Wall-alone lets NTP and DST inject or rewind game time; monotonic-alone silently loses a suspended laptop's hours, breaking tenet 2 |
| Long catch-up | Time-budgeted ticks per frame, remainder banked | Run every owed tick in one frame; cap and discard the excess | A multi-minute frozen frame reads as a hang; discarding punishes absence |
| In-process gap cap | The same 365-day cap and `truncated` flag as the load path | No cap while running | One absence rule; the player should not be able to feel whether the process was running |
| Tick accounting | Integer ticks owed, fractional remainder banked separately | Floating-point tick counter | A large backlog in `f64` loses whole ticks to precision |
| Banked ticks across a save | Carried as an explicit count beside a present timestamp | Back-date the timestamp so the interval re-derives them | Re-deriving re-prices the backlog at whatever speed is persisted, losing or conjuring ticks by the ratio between the speed they were earned at and the speed they were saved at |
| Frame ordering | Commands apply, then ticks run | Ticks first, then commands | A key pressed during the previous wait is older than that wait; it should not have to sit out a catch-up burst |
| Input handling | Bounded drain per frame; resizes coalesce to the last | Drain everything ready | An unbounded flood starves the sim and the renderer |
| Unreadable save | Stop with an explanation; leave the file untouched | Treat as corrupt and move aside | Unreadable is not unparseable — the contents are unknown, and touching the file could destroy a good save |
| Concurrent sessions | Lock beside the save; live holder refuses, stale clears itself, override forces | Last-writer-wins; second instance read-only | Two sessions on one save is silent progress loss; read-only doubles the UX surface for a mistake, not a use case |
| Stale-lock policy | Clear automatically, keyed on process id *and* start time | Require manual clearing; trust the id alone | A crash must not lock a player out of their own game; ids are recycled, so an id alone turns an unrelated process into a lockout |
| Terminal restore | Drop guard + panic hook + signal handlers | Any one alone | Each covers an exit path the others structurally cannot |
| Signals | Handle SIGTERM/SIGHUP via a flag the loop reads | Ignore them; do the work in the handler | The pane will be killed; handler-side work is not async-signal-safe |
| Corrupt save | Move aside, never overwrite | Overwrite with a new game; refuse to start | Human-readable saves exist to be hand-rescued; destroying one is the worst available outcome |
| Autosave failure | Report once, keep playing, retry next cadence | Abort the session; retry in a tight loop | A transient disk problem should not end a session or spam the log |
| Concurrency | Single thread; synchronous save | Background save thread; async runtime | The save is small; a thread costs the determinism story for milliseconds |
| Load ordering | Load before entering raw mode | Enter raw mode first | A load failure must be able to print to an ordinary terminal |

## Open Questions & Future Decisions

### Resolved
1. ✅ **Two instances on one save** — a lock beside the save; a live holder refuses the second launch, a stale one clears itself, an explicit override forces past either. See *One Session Per Save*.
2. ✅ **The Journal is saved state.** Its retention and its home in the save format belong to the sim and persistence segments, not here. What it settles for runtime: there is no second file and no shell-side persistence format — runtime writes one save, as it already did.

### Deferred
1. Frame cadence — a fixed rate versus redrawing only on change. The shell sub-HLD defers this to measurement on a real pane; the loop shape above accommodates either, since the poll timeout is the only knob.
2. The exact autosave interval and the per-frame catch-up budget. Both are single constants; they want tuning against a real long game rather than a guess now.
3. Whether the suspend-detection tolerance needs to adapt to platforms whose monotonic clock *does* include suspended time — a platform where the two clocks agree needs no correction, and the comparison is already a no-op there.

## References

- Parent: `../tui-design.md` (loop shape, the shell-owns-wall-clock decision)
- Consumes: `clowder_core::{GameState::tick, load_string, read_save, write_save, default_save_path, LoadError, LoadedGame, OfflineReport}` and `content::TICKS_PER_SECOND`
- Siblings: `navigation/` delivers key meaning and owns the offline-report presentation (TUI-NAV-027); `render/` owns drawing within the frame this loop schedules
- Persistence intent (the catch-up cap, atomic write, and version policy this leaf drives): `../../persistence/persistence-design.md`
