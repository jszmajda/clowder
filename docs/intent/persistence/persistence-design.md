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

- `save_version` — integer schema version, currently **3**. Loading a *newer* version than the binary knows fails with a clear error (never guess forward). Loading an *older* version walks migrations in sequence. **v1 → v2**: fields added by the depth expansion (speed, and each wave's new state blocks) receive their new-game defaults; a v1 save loads into a valid v2 state with its meadow exactly as it was. **v2 → v3**: the event journal arrives empty — a save written before the journal existed holds no history to recover, and an empty journal is the honest representation of that. Migration fidelity is tested against a committed fixture file per prior version.
- `saved_at_unix` — wall-clock seconds at save, supplied by the caller; the basis for catch-up.
- Default location: the platform data directory (`directories` crate) + `clowder/save.json`; every IO function takes an explicit path so tests and the shell can redirect.

Load returns a typed result the shell dispatches on: the loaded state + offline report, or `LoadError::NotFound` (first run — start a new game), `LoadError::TooNew { found }`, `LoadError::Corrupt { detail }` (unparseable JSON, missing fields, or unknown content ids — content ids can't be guessed at). Core never deletes, overwrites, or "repairs" a save file on load — what to do with a corrupt file is the shell's (and player's) call. Numeric state in a *parseable* save is normalized on load instead of rejected: numeric fields deserialize wide and clamp (a hand-edited `pity: 300` clamps rather than failing type-level deserialization), stocks clamp to caps, a stale `zoomies_until` is inert by construction — the format is advertised as hand-editable, so hand-edited numbers get physics, not errors. Structural oddities get physics too: an over-capacity roster simply blocks arrivals (free slots floor at zero), duplicate hand-edited names are kept, and a missing `away_seq` on an away kitten sorts lowest in the return queue.

## Atomic Writes

Write to `save.json.tmp` in the same directory, fsync it, then rename over `save.json` — without the fsync, a power loss after the rename could leave an empty "new" file, which would make the atomicity claim a lie. A crash mid-save leaves the previous save intact; a leftover `.tmp` from a crashed write is ignored by load and overwritten by the next save. Load falls back to nothing else — one save slot in v1.

## Offline Catch-up

On load, the caller supplies `now_unix`:

1. `elapsed = max(0, now_unix − saved_at_unix)` — a backwards clock yields zero ticks, never a regressed state.
2. `elapsed` is capped at 365 days — beyond that, accrual stops representing "time you were away" and starts representing clock damage.
3. `ticks_to_replay = elapsed × 4 × speed` (engine tick rate × the save's persisted game speed — away time accrues at exactly the rate attended time would have), replayed through the standard `tick()` in a loop.
4. Events from the replay are folded into an **`OfflineReport`**: elapsed span (plus a `truncated` flag when the 365-day cap bit), per-resource *net* deltas, kittens arrived/wandered/returned as *gross* name lists aggregated with counts (a kitten that left and came home appears in both, possibly several times over a long famine — that's the drama the player missed, told without spam), and seasons passed — the shell renders this as the "while you were away" screen. Commands cannot occur during replay, so builds and research never appear in the report. Full per-tick event streams are not retained (a week is ~2.4M ticks).

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
| JSON float fidelity | `serde_json` with the `float_roundtrip` feature | Default `serde_json` float parsing | Without it the parser is off-by-one-ULP for some values, so `parse(serialize(x)) ≠ x` and the byte-stable round-trip (SAVE-010) silently fails on unlucky stock values; the feature makes serialize/parse exact inverses |

## Open Questions & Future Decisions

### Deferred
1. Chunked catch-up with progress reporting (shell callback) if 365-day replays prove slow on real hardware — measure first.
2. Save-file compression toggle if saves grow past ~1 MB with roadmap systems — measure first.

## References

- Parent: `docs/high-level-design.md` (human-readable saves decision, one-simulation decision, idle-respect tenet)
- Replays `tick()` from `docs/intent/sim/engine/`
