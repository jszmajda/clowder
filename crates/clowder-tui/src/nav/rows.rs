//! The row model. Every player-facing command that belongs to a section is
//! reachable as a row in that section — mnemonics and the palette are pure
//! accelerators that gate nothing. @spec TUI-NAV-039

use clowder_core::{
    AdventureId, ArchId, ColonyId, Command, ContentId, CraftId, DreamLoomId, GameState, IdeaId,
    JobId, LockReason, RefinementId, ResourceId, RiteId, RuleId, SiteId, StructureId, TraditionId,
};

use super::action::CountSemantics;
use super::section::Section;
use super::view::ViewState;

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
    Act {
        command: Command,
        semantics: CountSemantics,
    },
    /// `+`/`-` adjust a count. @spec TUI-NAV-013
    Count {
        increment: Command,
        decrement: Command,
    },
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

/// The plain-word label a later row of a multi-row item draws in its name
/// column, and states in the inspector beneath the item's name, in place of
/// the item's own name — keyed by the row's own tag, so a row this table does
/// not name draws its item's name instead. `None` marks the first-listed row
/// of an item (or any row that carries no sibling rows), which draws the
/// item's own name. @spec TUI-NAV-089
pub fn later_row_label(key: &RowKey) -> Option<&'static str> {
    match key {
        RowKey::Content(ContentId::Craft(_), "tinker") => Some("tinkerers"),
        RowKey::Colony(_, "penpal") => Some("send a pen-pal"),
        _ => None,
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

/// One group of a section's list: `rows` gated and, where any survive, headed
/// by `title` directly above the first — a header never standing above an
/// empty group. The gate has to run here, before a group's emptiness is
/// judged, since a group all of whose rows the core gates shut is itself
/// empty and so wears no header.
/// @spec TUI-NAV-097, TUI-NAV-098, TUI-NAV-099, TUI-NAV-100
fn headed_group(state: &GameState, title: &'static str, rows: Vec<Row>) -> Vec<Row> {
    let listed: Vec<Row> = rows.into_iter().filter_map(|r| gated(state, r)).collect();
    if listed.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(listed.len() + 1);
    out.push(Row {
        key: RowKey::Fixed(title),
        kind: RowKind::Group,
        heavy: false,
    });
    out.extend(listed);
    out
}

pub fn rows(state: &GameState, section: Section) -> Vec<Row> {
    rows_ungated(state, section)
        .into_iter()
        .filter_map(|r| gated(state, r))
        .collect()
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
    let mut verbs = vec![
        Row {
            key: RowKey::Fixed("GatherCatnip"),
            kind: act(Command::GatherCatnip, CountSemantics::Single),
            heavy: false,
        },
        Row {
            key: RowKey::Fixed("Forage"),
            kind: act(Command::Forage, CountSemantics::Single),
            heavy: false,
        },
    ];
    if revealed(state, ContentId::Idea(IdeaId::CouchSpelunking))
        || state.ideas.contains(&IdeaId::CouchSpelunking)
    {
        verbs.push(Row {
            key: RowKey::Fixed("Rummage"),
            kind: act(Command::Rummage, CountSemantics::Single),
            heavy: false,
        });
    }
    let mut out = headed_group(state, "things to do", verbs);

    let mut structures = Vec::new();
    for s in StructureId::ALL {
        let id = ContentId::Structure(s);
        if revealed(state, id) {
            structures.push(Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::Build(s), CountSemantics::RepeatN),
                heavy: false,
            });
        }
    }
    out.extend(headed_group(state, "structures", structures));

    // The stores group: every revealed resource as an inspectable row.
    // @spec TUI-NAV-049
    let stores: Vec<Row> = state
        .resources_revealed()
        .into_iter()
        .map(|r| Row {
            key: RowKey::Resource(r),
            kind: RowKind::Display,
            heavy: false,
        })
        .collect();
    out.extend(headed_group(state, "stores", stores));
    out
}

fn clowder_rows(state: &GameState) -> Vec<Row> {
    let mut jobs = Vec::new();
    for j in JobId::ALL {
        let id = ContentId::Job(j);
        if revealed(state, id) {
            jobs.push(Row {
                key: RowKey::Content(id, ""),
                kind: RowKind::Count {
                    increment: Command::AssignJob(j),
                    decrement: Command::UnassignJob(j),
                },
                heavy: false,
            });
        }
    }
    let mut out = headed_group(state, "jobs", jobs);

    // Each kitten is exactly one row, drawn with its name and no label; its
    // Enter promotes it. AppointElder is never a row of its own — it is taken
    // only by this row's own row-bound `n` key (TUI-NAV-094).
    // @spec TUI-NAV-093
    let mut kittens = Vec::new();
    for (ix, _k) in state.kittens.iter().enumerate() {
        kittens.push(Row {
            key: RowKey::Kitten(ix, ""),
            kind: act(Command::Promote(ix), CountSemantics::Single),
            heavy: false,
        });
    }
    out.extend(headed_group(state, "kittens", kittens));

    out.extend(headed_group(
        state,
        "things to do",
        vec![
            Row {
                key: RowKey::Fixed("HostJamboree"),
                kind: act(Command::HostJamboree, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("Pounce"),
                kind: act(Command::Pounce(1), CountSemantics::CountTaking),
                heavy: false,
            },
        ],
    ));
    out
}

/// The key of the Ideas section's done group header. @spec TUI-NAV-111
pub const DONE_HEADER: &str = "done";

/// Whether the core names `id`'s item already the player's — the answer
/// behind a row's ✓, and the one rule for what is finished. The shell judges
/// by no rule of its own. @spec TUI-NAV-110
fn finished(state: &GameState, id: ContentId) -> bool {
    state.lock_reason(id) == Some(LockReason::AlreadyYours)
}

/// The Ideas section: its three kinds' unfinished rows in table order, then
/// one done group of every finished row, ideas first, then refinements, then
/// rules. @spec TUI-NAV-009, TUI-NAV-097, TUI-NAV-098, TUI-NAV-110
fn ideas_rows(state: &GameState) -> Vec<Row> {
    let (mut ideas, mut refinements, mut rules) = (Vec::new(), Vec::new(), Vec::new());
    let (mut done_ideas, mut done_refinements, mut done_rules) =
        (Vec::new(), Vec::new(), Vec::new());
    for i in IdeaId::ALL {
        let id = ContentId::Idea(i);
        if revealed(state, id) {
            let row = Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::ResearchIdea(i), CountSemantics::Single),
                heavy: false,
            };
            if finished(state, id) {
                done_ideas.push(row)
            } else {
                ideas.push(row)
            }
        }
    }
    for r in RefinementId::ALL {
        let id = ContentId::Refinement(r);
        if revealed(state, id) {
            let row = Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::BuyRefinement(r), CountSemantics::RepeatN),
                heavy: false,
            };
            if finished(state, id) {
                done_refinements.push(row)
            } else {
                refinements.push(row)
            }
        }
    }
    for r in RuleId::ALL {
        let id = ContentId::Rule(r);
        if revealed(state, id) {
            let row = Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::AdoptRule(r), CountSemantics::Single),
                heavy: true,
            };
            if finished(state, id) {
                done_rules.push(row)
            } else {
                rules.push(row)
            }
        }
    }

    let mut out = headed_group(state, "ideas", ideas);
    out.extend(headed_group(state, "refinements", refinements));
    out.extend(headed_group(state, "rules", rules));
    let mut done = done_ideas;
    done.extend(done_refinements);
    done.extend(done_rules);
    out.extend(headed_group(state, DONE_HEADER, done));
    out
}

/// How many rows the Ideas section's done group holds — zero when it is not
/// listed. @spec TUI-NAV-111
pub fn done_count(state: &GameState) -> usize {
    let list = rows(state, Section::Ideas);
    match list.iter().position(is_done_header) {
        Some(h) => list.len() - h - 1,
        None => 0,
    }
}

/// Whether `row` is the Ideas section's done group header. @spec TUI-NAV-111
pub fn is_done_header(row: &Row) -> bool {
    matches!(row.kind, RowKind::Group) && row.key == RowKey::Fixed(DONE_HEADER)
}

/// The rows `section`'s list draws under `view`: every listed row, save that
/// the Ideas section's done group, while folded, draws its header and none of
/// its rows. The cursor visits exactly the selectable rows of this list.
/// @spec TUI-NAV-113
pub fn drawn_rows(state: &GameState, section: Section, view: &ViewState) -> Vec<Row> {
    let mut list = rows(state, section);
    if section == Section::Ideas && !view.done_open {
        if let Some(h) = list.iter().position(is_done_header) {
            list.truncate(h + 1);
        }
    }
    list
}

/// A row of the Ideas section's place in its own kind's group, in the order
/// the core lists the kind, the kinds in the order the groups stand — the
/// place a finished row held before it moved to the done group. `None` for a
/// row that is not an idea, refinement or rule. @spec TUI-NAV-117
pub fn ideas_place(key: &RowKey) -> Option<(usize, usize)> {
    match key {
        RowKey::Content(ContentId::Idea(i), _) => {
            IdeaId::ALL.iter().position(|x| x == i).map(|p| (0, p))
        }
        RowKey::Content(ContentId::Refinement(r), _) => RefinementId::ALL
            .iter()
            .position(|x| x == r)
            .map(|p| (1, p)),
        RowKey::Content(ContentId::Rule(r), _) => {
            RuleId::ALL.iter().position(|x| x == r).map(|p| (2, p))
        }
        _ => None,
    }
}

fn crafts_rows(state: &GameState) -> Vec<Row> {
    let mut out = Vec::new();
    for c in CraftId::ALL {
        let id = ContentId::Craft(c);
        if revealed(state, id) {
            out.push(Row {
                key: RowKey::Content(id, "craft"),
                kind: act(Command::Craft(c), CountSemantics::RepeatN),
                heavy: false,
            });
            out.push(Row {
                key: RowKey::Content(id, "tinker"),
                kind: RowKind::Count {
                    increment: Command::AssignTinkerer(c),
                    decrement: Command::UnassignTinkerer(c),
                },
                heavy: false,
            });
        }
    }
    out
}

fn sunbeams_rows(state: &GameState) -> Vec<Row> {
    let mut out = headed_group(
        state,
        "things to do",
        vec![
            Row {
                key: RowKey::Fixed("Bask"),
                kind: act(Command::Bask, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("Reverie"),
                kind: act(Command::Reverie, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("Enlighten"),
                kind: act(Command::Enlighten, CountSemantics::Single),
                heavy: false,
            },
        ],
    );

    let mut rites = Vec::new();
    for r in RiteId::ALL {
        let id = ContentId::Rite(r);
        if revealed(state, id) {
            rites.push(Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::BuyRite(r), CountSemantics::RepeatN),
                heavy: false,
            });
        }
    }
    out.extend(headed_group(state, "rites", rites));
    out
}

fn wonders_rows(state: &GameState) -> Vec<Row> {
    let mut out = headed_group(
        state,
        "things to do",
        vec![
            Row {
                key: RowKey::Fixed("HoldParade"),
                kind: act(Command::HoldParade, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("RefineStarshard"),
                kind: act(Command::RefineStarshard, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("RefineEcho"),
                kind: act(Command::RefineEcho, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("CondenseJar"),
                kind: act(Command::CondenseJar, CountSemantics::Single),
                heavy: false,
            },
            Row {
                key: RowKey::Fixed("MakePromise"),
                kind: act(Command::MakePromise, CountSemantics::Single),
                heavy: false,
            },
        ],
    );

    let mut arches = Vec::new();
    for a in ArchId::ALL {
        let id = ContentId::Arch(a);
        if revealed(state, id) {
            arches.push(Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::BuyArch(a), CountSemantics::RepeatN),
                heavy: false,
            });
        }
    }
    out.extend(headed_group(state, "arches", arches));
    out
}

fn friends_rows(state: &GameState) -> Vec<Row> {
    let mut colonies = Vec::new();
    for c in ColonyId::ALL {
        if c == ColonyId::StarWhales || !state.discovered.contains(&c) {
            continue;
        }
        colonies.push(Row {
            key: RowKey::Colony(c, "trade"),
            kind: act(Command::Trade(c, 1), CountSemantics::CountTaking),
            heavy: false,
        });
        colonies.push(Row {
            key: RowKey::Colony(c, "penpal"),
            kind: act(Command::SendPenPal(c), CountSemantics::Single),
            heavy: false,
        });
    }
    let mut out = headed_group(state, "colonies", colonies);

    out.extend(headed_group(
        state,
        "things to do",
        vec![Row {
            key: RowKey::Fixed("ScoutPicnic"),
            kind: act(Command::ScoutPicnic, CountSemantics::Single),
            heavy: false,
        }],
    ));
    out
}

fn sky_rows(state: &GameState) -> Vec<Row> {
    let mut sites = Vec::new();
    for site in SiteId::ALL {
        if state.site_reachable(site) {
            sites.push(Row {
                key: RowKey::Site(site),
                kind: act(Command::LaunchExpedition(site), CountSemantics::Single),
                heavy: false,
            });
        }
    }
    let mut out = headed_group(state, "sites", sites);

    let mut verbs = vec![Row {
        key: RowKey::Fixed("SummonStarWhale"),
        kind: act(Command::SummonStarWhale, CountSemantics::Single),
        heavy: false,
    }];
    if state.whale_visit_years > 0 {
        verbs.push(Row {
            key: RowKey::Fixed("FeedWhale"),
            kind: act(Command::FeedWhale, CountSemantics::Single),
            heavy: false,
        });
    }
    out.extend(headed_group(state, "things to do", verbs));
    out
}

fn dreams_rows(state: &GameState) -> Vec<Row> {
    let mut out = headed_group(
        state,
        "things to do",
        vec![Row {
            key: RowKey::Fixed("DeepNap"),
            kind: act(Command::DeepNap(1), CountSemantics::CountTaking),
            // TUI-NAV-015 names four heavy actions and this is one: it skips
            // whole years. `heavy` is the arming contract — false here would
            // mean a held Enter would commit it outright.
            heavy: true,
        }],
    );

    let mut looms = Vec::new();
    for id in DreamLoomId::ALL {
        let cid = ContentId::DreamLoom(id);
        if revealed(state, cid) {
            looms.push(Row {
                key: RowKey::Content(cid, ""),
                kind: act(Command::BuyDreamLoom(id), CountSemantics::RepeatN),
                heavy: false,
            });
        }
    }
    out.extend(headed_group(state, "loom upgrades", looms));
    out
}

fn legacy_rows(state: &GameState) -> Vec<Row> {
    let mut out = headed_group(
        state,
        "things to do",
        vec![
            Row {
                key: RowKey::Fixed("GreatMeadowMove"),
                kind: act(Command::GreatMeadowMove, CountSemantics::Single),
                heavy: true,
            },
            Row {
                key: RowKey::Fixed("PassItDown"),
                kind: act(Command::PassItDown, CountSemantics::Single),
                heavy: true,
            },
        ],
    );

    let mut traditions = Vec::new();
    for t in TraditionId::ALL {
        let id = ContentId::Tradition(t);
        if revealed(state, id) {
            traditions.push(Row {
                key: RowKey::Content(id, ""),
                kind: act(Command::BuyTradition(t), CountSemantics::RepeatN),
                heavy: false,
            });
        }
    }
    out.extend(headed_group(state, "traditions", traditions));

    let adventures_open = state
        .traditions
        .contains(&clowder_core::TraditionId::AdventurousSpirit)
        || state.ideas.contains(&IdeaId::AdventurousSpirit);
    let mut adventures = Vec::new();
    if adventures_open {
        adventures.push(Row {
            key: RowKey::Adventure(None),
            kind: act(Command::DeclareAdventure(None), CountSemantics::Single),
            heavy: false,
        });
        for a in AdventureId::ALL {
            adventures.push(Row {
                key: RowKey::Adventure(Some(a)),
                kind: act(Command::DeclareAdventure(Some(a)), CountSemantics::Single),
                heavy: false,
            });
        }
    }
    out.extend(headed_group(state, "adventures", adventures));
    out
}

/// Which sections have at least one revealed row, in digit order — the tab bar
/// exists only where content does. @spec TUI-NAV-006
pub fn revealed_sections(state: &GameState) -> Vec<Section> {
    Section::ALL
        .into_iter()
        .filter(|s| !rows(state, *s).is_empty())
        .collect()
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
    Section::ALL
        .into_iter()
        .filter(|s| has_tab(state, focus, *s))
        .collect()
}

/// The list index the cursor sits at: the remembered row's position where it
/// is still listed, or else the list's first selectable row — never index 0
/// outright, since a group header may stand there and the cursor never lands
/// on one (TUI-NAV-032, TUI-NAV-099). `None` where nothing in the list is
/// selectable.
pub fn cursor_index(list: &[Row], key: Option<&RowKey>) -> Option<usize> {
    key.and_then(|k| list.iter().position(|r| &r.key == k && r.selectable()))
        .or_else(|| list.iter().position(|r| r.selectable()))
}

/// Whether `key` names a row still listed anywhere — every section, not only
/// the one it last lived in, since an arming carries no section of its own
/// and a row's identity is unique across the whole app. Used to end an
/// arming whose row has left its list. @spec TUI-NAV-076
pub fn row_still_listed(state: &GameState, key: &RowKey) -> bool {
    Section::ALL
        .iter()
        .any(|s| rows(state, *s).iter().any(|r| &r.key == key))
}
