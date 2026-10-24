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
  "owed_ticks": 0,
  "report_so_far": null,
  "state": { ... GameState ... }
}
```

- `save_version` — integer schema version, currently **8**. Loading a *newer* version than the binary knows fails with a clear error (never guess forward). Loading an *older* version walks migrations in sequence. **v1 → v2**: fields added by the depth expansion (speed, and each wave's new state blocks) receive their new-game defaults; a v1 save loads into a valid v2 state with its meadow exactly as it was. **v2 → v3**: the event journal arrives empty — a save written before the journal existed holds no history to recover, and an empty journal is the honest representation of that. **v3 → v4**: the owed count arrives at zero (SAVE-017). **v4 → v5**: the report so far arrives empty — a save written before it existed was never written by a catch-up set down mid-replay. **v5 → v6**: the revealed-resource set arrives filled from what the save itself shows (Migrating Revealed Resources, below) — the one step that derives a field instead of defaulting it. **v6 → v7**: what each colony has shown the clowder (colonies leaf: What a Neighbour Has Shown You) arrives empty — no colony reached, no good learned. The journal's retained trades are not mined for it: they hold only the trades still in the rolling window, so a save would learn some goods and not others for no reason the player could see, and the honest start is none. **v7 → v8**: every kitten and every Family Album memorial arrives with a profile (Migrating Profiles, below) — the second step that derives a field rather than defaulting it. Migration fidelity is tested against a committed fixture file per prior version.
- `saved_at_unix` — wall-clock seconds at save, supplied by the caller; the basis for catch-up.
- `owed_ticks` — ticks the caller had banked but not yet run when it saved, replayed as counted at the next load (SAVE-017).
- `report_so_far` — the report of a catch-up set down before it finished (Paced Catch-up, below), or `null`. It is canonical for the same reason the journal is: it records what the player's meadow has already lived through, and nothing else can recompute it. Its arrival is a version step rather than a defaulted field so that an older binary refuses the save as too new instead of loading it and silently dropping the tale it carries.
- *An event that gains data.* A journal entry holds its event as the core's vocabulary had it when the entry was written. Where an event later carries more — a trade's goods and its napped caravans (colonies leaf: What comes home) — an entry written before loads with the added data at its default: nothing listed, nothing counted. This is not a version step. An older binary loading a newer save drops the added data unread, and what it loses is the detail of a past occurrence, which a display then states without; the meadow runs on none of it. The revealed set and the report so far are version steps because losing them changes the meadow or loses a whole tale.
- Default location: the platform data directory (`directories` crate) + `clowder/save.json`; every IO function takes an explicit path so tests and the shell can redirect.

Load returns a typed result the shell dispatches on: the parsed, migrated, and normalized state with its offline catch-up pending (Paced Catch-up, below), or `LoadError::NotFound` (first run — start a new game), `LoadError::TooNew { found }`, `LoadError::Corrupt { detail }` (unparseable JSON, missing fields, or unknown content ids — content ids can't be guessed at). Core never deletes, overwrites, or "repairs" a save file on load — what to do with a corrupt file is the shell's (and player's) call. Numeric state in a *parseable* save is normalized on load instead of rejected: numeric fields deserialize wide and clamp (a hand-edited `pity: 300` clamps rather than failing type-level deserialization), stocks clamp to caps, a stale `zoomies_until` is inert by construction — the format is advertised as hand-editable, so hand-edited numbers get physics, not errors. Structural oddities get physics too: an over-capacity roster simply blocks arrivals (free slots floor at zero), duplicate hand-edited names are kept, and a missing `away_seq` on an away kitten sorts lowest in the return queue.

## Migrating Revealed Resources

The state carries the set of resources the meadow has revealed (engine leaf: Unlocks & Visibility, Revealed resources). It is canonical: that a store was once held and has since been spent to nothing can be recomputed from nothing else in the state. Its arrival is a version step rather than a defaulted field, for the reason the report so far's is: an older binary would load the save, drop the set unread, and write the save back without it — un-revealing every store then standing at nothing — where a version step makes that binary refuse the save as too new.

A save written before the set existed is not given an empty one. An empty set would list every store the save holds — the engine's query answers for a holding whether or not it is recorded — but would leave out the stores its player has met and spent to nothing, and would lose a store that the first replayed tick emptied before any check had recorded it. So the step runs the engine's own reveal rule once, over the save's state as loaded and normalized and before any catch-up tick, and records what it finds:

- **Every resource the save holds.** A prior version listed exactly the stores with a stock above zero, so every store that version showed for the save is revealed, and migration hides nothing its player could see. Normalization comes first, so a hand-edited stock that is negative or not finite is a zero, not a holding.
- **Every resource named in a price the core would state for that state** — the price of an item it reports Teased or Available, owned or not, or of a command it would price. This is the save's evidence for a store met and since spent: a price still in view names it, and the price of a thing already bought still counts, the thing staying listed once it is the player's.

Nothing else is revealed. A store once held, spent to nothing, and named by no such price leaves no trace in the save; it reveals as any store does, the next time the meadow holds some. The rule applied is the engine's own and no second one: what a migrated save reveals is what the engine would record for that state after any applied command.

## Migrating Profiles

A kitten's profile — what it looks like and what it loves (kittens leaf: Profiles) — is canonical state, stored on each kitten and on each Family Album memorial. Its arrival is a version step for the reason the revealed set's is: an older binary would load the save, drop every profile unread, and write the save back without them.

A save written before profiles existed is not given empty or placeholder ones; a kitten with no face would be a quiet loss. Its kittens are given the profiles they would have drawn, by the kittens leaf's own profile draw, keyed as an arrival keys it, on each kitten's name and generation:

- **Each album memorial**, oldest generation first in album order, from its name and the generation it records.
- **Each rostered kitten**, in roster order, from its name and the save's current generation.
- **Except a kitten already memorialized.** A Move carries kittens along in Traveling Baskets and memorializes every kitten in the generation it leaves, so a carried kitten stands in the album under each generation it lived through and in the roster too. Where a memorial's or a rostered kitten's name equals the name of a memorial of the generation just before its own, it takes that memorial's profile instead of drawing one, so a kitten looks the same on every page of the album and at home. A kitten that merely shares a name with one the generation before is read as that kitten; nothing in a save written before profiles tells the two apart, and two namesakes in consecutive generations looking alike is a harmless likeness, where the same kitten looking different across the album would not be.

The draw takes nothing from the state RNG, so a migrated meadow's every later random outcome is what it would have been, and nothing else in the save changes. The same rule gives a profile to any kitten or memorial that a hand-edited save of the current version holds without one — physics, not an error, as for every structural oddity a hand-edit can leave.

## Atomic Writes

Write to `save.json.tmp` in the same directory, fsync it, then rename over `save.json` — without the fsync, a power loss after the rename could leave an empty "new" file, which would make the atomicity claim a lie. A crash mid-save leaves the previous save intact; a leftover `.tmp` from a crashed write is ignored by load and overwritten by the next save. Load falls back to nothing else — one save slot in v1.

## Offline Catch-up

On load, the caller supplies `now_unix`:

1. `elapsed = max(0, now_unix − saved_at_unix)` — a backwards clock yields zero ticks, never a regressed state.
2. `elapsed` is capped at 365 days — beyond that, accrual stops representing "time you were away" and starts representing clock damage.
3. `ticks_to_replay = elapsed × 4 × speed` (engine tick rate × the save's persisted game speed — away time accrues at exactly the rate attended time would have), plus the ticks the save carries as owed (SAVE-017), replayed through the core's replay entry point — the standard `tick()` in every respect but journaling — as the caller advances the catch-up (Paced Catch-up, below).
4. Events from the replay are folded into an **`OfflineReport`** — continuing the save's report so far when it carries one (Paced Catch-up, below): elapsed span (plus a `truncated` flag when the 365-day cap bit), per-resource *net* deltas, kittens arrived/wandered/returned as *gross* name lists aggregated with counts (a kitten that left and came home appears in both, possibly several times over a long famine — that's the drama the player missed, told without spam), and seasons passed — the shell renders this as the "while you were away" screen. Commands cannot occur during replay, so builds and research never appear in the report. Full per-tick event streams are not retained (a week is ~2.4M ticks).

Catch-up performance is a design constraint: replaying 30 days must feel like loading a game, not compiling one (spec'd as a bound in release mode). The tick function stays allocation-light on the hot path for this reason.

## Paced Catch-up

Load does not replay the absence before it returns. It parses, migrates, and normalizes the save, works out how many ticks the absence owes (steps 1–3 above), and hands the caller a **catch-up in progress**: the state as loaded, the count of ticks owed in total, the count still to replay, and the report folding so far. The caller advances it by a count of ticks it chooses per call, so a shell can spread a long absence over frames and show how far it has come; the core never reads a clock to decide how much to run, which keeps how far the meadow has advanced a property of the ticks rather than of the hardware.

How the replay is divided changes nothing it produces. The same ticks run in the same order through the same replay step, and the report folds the same events, whether the caller advances one tick at a time or all of them in one call. A one-call load is simply a catch-up advanced through its whole count at once, and it remains the load that tests and tools use.

A catch-up ends one of two ways:

- **Finished.** When an advance replays the last owed tick, the catch-up completes exactly as a one-call load would: the report completed, recorded in the journal under SAVE-015's minimum-interval rule, and handed back with the caught-up state. The result is bit-identical to a one-call load of the same save at the same `now_unix`.
- **Set down.** A caller that must stop early — the player quits mid-replay — sets the catch-up down. The core hands back the state so far, the count of ticks still owed, and the **report so far**: its span and `truncated` flag the catch-up's own, its contents folded from the ticks replayed so far. Nothing is recorded in the journal. The caller saves all three (SAVE-017, SAVE-021), so not one owed tick and not one replayed event of the tale is lost.

**One absence, one tale.** A load whose save carries a report so far continues it rather than starting a new one, so an absence interrupted any number of times is still told once, whole, by the load that finally finishes it:

- **Span.** The carried span plus this load's capped interval — the real time the absence covers. The moments spent on the waking screen before a quit are not added: their ticks are owed in the count, but the span they would add is seconds against an absence long enough to need waking.
- **Truncated.** Set where either the carried report or this load's interval was cut by the cap, so a capped year interrupted mid-wake still reads as more than its span.
- **Net deltas.** The carried deltas plus this replay's deltas against the state as loaded. Because a save round-trips its state exactly (SAVE-010), the sum is the replay's whole change across both loads, measured from the meadow as the absence found it; a hand-edit between the two loads falls between the two measurements and is never counted as something that happened while the player was away.
- **Names and seasons.** Name lists merge, a name's counts summed; seasons passed sum.
- **Retention.** SAVE-015's minimum-interval rule judges the continued report by its whole span. A player who quits mid-wake and relaunches a minute later still has the absence's report retained, because the absence it tells is long, though the last interval was short.

The finished report tells the absence an uninterrupted wake would have told, extended by the time between the quit and the load that finishes it. Ticks a save owes from a live backlog rather than from a set-down catch-up (SAVE-017) carry no report so far: they fold into the next load's report under that load's own span.

## What Is Not Saved

Derived values (happiness, rates, season) — recomputed from state. Event history — the shell's presentation concern. Anything about terminal or UI state.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Format | Pretty JSON | RON, TOML, bincode | HLD decision (human-readable); JSON wins on ubiquitous tooling (`jq`) for hand-rescue; bincode saves bytes nobody counts |
| Catch-up mechanism | Replay real `tick()` | Closed-form accrual formulas | One simulation, no shadow math (HLD); formulas drift from rules the moment rules get interesting |
| Elapsed cap | 365 days | Uncapped, 30 days | Uncapped invites clock-damage absurdity; 30 days punishes a long real absence (violates idle-respect); a year is generous to humans and hostile only to broken clocks |
| Backwards clock | Zero ticks, load anyway | Refuse to load, error | The player's game should never be hostage to NTP; zero-tick load is safe by determinism |
| Catch-up pacing | The caller advances a catch-up in progress by counts it chooses; load returns before replaying | Replay everything inside load; a core-side progress callback; a core-side wall-time budget per call | Replaying inside load holds the terminal blank for the whole absence, which reads as a hang; a callback inverts control for no gain over an advance the caller already drives; a time budget would put a clock in the core and make how far the meadow advances depend on the machine |
| A catch-up stopped early | Carry the report so far in the save beside the owed ticks; the load that replays the rest continues it, and only the finished report is recorded | Record the report so far in the journal at set-down and owe the rest; discard the report so far | Recording at set-down splits one absence across two entries, and the rest would be told under only its own load's interval: a quick relaunch would replay most of the absence below the minimum interval with no entry and no banner, and a later one would lead with a short span over a long absence's changes, its `truncated` flag lost. Discarding loses the tale of time already replayed, though its effects stand in the meadow. Carrying it costs a save field and a format version, small against telling every absence once and whole |
| What a prior save reveals of its resources | Its holdings, and the resources named in a price the core would state for it | An empty set, left for play to fill; every resource revealed; the retained reports' net changes read as holdings too | An empty set keeps the stores a player has spent to nothing missing until some arrives again, which is the reveal un-revealing for every existing save. Revealing everything names stores the meadow has never met. A retained report outlives a Move, so its changes could reveal in this meadow what only an earlier one held, and it speaks only for stores that moved across an absence long enough to be kept |
| Data an event gains | Defaulted on load; no version step | A version step for each event that gains data | The loss a step guards against here is the detail of a past occurrence that an older binary drops: no stock, reveal, or tale goes with it, and a step for every richer event would make every fuller line a migration with its own fixture |
| Save slots | One, atomic-replace | Rotating backups, multiple slots | v1 simplicity; atomic write already prevents the corruption case backups mostly guard |
| A prior save's kittens' profiles | Drawn as an arrival would draw them, from each kitten's name and generation, a kitten named in the album the generation before taking that memorial's profile | Every kitten given one fixed placeholder profile; profiles left empty until a kitten next does something; a draw from the state RNG at load | A placeholder makes every existing kitten identical, and an empty profile is a kitten with no face, both a quiet loss of the individuality profiles exist for (Tenet 1). A state-RNG draw at load would change every later random outcome in the migrated meadow and leave arrival and migration as two different draws |
| JSON float fidelity | `serde_json` with the `float_roundtrip` feature | Default `serde_json` float parsing | Without it the parser is off-by-one-ULP for some values, so `parse(serialize(x)) ≠ x` and the byte-stable round-trip (SAVE-010) silently fails on unlucky stock values; the feature makes serialize/parse exact inverses |

## Open Questions & Future Decisions

### Resolved
1. ✅ **A long catch-up's wait.** Measured, a seven-week absence replays in about 6 s on a release build and in minutes on a debug build, and the 365-day cap projects to roughly 40 s on release — long enough that a replay finished before the shell draws reads as a hang. The catch-up is paced instead (Paced Catch-up), and the runtime leaf replays it across frames behind a waking screen.

### Deferred
1. Save-file compression toggle if saves grow past ~1 MB with roadmap systems — measure first.

## References

- Parent: `docs/high-level-design.md` (human-readable saves decision, one-simulation decision, idle-respect tenet)
- Replays `tick()` from `docs/intent/sim/engine/`
