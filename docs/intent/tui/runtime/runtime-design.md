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
  load save (or start a new game)          ← before raw mode, so failures print normally;
                                             the absence it owes is not yet replayed
  enter raw mode + alternate screen
  install panic hook, drop guard, signal handlers

wake (a load that owes catch-up)
  replay one bounded count of the catch-up per frame, drawing the waking screen,
  until it finishes — or skip the screen when the first count finishes it
  take the opening, queue the load's report

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

**Catch-up is paced, never dropped.** A long gap can owe millions of ticks, and running them all in one frame is indistinguishable from a hang. Each frame runs at most a bounded count of owed ticks and banks the rest, so a big catch-up drains over several frames while the UI stays alive. The bound is a count, not a slice of wall time: a time budget would read a clock inside the drain and buy more ticks on a fast machine than a slow one, making how far the meadow advances a property of the hardware. Ticks owed are counted in integers, never floats, so a large backlog cannot lose precision.

**A save carries its banked ticks as a count.** Because ticks can be banked, the state in memory may be behind real time — mid-catch-up, deliberately. The timestamp is simply the present, and the ticks earned but not yet run are written beside it.

The tempting alternative is to back-date the timestamp so the interval re-derives them, and it is wrong in a way that takes a while to see: the reload reconstructs ticks by multiplying an interval by whatever speed is persisted, so a backlog earned at one speed and saved at another is re-priced by the ratio between them. Earn fast, save slow, and most of it evaporates; the other way round, it multiplies. What the player earned is theirs, so it is stored as what it is.

## Waking Up

A load's catch-up is the largest backlog the game ever owes — a closed laptop's week, a year at the cap — and it is replayed after the terminal is taken, not before. Load parses, migrates, and normalizes the save before raw mode, so every load failure still prints on an ordinary terminal; what it hands back is the state with its catch-up pending (persistence: Paced Catch-up). The shell then enters the terminal and wakes the meadow: each frame advances the catch-up by at most a bounded count of ticks and draws the waking screen (navigation leaf), until the catch-up finishes and the game opens. A player returning from a long absence sees the meadow waking, with its progress, rather than a terminal that has not yet appeared.

The game is **awake** from the moment it opens (navigation leaf: the opening): at once for a new game, which owes no catch-up, and for a load whose first bound finishes its catch-up; when the wake finishes, for any other load. Everything the loop does as play — its frame-budget wait, live ticks, autosave — belongs to the awake game; until then the shell is waking.

- **Short absences show nothing.** Before the first frame the shell advances one bound. Where that finishes the catch-up — a short absence, a backwards clock, no absence at all — the game opens directly and no waking screen is ever drawn; a screen that would flash for one frame tells the player nothing. Otherwise the first frame is the waking screen, drawn having replayed no more than one bound.
- **The bound is a count.** As with the live drain (Time, above), a count rather than a time budget, for the same reasons. It is its own count, larger than live play's: the waking screen has only its progress and the quit key to keep responsive, so a bound can take most of a frame's worth of a release build's time. Its value is a tuning constant (Deferred).
- **No waiting while waking.** The poll for input does not wait out the frame budget while waking — it checks for a key and moves on — so the replay runs at the machine's pace and frames are its only pauses. Waiting out the budget between bounds would stretch a wake far beyond the replay's own cost.
- **The replay comes first.** No live tick runs while waking. Real time keeps passing, and the ordinary accumulator observes it at the save's speed, suspend excess and the absence cap included, banking it as it would any interval; those ticks run as live ticks from the first frame of play, after every replayed tick. The ticks therefore run in the order they were earned — the absence, then the time spent waking — and the wait costs the player no game time.
- **The frame that finishes opens the game.** When an advance finishes the catch-up, that frame runs no live tick; the shell takes the opening (navigation leaf) and queues the load's retained report against the caught-up state, and the first frame of play follows. The opening is taken against exactly the state a one-call replay of the same save at the same moment would have produced, bit for bit; the first frame of play then runs the live ticks banked while waking, as any frame runs its banked ticks, before it draws.
- **No autosave while waking.** The save on disk still owes everything the wake has not finished, so a crash or a kill mid-wake loses nothing — the next load replays from that save, its interval reaching to the new moment. An autosave mid-wake would write a meadow halfway through its replay, protecting nothing the untouched save does not already protect. The cadence keeps counting from startup, so a wake longer than the interval autosaves on the first frame of play, securing the caught-up meadow promptly.
- **Quitting mid-wake.** Input while waking answers quitting alone (navigation leaf). A quit key or a termination signal sets the catch-up down and saves promptly, the save's owed count carrying the catch-up's remaining ticks together with any live ticks banked while waking, and the save carrying the core's report so far beside it (persistence: Paced Catch-up). The next load wakes through the rest and continues that report, so the player who comes back is told the whole absence once, retained and bannered by its whole span however soon they return. Shutdown never waits for a wake to finish, any more than it waits for a live backlog.

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

**Cadence.** Autosave runs on a wall-clock interval once the game is awake (Waking Up), and on every clean shutdown, one taken while waking included; it does not run while waking (Waking Up), the save on disk already owing everything the wake has not finished. The write itself is the core's atomic temp-file-plus-rename, so a save interrupted mid-write never corrupts the previous one.

**Shutdown does not wait for the sim to catch up.** A quit or a signal arriving mid-backlog or mid-wake saves and exits promptly rather than draining first — a player pressing `q` should not watch the game refuse to close while it works through a week of absence. The banked ticks are carried in the save as a count, a wake's unreplayed ticks among them, and simply run at the next load. Shutdown is the same operation whether the sim is current or a month behind.

**Failure is not fatal.** A failed autosave (full disk, revoked permissions, a data directory that vanished) reports itself once and the game keeps running — the player keeps playing, and the next cadence retries. Killing a healthy session because a disk was briefly full would destroy more progress than the failed write did.

**Save path.** `default_save_path()` returns an `Option`; a platform with no resolvable data directory falls back to a path beside the working directory, and the game says where it landed rather than refusing to start.

## One Session Per Save

A player who lives in tmux is well placed to start clowder twice by accident — a second pane, a forgotten session, a shell history recall. Two sessions autosaving the same path is silent progress loss: both hold a divergent state in memory and the later write simply erases the other's afternoon.

Startup therefore takes a lock beside the save file, holding the owning process's id and start time. The lock is taken **before the save is read** — it is the mutual-exclusion primitive, and acquiring it after the load leaves a window in which two launches both read the same save and both believe they are alone. Every path that then refuses to start releases it, so a refusal never leaves a lock for the next launch to inherit. A second launch that finds a **live** holder refuses to start, says which process holds the game, and names the override — one session per save, enforced rather than hoped for.

A **stale** lock is cleared automatically and play continues. A lock outliving its process means clowder crashed or was killed, and the player's response to reopening their game should not be a lockout over a file their previous session failed to clean up. Recording the start time alongside the process id is what makes "stale" decidable: an id alone can be recycled by an unrelated process, and refusing to start because some other program inherited a number would be the same lockout by a different route. The explicit override force-clears even a live lock, for the case the checks get wrong.

**One launch decides at a time.** Deciding whether the save is free is several steps — read the lock, judge it stale, clear it, write a new one — and two launches interleaving them can each clear what the other has just written and both come away holding the save. So a launch makes the whole decision under an operating-system advisory lock on the save's directory, held only until the decision is made. The kernel grants it to one launch at a time and releases it when that launch returns or dies, so it can never itself go stale, and taking it on the directory leaves nothing beside the save but the session lock. Where a filesystem cannot grant it, the launch decides unguarded: a player is never refused their own game over a guard.

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

Two additions, for the same reason the navigation leaf needs its own: the rule belongs core-side, and duplicating it shell-side is how the two drift.

1. **The catch-up cap** — `CATCHUP_CAP_SECONDS` is private to the persistence module, but TUI-RUN-024 requires an in-process gap to be capped at exactly the same limit the load path applies. A shell-side copy of the number is a second source of truth for the one rule that must be identical on both paths; export it instead.
2. **The paced catch-up** — a load that returns its catch-up pending, advanced by counts the shell chooses, reporting its total and remaining ticks, and finished or set down by the core (SAVE-018, SAVE-019, SAVE-020); and a save write that carries a set-down catch-up's report so far beside its owed count (SAVE-021), which the next load continues (SAVE-022). Waking needs the replay to stay the core's: the shell drives the pace and reads the progress, and never runs replay ticks or folds a report itself.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Clock source | Monotonic for accumulation, wall-clock checkpoint for suspend detection | Wall clock alone; monotonic alone | Wall-alone lets NTP and DST inject or rewind game time; monotonic-alone silently loses a suspended laptop's hours, breaking tenet 2 |
| Long catch-up | A bounded count of ticks per frame, remainder banked | Run every owed tick in one frame; a wall-time budget per frame; cap and discard the excess | A multi-minute frozen frame reads as a hang; a time budget makes how far the meadow advances depend on the machine; discarding punishes absence |
| A load's catch-up | Replayed after the terminal is taken, a bounded count per frame behind a waking screen | Replay it all before the first frame; replay it on a background thread; draw the game's frame live as the replay advances | Replaying first holds a blank terminal for up to the whole absence; a thread costs the single-threaded determinism story; a live game frame shows tabs and numbers racing through states the player cannot act on, and the opening would have to judge tabs it watched arrive |
| Time spent waking | Banked at the save's speed and run as live ticks after the replay | Not counted; counted into the replay | Not counting it costs the player game time for the wait; folding it into the replay would change the replayed ticks and break bit-identity with a one-call load |
| Autosave while waking | None; the save on disk still owes the unfinished wake | Autosave on cadence mid-wake | An autosave mid-wake writes a meadow halfway through its replay, protecting nothing the untouched save does not already protect, and costs a set-down and a save on frames the wake would rather spend replaying |
| In-process gap cap | The same 365-day cap and `truncated` flag as the load path | No cap while running | One absence rule; the player should not be able to feel whether the process was running |
| Tick accounting | Integer ticks owed, fractional remainder banked separately | Floating-point tick counter | A large backlog in `f64` loses whole ticks to precision |
| Banked ticks across a save | Carried as an explicit count beside a present timestamp | Back-date the timestamp so the interval re-derives them | Re-deriving re-prices the backlog at whatever speed is persisted, losing or conjuring ticks by the ratio between the speed they were earned at and the speed they were saved at |
| Frame ordering | Commands apply, then ticks run | Ticks first, then commands | A key pressed during the previous wait is older than that wait; it should not have to sit out a catch-up burst |
| Input handling | Bounded drain per frame; resizes coalesce to the last | Drain everything ready | An unbounded flood starves the sim and the renderer |
| Unreadable save | Stop with an explanation; leave the file untouched | Treat as corrupt and move aside | Unreadable is not unparseable — the contents are unknown, and touching the file could destroy a good save |
| Concurrent sessions | Lock beside the save; live holder refuses, stale clears itself, override forces | Last-writer-wins; second instance read-only | Two sessions on one save is silent progress loss; read-only doubles the UX surface for a mistake, not a use case |
| Simultaneous launches | The launch's decision is made under an advisory lock on the save's directory, held only for the decision | Hold an advisory lock on the lock file for the whole session, with no recorded holder; rely on exclusive creation of the lock file alone | A kernel lock held for the session cannot name its holder to the launch it refuses, and removing its file on exit reopens the race; exclusive creation makes taking a free save atomic but not clearing a stale one, where each launch clears what it read a moment ago and removes a lock another has just taken |
| Stale-lock policy | Clear automatically, keyed on process id *and* start time | Require manual clearing; trust the id alone | A crash must not lock a player out of their own game; ids are recycled, so an id alone turns an unrelated process into a lockout |
| Terminal restore | Drop guard + panic hook + signal handlers | Any one alone | Each covers an exit path the others structurally cannot |
| Signals | Handle SIGTERM/SIGHUP via a flag the loop reads | Ignore them; do the work in the handler | The pane will be killed; handler-side work is not async-signal-safe |
| Corrupt save | Move aside, never overwrite | Overwrite with a new game; refuse to start | Human-readable saves exist to be hand-rescued; destroying one is the worst available outcome |
| Autosave failure | Report once, keep playing, retry next cadence | Abort the session; retry in a tight loop | A transient disk problem should not end a session or spam the log |
| Concurrency | Single thread; synchronous save | Background save thread; async runtime | The save is small; a thread costs the determinism story for milliseconds |
| Load ordering | Parse, migrate, and normalize before entering raw mode; replay after | Enter raw mode first; replay before raw mode too | A load failure must be able to print to an ordinary terminal; the replay cannot fail, and run before the terminal is taken it is a blank wait |

## Open Questions & Future Decisions

### Resolved
1. ✅ **Two instances on one save** — a lock beside the save; a live holder refuses the second launch, a stale one clears itself, an explicit override forces past either. See *One Session Per Save*.
2. ✅ **The Journal is saved state.** Its retention and its home in the save format belong to the sim and persistence segments, not here. What it settles for runtime: there is no second file and no shell-side persistence format — runtime writes one save, as it already did.

### Deferred
1. Frame cadence — a fixed rate versus redrawing only on change. The shell sub-HLD defers this to measurement on a real pane; the loop shape above accommodates either, since the poll timeout is the only knob.
2. The exact autosave interval and the two per-frame tick bounds — live play's backlog drain and waking's. Each is a single constant; they want tuning against a real long game rather than a guess now.
3. Whether the suspend-detection tolerance needs to adapt to platforms whose monotonic clock *does* include suspended time — a platform where the two clocks agree needs no correction, and the comparison is already a no-op there.

## References

- Parent: `../tui-design.md` (loop shape, the shell-owns-wall-clock decision)
- Consumes: `clowder_core::{GameState::tick, load_string, read_save, write_save, default_save_path, LoadError, LoadedGame, OfflineReport, CATCHUP_CAP_SECONDS}` and `content::TICKS_PER_SECOND`; and the paced catch-up the load hands back — advance by a count, total and remaining ticks, finish, set down — with the save write that carries a report so far (Core API Dependencies 2; SAVE-018 to SAVE-022)
- Siblings: `navigation/` delivers key meaning and owns the offline-report presentation (TUI-NAV-027) and the waking screen (TUI-NAV-102); `render/` owns drawing within the frame this loop schedules
- Persistence intent (the catch-up cap, atomic write, and version policy this leaf drives): `../../persistence/persistence-design.md`
