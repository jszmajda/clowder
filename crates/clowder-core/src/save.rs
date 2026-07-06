use crate::events::Event;
use crate::state::{GameState, ResourceId};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

/// v2: depth expansion — adds speed, revealed set, and each wave's state
/// blocks. Older versions load via serde defaults (the fields' new-game
/// values), verified against committed fixtures. @spec SAVE-013
pub const SAVE_VERSION: u32 = 2;
const CATCHUP_CAP_SECONDS: u64 = 365 * 86_400;
const TICKS_PER_SECOND: u64 = crate::content::TICKS_PER_SECOND;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub save_version: u32,
    pub saved_at_unix: u64,
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
#[derive(Debug, Clone, PartialEq)]
pub struct OfflineReport {
    pub elapsed_seconds: u64,
    pub truncated: bool,
    /// Net stock change per resource, indexed by `ResourceId::ix()`.
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
    let file = SaveFile { save_version: SAVE_VERSION, saved_at_unix, state: state.clone() };
    serde_json::to_string_pretty(&file).expect("GameState serialization is infallible")
}

/// Atomic write: temp file in the same directory, fsync, rename.
// @spec SAVE-003
pub fn write_save(path: &Path, state: &GameState, saved_at_unix: u64) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(save_string(state, saved_at_unix).as_bytes())?;
    file.sync_all()?; // without this, a power loss after rename could keep an empty "new" file
    drop(file);
    std::fs::rename(&tmp, path)
}

/// Parse a save and replay offline time through the standard tick().
// @spec SAVE-002, SAVE-005, SAVE-006, SAVE-007
pub fn load_string(s: &str, now_unix: u64) -> Result<LoadedGame, LoadError> {
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

    // Offline catch-up: same tick(), fast-forwarded at the save's own game
    // speed — away time accrues exactly as attended time would have. A
    // backwards clock is zero ticks, an absurd clock is capped and flagged.
    let raw_elapsed = now_unix.saturating_sub(file.saved_at_unix);
    let truncated = raw_elapsed > CATCHUP_CAP_SECONDS;
    let elapsed = raw_elapsed.min(CATCHUP_CAP_SECONDS);
    let ticks_to_replay = ((elapsed * TICKS_PER_SECOND) as f64 * state.speed).round() as u64;
    let before = state.resources;
    let mut report = OfflineReport {
        elapsed_seconds: elapsed,
        truncated,
        ..OfflineReport::default()
    };
    for _ in 0..ticks_to_replay {
        for event in state.tick() {
            fold_event(&mut report, event);
        }
    }
    for r in ResourceId::ALL {
        report.resource_deltas[r.ix()] = state.resources[r.ix()] - before[r.ix()];
    }
    Ok(LoadedGame { state, report })
}

fn fold_event(report: &mut OfflineReport, event: Event) {
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
