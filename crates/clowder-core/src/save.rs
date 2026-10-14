use crate::events::Event;
use crate::state::{GameState, ResourceId};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// v3: the event journal joins canonical state. v2 was the depth expansion —
/// speed, the revealed set, and each wave's state blocks. v4 added the
/// banked-tick count (SAVE-017). v5 adds the report so far of a catch-up set
/// down before it finished (SAVE-021) — a version step rather than a defaulted
/// field, so a save carrying a tale an older binary cannot represent is
/// refused as too new rather than loaded with that tale silently dropped.
/// Older versions load via serde defaults (the fields' new-game values),
/// verified against committed fixtures; a pre-journal save migrates to an
/// empty journal, which is the honest representation of history it never
/// recorded.
/// v6 adds the revealed-resource set (SAVE-023), filled on migration from the
/// save itself (SAVE-024).
/// @spec SAVE-013, SAVE-014, SAVE-021, SAVE-023
pub const SAVE_VERSION: u32 = 6;
/// The shortest absence worth reporting. Public for the same reason the cap
/// is: a caller deciding whether to surface a report must use this value, not
/// a copy of it. @spec SAVE-015
pub const MIN_REPORTABLE_ABSENCE_SECONDS: u64 = 60;

/// The one rule for how much absence is replayed. Public so a caller
/// applying it to an interval it observed itself uses this value rather than
/// a copy of it. @spec SAVE-016
pub const CATCHUP_CAP_SECONDS: u64 = 365 * 86_400;
const TICKS_PER_SECOND: u64 = crate::content::TICKS_PER_SECOND;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub save_version: u32,
    pub saved_at_unix: u64,
    /// Ticks the shell had banked but not yet run when this was written.
    ///
    /// Carried explicitly because the alternative — back-dating the timestamp
    /// so the interval re-derives them — silently re-prices the backlog at
    /// whatever speed happened to be set at save time. Ticks earned at one
    /// speed and saved at another were being lost or conjured by the ratio
    /// between them. What the player earned is theirs, so it is stored as
    /// what it is: a count. @spec SAVE-017
    #[serde(default)]
    pub owed_ticks: u64,
    /// The report so far of a catch-up set down before it finished, or
    /// `None`. Canonical for the same reason the journal is: it records what
    /// the meadow has already lived through, and nothing else can recompute
    /// it. Its arrival is a version step (SAVE_VERSION 5) rather than a
    /// defaulted field, so an older binary refuses the save as too new rather
    /// than silently dropping the tale it carries. @spec SAVE-021
    #[serde(default)]
    pub report_so_far: Option<OfflineReport>,
    pub state: GameState,
}

#[derive(Debug)]
pub enum LoadError {
    /// No save file — the caller starts a new game.
    NotFound,
    /// The save was written by a newer clowder; never guess forward.
    TooNew { found: u32 },
    /// Unparseable JSON, missing fields, or unknown content ids. Core never
    /// deletes or rewrites the file; what to do next is the shell's call.
    Corrupt { detail: String },
}

/// What happened while you were away, folded from the catch-up replay.
/// Name lists are gross with per-name counts — a kitten may appear in more
/// than one list, and more than once per list.
// @spec SAVE-008
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineReport {
    pub elapsed_seconds: u64,
    pub truncated: bool,
    /// Net stock change per resource, indexed by `ResourceId::ix()`.
    /// Serde has no array impl past 32, so this borrows the same helpers
    /// `GameState` uses; older reports zero-fill forward.
    #[serde(
        serialize_with = "crate::state::serialize_array",
        deserialize_with = "crate::state::zero_filled_array"
    )]
    pub resource_deltas: [f64; crate::state::RESOURCE_COUNT],
    pub arrived: Vec<(String, u32)>,
    pub wandered: Vec<(String, u32)>,
    pub returned: Vec<(String, u32)>,
    pub seasons_passed: u32,
}

impl Default for OfflineReport {
    fn default() -> Self {
        OfflineReport {
            elapsed_seconds: 0,
            truncated: false,
            resource_deltas: [0.0; crate::state::RESOURCE_COUNT],
            arrived: Vec::new(),
            wandered: Vec::new(),
            returned: Vec::new(),
            seasons_passed: 0,
        }
    }
}

#[derive(Debug)]
pub struct LoadedGame {
    pub state: GameState,
    pub report: OfflineReport,
}

/// Serialize a save to pretty JSON with a caller-supplied timestamp.
// @spec SAVE-001, SAVE-011
pub fn save_string(state: &GameState, saved_at_unix: u64) -> String {
    save_string_owing(state, saved_at_unix, 0)
}

/// As [`save_string`], carrying ticks the caller has banked but not yet run.
/// @spec SAVE-017
pub fn save_string_owing(state: &GameState, saved_at_unix: u64, owed_ticks: u64) -> String {
    save_string_carrying(state, saved_at_unix, owed_ticks, None)
}

/// As [`save_string_owing`], additionally carrying the report so far of a
/// catch-up set down before it finished (or `None`), so the load that
/// finishes it continues the tale rather than beginning a new one.
/// @spec SAVE-020, SAVE-021
pub fn save_string_carrying(
    state: &GameState,
    saved_at_unix: u64,
    owed_ticks: u64,
    report_so_far: Option<&OfflineReport>,
) -> String {
    let file = SaveFile {
        save_version: SAVE_VERSION,
        saved_at_unix,
        owed_ticks,
        report_so_far: report_so_far.cloned(),
        state: state.clone(),
    };
    serde_json::to_string_pretty(&file).expect("GameState serialization is infallible")
}

/// Atomic write: temp file in the same directory, fsync, rename.
// @spec SAVE-003
pub fn write_save(path: &Path, state: &GameState, saved_at_unix: u64) -> std::io::Result<()> {
    write_save_owing(path, state, saved_at_unix, 0)
}

/// As [`write_save`], carrying banked-but-unrun ticks. @spec SAVE-017
pub fn write_save_owing(
    path: &Path,
    state: &GameState,
    saved_at_unix: u64,
    owed_ticks: u64,
) -> std::io::Result<()> {
    write_save_carrying(path, state, saved_at_unix, owed_ticks, None)
}

/// As [`write_save_owing`], additionally carrying the report so far of a
/// catch-up set down before it finished (or `None`). @spec SAVE-020, SAVE-021
pub fn write_save_carrying(
    path: &Path,
    state: &GameState,
    saved_at_unix: u64,
    owed_ticks: u64,
    report_so_far: Option<&OfflineReport>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(
        save_string_carrying(state, saved_at_unix, owed_ticks, report_so_far).as_bytes(),
    )?;
    file.sync_all()?; // without this, a power loss after rename could keep an empty "new" file
    drop(file);
    std::fs::rename(&tmp, path)
}

/// A load's catch-up, parsed, migrated, and normalized but not yet replayed.
/// The caller advances it by a count of ticks of its own choosing
/// ([`CatchUp::advance`]), so a shell can spread a long absence over frames;
/// the core never reads a clock to decide how much to run. How the replay is
/// divided into calls changes nothing it produces: the same ticks run in the
/// same order through the same replay step, and the report folds the same
/// events, whichever way the total is divided.
/// @spec SAVE-018
pub struct CatchUp {
    state: GameState,
    total_ticks: u64,
    remaining_ticks: u64,
    /// Resource stocks as loaded, before any tick of this catch-up ran — the
    /// baseline this catch-up's own net deltas are measured against.
    before: [f64; crate::state::RESOURCE_COUNT],
    /// The report so far this save carried (SAVE-021), or its default (every
    /// field zero/empty) when it carried none — continuing a report that
    /// started empty is simply this catch-up's own tale. @spec SAVE-022
    carried: OfflineReport,
    /// This load's own capped interval and truncation flag — never summed
    /// with the carried report's until a result is built. @spec SAVE-022
    this_elapsed_seconds: u64,
    this_truncated: bool,
    /// Folded from the ticks this catch-up itself has replayed so far: names
    /// and seasons only, since resource deltas are recomputed from `before`.
    partial: OfflineReport,
}

/// What advancing a catch-up produced: still owing ticks, or finished.
///
/// The size gap between variants is inherent to the surface callers advance
/// against one tick-count at a time (SAVE-018) — boxing `CatchUp` would cost
/// every caller an allocation per advance to save space a few-frames-long
/// wake never notices.
/// @spec SAVE-018, SAVE-019
#[allow(clippy::large_enum_variant)]
pub enum Advance {
    Pending(CatchUp),
    Finished(LoadedGame),
}

/// A catch-up set down before it finished: the state it reached, the ticks
/// still owed, and the report so far — folded from the ticks replayed,
/// continuing any report the save carried. Nothing is recorded in the
/// journal; the load that finally finishes it does that once.
/// @spec SAVE-020
pub struct SetDown {
    pub state: GameState,
    pub owed_ticks: u64,
    pub report_so_far: OfflineReport,
}

/// Sum two per-name count lists, a name's counts added where it appears in
/// both. Order: `a`'s names first (in `a`'s order), then any name new to `b`
/// in `b`'s order — deterministic given deterministic inputs, which is all
/// SAVE-022's continuation needs. @spec SAVE-022
fn merge_lists(a: &[(String, u32)], b: &[(String, u32)]) -> Vec<(String, u32)> {
    let mut out = a.to_vec();
    for (name, n) in b {
        if let Some(entry) = out.iter_mut().find(|(nm, _)| nm == name) {
            entry.1 += n;
        } else {
            out.push((name.clone(), *n));
        }
    }
    out
}

impl CatchUp {
    /// The catch-up's total owed ticks: this load's own interval plus any the
    /// save carried as banked (SAVE-017). Fixed for the catch-up's lifetime.
    /// @spec SAVE-018
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }

    /// Ticks not yet replayed. @spec SAVE-018
    pub fn remaining_ticks(&self) -> u64 {
        self.remaining_ticks
    }

    /// The save's own game speed — the rate the shell banks real time passing
    /// while this catch-up replays (runtime leaf: Waking Up). @spec TUI-RUN-036
    pub fn speed(&self) -> f64 {
        self.state.speed
    }

    /// The report so far: the carried report continued by what this
    /// catch-up's own replay (`partial`) has folded, measured against
    /// `before`. @spec SAVE-020, SAVE-022
    fn built_report(&self) -> OfflineReport {
        let mut resource_deltas = [0.0; crate::state::RESOURCE_COUNT];
        for r in ResourceId::ALL {
            resource_deltas[r.ix()] =
                self.carried.resource_deltas[r.ix()] + (self.state.resources[r.ix()] - self.before[r.ix()]);
        }
        OfflineReport {
            elapsed_seconds: self.carried.elapsed_seconds + self.this_elapsed_seconds,
            truncated: self.carried.truncated || self.this_truncated,
            resource_deltas,
            arrived: merge_lists(&self.carried.arrived, &self.partial.arrived),
            wandered: merge_lists(&self.carried.wandered, &self.partial.wandered),
            returned: merge_lists(&self.carried.returned, &self.partial.returned),
            seasons_passed: self.carried.seasons_passed + self.partial.seasons_passed,
        }
    }

    /// Replay at most `max_ticks` of what remains, through the same replay
    /// step live play's `tick()` uses in every respect but journaling. When
    /// this replays the last owed tick, the catch-up finishes exactly as a
    /// one-call load would (SAVE-019): the report completed and recorded in
    /// the journal, and the caught-up state and report handed back.
    /// Otherwise the catch-up comes back still pending. @spec SAVE-018,
    /// SAVE-019
    pub fn advance(mut self, max_ticks: u64) -> Advance {
        let n = max_ticks.min(self.remaining_ticks);
        for _ in 0..n {
            // @spec SIM-ENG-028, SIM-ENG-040
            for event in self.state.tick_replay() {
                fold_event(&mut self.partial, event);
            }
        }
        self.remaining_ticks -= n;
        if self.remaining_ticks == 0 {
            let report = self.built_report();
            let mut state = self.state;
            // @spec SAVE-015, SAVE-022
            state.journal.record_away(state.ticks, &report);
            Advance::Finished(LoadedGame { state, report })
        } else {
            Advance::Pending(self)
        }
    }

    /// Stop early: hand back the state as it stands, the ticks still owed,
    /// and the report so far — nothing recorded in the journal, so a save
    /// carrying all three loses no owed tick and no replayed part of the
    /// tale. @spec SAVE-020
    pub fn set_down(self) -> SetDown {
        let report_so_far = self.built_report();
        SetDown { state: self.state, owed_ticks: self.remaining_ticks, report_so_far }
    }
}

/// Parse, migrate, and normalize a save, and hand back its catch-up pending —
/// the state as loaded, the ticks it owes in total, and the report folding so
/// far — without replaying any of it. @spec SAVE-002, SAVE-005, SAVE-006,
/// SAVE-007, SAVE-018
pub fn begin_load(s: &str, now_unix: u64) -> Result<CatchUp, LoadError> {
    // Version-check on the loose parse first: a newer save may contain
    // variants this binary can't deserialize, and that must read as TooNew,
    // not Corrupt.
    let value: serde_json::Value =
        serde_json::from_str(s).map_err(|e| LoadError::Corrupt { detail: e.to_string() })?;
    let found = value
        .get("save_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| LoadError::Corrupt { detail: "missing save_version".into() })? as u32;
    if found > SAVE_VERSION {
        return Err(LoadError::TooNew { found });
    }
    let file: SaveFile = serde_json::from_value(value)
        .map_err(|e| LoadError::Corrupt { detail: e.to_string() })?;
    let mut state = file.state;
    // The reveal worklists aren't serialized; rebuild them from `revealed`
    // before any tick (catch-up) runs, so a reloaded game reveals exactly as
    // live play would. @spec SIM-ENG-023
    state.rebuild_unrevealed();

    // Normalize hand-edited numerics: physics, not errors. (Counters were
    // already clamped by their wide deserializers.)
    if !state.speed.is_finite() {
        state.speed = 1.0;
    }
    state.speed = state.speed.clamp(crate::state::SPEED_MIN, crate::state::SPEED_MAX);
    if !state.mess.is_finite() || state.mess < 0.0 {
        state.mess = 0.0;
    }
    if !state.kites_high_water.is_finite() || state.kites_high_water < 0.0 {
        state.kites_high_water = 0.0;
    }
    for r in ResourceId::ALL {
        let cap = state.cap(r);
        if state.resources[r.ix()] > cap {
            state.resources[r.ix()] = cap;
        }
        if !state.resources[r.ix()].is_finite() || state.resources[r.ix()] < 0.0 {
            state.resources[r.ix()] = 0.0;
        }
    }
    // The high-water is a ratchet: it can never sit below the kites in paw.
    // @spec SIM-ECO-031
    state.kites_high_water = state.kites_high_water.max(state.stock(ResourceId::Kites));

    // Society-wave normalization: learning is physics too. Skills and exp
    // floor at zero and must be finite; an out-of-roster Elder index is no
    // Elder at all. (Ranks were already clamped by their wide deserializer;
    // jamboree_until and tinkering recipes are valid by construction.)
    // @spec SIM-KIT-027
    for k in &mut state.kittens {
        if !k.exp.is_finite() || k.exp < 0.0 {
            k.exp = 0.0;
        }
        for s in &mut k.skills {
            if !s.is_finite() || *s < 0.0 {
                *s = 0.0;
            }
        }
    }
    if state.elder.is_some_and(|ix| ix >= state.kittens.len()) {
        state.elder = None;
    }

    // Wave-4 normalization: devotion, serenity, peak_devotion, and dream-drift
    // are physics — floor at zero and stay finite. The enlightenment tier,
    // pen-pal levels, discoveries, rites, and Promises are valid by
    // construction. @spec SAVE-013
    for value in [
        &mut state.devotion,
        &mut state.serenity,
        &mut state.peak_devotion,
        &mut state.dream_drift,
    ] {
        if !value.is_finite() || *value < 0.0 {
            *value = 0.0;
        }
    }
    state.peak_devotion = state.peak_devotion.max(state.devotion);

    // Wave-5 normalization: the legacy currencies and adventure reward pools are
    // physics — floor at zero and stay finite. Generation, completions, and the
    // armed/active adventure slots are valid by construction (their
    // deserializers). @spec SAVE-013
    for value in [
        &mut state.elder_wisdom,
        &mut state.warm_memories,
        &mut state.passed_down_wisdom,
        // Wave-6: stirred-dreams heat is physics too — floor at zero and stay
        // finite. Sites, looms, whale counters, and deja_vu_until are valid by
        // construction (their deserializers). @spec SAVE-013
        &mut state.stirred_dreams,
    ] {
        if !value.is_finite() || *value < 0.0 {
            *value = 0.0;
        }
    }
    for reward in &mut state.adventure_rewards {
        if !reward.is_finite() || *reward < 0.0 {
            *reward = 0.0;
        }
    }

    // v5 -> v6: the revealed-resource set arrives filled by the engine's own
    // reveal rule, applied once to the state as normalized and before any
    // catch-up tick. @spec SAVE-024
    if found < 6 {
        state.record_reveals();
    }

    // Offline catch-up: same tick(), fast-forwarded at the save's own game
    // speed — away time accrues exactly as attended time would have. A
    // backwards clock is zero ticks, an absurd clock is capped and flagged.
    let raw_elapsed = now_unix.saturating_sub(file.saved_at_unix);
    let truncated = raw_elapsed > CATCHUP_CAP_SECONDS;
    let elapsed = raw_elapsed.min(CATCHUP_CAP_SECONDS);
    // Ticks the previous session had already earned are added as-is — they were
    // priced when they were earned, and re-deriving them from an interval would
    // re-price them at whatever speed is set now. @spec SAVE-017
    let ticks_to_replay =
        ((elapsed * TICKS_PER_SECOND) as f64 * state.speed).round() as u64 + file.owed_ticks;
    let before = state.resources;
    Ok(CatchUp {
        state,
        total_ticks: ticks_to_replay,
        remaining_ticks: ticks_to_replay,
        before,
        carried: file.report_so_far.unwrap_or_default(),
        this_elapsed_seconds: elapsed,
        this_truncated: truncated,
        partial: OfflineReport::default(),
    })
}

/// Parse a save and replay its whole catch-up at once through
/// [`begin_load`]/[`CatchUp::advance`] — the load tests and tools use.
/// @spec SAVE-002, SAVE-005, SAVE-006, SAVE-007
pub fn load_string(s: &str, now_unix: u64) -> Result<LoadedGame, LoadError> {
    match begin_load(s, now_unix)?.advance(u64::MAX) {
        Advance::Finished(g) => Ok(g),
        Advance::Pending(_) => unreachable!("advancing by u64::MAX always finishes a catch-up"),
    }
}

/// Fold one replayed event into a report in progress. Shared by offline
/// catch-up and the Deep Nap (SIM-DREAM-008) so both bulk-time routes fold
/// identically. @spec SAVE-008, SIM-DREAM-008
pub(crate) fn fold_event(report: &mut OfflineReport, event: Event) {
    match event {
        Event::KittenArrived { name } => bump(&mut report.arrived, &name),
        Event::KittenWanderedOff { name } => bump(&mut report.wandered, &name),
        Event::KittenReturned { name } => bump(&mut report.returned, &name),
        Event::SeasonChanged { .. } => report.seasons_passed += 1,
        _ => {}
    }
}

fn bump(list: &mut Vec<(String, u32)>, name: &str) {
    if let Some(entry) = list.iter_mut().find(|(n, _)| n == name) {
        entry.1 += 1;
    } else {
        list.push((name.to_string(), 1));
    }
}

// @spec SAVE-004
pub fn read_save(path: &Path, now_unix: u64) -> Result<LoadedGame, LoadError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(LoadError::NotFound),
        Err(e) => return Err(LoadError::Corrupt { detail: e.to_string() }),
    };
    load_string(&contents, now_unix)
}

/// Platform data dir + clowder/save.json.
// @spec SAVE-012
pub fn default_save_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "clowder")
        .map(|dirs| dirs.data_dir().join("save.json"))
}
