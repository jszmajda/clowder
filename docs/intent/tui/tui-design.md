---
parent: high-level-design
prefix: TUI
---

# TUI Shell (sub-HLD)

## Context and Design Philosophy

`clowder-tui` is the ratatui binary — the only frontend clowder ships. It owns everything between the terminal and the deterministic `clowder-core`: terminal lifecycle, the input→command mapping, the render loop, the screens and widgets, the truecolor theme, autosave, and the speed-control surface. It contains **no game rules** — every player action becomes a `clowder_core::Command`, and every pixel is a render of a `GameState` snapshot or a derived query. This keeps the split the HLD ratified: the sim stays headless-testable, and the shell stays a thin, replaceable window.

The shell's guiding constraint is the HLD's tmux-first keyboard model plus idle-respect: the game lives in a tmux pane for weeks, is played entirely from tmux-safe keys, and must be *glanceable* — a two-second look answers "does anything need me?" without ever demanding attention.

## Crate & Loop Shape

`clowder-tui` is a binary crate in the workspace, depending on `clowder-core`. Its top level is a single-threaded event loop:

```
loop {
    // 1. drain input (crossterm events, non-blocking poll with a frame timeout)
    // 2. on each key: map to an Action (nav or a Command); apply Commands to GameState
    // 3. advance the sim: run core.tick() the right number of times for elapsed
    //    wall-time × speed (accumulator; catch-up on a slow frame)
    // 4. autosave if the cadence elapsed
    // 5. render one frame from the current snapshot + derived queries
}
```

The loop is the sim's clock at the boundary (the HLD's "shell drives ticks" decision): it reads the wall clock *here only*, converts elapsed real-time × `speed` into a tick count, and calls `tick()`. `clowder-core` never sees a clock. On startup the shell loads the save and shows the offline report; on quit it saves.

## Child Leaves

| Leaf | Prefix | Owns |
|---|---|---|
| `navigation/` | `TUI-NAV` | The input model, key map, section/tab navigation, the action model (select/act/repeat/bulk/confirm), the inspector, glanceability chrome, discoverability, and the event-log/offline-report surfacing — the whole UX grammar |
| `theme/` (later) | `TUI-THEME` | The truecolor palette, per-tier resource colors, the CVD-safe glyph+color channel discipline, light/dark posture |
| `render/` (later) | `TUI-RENDER` | Widget layout, the responsive breakpoints, per-screen rendering, the frame budget |
| `runtime/` (later) | `TUI-RUN` | Terminal lifecycle (raw mode, alt screen, panic-safe restore), the event loop, the tick accumulator, autosave cadence, save-path resolution |

Only `navigation` is designed now (the UX is the part the player lives in, and it constrains everything else). The others are created when built.

## Load-Bearing Decisions

- **Single-threaded event loop, no async.** A sequential sim with a keyboard and a screen needs no tasks or channels; a poll-with-timeout loop is deterministic, trivially correct, and matches the HLD's rejection of the async architecture.
- **The shell owns wall-clock time; the sim owns none.** Elapsed real-time × speed → tick count, at the loop boundary. This is what makes live play and offline catch-up the same simulation.
- **Render from snapshots + derived queries, never from shell-side game state.** The shell caches nothing the core owns; if it needs a number the core computes, it asks the core (see the navigation LLD's core-API dependencies). This keeps rule knowledge out of the shell.
- **Panic-safe terminal restore.** Any panic must restore the terminal (cooked mode, main screen) before unwinding, or a crash leaves the user's terminal wrecked — a hard requirement for a program that lives in a long-running pane.

## Decisions & Alternatives

| Decision | Chosen | Alternatives | Rationale |
|---|---|---|---|
| Concurrency | Single-threaded poll loop | tokio tasks + channels | Deterministic, minimal; the sim is sequential (HLD decision 1) |
| Time source | Shell converts wall-time × speed to ticks at the loop | Sim reads a clock; a timer thread | One simulation, no shadow math (HLD decision 2 & 7) |
| Shell state | Renders core snapshots + queries; owns only view state (cursor, open overlay, armed row) | Shell mirrors game state for speed | Mirroring risks divergence from the one source of truth |

## Open Questions & Future Decisions

### Deferred
1. Theme palette, render breakpoints, and runtime lifecycle — their own leaves when built.
2. Whether the render loop needs a dirty-frame optimization (only redraw on change) or a fixed ~10–20 fps is fine — measure on a real pane.

## References

- Parent: `docs/high-level-design.md` (thin-shell decision, tmux-first keyboard, idle-respect, truecolor)
- Child: `navigation/` (the UX grammar); consumes `clowder-core` commands, events, and derived queries
