//! The row model. Every player-facing command that belongs to a section is
//! reachable as a row in that section — mnemonics and the palette are pure
//! accelerators that gate nothing. @spec TUI-NAV-039

use clowder_core::{
    AdventureId, ArchId, ColonyId, Command, ContentId, CraftId, DreamLoomId, GameState, IdeaId,
    JobId, RefinementId, ResourceId, RiteId, RuleId, SiteId, StructureId, TraditionId,
};

use super::action::CountSemantics;
use super::section::Section;

/// A row's stable identity. The cursor remembers *this*, never a list index,
/// so returning to a section restores the same item even after intervening
/// reveals inserted rows above it. @spec TUI-NAV-008
///
/// Jobs are addressed through `Content(ContentId::Job(..))` rather than a
/// variant of their own: two spellings of one row would make cursor-memory
/// equality miss whenever the two were used inconsistently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKey {
    /// A piece of content, tagged where one item carries more than one row
    /// (an empty tag for the ordinary single-row case) — see `Kitten`.
    Content(ContentId, &'static str),
    Resource(ResourceId),
    /// A colony, tagged by which of its rows this is — see `Kitten`.
    Colony(ColonyId, &'static str),
    Site(SiteId),
    Adventure(Option<AdventureId>),
    /// A kitten by roster position. The roster is push-only for the life of a
    /// meadow — nothing is ever removed from it, which tenet 1 guarantees —
    /// so position is stable identity. A Move replaces the roster wholesale,
    /// and TUI-NAV-032's fallback covers that. @spec TUI-NAV-008
    ///
    /// The tag distinguishes the rows a single kitten carries: a key names a
    /// ROW, not the thing the row is about. Without it the second row of a
    /// pair can never be focused, remembered, or inspected, because the cursor
    /// resolves by first match.
    Kitten(usize, &'static str),
    /// A group header or a fixed row that is not content.
    Fixed(&'static str),
}

/// What a row does when acted on.
#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    /// Fires a command. @spec TUI-NAV-002
    Act { command: Command, semantics: CountSemantics },
    /// `+`/`-` adjust a count. @spec TUI-NAV-013
    Count { increment: Command, decrement: Command },
    /// Inspectable but not actionable — a store, an album entry.
    Display,
    /// A group header; skipped by the cursor, jumped to by `[` and `]`.
    Group,
}

/// One line in a section's list.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub key: RowKey,
    pub kind: RowKind,
    /// Whether this row's action is heavy: armed by the first Enter and
    /// committed only by a distinct second one, never by auto-repeat, and
    /// never reachable by `x` or `X`. @spec TUI-NAV-015, TUI-NAV-051
    pub heavy: bool,
}

impl Row {
    /// Whether the cursor may land here. @spec TUI-NAV-032
    pub fn selectable(&self) -> bool {
        !matches!(self.kind, RowKind::Group)
    }
}

fn revealed(state: &GameState, id: ContentId) -> bool {
    state.availability(id) != clowder_core::Availability::Hidden
}

fn act(command: Command, semantics: CountSemantics) -> RowKind {
    RowKind::Act { command, semantics }
}

/// Build a section's rows from current state.
///
/// The single source of the row list: input resolution and drawing both read
/// it, so a row the cursor can reach is exactly a row that gets drawn. Two
/// builders would drift the moment either changed.
/// @spec TUI-NAV-009, TUI-NAV-039, TUI-NAV-031, TUI-NAV-049, TUI-NAV-053
/// A fixed row exists only where its gate is open. Asking the core keeps the
/// rule in one place — and keeps a fresh meadow from listing every action the
/// game will ever offer. @spec TUI-NAV-006, TUI-NAV-053, SIM-ENG-045
fn gated(state: &GameState, row: Row) -> Option<Row> {
    let cmd = match &row.kind {
        RowKind::Act { command, .. } => Some(command),
        RowKind::Count { increment, .. } => Some(increment),
        _ => None,
    };
    match cmd {
        Some(c) if !state.command_gate_open(c) => None,
        _ => Some(row),
    }
}

pub fn rows(state: &GameState, section: Section) -> Vec<Row> {
    rows_ungated(state, section).into_iter().filter_map(|r| gated(state, r)).collect()
}

fn rows_ungated(state: &GameState, section: Section) -> Vec<Row> {
    match section {
        Section::Meadow => meadow_rows(state),
        Section::Clowder => clowder_rows(state),
        Section::Ideas => ideas_rows(state),
        Section::Crafts => crafts_rows(state),
        Section::Sunbeams => sunbeams_rows(state),
        Section::Wonders => wonders_rows(state),
        Section::Friends => friends_rows(state),
        Section::Sky => sky_rows(state),
        Section::Dreams => dreams_rows(state),
        Section::Legacy => legacy_rows(state),
    }
}

fn meadow_rows(state: &GameState) -> Vec<Row> {
    let mut out = vec![
        Row { key: RowKey::Fixed("GatherCatnip"), kind: act(Command::GatherCatnip, CountSemantics::Single), heavy: false },
        Row { key: RowKey::Fixed("Forage"), kind: act(Command::Forage, CountSemantics::Single), heavy: false },
    ];
    if revealed(state, ContentId::Idea(IdeaId::CouchSpelunking)) || state.ideas.contains(&IdeaId::CouchSpelunking) {
        out.push(Row { key: RowKey::Fixed("Rummage"), kind: act(Command::Rummage, CountSemantics::Single), heavy: false });
    }
    for s in StructureId::ALL {
        let id = ContentId::Structure(s);
        if revealed(state, id) {
            out.push(Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::Build(s), CountSemantics::RepeatN),
                heavy: false,
            });
        }
    }

    // The stores group: every revealed resource as an inspectable row.
    // @spec TUI-NAV-049
    let held: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| state.stock(*r) > 0.0)
        .collect();
    if !held.is_empty() {
        out.push(Row { key: RowKey::Fixed("stores"), kind: RowKind::Group, heavy: false });
        for r in held {
            out.push(Row { key: RowKey::Resource(r), kind: RowKind::Display, heavy: false });
        }
    }
    out
}

fn clowder_rows(state: &GameState) -> Vec<Row> {
    let mut out = Vec::new();
    for j in JobId::ALL {
        let id = ContentId::Job(j);
        if revealed(state, id) {
            out.push(Row {
                key: RowKey::Content(id, ""),
                kind: RowKind::Count { increment: Command::AssignJob(j), decrement: Command::UnassignJob(j) },
                heavy: false,
            });
        }
    }
    for (ix, _k) in state.kittens.iter().enumerate() {
        out.push(Row {
            key: RowKey::Kitten(ix, "promote"),
            kind: act(Command::Promote(ix), CountSemantics::Single),
            heavy: false,
        });
        out.push(Row {
            key: RowKey::Kitten(ix, "elder"),
            kind: act(Command::AppointElder(ix), CountSemantics::Single),
            heavy: false,
        });
    }
    out.push(Row { key: RowKey::Fixed("HostJamboree"), kind: act(Command::HostJamboree, CountSemantics::Single), heavy: false });
    out.push(Row {
        key: RowKey::Fixed("Pounce"),
        kind: act(Command::Pounce(1), CountSemantics::CountTaking),
        heavy: false,
    });
    out
}

fn ideas_rows(state: &GameState) -> Vec<Row> {
    let mut out = Vec::new();
    for i in IdeaId::ALL {
        let id = ContentId::Idea(i);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, ""), kind: act(Command::ResearchIdea(i), CountSemantics::Single), heavy: false });
        }
    }
    for r in RefinementId::ALL {
        let id = ContentId::Refinement(r);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, ""), kind: act(Command::BuyRefinement(r), CountSemantics::Single), heavy: false });
        }
    }
    for r in RuleId::ALL {
        let id = ContentId::Rule(r);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, ""), kind: act(Command::AdoptRule(r), CountSemantics::Single), heavy: true });
        }
    }
    out
}

fn crafts_rows(state: &GameState) -> Vec<Row> {
    let mut out = Vec::new();
    for c in CraftId::ALL {
        let id = ContentId::Craft(c);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, "craft"), kind: act(Command::Craft(c), CountSemantics::RepeatN), heavy: false });
            out.push(Row {
                key: RowKey::Content(id, "tinker"),
                kind: RowKind::Count { increment: Command::AssignTinkerer(c), decrement: Command::UnassignTinkerer(c) },
                heavy: false,
            });
        }
    }
    out
}

fn sunbeams_rows(state: &GameState) -> Vec<Row> {
    let mut out = vec![
        Row { key: RowKey::Fixed("Bask"), kind: act(Command::Bask, CountSemantics::Single), heavy: false },
        Row { key: RowKey::Fixed("Reverie"), kind: act(Command::Reverie, CountSemantics::Single), heavy: false },
    ];
    for r in RiteId::ALL {
        let id = ContentId::Rite(r);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, ""), kind: act(Command::BuyRite(r), CountSemantics::Single), heavy: false });
        }
    }
    out.push(Row { key: RowKey::Fixed("Enlighten"), kind: act(Command::Enlighten, CountSemantics::Single), heavy: false });
    out
}

fn wonders_rows(state: &GameState) -> Vec<Row> {
    let mut out = vec![Row { key: RowKey::Fixed("HoldParade"), kind: act(Command::HoldParade, CountSemantics::Single), heavy: false }];
    for a in ArchId::ALL {
        let id = ContentId::Arch(a);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, ""), kind: act(Command::BuyArch(a), CountSemantics::Single), heavy: false });
        }
    }
    out.push(Row { key: RowKey::Fixed("RefineStarshard"), kind: act(Command::RefineStarshard, CountSemantics::Single), heavy: false });
    out.push(Row { key: RowKey::Fixed("RefineEcho"), kind: act(Command::RefineEcho, CountSemantics::Single), heavy: false });
    out.push(Row { key: RowKey::Fixed("CondenseJar"), kind: act(Command::CondenseJar, CountSemantics::Single), heavy: false });
    out.push(Row { key: RowKey::Fixed("MakePromise"), kind: act(Command::MakePromise, CountSemantics::Single), heavy: false });
    out
}

fn friends_rows(state: &GameState) -> Vec<Row> {
    let mut out = Vec::new();
    for c in ColonyId::ALL {
        if c == ColonyId::StarWhales || !state.discovered.contains(&c) {
            continue;
        }
        out.push(Row {
            key: RowKey::Colony(c, "trade"),
            kind: act(Command::Trade(c, 1), CountSemantics::CountTaking),
            heavy: false,
        });
        out.push(Row { key: RowKey::Colony(c, "penpal"), kind: act(Command::SendPenPal(c), CountSemantics::Single), heavy: false });
    }
    out.push(Row { key: RowKey::Fixed("ScoutPicnic"), kind: act(Command::ScoutPicnic, CountSemantics::Single), heavy: false });
    out
}

fn sky_rows(state: &GameState) -> Vec<Row> {
    let mut out = Vec::new();
    for site in SiteId::ALL {
        if state.site_reachable(site) {
            out.push(Row { key: RowKey::Site(site), kind: act(Command::LaunchExpedition(site), CountSemantics::Single), heavy: false });
        }
    }
    out.push(Row { key: RowKey::Fixed("SummonStarWhale"), kind: act(Command::SummonStarWhale, CountSemantics::Single), heavy: false });
    if state.whale_visit_years > 0 {
        out.push(Row { key: RowKey::Fixed("FeedWhale"), kind: act(Command::FeedWhale, CountSemantics::Single), heavy: false });
    }
    out
}

fn dreams_rows(state: &GameState) -> Vec<Row> {
    let mut out = vec![Row {
        key: RowKey::Fixed("DeepNap"),
        kind: act(Command::DeepNap(1), CountSemantics::CountTaking),
        // TUI-NAV-015 names four heavy actions and this is one: it skips whole
        // years. `heavy` is the arming contract — false here means a held Enter
        // would commit it outright.
        heavy: true,
    }];
    for id in DreamLoomId::ALL {
        let cid = ContentId::DreamLoom(id);
        if revealed(state, cid) {
            out.push(Row { key: RowKey::Content(cid, ""), kind: act(Command::BuyDreamLoom(id), CountSemantics::Single), heavy: false });
        }
    }
    out
}

fn legacy_rows(state: &GameState) -> Vec<Row> {
    let mut out = vec![
        Row { key: RowKey::Fixed("GreatMeadowMove"), kind: act(Command::GreatMeadowMove, CountSemantics::Single), heavy: true },
        Row { key: RowKey::Fixed("PassItDown"), kind: act(Command::PassItDown, CountSemantics::Single), heavy: true },
    ];
    for t in TraditionId::ALL {
        let id = ContentId::Tradition(t);
        if revealed(state, id) {
            out.push(Row { key: RowKey::Content(id, ""), kind: act(Command::BuyTradition(t), CountSemantics::Single), heavy: false });
        }
    }

    let adventures_open = state.traditions.contains(&clowder_core::TraditionId::AdventurousSpirit)
        || state.ideas.contains(&IdeaId::AdventurousSpirit);
    if adventures_open {
        out.push(Row {
            key: RowKey::Adventure(None),
            kind: act(Command::DeclareAdventure(None), CountSemantics::Single),
            heavy: false,
        });
        for a in AdventureId::ALL {
            out.push(Row {
                key: RowKey::Adventure(Some(a)),
                kind: act(Command::DeclareAdventure(Some(a)), CountSemantics::Single),
                heavy: false,
            });
        }
    }
    out
}

/// Which sections have at least one revealed row, in digit order — the tab bar
/// exists only where content does. @spec TUI-NAV-006
pub fn revealed_sections(state: &GameState) -> Vec<Section> {
    Section::ALL.into_iter().filter(|s| !rows(state, *s).is_empty()).collect()
}

/// Whether `section` lists at least one row the cursor can land on — a group
/// header never counting. This, not "has any row at all", is what earns a
/// section its tab: a section that lists only headers has nothing in it for
/// the player to do yet. @spec TUI-NAV-006
pub fn lists_selectable(state: &GameState, section: Section) -> bool {
    rows(state, section).iter().any(|r| r.selectable())
}

/// Whether `section` currently wears a tab: it lists a selectable row, or it
/// holds `focus` — a section emptied while the player stands in it keeps its
/// tab for as long as focus stays there, and loses it the moment focus moves
/// on. @spec TUI-NAV-006, TUI-NAV-085
pub fn has_tab(state: &GameState, focus: Option<Section>, section: Section) -> bool {
    lists_selectable(state, section) || Some(section) == focus
}

/// Every section that currently wears a tab, in digit order — what the tab
/// bar draws and what a section cycle rings over. @spec TUI-NAV-006,
/// TUI-NAV-085
pub fn tab_sections(state: &GameState, focus: Option<Section>) -> Vec<Section> {
    Section::ALL.into_iter().filter(|s| has_tab(state, focus, *s)).collect()
}

/// Whether `key` names a row still listed anywhere — every section, not only
/// the one it last lived in, since an arming carries no section of its own
/// and a row's identity is unique across the whole app. Used to end an
/// arming whose row has left its list. @spec TUI-NAV-076
pub fn row_still_listed(state: &GameState, key: &RowKey) -> bool {
    Section::ALL.iter().any(|s| rows(state, *s).iter().any(|r| &r.key == key))
}
