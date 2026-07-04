---
parent: high-level-design
prefix: SAVE
---

# Persistence

## Context and Design Philosophy

Persistence turns the deterministic sim into a game that survives closing the terminal: versioned human-readable saves, atomic writes, and offline catch-up that replays the *same* `tick()` the live game runs (HLD: one simulation, no shadow math). The module lives in `clowder-core` — serialization, catch-up, and file IO with an injectable path — while the shell owns *when* to save and supplies the wall clock. The sim itself never reads clocks; time enters exactly here, at the boundary.

## Save Format

JSON via serde, pretty-printed (hand-rescuable with a text editor and `jq`):

```json
{
  "save_version": 1,
  "saved_at_unix": 1751628800,
  "state": { ... GameState ... }
}
```

- `save_version` — integer schema version. Loading a *newer* version than the binary knows fails with a clear error (never guess forward). Loading an *older* version walks migrations (v1: none exist; the hook is the point).
- `saved_at_unix` — wall-clock seconds at save, supplied by the caller; the basis for catch-up.
- Default location: the platform data directory (`directories` crate) + `clowder/save.json`; every IO function takes an explicit path so tests and the shell can redirect.

Load returns a typed result the shell dispatches on: the loaded state + offline report, or `LoadError::NotFound` (first run — start a new game), `LoadError::TooNew { found }`, `LoadError::Corrupt { detail }` (unparseable JSON, missing fields, or unknown content ids — content ids can't be guessed at). Core never deletes, overwrites, or "repairs" a save file on load — what to do with a corrupt file is the shell's (and player's) call. Numeric state in a *parseable* save is normalized on load instead of rejected: stocks clamp to caps, saturating counters clamp to range, a stale `zoomies_until` is inert by construction — the format is advertised as hand-editable, so hand-edited numbers get physics, not errors.

## Atomic Writes

Write to `save.json.tmp` in the same directory, fsync it, then rename over `save.json` — without the fsync, a power loss after the rename could leave an empty "new" file, which would make the atomicity claim a lie. A crash mid-save leaves the previous save intact; a leftover `.tmp` from a crashed write is ignored by load and overwritten by the next save. Load falls back to nothing else — one save slot in v1.

## Offline Catch-up

On load, the caller supplies `now_unix`:

1. `elapsed = max(0, now_unix − saved_at_unix)` — a backwards clock yields zero ticks, never a regressed state.
2. `elapsed` is capped at 365 days — beyond that, accrual stops representing "time you were away" and starts representing clock damage.
3. `ticks_to_replay = elapsed × 4` (engine tick rate), replayed through the standard `tick()` in a loop.
4. Events from the replay are folded into an **`OfflineReport`**: elapsed span (plus a `truncated` flag when the 365-day cap bit), per-resource *net* deltas, kittens arrived/wandered/returned as *gross* name lists (a kitten that left and came home appears in both — that's the drama the player missed), ideas completed, seasons passed — the shell renders this as the "while you were away" screen. Full per-tick event streams are not retained (a week is ~2.4M ticks).

Catch-up performance is a design constraint: replaying 30 days must feel like loading a game, not compiling one (spec'd as a bound in release mode). The tick function stays allocation-light on the hot path for this reason.

## What Is Not Saved

Derived values (happiness, rates, season) — recomputed from state. Event history — the shell's presentation concern. Anything about terminal or UI state.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Format | Pretty JSON | RON, TOML, bincode | HLD decision (human-readable); JSON wins on ubiquitous tooling (`jq`) for hand-rescue; bincode saves bytes nobody counts |
| Catch-up mechanism | Replay real `tick()` | Closed-form accrual formulas | One simulation, no shadow math (HLD); formulas drift from rules the moment rules get interesting |
| Elapsed cap | 365 days | Uncapped, 30 days | Uncapped invites clock-damage absurdity; 30 days punishes a long real absence (violates idle-respect); a year is generous to humans and hostile only to broken clocks |
| Backwards clock | Zero ticks, load anyway | Refuse to load, error | The player's game should never be hostage to NTP; zero-tick load is safe by determinism |
| Save slots | One, atomic-replace | Rotating backups, multiple slots | v1 simplicity; atomic write already prevents the corruption case backups mostly guard |

## Open Questions & Future Decisions

### Deferred
1. Chunked catch-up with progress reporting (shell callback) if 365-day replays prove slow on real hardware — measure first.
2. Save-file compression toggle if saves grow past ~1 MB with roadmap systems — measure first.

## References

- Parent: `docs/high-level-design.md` (human-readable saves decision, one-simulation decision, idle-respect tenet)
- Replays `tick()` from `docs/intent/sim/engine/`
