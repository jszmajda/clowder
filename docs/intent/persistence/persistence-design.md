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
  "save_version": 11,
  "saved_at_unix": 1751628800,
  "owed_ticks": 0,
  "report_so_far": null,
  "state": { ... GameState ... }
}
```

- `save_version` — integer schema version, currently **11**. Loading a *newer* version than the binary knows fails with a clear error (never guess forward). Loading an *older* version walks migrations in sequence. **v1 → v2**: fields added by the depth expansion (speed, and each wave's new state blocks) receive their new-game defaults; a v1 save loads into a valid v2 state with its meadow exactly as it was. **v2 → v3**: the event journal arrives empty — a save written before the journal existed holds no history to recover, and an empty journal is the honest representation of that. **v3 → v4**: the owed count arrives at zero (SAVE-017). **v4 → v5**: the report so far arrives empty — a save written before it existed was never written by a catch-up set down mid-replay. **v5 → v6**: the revealed-resource set arrives filled from what the save itself shows (Migrating Revealed Resources, below) — the one step that derives a field instead of defaulting it. **v6 → v7**: what each colony has shown the clowder (colonies leaf: What a Neighbour Has Shown You) arrives empty — no colony reached, no good learned. The journal's retained trades are not mined for it: they hold only the trades still in the rolling window, so a save would learn some goods and not others for no reason the player could see, and the honest start is none. **v7 → v8**: every kitten and every Family Album memorial arrives with a profile (Migrating Profiles, below) — the second step that derives a field rather than defaulting it. **v8 → v9**: every profile is drawn again, its fur and eyes coming out as they were (Migrating Profiles, below). **v9 → v10**: the achievement record arrives empty; the achievements leaf's state checks credit, as the meadow plays on, everything it already holds (achievements leaf: Earning). A catch-up also answers which achievements the record held as its absence began, before any tick replayed, so a frontend can tell what an absence earned (navigation leaf: The Achievements Overlay; One Absence, One Tale, below). **v10 → v11**: no field changes; the step marks a save as one that may have reached a surface before the idea that now gates it (A Prior Save's Surfaces). Migration fidelity is tested against a committed fixture file per prior version.
- `saved_at_unix` — wall-clock seconds at save, supplied by the caller; the basis for catch-up.
- `owed_ticks` — ticks the caller had banked but not yet run when it saved, replayed as counted at the next load (SAVE-017).
- `report_so_far` — the report of a catch-up set down before it finished (Paced Catch-up, below), or `null`. It is canonical for the same reason the journal is: it records what the player's meadow has already lived through, and nothing else can recompute it. Its arrival is a version step rather than a defaulted field so that an older binary refuses the save as too new instead of loading it and silently dropping the tale it carries.
- *An event or a report that gains data.* A journal entry holds its event or its report as the core's vocabulary had it when the entry was written. Where an event later carries more — a trade's goods and its napped caravans (colonies leaf: What comes home) — an entry written before loads with the added data at its default: nothing listed, nothing counted. This is not a version step. An older binary loading a newer save drops the added data unread, and what it loses is the detail of a past occurrence, which a display then states without; the meadow runs on none of it. The revealed set and the report so far are version steps because losing them changes the meadow or loses a whole tale.
- Default location: the platform data directory (`directories` crate) + `clowder/save.json`; every IO function takes an explicit path so tests and the shell can redirect.

Load returns a typed result the shell dispatches on: the parsed, migrated, and normalized state with its offline catch-up pending (Paced Catch-up, below), or `LoadError::NotFound` (first run — start a new game), `LoadError::TooNew { found }`, `LoadError::Corrupt { detail }` (unparseable JSON, missing fields, or unknown content ids — content ids can't be guessed at). Core never deletes, overwrites, or "repairs" a save file on load — what to do with a corrupt file is the shell's (and player's) call. Numeric state in a *parseable* save is normalized on load instead of rejected: an integer field reads the number it holds as the nearest value it can hold (a hand-edited `pity: 300` reads as 255, a `-1` as 0, a `2.5` as 2), floating-point fields floor at zero and stay finite, stocks clamp to caps, a stale `zoomies_until` is inert by construction — the format is advertised as hand-editable, so hand-edited numbers get physics, not errors. Structural oddities get physics too: an over-capacity roster simply blocks arrivals (free slots floor at zero), duplicate hand-edited names are kept, and a missing `away_seq` on an away kitten sorts lowest in the return queue.

**One reader for every integer.** A load reads the parsed save through a reader of its own, which answers every request for an integer with the nearest value that integer's type can hold: a negative reads as zero, a fraction rounds down, and one too large reads as the type's greatest. The rule lives in the reader rather than on each field, so the journal's events, a report's counts, and a field added later all hold it without anyone remembering to ask. Something that is not a number where a number belongs is still Corrupt, since a string has no nearest integer.

Three readings narrow that:

- **The version names a format.** `save_version` is judged on the raw number before the reader runs (SAVE-002), so one that is not a whole number is Corrupt rather than read as the nearest version.
- **An Elder index names a position.** A negative one is no Elder, as one past the roster is. The nearest position would seat the first kitten in a chair nobody asked it to take.
- **What keeps counting keeps room.** Clocks and sequence stamps read no higher than half their range, and copies and the other small counts no higher than `i32::MAX`, which no meadow reaches by playing. At the top of a range the next addition would overflow, and a next copy's price raises its ratio to a signed power of the copies held.

An owed tick count too large is read as the greatest and honoured: it was the save's own claim, the wake stays paced, and a player who sets it down can edit it again.

**What a save must hold.** A save lacking a field every version of the format has written is Corrupt: `save_version`, `saved_at_unix`, and `state`; within the state its clock (`ticks`), `rng`, `resources`, `structures`, `kittens`, `ideas`, `pity`, `famine_days`, `recovery_days`, `grumpy`, `next_seq`, and `zoomies_until`; within a kitten its `name`, `location`, and `arrived_day`; within a memorial its `name` and `trait_id`. Every other field takes its new-game value when missing, in a save of any version. A save cannot show whether it was written before a field existed or edited since, and a deleted line gets physics, as a mistyped number does.

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

The draw takes nothing from the state RNG, so a migrated meadow's every later random outcome is what it would have been, and nothing else in the save changes.

**A v8 profile is drawn again.** Format v8 stored profiles whose favourites were the same six kinds on every kitten, with no pose (kittens leaf: Kept for life). Loading a v8 save gives each kitten and memorial the profile the rule above gives one that holds none, keeping the fur and eyes its stored profile held: those are what the player recognises a kitten by, and they stay exactly as they were, while its favourites and its pose are drawn as an arrival draws them now. (The profile draw takes the fur and eyes first and draws them as it always has, so for every kitten a v8 binary drew, the two agree anyway.) The v8 profile has no part an older binary would keep that a v9 binary could not draw again, and it is a version step because the profile's shape changed: a v8 binary handed a v9 profile could not read it, and refuses the save as too new instead. The same rule gives a profile to any kitten or memorial that a hand-edited save of the current version holds without one — physics, not an error, as for every structural oddity a hand-edit can leave.

## A Prior Save's Surfaces

A version may start gating a surface on an idea that the surface used to open without, so a save can arrive already past the new gate (curiosity leaf: What an Idea Unlocks). Nothing is taken from a player for that. A save from before version 11 is granted each such idea it could already reach, free: Scout Picnicking where it holds Gift Economy, Dream Tending where it holds Parade Routes, and Hushfelt Seeking where it has reached The Brook. The grant is what researching would have recorded, the idea held and revealed, with no price paid and no event raised, and nothing else in the save changes. The Dream Loom needs no grant: it now opens earlier than it did, and a save that could reach it holds the idea that opens it.

## Atomic Writes

Write to `save.json.tmp` in the same directory, fsync it, then rename over `save.json` — without the fsync, a power loss after the rename could leave an empty "new" file, which would make the atomicity claim a lie. A crash mid-save leaves the previous save intact; a leftover `.tmp` from a crashed write is ignored by load and overwritten by the next save. Load falls back to nothing else — one save slot in v1.

## Offline Catch-up

On load, the caller supplies `now_unix`:

1. `elapsed = max(0, now_unix − saved_at_unix)` — a backwards clock yields zero ticks, never a regressed state.
2. `elapsed` is capped at 365 days — beyond that, accrual stops representing "time you were away" and starts representing clock damage.
3. `ticks_to_replay = elapsed × 4 × speed` (engine tick rate × the save's persisted game speed — away time accrues at exactly the rate attended time would have), plus the ticks the save carries as owed (SAVE-017), replayed through the core's replay entry point — the standard `tick()` in every respect but journaling — as the caller advances the catch-up (Paced Catch-up, below).
4. Events from the replay are folded into an **`OfflineReport`** — continuing the save's report so far when it carries one (Paced Catch-up, below): elapsed span (plus a `truncated` flag when the 365-day cap bit), per-resource *net* deltas, kittens arrived/wandered/returned as *gross* name lists aggregated with counts (a kitten that left and came home appears in both, possibly several times over a long famine — that's the drama the player missed, told without spam), seasons passed, and the achievements earned — the shell renders this as the "while you were away" screen. Commands cannot occur during replay, so builds and research never appear in the report. Full per-tick event streams are not retained (a week is ~2.4M ticks).

## What a Catch-up May Cost

Catch-up performance is a design constraint: a long absence must feel like loading a game, not compiling one. What a replay costs depends on the meadow, so the bound names one. A tick walks every structure kind held, every kitten, and every effect source that applies, so one that has reached everything costs several times what an early one does; and an early meadow has its own costs, a long list of things not yet revealed being scanned each tick. Each is bounded on a release build, over 30 days at the default speed, which is 10,368,000 ticks:

- **An early meadow, under 10 seconds** (SAVE-009): a handful of structures and the few kittens they house. An absence this short of play should hardly show.
- **A late meadow, under 60 seconds** (SAVE-033): twenty copies of every structure kind, every refinement, house rule, rite, arch upgrade, tradition, and dream loom upgrade it can hold, and as many kittens as those copies house, about two hundred. Traditions arrive only after a Move empties the structures, so ordinary play holds less than this at any one time, and the bound stands above late play rather than at it. A minute is long for a load, and the waking screen is what makes it bearable (Paced Catch-up); ordinary absences cost far less, a night away taking under a second and a week about six.

**Speed scales the wait, not the bound.** Speed changes how many ticks an absence owes, never what a tick costs (engine leaf: Game Speed), so both bounds hold at the default speed and an absence at any other speed takes time in proportion to it: at the fastest, 30 days owes eight times the ticks and takes about eight times as long. A player who turned the speed up asked for more game per hour, and the wake stays paced, so a longer one still shows its progress and can be set down. For the same reason the 365-day cap lies outside the bounds.

**Headroom.** Each bound stands at more than twice what its meadow measures, so a guard fails on a real regression rather than on a busy machine. A guard run alone, against a build of the same minute, is what tells a small per-tick cost from noise; the bound itself is for what a player would notice.

The tick function stays allocation-free on the hot path for this reason, and an effect sum reads only the sources that can carry its key (engine leaf: Effect-Key System).

## Paced Catch-up

Load does not replay the absence before it returns. It parses, migrates, and normalizes the save, works out how many ticks the absence owes (steps 1–3 above), and hands the caller a **catch-up in progress**: the state as loaded, the count of ticks owed in total, the count still to replay, and the report folding so far. The caller advances it by a count of ticks it chooses per call, so a shell can spread a long absence over frames and show how far it has come; the core never reads a clock to decide how much to run, which keeps how far the meadow has advanced a property of the ticks rather than of the hardware.

How the replay is divided changes nothing it produces. The same ticks run in the same order through the same replay step, and the report folds the same events, whether the caller advances one tick at a time or all of them in one call. A one-call load is simply a catch-up advanced through its whole count at once, and it remains the load that tests and tools use.

A catch-up ends one of two ways:

- **Finished.** When an advance replays the last owed tick, the catch-up completes exactly as a one-call load would: the report completed, recorded in the journal under SAVE-015's minimum-interval rule, and handed back with the caught-up state. The result is bit-identical to a one-call load of the same save at the same `now_unix`.
- **Set down.** A caller that must stop early — the player quits mid-replay — sets the catch-up down. The core hands back the state so far, the count of ticks still owed, and the **report so far**: its span and `truncated` flag the catch-up's own, its contents folded from the ticks replayed so far. Nothing is recorded in the journal. The caller saves all three (SAVE-017, SAVE-021), so not one owed tick and not one replayed event of the tale is lost.

**An absence the caller saw.** A caller that stayed running across an absence hands the core its state, the ticks owed, and the span with its truncation, and gets a catch-up in progress from it, with no save read and no migration. It advances, finishes, and sets down as a load's does, and tells its report the same way. A further absence may join a catch-up in progress, adding its ticks and its span, so that however a wake is interrupted the absence is told once. The core reads no clock here either: the caller measured the span, against the same cap (SAVE-016).

**One absence, one tale.** A load whose save carries a report so far continues it rather than starting a new one, so an absence interrupted any number of times is still told once, whole, by the load that finally finishes it:

- **Span.** The carried span plus this load's capped interval — the real time the absence covers. The moments spent on the waking screen before a quit are not added: their ticks are owed in the count, but the span they would add is seconds against an absence long enough to need waking.
- **Truncated.** Set where either the carried report or this load's interval was cut by the cap, so a capped year interrupted mid-wake still reads as more than its span.
- **Net deltas.** The carried deltas plus this replay's deltas against the state as loaded. Because a save round-trips its state exactly (SAVE-010), the sum is the replay's whole change across both loads, measured from the meadow as the absence found it; a hand-edit between the two loads falls between the two measurements and is never counted as something that happened while the player was away.
- **Names and seasons.** Name lists merge, a name's counts summed; seasons passed sum.
- **Achievements.** The carried list runs on into this replay's, and the record the catch-up answers as its absence's start is the record as loaded less the carried list. A save set down mid-wake has already written those achievements into its record, so without this a quit would turn what the absence earned into what the player already knew, and nothing would ever tell them. The list is empty on nearly every report, so a report that earned nothing writes no field for it.
- **Retention.** SAVE-015's minimum-interval rule judges the continued report by its whole span. A player who quits mid-wake and relaunches a minute later still has the absence's report retained, because the absence it tells is long, though the last interval was short.

The finished report tells the absence an uninterrupted wake would have told, extended by the time between the quit and the load that finishes it. Ticks a save owes from a live backlog rather than from a set-down catch-up (SAVE-017) carry no report so far: they fold into the next load's report under that load's own span.

## What Is Not Saved

Derived values (happiness, rates, season) — recomputed from state. Events beyond what the journal keeps (SAVE-011) — the shell's presentation concern. Anything about terminal or UI state.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Format | Pretty JSON | RON, TOML, bincode | HLD decision (human-readable); JSON wins on ubiquitous tooling (`jq`) for hand-rescue; bincode saves bytes nobody counts |
| Catch-up mechanism | Replay real `tick()` | Closed-form accrual formulas | One simulation, no shadow math (HLD); formulas drift from rules the moment rules get interesting |
| Elapsed cap | 365 days | Uncapped, 30 days | Uncapped invites clock-damage absurdity; 30 days punishes a long real absence (violates idle-respect); a year is generous to humans and hostile only to broken clocks |
| Backwards clock | Zero ticks, load anyway | Refuse to load, error | The player's game should never be hostage to NTP; zero-tick load is safe by determinism |
| What the catch-up bound covers | Two meadows, early and late, each over 30 days at the default speed; a wait at any other speed in proportion to it | One bound for every meadow at 5 seconds; one bound for every meadow and speed; a replay rate per tick | Measured on a real late save the replay took over 200 seconds and its cheapest honest form still takes over 20, so one 5-second bound promises what replaying every tick cannot keep, and closed-form replay is the shadow math the HLD rejects. One number for every speed would be set by the fastest, eight times over, and say little about the speed nearly every absence is at. A rate per tick is exact but says nothing a reader can picture; 30 days is the promise, and its tick count is stated beside it |
| A catch-up for an absence the caller saw | An entry point taking the state, the ticks, and the span, returning the same catch-up a load does | Writing a save and loading it to reuse the load path; a second replay for the shell | A round trip through the disk writes a save that exists only to be read back, and would stamp a moment the absence has already passed. A second replay is the shadow path the single catch-up exists to prevent |
| Catch-up pacing | The caller advances a catch-up in progress by counts it chooses; load returns before replaying | Replay everything inside load; a core-side progress callback; a core-side wall-time budget per call | Replaying inside load holds the terminal blank for the whole absence, which reads as a hang; a callback inverts control for no gain over an advance the caller already drives; a time budget would put a clock in the core and make how far the meadow advances depend on the machine |
| What a set-down wake remembers of achievements | The report so far lists those it earned, and the next load takes them off the record it answers as held before the absence | A second list beside the report in the save; the record as the save was loaded, so a quit mid-wake loses their being new | A report is already what a wake sets down, and folding an achievement as it folds the rest means a nap's report and an absence's stay one fold. Answering only the record as loaded leaves a returning player unaware of an achievement no journal line tells of either, a replay not journaling its events |
| A catch-up stopped early | Carry the report so far in the save beside the owed ticks; the load that replays the rest continues it, and only the finished report is recorded | Record the report so far in the journal at set-down and owe the rest; discard the report so far | Recording at set-down splits one absence across two entries, and the rest would be told under only its own load's interval: a quick relaunch would replay most of the absence below the minimum interval with no entry and no banner, and a later one would lead with a short span over a long absence's changes, its `truncated` flag lost. Discarding loses the tale of time already replayed, though its effects stand in the meadow. Carrying it costs a save field and a format version, small against telling every absence once and whole |
| What a prior save reveals of its resources | Its holdings, and the resources named in a price the core would state for it | An empty set, left for play to fill; every resource revealed; the retained reports' net changes read as holdings too | An empty set keeps the stores a player has spent to nothing missing until some arrives again, which is the reveal un-revealing for every existing save. Revealing everything names stores the meadow has never met. A retained report outlives a Move, so its changes could reveal in this meadow what only an earlier one held, and it speaks only for stores that moved across an absence long enough to be kept |
| Data an event gains | Defaulted on load; no version step | A version step for each event that gains data | The loss a step guards against here is the detail of a past occurrence that an older binary drops: no stock, reveal, or tale goes with it, and a step for every richer event would make every fuller line a migration with its own fixture |
| How a save's integers read wide | One reader answering every request for an integer | A wide deserializer named on each of some fifty integer fields; a pass over the parsed JSON clamping what it finds | Named per field, a field added later is strict until someone remembers, and a save then refuses over it. A pass over the JSON cannot tell an integer from a floating-point field that may rightly be negative, and does not know any field's range |
| A field missing from a save | Corrupt for the fields every version has written; otherwise its new-game value, in a save of any version | Corrupt for any field the save's own version wrote | Holding each version to its own field list keeps a table per version to protect nothing a player would miss: a line deleted by hand is the same mistake as a number mistyped, and gets the same physics |
| Save slots | One, atomic-replace | Rotating backups, multiple slots | v1 simplicity; atomic write already prevents the corruption case backups mostly guard |
| A prior save's kittens' profiles | Drawn as an arrival would draw them, from each kitten's name and generation, a kitten named in the album the generation before taking that memorial's profile | Every kitten given one fixed placeholder profile; profiles left empty until a kitten next does something; a draw from the state RNG at load | A placeholder makes every existing kitten identical, and an empty profile is a kitten with no face, both a quiet loss of the individuality profiles exist for (Tenet 1). A state-RNG draw at load would change every later random outcome in the migrated meadow and leave arrival and migration as two different draws |
| A v8 save's profiles | Favourites and pose drawn on load, fur and eyes kept as stored | Kept, each given a drawn pose and its six favourites as six kinds | Kept, every kitten of the save would carry the same six kinds for the life of that clowder, while every later arrival varies; drawn again, the fur and eyes the player recognises a kitten by are unchanged |
| JSON float fidelity | `serde_json` with the `float_roundtrip` feature | Default `serde_json` float parsing | Without it the parser is off-by-one-ULP for some values, so `parse(serialize(x)) ≠ x` and the byte-stable round-trip (SAVE-010) silently fails on unlucky stock values; the feature makes serialize/parse exact inverses |

## Open Questions & Future Decisions

### Resolved
1. ✅ **A long catch-up's wait.** On a release build a late meadow's 365-day cap replays in about five minutes, and an early meadow's in under a minute, either long enough that a replay finished before the shell draws reads as a hang. The catch-up is paced instead (Paced Catch-up), and the runtime leaf replays it across frames behind a waking screen.

### Deferred
1. Save-file compression toggle if saves grow past ~1 MB with roadmap systems — measure first.

## References

- Parent: `docs/high-level-design.md` (human-readable saves decision, one-simulation decision, idle-respect tenet)
- Replays `tick()` from `docs/intent/sim/engine/`
