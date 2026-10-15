//! The inspector's lock reason in plain words (TUI-NAV-079).
//!
//! The core defines six reasons an item cannot be taken now (SIM-ENG-031), and
//! the inspector states each in its own words, exactly where the core reports
//! one, and only on the row that takes the item. Two of the six — a prerequisite
//! unmet, and no room — are reasons no listed row meets: every kind whose
//! prerequisite could hold is hidden from its list until it is met, and no kind
//! the lists carry has seats to run out of. Their wordings can still be read,
//! and must still be distinct, so the wording of a reason is read through the
//! one function that decides it, pinned here as
//!
//! ```ignore
//! clowder_tui::render::widgets::inspector::lock_reason_text(&LockReason) -> String
//! ```
//!
//! — the plain words the inspector states for that reason, the same words the
//! drawing carries. Every wording test reads that function; every placement
//! test reads the drawing and looks for those words in it. Kept in their own
//! binary, these fail to compile until that function lands without taking the
//! existing inspector suite down with them.
//!
//! What the words *are* is still nobody's to pin. What is pinned is what they
//! must and must not carry: a shortfall's amounts and components, and nothing
//! further for the reasons that say nothing further.

use std::collections::BTreeSet;

use clowder_core::content as tables;
use clowder_core::{
    ArchId, ContentId, CraftId, DreamLoomId, GameState, IdeaId, JobId, Kitten, KittenTrait,
    Location, LockReason, RefinementId, ResourceId, RiteId, RuleId, SiteId, StructureGate,
    StructureId, TraditionId,
};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, Row};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::fmt;
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::inspector::{self, lock_reason_text};

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

const WIDE: u16 = 200;
const TALL: u16 = 40;

/// The view a player could be in with the same row under the cursor: where
/// that row is an Ideas row folded into the done group, the group opened by
/// its key first, the focus kept on the row (TUI-NAV-113).
fn reachable(state: &GameState, view: &ViewState) -> ViewState {
    let mut v = view.clone();
    let Some(section) = v.section else { return v };
    let Some(key) = v.focus[section.index()].clone() else {
        return v;
    };
    if clowder_tui::nav::rows::drawn_rows(state, section, &v)
        .iter()
        .any(|r| r.key == key)
        || !rows(state, section).iter().any(|r| r.key == key)
    {
        return v;
    }
    let d = clowder_tui::nav::keys::resolve(
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ),
        &v,
    );
    clowder_tui::nav::update(d, &mut v, state);
    v.last_repeat = None;
    v.focus[section.index()] = Some(key);
    v
}

fn pane_text(state: &GameState, view: &ViewState) -> String {
    let rect = match compute(Rect::new(0, 0, WIDE, TALL), None, false).body {
        Body::Wide { inspector, .. } => inspector,
        other => {
            panic!("fixture: {WIDE}x{TALL} should lay out three columns, but laid out {other:?}")
        }
    };
    let view = &reachable(state, view);
    let mut term = Terminal::new(TestBackend::new(WIDE, TALL)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, view))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whitespace collapsed, so words wrapped across the pane read whole.
fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn numbers(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || c == '.' {
            cur.push(c);
        } else {
            let token = cur.trim_matches('.').to_string();
            if !token.is_empty() {
                out.insert(token);
            }
            cur.clear();
        }
    }
    out
}

fn words(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_alphabetic() {
            cur.extend(c.to_lowercase());
        } else {
            if cur.chars().count() >= 3 {
                out.insert(cur.clone());
            }
            cur.clear();
        }
    }
    out
}

fn spaced(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn resource_words(r: ResourceId) -> BTreeSet<String> {
    words(&spaced(&format!("{r:?}")))
}

fn bare(x: f64) -> String {
    fmt::exact(x).trim_start_matches('-').to_string()
}

/// One example of every reason the core defines. The match has no wildcard: a
/// reason the core adds is a reason this list must name, so the set of wordings
/// tested is always the core's whole set.
fn every_reason() -> Vec<LockReason> {
    let sample = LockReason::ShortBy {
        missing: vec![(ResourceId::ALL[0], 12.5)],
    };
    let all = vec![
        LockReason::NotYetReached,
        LockReason::Prerequisite,
        LockReason::AlreadyYours,
        LockReason::NoOneFree,
        LockReason::NoRoom,
        sample,
    ];
    for r in &all {
        match r {
            LockReason::NotYetReached
            | LockReason::Prerequisite
            | LockReason::AlreadyYours
            | LockReason::NoOneFree
            | LockReason::NoRoom
            | LockReason::ShortBy { .. } => {}
        }
    }
    all
}

/// The wordings of every reason that carries no figures, plus the shortfall
/// naming no component — what must be absent wherever the core reports none.
fn fixed_wordings() -> Vec<String> {
    [
        LockReason::NotYetReached,
        LockReason::Prerequisite,
        LockReason::AlreadyYours,
        LockReason::NoOneFree,
        LockReason::NoRoom,
        LockReason::ShortBy {
            missing: Vec::new(),
        },
    ]
    .iter()
    .map(|r| flat(&lock_reason_text(r)))
    .collect()
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(i: usize) -> Kitten {
    Kitten {
        name: format!("kitten-{i}"),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// A meadow where every kind of row is listed, stocks distinct per resource.
/// With `ideas_held`, every idea is researched and one item of each kind held
/// once is already the player's; without, the ideas are revealed only.
fn world(ideas_held: bool) -> GameState {
    let mut s = GameState::new(11);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1_000.5 + r.ix() as f64 * 0.25;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.revealed = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .chain(IdeaId::ALL.into_iter().map(ContentId::Idea))
        .collect();
    s.generation = 1;
    s.peak_devotion = 1.0e12;
    s.kittens = (0..6).map(kitten).collect();
    if ideas_held {
        s.ideas = IdeaId::ALL.into_iter().collect();
        s.sites_reached = SiteId::ALL.into_iter().collect();
        s.arches_bought.insert(ArchId::ALL[0]);
        s.dream_looms.insert(DreamLoomId::ALL[0]);
        s.traditions.insert(TraditionId::ALL[0]);
        s.refinements.insert(RefinementId::ALL[0]);
        s.rules.insert(RuleId::ALL[0]);
        s.rite_counts[RiteId::ALL[0].ix()] = 1;
    }
    s
}

/// `world(true)` with every kitten at work — so every job's row meets the
/// core's no-one-free reason — and the recipes' inputs spent, so every craft
/// is short.
fn busy_world() -> GameState {
    let mut s = world(true);
    let job = JobId::ALL[0];
    for k in s.kittens.iter_mut() {
        k.job = Some(job);
    }
    for c in CraftId::ALL {
        for (r, _) in tables::craft_def(c).inputs {
            s.resources[r.ix()] = 0.0;
        }
    }
    s
}

fn every_row(state: &GameState) -> Vec<(Section, Row)> {
    Section::ALL
        .into_iter()
        .flat_map(|section| {
            rows(state, section)
                .into_iter()
                .filter(|r| r.selectable())
                .map(move |r| (section, r))
        })
        .collect()
}

fn focused(section: Section, key: RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(section),
        ..Default::default()
    };
    v.focus[section.index()] = Some(key);
    v
}

// ---------------------------------------------------------------------------
// The wordings
// ---------------------------------------------------------------------------

/// Each of the core's six reasons reads in its own words: no two alike, so
/// "why can't I?" is never answered with "you can't". A shortfall naming no
/// component is still a shortfall, worded apart from the other five.
/// @spec TUI-NAV-079
#[test]
fn each_of_the_cores_six_reasons_reads_in_its_own_words() {
    let mut texts: Vec<(String, String)> = every_reason()
        .iter()
        .map(|r| (format!("{r:?}"), flat(&lock_reason_text(r))))
        .collect();
    for (reason, text) in &texts {
        assert!(
            !text.is_empty(),
            "{reason} must be stated in words; it read nothing"
        );
    }
    let unnamed = flat(&lock_reason_text(&LockReason::ShortBy {
        missing: Vec::new(),
    }));
    assert!(
        !unnamed.is_empty(),
        "a shortfall naming no component must still be stated"
    );
    for (reason, text) in texts.iter().filter(|(r, _)| !r.starts_with("ShortBy")) {
        assert_ne!(
            &unnamed, text,
            "a shortfall naming no component reads the same as {reason}: {text:?}"
        );
    }
    texts.sort_by(|a, b| a.1.cmp(&b.1));
    for pair in texts.windows(2) {
        assert_ne!(
            pair[0].1, pair[1].1,
            "{} and {} read alike ({:?}); each of the core's reasons has its own wording",
            pair[0].0, pair[1].0, pair[0].1
        );
    }
}

/// A shortfall names each component the core reports short and the amount it
/// is short by, in the exact form. Quantified over every resource the core can
/// name, each short by a distinct amount.
/// @spec TUI-NAV-079
#[test]
fn a_shortfall_names_every_short_component_and_the_amount_it_is_short_by() {
    let missing: Vec<(ResourceId, f64)> = ResourceId::ALL
        .into_iter()
        .map(|r| (r, 0.5 + r.ix() as f64 * 1.25))
        .collect();
    let text = lock_reason_text(&LockReason::ShortBy {
        missing: missing.clone(),
    });
    let said_numbers = numbers(&text);
    let said_words = words(&text);
    for (r, amount) in missing {
        assert!(
            said_numbers.contains(&bare(amount)),
            "{r:?} is short by {}, and the shortfall must say so; it read {text:?}",
            bare(amount)
        );
        for w in resource_words(r) {
            assert!(
                said_words.contains(&w),
                "the shortfall must name {r:?}; it read {text:?}"
            );
        }
    }
}

/// A shortfall the core reports naming no component — a price outside the
/// resource vocabulary — says the price is not yet met and names no amount.
/// @spec TUI-NAV-079
#[test]
fn a_shortfall_naming_no_component_says_the_price_is_unmet_and_names_no_amount() {
    let text = lock_reason_text(&LockReason::ShortBy {
        missing: Vec::new(),
    });
    assert!(!flat(&text).is_empty(), "the unmet price must be stated");
    assert!(
        !text.chars().any(|c| c.is_ascii_digit()),
        "the core names no amount, so the inspector states none; it read {text:?}"
    );
}

/// Not-yet-reached says only that — no figure, no resource, nothing of a price
/// or gate the player has not reached (SIM-ENG-037). A prerequisite unmet is
/// said without naming it: the core names none, and the shell may not work it
/// out from the gates.
/// @spec TUI-NAV-079
#[test]
fn not_yet_reached_and_a_prerequisite_name_nothing_further() {
    let all_resource_words: BTreeSet<String> = ResourceId::ALL
        .into_iter()
        .flat_map(resource_words)
        .collect();
    for reason in [LockReason::NotYetReached, LockReason::Prerequisite] {
        let text = lock_reason_text(&reason);
        assert!(
            !text.chars().any(|c| c.is_ascii_digit()),
            "{reason:?} names no figure; it read {text:?}"
        );
        let said = words(&text);
        let named: Vec<&String> = said.intersection(&all_resource_words).collect();
        assert!(
            named.is_empty(),
            "{reason:?} names no resource ({named:?}); it read {text:?}"
        );
    }
    let prereq = lock_reason_text(&LockReason::Prerequisite);
    let names = IdeaId::ALL
        .into_iter()
        .map(|i| tables::idea_def(i).name)
        .chain(
            StructureId::ALL
                .into_iter()
                .map(|s| tables::structure_def(s).name),
        )
        .chain(CraftId::ALL.into_iter().map(|c| tables::craft_def(c).name))
        .chain(JobId::ALL.into_iter().map(|j| tables::job_def(j).name));
    for name in names {
        assert!(
            !prereq.contains(name),
            "the prerequisite reason names no prerequisite, but it read {prereq:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Where the reason is stated
// ---------------------------------------------------------------------------

/// The reason is stated exactly where the core reports one, and only on a row
/// that takes the item — a build, research, purchase, or make row, or a job's
/// count row. Where the core reports none, none is stated; a craft's tinkerer
/// count row and every row with no content item state none. Quantified over
/// every row of a full meadow in four states: ideas researched, ideas only
/// revealed, every kitten at work with the recipes' inputs spent, and a
/// structure whose stock sits a hair under its need, which the core's gate
/// accepts.
/// @spec TUI-NAV-079
#[test]
fn the_reason_is_stated_exactly_where_the_core_reports_one_on_the_row_that_takes_the_item() {
    let fixed = fixed_wordings();
    let mut stated = BTreeSet::new();
    let mut tinker_reasoned = 0;
    for state in [world(true), world(false), busy_world()] {
        for (section, row) in every_row(&state) {
            let shown = flat(&pane_text(&state, &focused(section, row.key.clone())));
            let takes = match &row.key {
                RowKey::Content(id, tag) if *tag != "tinker" => Some(*id),
                _ => None,
            };
            match takes.and_then(|id| state.lock_reason(id)) {
                Some(reason) => {
                    let words = flat(&lock_reason_text(&reason));
                    assert!(
                        shown.contains(&words),
                        "{:?}: the core reports {reason:?}, and the inspector must state it as \
                         {words:?}; it read:\n{shown}",
                        row.key
                    );
                    stated.insert(
                        format!("{reason:?}")
                            .split([' ', '{'])
                            .next()
                            .unwrap_or_default()
                            .to_string(),
                    );
                }
                None => {
                    for w in &fixed {
                        assert!(
                            !shown.contains(w.as_str()),
                            "{:?}: the core reports no reason here, yet the inspector says {w:?}; \
                             it read:\n{shown}",
                            row.key
                        );
                    }
                    if let RowKey::Content(id, "tinker") = &row.key {
                        if let Some(reason) = state.lock_reason(*id) {
                            tinker_reasoned += 1;
                            let words = flat(&lock_reason_text(&reason));
                            assert!(
                                !shown.contains(&words),
                                "{:?}: a tinkerer count row binds rather than makes, so the \
                                 recipe's reason {words:?} is not stated on it; it read:\n{shown}",
                                row.key
                            );
                        }
                    }
                }
            }
        }
    }
    for reachable in ["NotYetReached", "AlreadyYours", "NoOneFree", "ShortBy"] {
        assert!(
            stated.contains(reachable),
            "fixture: some listed row meets {reachable} ({stated:?})"
        );
    }
    assert!(
        tinker_reasoned > 0,
        "fixture: some recipe the tinker rows bind is held back"
    );

    // A hair under the need: the core's gate accepts it and reports nothing,
    // so nothing is stated — least of all a shortfall of that hair.
    let s = StructureId::ALL
        .into_iter()
        .find(|s| {
            matches!(tables::structure_def(*s).gate, StructureGate::Always)
                && !tables::structure_def(*s).base_costs.is_empty()
        })
        .expect("fixture: some ungated structure has a price");
    let mut hair = GameState::new(11);
    hair.revealed.insert(ContentId::Structure(s));
    let cost = hair.next_cost(s);
    for (r, need) in &cost {
        hair.resources[r.ix()] = need + 100.5;
    }
    let (r0, need0) = cost[0];
    hair.resources[r0.ix()] = need0 - 1.0e-10;
    assert_eq!(
        hair.lock_reason(ContentId::Structure(s)),
        None,
        "fixture: the gate accepts a hair under"
    );
    let key = RowKey::Content(ContentId::Structure(s), "");
    let shown = flat(&pane_text(&hair, &focused(Section::Meadow, key)));
    let shell_shortfall = flat(&lock_reason_text(&LockReason::ShortBy {
        missing: vec![(r0, need0 - hair.stock(r0))],
    }));
    assert!(
        !shown.contains(&shell_shortfall),
        "the core reports no reason, so no shortfall is inferred from stock; it read:\n{shown}"
    );
    for w in &fixed {
        assert!(
            !shown.contains(w.as_str()),
            "the core reports no reason; it read:\n{shown}"
        );
    }
}
