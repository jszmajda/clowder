//! The row builder — `nav::rows::rows`, the single source of a section's list.
//!
//! Both the input path and the draw path read this one function, so a row the
//! cursor can reach is exactly a row that gets drawn. That is why these tests
//! assert on *sets of rows* rather than on rendered text: at this layer a row
//! is an identity (`RowKey`), a verb (`RowKind`), and a weight (`heavy`) — the
//! marker glyph, flavour name, owned count and next cost are all derived at
//! draw time from that identity plus a core query, which is precisely what
//! TUI-NAV-053 demands. Nothing here may be computed shell-side.
//!
//! `clowder-tui` is a binary with no library target, so the module under test
//! is included by path. `nav/mod.rs` is included whole rather than
//! `nav/rows.rs` alone, because `rows.rs` reaches its siblings through
//! `super::` — including the directory's `mod.rs` is what makes those paths
//! resolve without editing the implementation to suit the test.

use clowder_tui::nav;

use std::collections::{BTreeSet, HashMap, HashSet};

use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
use clowder_core::{
    ArchId, Availability, ColonyId, Command, ContentId, CraftId, DreamLoomId, GameState, IdeaId,
    JobId, Kitten, KittenTrait, Location, RefinementId, ResourceId, RiteId, RuleId, SiteId,
    StructureId, TraditionId,
};

use nav::rows::{drawn_rows, lists_selectable, rows, tab_sections, Row, RowKey, RowKind};
use nav::{CountSemantics, Section, ViewState};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ---------------------------------------------------------------------------
// The command census
// ---------------------------------------------------------------------------

/// Every `Command` variant, named. The match is exhaustive on purpose: adding a
/// variant to the core breaks this file's compilation, which is the forcing
/// function that keeps TUI-NAV-039's coverage claim honest as the game grows.
fn variant(c: &Command) -> &'static str {
    match c {
        Command::GatherCatnip => "GatherCatnip",
        Command::Forage => "Forage",
        Command::Rummage => "Rummage",
        Command::Build(_) => "Build",
        Command::AssignJob(_) => "AssignJob",
        Command::UnassignJob(_) => "UnassignJob",
        Command::ResearchIdea(_) => "ResearchIdea",
        Command::Craft(_) => "Craft",
        Command::ObserveSky => "ObserveSky",
        Command::SetSpeed(_) => "SetSpeed",
        Command::Pounce(_) => "Pounce",
        Command::HostJamboree => "HostJamboree",
        Command::Promote(_) => "Promote",
        Command::AppointElder(_) => "AppointElder",
        Command::BuyRefinement(_) => "BuyRefinement",
        Command::AdoptRule(_) => "AdoptRule",
        Command::AssignTinkerer(_) => "AssignTinkerer",
        Command::UnassignTinkerer(_) => "UnassignTinkerer",
        Command::Bask => "Bask",
        Command::Reverie => "Reverie",
        Command::BuyRite(_) => "BuyRite",
        Command::Enlighten => "Enlighten",
        Command::HoldParade => "HoldParade",
        Command::BuyArch(_) => "BuyArch",
        Command::RefineStarshard => "RefineStarshard",
        Command::RefineEcho => "RefineEcho",
        Command::CondenseJar => "CondenseJar",
        Command::MakePromise => "MakePromise",
        Command::Trade(_, _) => "Trade",
        Command::SendPenPal(_) => "SendPenPal",
        Command::ScoutPicnic => "ScoutPicnic",
        Command::GreatMeadowMove => "GreatMeadowMove",
        Command::PassItDown => "PassItDown",
        Command::BuyTradition(_) => "BuyTradition",
        Command::DeclareAdventure(_) => "DeclareAdventure",
        Command::LaunchExpedition(_) => "LaunchExpedition",
        Command::SummonStarWhale => "SummonStarWhale",
        Command::FeedWhale => "FeedWhale",
        Command::DeepNap(_) => "DeepNap",
        Command::BuyDreamLoom(_) => "BuyDreamLoom",
    }
}

/// One instance of every `Command` variant. Paired with the exhaustive match in
/// `variant`, this is the enumeration TUI-NAV-039's coverage claim quantifies
/// over.
fn one_of_every_command_variant() -> Vec<Command> {
    vec![
        Command::GatherCatnip,
        Command::Forage,
        Command::Rummage,
        Command::Build(StructureId::ALL[0]),
        Command::AssignJob(JobId::ALL[0]),
        Command::UnassignJob(JobId::ALL[0]),
        Command::ResearchIdea(IdeaId::ALL[0]),
        Command::Craft(CraftId::ALL[0]),
        Command::ObserveSky,
        Command::SetSpeed(1.0),
        Command::Pounce(1),
        Command::HostJamboree,
        Command::Promote(0),
        Command::AppointElder(0),
        Command::BuyRefinement(RefinementId::ALL[0]),
        Command::AdoptRule(RuleId::ALL[0]),
        Command::AssignTinkerer(CraftId::ALL[0]),
        Command::UnassignTinkerer(CraftId::ALL[0]),
        Command::Bask,
        Command::Reverie,
        Command::BuyRite(RiteId::ALL[0]),
        Command::Enlighten,
        Command::HoldParade,
        Command::BuyArch(ArchId::ALL[0]),
        Command::RefineStarshard,
        Command::RefineEcho,
        Command::CondenseJar,
        Command::MakePromise,
        Command::Trade(ColonyId::ALL[0], 1),
        Command::SendPenPal(ColonyId::ALL[0]),
        Command::ScoutPicnic,
        Command::GreatMeadowMove,
        Command::PassItDown,
        Command::BuyTradition(TraditionId::ALL[0]),
        Command::DeclareAdventure(None),
        Command::LaunchExpedition(SiteId::ALL[0]),
        Command::SummonStarWhale,
        Command::FeedWhale,
        Command::DeepNap(1),
        Command::BuyDreamLoom(DreamLoomId::ALL[0]),
    ]
}

/// The design's section map: which section hosts each player-facing command.
/// Membership only — a command may legitimately surface in a second place (the
/// whale exchange is routed to Sky while a whale visits), but the section named
/// here must always be able to reach it.
fn sectioned_commands() -> Vec<(&'static str, Section)> {
    vec![
        ("GatherCatnip", Section::Meadow),
        ("Forage", Section::Meadow),
        ("Rummage", Section::Meadow),
        ("Build", Section::Meadow),
        ("AssignJob", Section::Clowder),
        ("UnassignJob", Section::Clowder),
        ("Promote", Section::Clowder),
        ("AppointElder", Section::Clowder),
        ("HostJamboree", Section::Clowder),
        ("Pounce", Section::Clowder),
        ("ResearchIdea", Section::Ideas),
        ("BuyRefinement", Section::Ideas),
        ("AdoptRule", Section::Ideas),
        ("Craft", Section::Crafts),
        ("AssignTinkerer", Section::Crafts),
        ("UnassignTinkerer", Section::Crafts),
        ("Bask", Section::Sunbeams),
        ("Reverie", Section::Sunbeams),
        ("BuyRite", Section::Sunbeams),
        ("Enlighten", Section::Sunbeams),
        ("HoldParade", Section::Wonders),
        ("BuyArch", Section::Wonders),
        ("RefineStarshard", Section::Wonders),
        ("RefineEcho", Section::Wonders),
        ("CondenseJar", Section::Wonders),
        ("MakePromise", Section::Wonders),
        ("Trade", Section::Friends),
        ("SendPenPal", Section::Friends),
        ("ScoutPicnic", Section::Friends),
        ("LaunchExpedition", Section::Sky),
        ("SummonStarWhale", Section::Sky),
        ("FeedWhale", Section::Sky),
        ("DeepNap", Section::Dreams),
        ("BuyDreamLoom", Section::Dreams),
        ("GreatMeadowMove", Section::Legacy),
        ("PassItDown", Section::Legacy),
        ("BuyTradition", Section::Legacy),
        ("DeclareAdventure", Section::Legacy),
    ]
}

/// The two commands that belong to no section. They are reachable by a global
/// key from every screen instead, which is why TUI-NAV-039 exempts them — and
/// why a row offering either would be a bug, not a bonus.
const SECTIONLESS: [&str; 2] = ["SetSpeed", "ObserveSky"];

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(name: &str, job: Option<JobId>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: Location::Home,
        arrived_day: 0,
        job_seq: job.map(|_| 1),
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e6,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// Every surface the core can be told is open, is open — every idea researched,
/// every structure built, every content id revealed, a full roster, a visiting
/// whale, a meadow that has already Moved once.
///
/// This is the state in which TUI-NAV-039's "every sectioned command is a row"
/// is quantified: a command whose row is missing here is missing because the
/// row builder never offers it, not because the fixture forgot to unlock it. A
/// later gate that this fixture does not satisfy belongs *here*, not in a
/// weakened assertion.
fn maximal() -> GameState {
    let mut s = GameState::new(7);

    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    for b in StructureId::ALL {
        s.structures[b.ix()] = 5;
    }
    s.ideas = IdeaId::ALL.iter().copied().collect::<BTreeSet<_>>();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect::<BTreeSet<_>>();

    // A roster large enough for the heaviest action in the game, with idle
    // kittens to assign and bound Tinkerers to release.
    let mut roster: Vec<Kitten> = (0..MEADOW_MOVE_MIN_KITTENS + 9)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    for k in roster.iter_mut().take(4) {
        k.job = Some(JobId::Tinkerer);
    }
    roster[0].tinkering = Some(CraftId::ALL[0]);
    s.kittens = roster;
    s.elder = Some(0);

    // Ceremonies: the worship ratchet is high, and the two rites other
    // ceremonies depend on are held.
    s.peak_devotion = 1.0e12;
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.rites = [RiteId::TheReverie, RiteId::Enlightenment]
        .into_iter()
        .collect::<BTreeSet<_>>();

    // Wonders: the arch that opens jars and promises.
    s.arches_bought = [ArchId::EverdreamArch].into_iter().collect::<BTreeSet<_>>();

    // Friends: every colony found.
    s.discovered = ColonyId::ALL.iter().copied().collect::<BTreeSet<_>>();

    // Sky: no site reached yet, so the first rung is launchable; a whale is
    // visiting, which is what routes the whale rows onto the screen.
    s.sites_reached = BTreeSet::new();
    s.whale_visit_years = 5;
    s.whale_contentment = 1;

    // Legacy: a meadow that has Moved once, with wisdom to spend.
    s.generation = 1;
    s.elder_wisdom = 1.0e9;
    s.warm_memories = 1.0e3;

    s
}

/// Everything unlocked, nothing affordable: the same open surfaces as
/// `maximal`, with the stocks and spendable pools emptied. Worship's
/// high-water mark stays — it is a ratchet the player has already earned, not
/// a stock they can spend away.
fn poor_but_unlocked() -> GameState {
    let mut s = maximal();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 0.0;
    }
    s.elder_wisdom = 0.0;
    s.serenity = 0.0;
    s.devotion = 0.0;
    s
}

/// A brand-new meadow with unlimited stock and nothing unlocked: wealth without
/// progress. Whatever a row's marker says, money must never conjure a row.
fn rich_but_locked() -> GameState {
    let mut s = GameState::new(3);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e12;
    }
    s
}

/// Every content id the core knows, across every kind.
fn every_content_id() -> Vec<ContentId> {
    let mut ids = Vec::new();
    ids.extend(StructureId::ALL.iter().map(|x| ContentId::Structure(*x)));
    ids.extend(IdeaId::ALL.iter().map(|x| ContentId::Idea(*x)));
    ids.extend(CraftId::ALL.iter().map(|x| ContentId::Craft(*x)));
    ids.extend(JobId::ALL.iter().map(|x| ContentId::Job(*x)));
    ids.extend(RiteId::ALL.iter().map(|x| ContentId::Rite(*x)));
    ids.extend(ArchId::ALL.iter().map(|x| ContentId::Arch(*x)));
    ids.extend(RefinementId::ALL.iter().map(|x| ContentId::Refinement(*x)));
    ids.extend(RuleId::ALL.iter().map(|x| ContentId::Rule(*x)));
    ids.extend(TraditionId::ALL.iter().map(|x| ContentId::Tradition(*x)));
    ids.extend(DreamLoomId::ALL.iter().map(|x| ContentId::DreamLoom(*x)));
    ids
}

// ---------------------------------------------------------------------------
// Helpers over a built list
// ---------------------------------------------------------------------------

fn all_rows(state: &GameState) -> Vec<(Section, Row)> {
    Section::ALL
        .iter()
        .flat_map(|s| rows(state, *s).into_iter().map(move |r| (*s, r)))
        .collect()
}

/// Every command a row can fire — the act itself, or both ends of a count row.
fn commands_of(row: &Row) -> Vec<Command> {
    match &row.kind {
        RowKind::Act { command, .. } => vec![*command],
        RowKind::Count {
            increment,
            decrement,
        } => vec![*increment, *decrement],
        RowKind::Display | RowKind::Group => Vec::new(),
    }
}

fn command_names(rows: &[Row]) -> HashSet<&'static str> {
    rows.iter()
        .flat_map(commands_of)
        .map(|c| variant(&c))
        .collect()
}

/// Content ids that appear as a row, anywhere, in section order.
fn content_keys(state: &GameState) -> Vec<(Section, ContentId)> {
    all_rows(state)
        .into_iter()
        .filter_map(|(s, r)| match r.key {
            RowKey::Content(id, _) => Some((s, id)),
            _ => None,
        })
        .collect()
}

/// The home section, addressed the way the player addresses it. Home is digit
/// `1` by the static map; nothing in the game ever renumbers it.
fn home() -> Section {
    let home = Section::from_digit('1').expect("digit 1 addresses a section");
    assert_eq!(
        home,
        Section::Meadow,
        "home — where the stores live — is digit 1"
    );
    assert_eq!(
        home.digit(),
        '1',
        "and home answers to that digit in both directions"
    );
    home
}

fn resource_rows(state: &GameState, section: Section) -> Vec<ResourceId> {
    rows(state, section)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Resource(id) => Some(id),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// TUI-NAV-039 — every sectioned command is reachable as a row
// ---------------------------------------------------------------------------

/// The discoverable backbone: no verb in the game is reachable only through a
/// mnemonic, the palette, or an accelerator key. A player who knows nothing but
/// ↑↓ and Enter can still do everything.
///
/// @spec TUI-NAV-039
#[test]
fn every_sectioned_command_is_reachable_as_a_row_in_its_section() {
    let state = maximal();

    for (name, section) in sectioned_commands() {
        // AppointElder is this table's one exception (TUI-NAV-039): it is
        // reachable only by the row-bound `n` key TUI-NAV-094 states, never by
        // a row of its own; TUI-NAV-094's own tests cover its reachability.
        if name == "AppointElder" {
            continue;
        }
        let offered = command_names(&rows(&state, section));
        assert!(
            offered.contains(name),
            "{name} belongs to the {} section but no row there offers it — \
             with every surface in the game unlocked, it is reachable by nothing but an accelerator",
            section.title()
        );
    }
}

/// The census itself: the section map accounts for every command the core
/// defines, and every command a row offers is one the map placed. Without this,
/// the coverage test above would quietly stop covering a command the day the
/// core gained one.
///
/// @spec TUI-NAV-039
#[test]
fn the_section_map_accounts_for_every_command_variant() {
    let every = one_of_every_command_variant();
    let all_names: HashSet<&str> = every.iter().map(variant).collect();
    assert_eq!(
        all_names.len(),
        every.len(),
        "the command census lists each variant exactly once"
    );

    let mapped: HashSet<&str> = sectioned_commands()
        .into_iter()
        .map(|(n, _)| n)
        .chain(SECTIONLESS)
        .collect();
    assert_eq!(
        all_names, mapped,
        "every command is either placed in a section or explicitly sectionless"
    );

    // And nothing is offered that the map does not know about.
    let sectioned: HashSet<&str> = sectioned_commands().into_iter().map(|(n, _)| n).collect();
    for (section, row) in all_rows(&maximal()) {
        for c in commands_of(&row) {
            assert!(
                sectioned.contains(variant(&c)),
                "the {} section offers {}, which the section map does not place",
                section.title(),
                variant(&c)
            );
        }
    }
}

/// Speed and sky-watching answer to a key from anywhere, not to a row. A row
/// for either would be a second, section-bound way to reach a command that is
/// deliberately global.
///
/// @spec TUI-NAV-039
#[test]
fn the_two_sectionless_commands_are_offered_by_no_row() {
    for state in [
        maximal(),
        poor_but_unlocked(),
        rich_but_locked(),
        GameState::new(5),
    ] {
        for (section, row) in all_rows(&state) {
            for c in commands_of(&row) {
                assert!(
                    !SECTIONLESS.contains(&variant(&c)),
                    "the {} section offers {}, which belongs to no section — it is reached by its global key",
                    section.title(),
                    variant(&c)
                );
            }
        }
    }
}

/// "Declare nothing" is a choice on the list, not the absence of one. TUI-NAV-039
/// names it explicitly: standing down must be as reachable by cursor and Enter
/// as declaring is.
///
/// @spec TUI-NAV-039
#[test]
fn standing_down_is_a_row_on_the_declare_list() {
    let legacy = rows(&maximal(), Section::Legacy);

    assert!(
        legacy.iter().any(|r| matches!(
            &r.kind,
            RowKind::Act {
                command: Command::DeclareAdventure(None),
                ..
            }
        )),
        "the declare list offers no stand-down row, so declaring nothing is reachable only by Esc"
    );
    assert!(
        legacy.iter().any(|r| matches!(
            &r.kind,
            RowKind::Act { command: Command::DeclareAdventure(Some(_)), .. }
        )),
        "fixture: the declare list should also offer real adventures, so standing down is a choice among choices"
    );
}

/// Unassigning is the `-` on a count row — TUI-NAV-039 names it as one of the
/// two cases that could otherwise be missed, because it is a command with no
/// row of its own. Every job and every recipe the core reports as open must
/// carry both ends of its count row, and both ends must address the same thing:
/// a `+` and a `-` that disagree about which job they mean is worse than no
/// row at all.
///
/// @spec TUI-NAV-039
#[test]
fn assign_and_unassign_are_the_two_ends_of_one_count_row() {
    let state = maximal();

    let clowder = rows(&state, Section::Clowder);
    for j in JobId::ALL {
        if state.availability(ContentId::Job(j)) == Availability::Hidden {
            continue;
        }
        assert!(
            clowder.iter().any(|r| matches!(
                &r.kind,
                RowKind::Count {
                    increment: Command::AssignJob(a),
                    decrement: Command::UnassignJob(b),
                } if a == b && *a == j
            )),
            "the job {j:?} has no count row pairing assign with unassign for that same job"
        );
    }

    let crafts = rows(&state, Section::Crafts);
    for c in CraftId::ALL {
        if state.availability(ContentId::Craft(c)) == Availability::Hidden {
            continue;
        }
        assert!(
            crafts.iter().any(|r| matches!(
                &r.kind,
                RowKind::Count {
                    increment: Command::AssignTinkerer(a),
                    decrement: Command::UnassignTinkerer(b),
                } if a == b && *a == c
            )),
            "the recipe {c:?} has no tinkerer count row pairing assign with unassign for that same recipe"
        );
    }
}

/// One builder, read twice, gives one list. Input resolution and drawing both
/// call this function on the same state within a frame; if it were not a pure
/// function of state, the row the cursor acted on and the row the player saw
/// could differ.
///
/// @spec TUI-NAV-039
#[test]
fn the_row_list_is_a_pure_function_of_state() {
    let state = maximal();
    for section in Section::ALL {
        let first = rows(&state, section);
        let second = rows(&state, section);
        assert_eq!(
            first,
            second,
            "two reads of the {} list disagree; the input path and the draw path would drift within a frame",
            section.title()
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-031 — count-taking vs repeat-N
// ---------------------------------------------------------------------------

/// The count is a real argument for exactly three commands — squads, caravans,
/// years — and for no others. The tag is not a hint: `x` passes N *into* a
/// count-taking command and repeat-*calls* everything else, so mistagging a
/// command either drops the count or fires it N times.
///
/// The reverse direction is the load-bearing half: no other command has
/// anywhere to put a count.
///
/// @spec TUI-NAV-031
#[test]
fn count_taking_is_exactly_the_three_commands_that_take_a_count_argument() {
    let state = maximal();
    let mut seen: HashSet<&str> = HashSet::new();

    for (section, row) in all_rows(&state) {
        let RowKind::Act { command, semantics } = &row.kind else {
            continue;
        };
        let takes_a_count = matches!(
            command,
            Command::Pounce(_) | Command::Trade(_, _) | Command::DeepNap(_)
        );
        if takes_a_count {
            seen.insert(variant(command));
        }
        assert_eq!(
            *semantics == CountSemantics::CountTaking,
            takes_a_count,
            "the {} section tags {} as {:?}; the count is {}a real argument of that command",
            section.title(),
            variant(command),
            semantics,
            if takes_a_count { "" } else { "not " }
        );
    }

    assert_eq!(
        seen,
        HashSet::from(["Pounce", "Trade", "DeepNap"]),
        "fixture: all three count-taking commands must actually appear, or the tagging assertion is vacuous"
    );
}

/// The shell calls these N times rather than passing N — partial success is the
/// intent, and the core's per-call all-or-nothing keeps each one clean.
///
/// @spec TUI-NAV-031
#[test]
fn building_and_crafting_repeat_rather_than_pass_a_count() {
    let state = maximal();
    let mut found = 0;

    for (section, row) in all_rows(&state) {
        let RowKind::Act { command, semantics } = &row.kind else {
            continue;
        };
        if !matches!(command, Command::Build(_) | Command::Craft(_)) {
            continue;
        }
        found += 1;
        assert_eq!(
            *semantics,
            CountSemantics::RepeatN,
            "the {} section tags {} as {:?}; the shell repeats it N times, it does not pass N",
            section.title(),
            variant(command),
            semantics
        );
    }

    assert!(
        found > 0,
        "fixture: build and craft rows must be present for this to mean anything"
    );
}

/// A command carries one count tag wherever it is offered. A later chunk reads
/// the tag off whichever row the cursor happens to be on, so two rows for the
/// same command that disagree would make `x` mean different things depending on
/// how the player navigated there.
///
/// @spec TUI-NAV-031
#[test]
fn a_command_carries_the_same_count_tag_wherever_it_appears() {
    let mut tags: HashMap<&'static str, (CountSemantics, Section)> = HashMap::new();

    for (section, row) in all_rows(&maximal()) {
        let RowKind::Act { command, semantics } = &row.kind else {
            continue;
        };
        let name = variant(command);
        match tags.get(name) {
            Some((first, first_section)) => assert_eq!(
                first,
                semantics,
                "{name} is tagged {:?} in {} and {:?} in {}",
                first,
                first_section.title(),
                semantics,
                section.title()
            ),
            None => {
                tags.insert(name, (*semantics, section));
            }
        }
    }

    assert!(!tags.is_empty(), "fixture: some actionable rows must exist");
}

/// The tag the spec names for a command: count-taking for the three whose
/// count is an argument, repeat-N for building, crafting, and every buy
/// command, and single for every other command.
fn tag_the_spec_names(command: &Command) -> CountSemantics {
    match command {
        Command::Pounce(_) | Command::Trade(_, _) | Command::DeepNap(_) => {
            CountSemantics::CountTaking
        }
        Command::Build(_)
        | Command::Craft(_)
        | Command::BuyRefinement(_)
        | Command::BuyRite(_)
        | Command::BuyArch(_)
        | Command::BuyDreamLoom(_)
        | Command::BuyTradition(_) => CountSemantics::RepeatN,
        _ => CountSemantics::Single,
    }
}

/// Every action row carries exactly the tag the spec names for its command —
/// count-taking, repeat-N, or single — in every meadow the row can be listed
/// in, so the tag is the command's and never the state's. The repeat-N set is
/// building, crafting, and every buy command: how many copies of an item a buy
/// takes is the core's per-item refusal to decide, so a buy row is repeat-called
/// rather than tagged single. Every command the spec names under count-taking
/// and repeat-N must actually be offered by some row, or the quantifier is
/// vacuous for it.
///
/// @spec TUI-NAV-031
#[test]
fn every_action_row_carries_exactly_the_tag_the_spec_names_for_its_command() {
    let named: HashSet<&str> = HashSet::from([
        "Pounce",
        "Trade",
        "DeepNap",
        "Build",
        "Craft",
        "BuyRefinement",
        "BuyRite",
        "BuyArch",
        "BuyDreamLoom",
        "BuyTradition",
    ]);
    let mut seen: HashSet<&str> = HashSet::new();
    let mut singles = 0;

    for (label, state) in [
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
        ("wealth without progress", rich_but_locked()),
        ("a brand-new meadow", GameState::new(5)),
    ] {
        for (section, row) in all_rows(&state) {
            let RowKind::Act { command, semantics } = &row.kind else {
                continue;
            };
            let expected = tag_the_spec_names(command);
            if named.contains(variant(command)) {
                seen.insert(variant(command));
            } else {
                singles += 1;
            }
            assert_eq!(
                *semantics,
                expected,
                "in {label}, the {} section tags {} as {:?}; the spec names it {:?}",
                section.title(),
                variant(command),
                semantics,
                expected
            );
        }
    }

    assert_eq!(
        seen, named,
        "fixture: every command the spec tags count-taking or repeat-N must be offered by some \
         row, or the tagging assertion says nothing about it"
    );
    assert!(
        singles > 0,
        "fixture: some single-tagged row must be listed"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-053 — the core answers; the shell renders
// ---------------------------------------------------------------------------

/// Hidden content is absent entirely — not dimmed, not greyed, not a silhouette.
/// The core decides what is hidden, and no row may name something it calls so.
///
/// @spec TUI-NAV-053, TUI-NAV-009
#[test]
fn no_row_names_content_the_core_calls_hidden() {
    for (label, state) in [
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
        ("wealth without progress", rich_but_locked()),
        ("a brand-new meadow", GameState::new(5)),
    ] {
        for (section, id) in content_keys(&state) {
            assert_ne!(
                state.availability(id),
                Availability::Hidden,
                "in {label}, the {} section lists {id:?}, which the core reports as Hidden",
                section.title()
            );
        }
    }
}

/// The other direction: content the core reports as revealed has a row. Between
/// this and the test above, the row list *is* the core's availability answer —
/// there is no shell-side filter of its own on top.
///
/// @spec TUI-NAV-053
#[test]
fn every_content_the_core_reports_as_revealed_has_a_row() {
    let state = maximal();
    let listed: HashSet<ContentId> = content_keys(&state).into_iter().map(|(_, id)| id).collect();

    for id in every_content_id() {
        if state.availability(id) == Availability::Hidden {
            continue;
        }
        assert!(
            listed.contains(&id),
            "the core reports {id:?} as {:?}, but no section lists it",
            state.availability(id)
        );
    }
}

/// Affordability is a marker, never a filter. Emptying every stock and every
/// spendable pool must leave the list exactly as it was: the same rows, in the
/// same sections, in the same order. If the shell were deciding for itself what
/// the player can afford, this is where the list would shrink.
///
/// @spec TUI-NAV-053
#[test]
fn wealth_changes_the_marker_never_the_list() {
    let rich = maximal();
    let broke = poor_but_unlocked();

    // Fixture check: the two states really do differ in affordability, or the
    // comparison below proves nothing.
    let contrast = every_content_id()
        .into_iter()
        .filter(|id| {
            (rich.affordability(*id) - broke.affordability(*id)).abs() > 1e-9
                || rich.lock_reason(*id) != broke.lock_reason(*id)
        })
        .count();
    assert!(
        contrast > 0,
        "fixture: the rich and broke states must differ in what the core says is affordable"
    );

    let rich_keys = content_keys(&rich);
    let broke_keys = content_keys(&broke);

    let rich_set: HashSet<&(Section, ContentId)> = rich_keys.iter().collect();
    let broke_set: HashSet<&(Section, ContentId)> = broke_keys.iter().collect();
    let vanished: Vec<String> = rich_set
        .difference(&broke_set)
        .map(|k| format!("{:?} in {}", k.1, k.0.title()))
        .collect();
    assert!(
        vanished.is_empty(),
        "these vanished when the player went broke; affordability belongs in the marker, \
         not in what is listed: {vanished:?}"
    );
    let appeared: Vec<String> = broke_set
        .difference(&rich_set)
        .map(|k| format!("{:?} in {}", k.1, k.0.title()))
        .collect();
    assert!(
        appeared.is_empty(),
        "these appeared only once the player was broke: {appeared:?}"
    );

    // Same rows, and in the same places: a list that re-sorted itself as stocks
    // moved would slide rows out from under a cursor that remembers an item by
    // identity.
    assert_eq!(
        rich_keys, broke_keys,
        "the list holds the same rows rich or broke but in a different order; the cursor would jump as stocks move"
    );
}

/// A kind the core answers with no reveal ladder is two-state: there to act on,
/// or not there at all. The shell must not invent a middle tier for it — a
/// dimmed "coming soon" the core never reported would be the shell asserting a
/// rule it does not own.
///
/// @spec TUI-NAV-053
#[test]
fn a_kind_with_no_reveal_ladder_is_present_or_absent_and_never_in_between() {
    for (label, state) in [
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
    ] {
        let listed: HashSet<ContentId> =
            content_keys(&state).into_iter().map(|(_, id)| id).collect();

        let mut two_state = 0;
        for id in every_content_id() {
            if id.is_reveal_tracked() {
                continue;
            }
            two_state += 1;

            let reported = state.availability(id);
            assert_ne!(
                reported,
                Availability::Teased,
                "fixture: in {label} the core reported a middle tier for {id:?}, which carries no reveal ladder"
            );
            assert_eq!(
                listed.contains(&id),
                reported == Availability::Available,
                "in {label}, {id:?} is reported {reported:?} but {} listed",
                if listed.contains(&id) { "is" } else { "is not" }
            );
        }
        assert!(two_state > 0, "fixture: some kinds carry no reveal ladder");
    }
}

/// The rows each content item carries, where, and the command whose gate
/// governs each: the item's own verb, and for a craft its tinkerer count row
/// beside it. A job's count row is gated by its assign end.
fn content_rows_of(id: ContentId) -> Vec<(Section, &'static str, Command)> {
    match id {
        ContentId::Structure(s) => vec![(Section::Meadow, "", Command::Build(s))],
        ContentId::Idea(i) => vec![(Section::Ideas, "", Command::ResearchIdea(i))],
        ContentId::Refinement(r) => vec![(Section::Ideas, "", Command::BuyRefinement(r))],
        ContentId::Rule(r) => vec![(Section::Ideas, "", Command::AdoptRule(r))],
        ContentId::Craft(c) => vec![
            (Section::Crafts, "craft", Command::Craft(c)),
            (Section::Crafts, "tinker", Command::AssignTinkerer(c)),
        ],
        ContentId::Job(j) => vec![(Section::Clowder, "", Command::AssignJob(j))],
        ContentId::Rite(r) => vec![(Section::Sunbeams, "", Command::BuyRite(r))],
        ContentId::Arch(a) => vec![(Section::Wonders, "", Command::BuyArch(a))],
        ContentId::Tradition(t) => vec![(Section::Legacy, "", Command::BuyTradition(t))],
        ContentId::DreamLoom(l) => vec![(Section::Dreams, "", Command::BuyDreamLoom(l))],
    }
}

/// The same meadow with every stock emptied, or filled to the brim — including
/// the spendable pool that is not a resource. Everything else (reveals, gates,
/// ownership) is left exactly as it was.
fn with_every_stock(state: &GameState, amount: f64) -> GameState {
    let mut s = state.clone();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = amount;
    }
    s.elder_wisdom = amount;
    s
}

/// The whole open meadow with no kittens at all: every job the core reports is
/// there to be had, but the gate on assigning one is shut.
fn maximal_without_a_roster() -> GameState {
    let mut s = maximal();
    s.kittens = Vec::new();
    s.elder = None;
    s
}

/// A meadow that has never Moved: the prestige shop's kind answers absent.
fn maximal_first_generation() -> GameState {
    let mut s = maximal();
    s.generation = 0;
    s
}

/// A content row is listed exactly while the core reports its item Teased or
/// Available *and* the gate of the row's own command open — never on the
/// shell's own say-so, and never on stock. Quantified over every content item
/// of every kind, every row each item carries, and a spread of meadows that
/// between them open and shut every one of those conditions; each meadow is
/// also re-asked with its stocks emptied and filled, which must move nothing.
///
/// @spec TUI-NAV-053
#[test]
fn a_content_row_is_listed_exactly_while_the_core_reports_it_revealed_and_its_gate_open() {
    let meadows: Vec<(&str, GameState)> = vec![
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
        ("wealth without progress", rich_but_locked()),
        ("a brand-new meadow", GameState::new(5)),
        ("an open meadow with no kittens", maximal_without_a_roster()),
        (
            "an open meadow that has never Moved",
            maximal_first_generation(),
        ),
    ];

    let mut listed_seen = 0;
    let mut hidden_seen = 0;
    let mut shut_gate_seen = 0;

    for (label, base) in &meadows {
        for (stock_label, state) in [
            ("as built", base.clone()),
            ("with every stock emptied", with_every_stock(base, 0.0)),
            ("with every stock filled", with_every_stock(base, 1.0e12)),
        ] {
            let listed: HashSet<(Section, ContentId, &'static str)> = all_rows(&state)
                .into_iter()
                .filter_map(|(s, r)| match r.key {
                    RowKey::Content(id, tag) => Some((s, id, tag)),
                    _ => None,
                })
                .collect();

            let mut expected_rows: HashSet<(Section, ContentId, &'static str)> = HashSet::new();
            for id in every_content_id() {
                let reported = state.availability(id);
                for (section, tag, command) in content_rows_of(id) {
                    expected_rows.insert((section, id, tag));
                    let revealed =
                        matches!(reported, Availability::Teased | Availability::Available);
                    let gate = state.command_gate_open(&command);
                    let want = revealed && gate;
                    let got = listed.contains(&(section, id, tag));
                    assert_eq!(
                        got,
                        want,
                        "in {label} {stock_label}, the {tag:?} row of {id:?} in {} is {} — but the \
                         core reports the item {reported:?} and the gate of {} {}",
                        section.title(),
                        if got { "listed" } else { "not listed" },
                        variant(&command),
                        if gate { "open" } else { "shut" },
                    );
                    if want {
                        listed_seen += 1;
                    } else if !revealed {
                        hidden_seen += 1;
                    } else {
                        shut_gate_seen += 1;
                    }
                }
            }

            // And nothing else: no content row turns up anywhere the item
            // does not carry one.
            let stray: Vec<String> = listed
                .difference(&expected_rows)
                .map(|(s, id, tag)| format!("{id:?} ({tag:?}) in {}", s.title()))
                .collect();
            assert!(
                stray.is_empty(),
                "in {label} {stock_label}, content rows are listed where the item carries none: {stray:?}"
            );
        }
    }

    assert!(listed_seen > 0, "fixture: some content rows must be listed");
    assert!(
        hidden_seen > 0,
        "fixture: some items must be reported Hidden"
    );
    assert!(
        shut_gate_seen > 0,
        "fixture: some item must be revealed while its command's gate is shut, or the gate half \
         of the claim is not under test"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-053 — the marker each row wears
//
// The marker is derived at draw time, so these tests read it off a drawn
// frame: each row is focused in turn, and the cell after the cursor glyph is
// the marker the player sees.
// ---------------------------------------------------------------------------

/// What one drawn row shows in its marker slot.
struct DrawnMarker {
    /// The marker glyph, or `' '` for an empty slot.
    glyph: char,
    /// The marker cell's style, for the dimming checks.
    style: ratatui::style::Style,
    /// Whether the name starts where every other row's does — one space after
    /// the marker slot — so a blank slot still keeps the names aligned.
    name_aligned: bool,
}

/// Draw `section` with `key` focused, and read the focused row's marker slot.
fn drawn_marker(state: &GameState, section: Section, key: &RowKey) -> DrawnMarker {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let (w, h) = (150u16, 60u16);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut view = nav::ViewState {
        section: Some(section),
        ..Default::default()
    };
    view.focus[section.index()] = Some(key.clone());
    // A finished Ideas row is drawn only while the done group is open
    // (TUI-NAV-113), so it is read with the group opened.
    unfold_focus(state, &mut view);
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    let list = match layout.expect("frame returns its layout").body {
        clowder_tui::render::layout::Body::Wide { list, .. } => list,
        other => panic!("fixture: {w}x{h} lays out a list pane, not {other:?}"),
    };

    let buf = term.backend().buffer();
    let sym = |x: u16, y: u16| -> String {
        buf.cell((x, y))
            .map(|c| c.symbol().to_string())
            .unwrap_or_default()
    };
    for y in list.y..list.bottom() {
        for x in list.x..list.right() {
            if sym(x, y) == "▸" {
                let glyph = sym(x + 2, y).chars().next().unwrap_or(' ');
                let style = buf.cell((x + 2, y)).map(|c| c.style()).unwrap_or_default();
                let name_aligned = sym(x + 3, y) == " " && sym(x + 4, y).trim() != "";
                return DrawnMarker {
                    glyph,
                    style,
                    name_aligned,
                };
            }
        }
    }
    panic!(
        "fixture: no cursor line drawn for {key:?} in {}",
        section.title()
    )
}

/// The marker TUI-NAV-053 gives a listed row, read from nothing but the core's
/// answers: the lock reason decides, and the coverage fraction only splits a
/// shortfall into some or none. `None` is an empty slot.
fn marker_the_core_gives(state: &GameState, row: &Row) -> Option<char> {
    use clowder_core::LockReason;
    match (&row.kind, &row.key) {
        (RowKind::Display, _) => None,
        (_, RowKey::Content(ContentId::Craft(c), "tinker")) => {
            Some(match state.tinkerer_bind_reason(*c) {
                None => '●',
                Some(_) => '○',
            })
        }
        (_, RowKey::Content(id, _)) => Some(match state.lock_reason(*id) {
            Some(LockReason::AlreadyYours) => '✓',
            None => '●',
            Some(LockReason::ShortBy { .. }) => {
                if state.affordability(*id) > 0.0 {
                    '◐'
                } else {
                    '○'
                }
            }
            Some(_) => '○',
        }),
        _ => Some('●'),
    }
}

/// A revealed structure whose price is half in hand: Teased, short, and partly
/// covered — the one state where a dimmed row wears `◐`.
fn teased_and_half_covered() -> GameState {
    for s in StructureId::ALL {
        let id = ContentId::Structure(s);
        let mut state = GameState::new(17);
        state.revealed.insert(id);
        let cost = state.cost_of_next(id, 1);
        if cost.is_empty() {
            continue;
        }
        for (r, amount) in cost {
            state.resources[r.ix()] = amount * 0.5;
        }
        let covered = state.affordability(id);
        if state.availability(id) == Availability::Teased
            && matches!(
                state.lock_reason(id),
                Some(clowder_core::LockReason::ShortBy { .. })
            )
            && covered > 0.0
            && covered < 1.0
        {
            return state;
        }
    }
    panic!("fixture: no structure can be revealed, short, and partly covered on a new meadow")
}

/// The open meadow with every kitten already at work: the jobs are there, cost
/// nothing, and no one is free to take one.
fn maximal_all_hands_busy() -> GameState {
    let mut s = maximal();
    for k in s.kittens.iter_mut() {
        if k.job.is_none() {
            k.job = Some(JobId::ALL[0]);
            k.job_seq = Some(1);
        }
    }
    s
}

/// Every listed row in every section wears exactly the marker the core's
/// answers give it: `✓` where the lock reason is ownership, `●` where there is
/// none, `◐`/`○` by coverage where it is a shortfall, `○` for any other reason
/// whatever the coverage; a craft's tinkerer count row and every actionable
/// row with no content item wear `●`; a display-only row wears none, its slot
/// left blank with the name still aligned. Quantified over every row of every
/// section in a spread of meadows that between them reach every branch — and
/// the test checks that they did.
///
/// @spec TUI-NAV-053
#[test]
fn every_row_wears_the_marker_the_cores_lock_reason_and_coverage_give_it() {
    use clowder_core::LockReason;

    let meadows: Vec<(&str, GameState)> = vec![
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
        (
            "a revealed structure half paid for",
            teased_and_half_covered(),
        ),
        (
            "an open meadow with every kitten at work",
            maximal_all_hands_busy(),
        ),
        ("a brand-new meadow", GameState::new(5)),
    ];

    let mut reached: BTreeSet<&'static str> = BTreeSet::new();
    let mut wrong: Vec<String> = Vec::new();

    for (label, state) in &meadows {
        for (section, row) in all_rows(state) {
            if !row.selectable() {
                continue;
            }
            let want = marker_the_core_gives(state, &row);
            let drawn = drawn_marker(state, section, &row.key);

            match (&row.key, want) {
                (_, None) => {
                    reached.insert("a display-only row");
                    if !drawn.name_aligned {
                        wrong.push(format!(
                            "in {label}, the display row {:?} in {} leaves its name out of line \
                             with the rows around it",
                            row.key,
                            section.title()
                        ));
                    }
                }
                (RowKey::Content(id, "tinker"), _) => {
                    if state.lock_reason(*id).is_some() {
                        reached.insert("a tinkerer row whose craft the core reports locked");
                    }
                }
                (RowKey::Content(id, _), Some(_)) => {
                    let reason = state.lock_reason(*id);
                    let covered = state.affordability(*id);
                    match &reason {
                        Some(LockReason::AlreadyYours) => {
                            reached.insert("ownership");
                        }
                        None => {
                            reached.insert("no lock reason");
                        }
                        Some(LockReason::ShortBy { .. }) if covered > 0.0 => {
                            reached.insert("a shortfall with some coverage");
                            if covered >= 1.0 {
                                reached.insert("a shortfall the coverage reports whole");
                            }
                            if state.availability(*id) == Availability::Teased {
                                reached.insert("a Teased row with some coverage");
                                // A Teased row is dimmed whatever its marker
                                // (TUI-NAV-009): a partly-covered Teased row
                                // still wears the row colour that says
                                // "revealed but not actionable", never the
                                // Partial colour a two-state kind's own
                                // partial coverage wears.
                                let dim = clowder_tui::theme::Role::Dim.color();
                                if drawn.style.fg != dim {
                                    wrong.push(format!(
                                        "in {label}, {:?} in {} is Teased with some coverage but \
                                         is not drawn dim",
                                        row.key,
                                        section.title()
                                    ));
                                }
                            }
                        }
                        Some(LockReason::ShortBy { .. }) => {
                            reached.insert("a shortfall with no coverage");
                        }
                        Some(_) if covered > 0.0 => {
                            reached.insert("another reason, with some coverage");
                        }
                        Some(_) => {}
                    }
                }
                _ => {
                    reached.insert("an actionable row with no content item");
                }
            }

            let want_glyph = want.unwrap_or(' ');
            if drawn.glyph != want_glyph {
                let answers = match &row.key {
                    RowKey::Content(id, _) => format!(
                        " (core: {:?}, lock reason {:?}, coverage {:.3})",
                        state.availability(*id),
                        state.lock_reason(*id),
                        state.affordability(*id)
                    ),
                    _ => String::new(),
                };
                wrong.push(format!(
                    "in {label}, {:?} in {} wears {:?} but the core's answers give {:?}{answers}",
                    row.key,
                    section.title(),
                    drawn.glyph,
                    want_glyph
                ));
            }
        }
    }

    for branch in [
        "ownership",
        "no lock reason",
        "a shortfall with some coverage",
        "a shortfall the coverage reports whole",
        "a shortfall with no coverage",
        "another reason, with some coverage",
        "a Teased row with some coverage",
        "a tinkerer row whose craft the core reports locked",
        "an actionable row with no content item",
        "a display-only row",
    ] {
        assert!(
            reached.contains(branch),
            "fixture: no meadow here reaches {branch}, so that branch of the marker rule is not \
             under test"
        );
    }

    assert!(
        wrong.is_empty(),
        "{} rows wear a marker other than the one the core's answers give:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// Every row the core reports no lock reason for — a craft's tinkerer count
/// row, and every actionable row with no content item (a fixed action, a
/// kitten's promote and Elder rows, a colony's trade and pen-pal rows, a
/// site's row, the declare-list rows) — wears `●` while it is listed, whatever
/// the stock and whatever the lock reason of the craft beside it. The shell
/// works out no coverage of its own for these: not from the one-unit price the
/// core reports for a count-taking row, not from the content tables. So the
/// meadows here include ones with every stock emptied, where any such
/// derivation would put `○` on the row. Quantified over every such row of
/// every section in each meadow, and over every one of those row shapes — the
/// test checks each shape was reached.
///
/// @spec TUI-NAV-053
#[test]
fn every_row_the_core_reports_no_lock_reason_for_wears_ready_whatever_the_stock() {
    let meadows: Vec<(&str, GameState)> = vec![
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
        (
            "an open meadow with every stock emptied",
            with_every_stock(&maximal(), 0.0),
        ),
        (
            "an open meadow with every kitten at work",
            maximal_all_hands_busy(),
        ),
        ("wealth without progress", rich_but_locked()),
        ("a brand-new meadow", GameState::new(5)),
        (
            "a brand-new meadow with every stock emptied",
            with_every_stock(&GameState::new(5), 0.0),
        ),
    ];

    let mut reached: BTreeSet<&'static str> = BTreeSet::new();
    let mut wrong: Vec<String> = Vec::new();

    for (label, state) in &meadows {
        let emptied = ResourceId::ALL
            .iter()
            .all(|r| state.resources[r.ix()] == 0.0);
        for (section, row) in all_rows(state) {
            if !matches!(row.kind, RowKind::Act { .. } | RowKind::Count { .. }) {
                continue;
            }
            let shape: &'static str = match &row.key {
                RowKey::Content(id, "tinker") => {
                    if state.lock_reason(*id).is_some() {
                        reached.insert("a tinkerer row whose craft the core reports locked");
                    }
                    if emptied {
                        reached.insert("a tinkerer row with every stock emptied");
                    }
                    "a craft's tinkerer count row"
                }
                RowKey::Content(..) | RowKey::Bench | RowKey::Energy | RowKey::Mess => continue,
                RowKey::Fixed("Pounce") => "the fixed count-taking row priced per unit",
                RowKey::Fixed(_) => "a fixed action row",
                RowKey::Kitten(..) => "a kitten's row",
                RowKey::Colony(_, "trade") => "a colony's trade row",
                RowKey::Colony(_, "penpal") => "a colony's pen-pal row",
                RowKey::Colony(..) => "a colony row of another shape",
                RowKey::Site(_) => "a site's row",
                RowKey::Adventure(_) => "a declare-list row",
                RowKey::Resource(_) => "an actionable resource row",
            };
            reached.insert(shape);
            if emptied
                && matches!(
                    row.key,
                    RowKey::Fixed("Pounce") | RowKey::Colony(_, "trade")
                )
            {
                reached.insert("a per-unit-priced row with every stock emptied");
            }

            let drawn = drawn_marker(state, section, &row.key);
            if drawn.glyph != '●' {
                wrong.push(format!(
                    "in {label}, {:?} ({shape}) in {} wears {:?}, but the core reports no lock \
                     reason for it, so it wears ● while listed",
                    row.key,
                    section.title(),
                    drawn.glyph
                ));
            }
        }
    }

    for shape in [
        "a craft's tinkerer count row",
        "a tinkerer row whose craft the core reports locked",
        "a tinkerer row with every stock emptied",
        "a fixed action row",
        "the fixed count-taking row priced per unit",
        "a kitten's row",
        "a colony's trade row",
        "a colony's pen-pal row",
        "a site's row",
        "a declare-list row",
        "a per-unit-priced row with every stock emptied",
    ] {
        assert!(
            reached.contains(shape),
            "fixture: no meadow here lists {shape}, so that part of the claim is not under test"
        );
    }

    assert!(
        wrong.is_empty(),
        "{} rows the core reports no lock reason for wear a marker other than ●:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// A kind with no reveal ladder is present or absent, never a Teased row in
/// between: where it is listed and its marker says it can be acted on (`●`) or
/// is partly covered (`◐`), the marker is not drawn in the dim a Teased row
/// wears. Quantified over every listed row of every two-state kind.
///
/// @spec TUI-NAV-053
#[test]
fn a_listed_row_of_a_kind_with_no_reveal_ladder_is_never_dimmed_as_teased() {
    use ratatui::style::Modifier;

    let dim = clowder_tui::theme::Role::Dim.color();
    let mut checked = 0;

    for (label, state) in [
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
    ] {
        for (section, row) in all_rows(&state) {
            let RowKey::Content(id, _) = row.key else {
                continue;
            };
            if id.is_reveal_tracked() {
                continue;
            }
            assert_ne!(
                state.availability(id),
                Availability::Teased,
                "fixture: in {label} the core reports a middle tier for {id:?}, which carries no \
                 reveal ladder"
            );
            let drawn = drawn_marker(&state, section, &row.key);
            if drawn.glyph != '●' && drawn.glyph != '◐' {
                continue;
            }
            checked += 1;
            assert!(
                drawn.style.fg != dim && !drawn.style.add_modifier.contains(Modifier::DIM),
                "in {label}, {:?} in {} is reported {:?} and wears {:?}, but is drawn dimmed \
                 like a Teased row",
                row.key,
                section.title(),
                state.availability(id),
                drawn.glyph
            );
        }
    }

    assert!(checked > 0, "fixture: some two-state row must wear ● or ◐");
}

// ---------------------------------------------------------------------------
// TUI-NAV-009 — what a row is
// ---------------------------------------------------------------------------

/// Teased *is* revealed: it renders dim, with its marker, so the player can see
/// what is coming and ask the inspector why it is locked. Hidden is absent.
/// The difference is decided by the core's reveal ladder, not by stock — the
/// second half of this test is a player with unlimited resources and no reveals,
/// who sees nothing at all.
///
/// @spec TUI-NAV-009
#[test]
fn teased_content_is_listed_and_hidden_content_is_not() {
    let target = StructureId::ALL[0];
    let id = ContentId::Structure(target);

    // Revealed but unaffordable: the core calls this Teased.
    let mut teased = GameState::new(11);
    teased.revealed.insert(id);
    assert_eq!(
        teased.availability(id),
        Availability::Teased,
        "fixture: a revealed structure with nothing in the stores is Teased"
    );
    assert!(
        content_keys(&teased).iter().any(|(_, listed)| *listed == id),
        "a Teased structure is missing from the list; Teased renders dim with its marker, it does not vanish"
    );

    // Unrevealed but affordable many times over: the core calls this Hidden.
    let rich = rich_but_locked();
    assert_eq!(
        rich.availability(id),
        Availability::Hidden,
        "fixture: an unrevealed structure is Hidden however full the stores are"
    );
    assert!(
        !content_keys(&rich).iter().any(|(_, listed)| *listed == id),
        "an unrevealed structure appeared once the player could afford it; stock does not reveal content"
    );
}

/// Every content row is actionable. Enter on a Teased or unaffordable row still
/// *attempts* the command — the core is the single arbiter of whether it lands,
/// and the rejection becomes a footer, not a pre-emptive refusal to offer the
/// row. A content row that was merely display would be the shell deciding.
///
/// @spec TUI-NAV-009
#[test]
fn every_content_row_is_actionable() {
    let state = maximal();
    let mut counted = 0;

    for (section, row) in all_rows(&state) {
        if !matches!(row.key, RowKey::Content(_, _)) {
            continue;
        }
        counted += 1;
        assert!(
            matches!(row.kind, RowKind::Act { .. } | RowKind::Count { .. }),
            "the {} section lists {:?} as {:?}; content rows act, so Enter always has something to attempt",
            section.title(),
            row.key,
            row.kind
        );
    }

    assert!(counted > 0, "fixture: content rows must exist");
}

// ---------------------------------------------------------------------------
// TUI-NAV-089 — an item's rows: the name heads them once
// ---------------------------------------------------------------------------

/// The plain-word label TUI-NAV-089 names for a row that does one of an item's
/// several acts, keyed by the row's own tag — never by its position, so a
/// later row the table does not name fails loudly rather than passing as
/// whatever the implementation drew.
fn act_label(key: &RowKey) -> Option<&'static str> {
    match key {
        RowKey::Content(ContentId::Craft(_), "tinker") => Some("tinkerers"),
        RowKey::Colony(_, "penpal") => Some("send a pen-pal"),
        _ => None,
    }
}

/// The item a row is about — the thing the palette counts once (TUI-NAV-063):
/// a content id, a kitten, or a colony, whichever of its rows the key names.
/// Every other row is an item of its own.
fn item_of(key: &RowKey) -> String {
    match key {
        RowKey::Content(id, _) => format!("content {id:?}"),
        RowKey::Kitten(ix, _) => format!("kitten {ix}"),
        RowKey::Colony(c, _) => format!("colony {c:?}"),
        other => format!("{other:?}"),
    }
}

/// The item's name as the inspector states it on the first line of its
/// description (TUI-NAV-018: the flavor name of the item the row is about),
/// read at a width where nothing needs shortening.
fn item_name(state: &GameState, section: Section, key: &RowKey) -> String {
    use clowder_tui::render::widgets::inspector;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    let (w, h) = (160u16, 12u16);
    let mut view = nav::ViewState {
        section: Some(section),
        ..Default::default()
    };
    view.focus[section.index()] = Some(key.clone());
    unfold_focus(state, &mut view);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, Rect::new(0, 0, w, h), state, &view))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer();
    (0..w)
        .map(|x| {
            buf.cell((x, 0))
                .map(|c| c.symbol().to_string())
                .unwrap_or_default()
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Every row of `section` drawn at `w`×`h` with nothing focused, each with what
/// its name column reads: the text from where a row's name begins — after the
/// cursor slot, the marker, and one space — up to the first run of two blanks,
/// leading blanks set aside.
fn drawn_name_fields(state: &GameState, section: Section, w: u16, h: u16) -> Vec<(Row, String)> {
    use clowder_tui::render::layout::Body;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut view = nav::ViewState {
        section: Some(section),
        ..Default::default()
    };
    // Every listed row is drawn, in list order, only with the Ideas section's
    // done group open (TUI-NAV-113).
    if drawn_rows(state, section, &view) != rows(state, section) {
        press_done_key(state, &mut view);
    }
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    // The list's own space, dividers set aside, as the frame draws it.
    let (x0, x1, y0, height) = match layout.expect("frame returns its layout").body {
        Body::Wide { list, .. } => (list.x + 1, list.right() - 1, list.y, list.height),
        Body::Narrow { main, .. } => (main.x + 1, main.right(), main.y, main.height),
        other => panic!("fixture: {w}x{h} lays out a list, not {other:?}"),
    };
    let listed = rows(state, section);
    assert!(
        listed.len() <= height as usize,
        "fixture: the {} list ({} rows) must fit its {height}-line pane at {w}x{h} unscrolled",
        section.title(),
        listed.len()
    );

    let buf = term.backend().buffer();
    listed
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let y = y0 + i as u16;
            let line: String = (x0..x1)
                .map(|x| {
                    buf.cell((x, y))
                        .map(|c| c.symbol().to_string())
                        .unwrap_or_default()
                })
                .collect();
            let after_marker: String = line.chars().skip(4).collect();
            let field = after_marker
                .trim_start()
                .split("  ")
                .next()
                .unwrap_or("")
                .trim_end()
                .to_string();
            (row, field)
        })
        .collect()
}

/// Whether a drawn name column reads as `whole`: the whole of it, or the list's
/// ellipsis-shortened form of it (TUI-RENDER-014) — a label being fitted as a
/// name is.
fn reads_as(field: &str, whole: &str) -> bool {
    if field == whole {
        return true;
    }
    match field.strip_suffix('\u{2026}') {
        Some(kept) => !kept.is_empty() && whole.starts_with(kept),
        None => false,
    }
}

/// A meadow where every item that carries several rows is listed — every
/// recipe revealed, every colony found, a small roster of distinctly named
/// kittens — so the lists fit a pane unscrolled.
fn every_multi_row_item_listed() -> GameState {
    let mut s = maximal();
    let names = ["Biscuit", "Clover", "Dumpling", "Juniper", "Waffle"];
    s.kittens.truncate(names.len());
    for (k, name) in s.kittens.iter_mut().zip(names) {
        k.name = name.to_string();
    }
    s
}

/// Where an item carries several rows in its section, each stays a row of its
/// own; the first of them that is listed draws the item's name, and every
/// later one draws in the name's place its own plain-word, glyph-free label —
/// never the item's name — so no two of the item's rows read alike and no list
/// shows the item's name twice. Quantified over every item with several rows in
/// every section — every recipe's make and tinkerer rows, every kitten's two
/// rows, every colony's two rows — in both the wide and the narrow layout.
///
/// @spec TUI-NAV-089
#[test]
fn an_items_first_listed_row_draws_its_name_and_each_later_row_its_own_label() {
    let state = every_multi_row_item_listed();
    let crafts_revealed = CraftId::ALL
        .iter()
        .filter(|c| state.availability(ContentId::Craft(**c)) != Availability::Hidden)
        .count();
    assert!(crafts_revealed > 1, "fixture: several recipes are listed");
    assert!(state.discovered.len() > 1, "fixture: several colonies");

    for (w, h) in [(200u16, 150u16), (80, 150)] {
        let mut multi_items: HashMap<&'static str, usize> = HashMap::new();
        for section in Section::ALL {
            let drawn = drawn_name_fields(&state, section, w, h);

            // Group the selectable rows by the item they are about, in list order.
            let mut items: Vec<(String, Vec<(Row, String)>)> = Vec::new();
            for (row, field) in drawn.iter().filter(|(r, _)| r.selectable()) {
                let item = item_of(&row.key);
                match items.iter_mut().find(|(i, _)| *i == item) {
                    Some((_, rs)) => rs.push((row.clone(), field.clone())),
                    None => items.push((item, vec![(row.clone(), field.clone())])),
                }
            }

            for (item, item_rows) in items.iter().filter(|(_, rs)| rs.len() > 1) {
                let kind = match &item_rows[0].0.key {
                    RowKey::Content(ContentId::Craft(_), _) => "recipe",
                    RowKey::Colony(..) => "colony",
                    _ => "other",
                };
                *multi_items.entry(kind).or_default() += 1;
                let whr = format!("at {w}x{h}, in the {} list, {item}", section.title());
                let name = item_name(&state, section, &item_rows[0].0.key);
                assert!(
                    !name.is_empty(),
                    "fixture: {whr} has a name the inspector states"
                );

                // Each is a row of its own, with an identity of its own.
                let keys: HashSet<String> = item_rows
                    .iter()
                    .map(|(r, _)| format!("{:?}", r.key))
                    .collect();
                assert_eq!(
                    keys.len(),
                    item_rows.len(),
                    "{whr}: two of the item's rows share one identity"
                );

                // The first listed row draws the item's name.
                let (first, first_field) = &item_rows[0];
                assert!(
                    reads_as(first_field, &name),
                    "{whr}: the first listed row {:?} draws {first_field:?} in its name column, \
                     but the first of an item's listed rows carries the item's name {name:?}",
                    first.key
                );

                // Each later listed row draws its own label, in the name's place.
                for (later, field) in &item_rows[1..] {
                    let label = act_label(&later.key).unwrap_or_else(|| {
                        panic!(
                            "{whr}: the later row {:?} has no label TUI-NAV-089 names",
                            later.key
                        )
                    });
                    assert!(
                        reads_as(field, label),
                        "{whr}: the later row {:?} draws {field:?} in its name column, but a later \
                         row of an item draws, in the name's place, its own label {label:?}",
                        later.key
                    );
                    assert!(
                        !reads_as(field, &name) && !field.contains(name.as_str()),
                        "{whr}: the later row {:?} draws the item's name {name:?}; a label is its \
                         row's own and never the item's name",
                        later.key
                    );
                    assert!(
                        field.chars().all(|c| c.is_alphabetic() || c == ' ' || c == '-' || c == '\u{2026}'),
                        "{whr}: the later row {:?} draws {field:?}; its label is plain words with no glyph",
                        later.key
                    );
                }

                // No two of the item's rows read alike.
                let fields: HashSet<&str> = item_rows.iter().map(|(_, f)| f.as_str()).collect();
                assert_eq!(
                    fields.len(),
                    item_rows.len(),
                    "{whr}: two of the item's rows read alike: {:?}",
                    item_rows.iter().map(|(_, f)| f).collect::<Vec<_>>()
                );

                // And the list shows the item's name once.
                let named = drawn
                    .iter()
                    .filter(|(r, f)| r.selectable() && reads_as(f, &name))
                    .count();
                assert_eq!(
                    named, 1,
                    "{whr}: the list shows the item's name {name:?} on {named} rows; it heads the \
                     item's rows once"
                );
            }
        }

        let craft_rows = multi_items.get("recipe").copied().unwrap_or(0);
        let colony_rows = multi_items.get("colony").copied().unwrap_or(0);
        assert_eq!(
            craft_rows, crafts_revealed,
            "at {w}x{h}: every listed recipe carries several rows"
        );
        let colonies = rows(&state, Section::Friends)
            .iter()
            .filter_map(|r| match r.key {
                RowKey::Colony(c, _) => Some(c),
                _ => None,
            })
            .collect::<HashSet<ColonyId>>()
            .len();
        assert!(colonies > 1, "fixture: several colonies are listed");
        assert_eq!(
            colony_rows, colonies,
            "at {w}x{h}: every listed colony carries several rows"
        );
        assert_eq!(
            multi_items.get("other").copied().unwrap_or(0),
            0,
            "at {w}x{h}: an item carries several rows beyond the two kinds TUI-NAV-089 names"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-093 — a kitten's one row
// ---------------------------------------------------------------------------

/// Each kitten in the roster is exactly one row, in the Clowder section and in
/// no other, whose Enter promotes that kitten; no row anywhere takes
/// AppointElder as its Enter or as a `+`/`-`, since that act is the row-bound
/// key's alone. Quantified over every kitten of a long roster with every
/// surface open, the same roster with nothing affordable, and a short roster.
///
/// @spec TUI-NAV-093
#[test]
fn each_kitten_is_exactly_one_clowder_row_whose_enter_promotes_it() {
    for (label, state) in [
        ("a maximal meadow", maximal()),
        (
            "everything unlocked, nothing affordable",
            poor_but_unlocked(),
        ),
        ("a short roster", every_multi_row_item_listed()),
    ] {
        assert!(
            state.kittens.len() > 1,
            "fixture: {label} has several kittens"
        );
        let mut per_kitten: HashMap<usize, Vec<(Section, Row)>> = HashMap::new();
        for (section, row) in all_rows(&state) {
            if let RowKey::Kitten(ix, _) = row.key {
                per_kitten
                    .entry(ix)
                    .or_default()
                    .push((section, row.clone()));
            }
            let takes_elder = match &row.kind {
                RowKind::Act { command, .. } => matches!(command, Command::AppointElder(_)),
                RowKind::Count {
                    increment,
                    decrement,
                } => {
                    matches!(increment, Command::AppointElder(_))
                        || matches!(decrement, Command::AppointElder(_))
                }
                RowKind::Display | RowKind::Group => false,
            };
            assert!(
                !takes_elder,
                "in {label}, the {} row {:?} takes AppointElder; that act is taken only by the \
                 row-bound key on a kitten's row, never as a row of its own",
                section.title(),
                row.key
            );
        }

        for ix in 0..state.kittens.len() {
            let listed = per_kitten.remove(&ix).unwrap_or_default();
            assert_eq!(
                listed.len(),
                1,
                "in {label}, kitten {ix} is listed as {} rows ({:?}); each kitten is exactly one \
                 row",
                listed.len(),
                listed
                    .iter()
                    .map(|(s, r)| (s.title(), r.key.clone()))
                    .collect::<Vec<_>>()
            );
            let (section, row) = &listed[0];
            assert_eq!(
                *section,
                Section::Clowder,
                "in {label}, kitten {ix}'s row is not in the Clowder section"
            );
            match &row.kind {
                RowKind::Act { command, .. } => assert_eq!(
                    *command,
                    Command::Promote(ix),
                    "in {label}, kitten {ix}'s one row fires {command:?}; its Enter promotes that \
                     kitten"
                ),
                other => panic!("in {label}, kitten {ix}'s one row is {other:?}; its Enter promotes that kitten"),
            }
            assert!(
                !row.heavy,
                "in {label}, kitten {ix}'s row is heavy; a kitten's row never is"
            );
        }
        assert!(
            per_kitten.is_empty(),
            "in {label}, rows are listed for kittens not in the roster: {:?}",
            per_kitten.keys().collect::<Vec<_>>()
        );
    }
}

/// A kitten's row is drawn with the kitten's name — the whole of it, followed
/// at most by the row-bound key's own `(n: Elder)` hint or, on the row of the
/// kitten the core holds as the Elder, by the Elder's mark, whole or as its
/// glyph alone (TUI-NAV-104) — and with no label in the name's place, and no
/// list shows one kitten's name on two rows.
/// Quantified over every kitten of a short, distinctly named roster, over
/// every section's list, in both the wide and the narrow layout.
///
/// @spec TUI-NAV-093
#[test]
fn a_kittens_row_is_drawn_with_its_name_and_no_list_shows_a_kitten_twice() {
    let state = every_multi_row_item_listed();
    let names: Vec<String> = state.kittens.iter().map(|k| k.name.clone()).collect();
    assert!(names.len() > 1, "fixture: several kittens");

    for (w, h) in [(200u16, 150u16), (80, 150)] {
        let mut shown: HashMap<String, Vec<String>> = HashMap::new();
        for section in Section::ALL {
            for (row, field) in drawn_name_fields(&state, section, w, h) {
                if !row.selectable() {
                    continue;
                }
                // What a kitten's row may carry after its name: the hint, or
                // the Elder's mark as far as its column holds it.
                let asides = ["(n: Elder)", "\u{25c6} Elder", "\u{25c6}"];
                let bare = asides
                    .iter()
                    .find_map(|a| field.strip_suffix(a))
                    .map(str::trim_end)
                    .unwrap_or(&field);
                for name in &names {
                    if reads_as(bare, name) {
                        shown.entry(name.clone()).or_default().push(format!(
                            "{} {:?}",
                            section.title(),
                            row.key
                        ));
                    }
                }
                let RowKey::Kitten(ix, _) = row.key else {
                    continue;
                };
                let name = &state.kittens[ix].name;
                let whr = format!("at {w}x{h}, kitten {ix}'s row {:?}", row.key);
                let rest = field.strip_prefix(name.as_str()).unwrap_or_else(|| {
                    panic!(
                        "{whr} draws {field:?} in its name column; a kitten's one row is drawn \
                         with the kitten's name {name:?} and no label"
                    )
                });
                let is_elder = state.elder == Some(ix);
                assert!(
                    rest.trim().is_empty()
                        || (!is_elder && rest.trim() == asides[0])
                        || (is_elder && asides[1..].contains(&rest.trim())),
                    "{whr} draws {field:?}: after the kitten's name it carries {rest:?}, but a \
                     kitten's row carries no label — only, where it is drawn, the `(n: Elder)` \
                     hint, or on the Elder's own row the Elder's mark"
                );
            }
        }
        for name in &names {
            let on = shown.get(name).cloned().unwrap_or_default();
            assert_eq!(
                on.len(),
                1,
                "at {w}x{h}, the kitten {name:?} is shown on {} rows ({on:?}); no list shows one \
                 kitten twice",
                on.len()
            );
            assert!(
                on[0].starts_with("Clowder"),
                "at {w}x{h}, the kitten {name:?} is shown outside the Clowder list: {on:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-049 — the stores group
// ---------------------------------------------------------------------------

/// The sidebar is a curated glance; the complete table is here, in home, where
/// every resource can be selected and inspected. A resource in hand that has no
/// row is a resource the player cannot ask about.
///
/// @spec TUI-NAV-049
#[test]
fn every_held_resource_is_a_row_in_the_home_sections_stores_group() {
    // A player holding two things sees those two things.
    let mut early = GameState::new(13);
    early.resources[ResourceId::Catnip.ix()] = 10.0;
    early.resources[ResourceId::Twigs.ix()] = 5.0;

    let listed = resource_rows(&early, home());
    for r in [ResourceId::Catnip, ResourceId::Twigs] {
        assert!(
            listed.contains(&r),
            "{r:?} is in the stores but has no row in home"
        );
    }
    assert!(
        listed.len() < ResourceId::ALL.len(),
        "home lists all {} resources on a nearly-new meadow; the stores group carries revealed resources, \
         not the whole table the player has not met yet",
        ResourceId::ALL.len()
    );

    // Acquiring one more adds it and takes nothing away.
    let mut later = early.clone();
    later.resources[ResourceId::Pebbles.ix()] = 1.0;
    let grown = resource_rows(&later, home());
    assert!(
        grown.contains(&ResourceId::Pebbles),
        "a newly acquired resource gained no row"
    );
    for r in &listed {
        assert!(
            grown.contains(r),
            "{r:?} lost its row when another resource appeared"
        );
    }

    // And a meadow that holds everything lists everything.
    let full = resource_rows(&maximal(), home());
    for r in ResourceId::ALL {
        assert!(
            full.contains(&r),
            "{r:?} is held but has no row; the full truth lives in home"
        );
    }
}

/// Inspectable, not actionable, and gathered under one header. A store is a
/// thing to ask about — the cursor must be able to land on it (that is what
/// makes Space work), and there must be nothing to fire when it does.
///
/// @spec TUI-NAV-049
#[test]
fn the_stores_rows_are_inspectable_display_rows_under_a_group_header() {
    let state = maximal();
    let list = rows(&state, home());

    let positions: Vec<usize> = list
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.key, RowKey::Resource(_)))
        .map(|(i, _)| i)
        .collect();
    assert!(!positions.is_empty(), "home lists no stores at all");

    for i in &positions {
        let row = &list[*i];
        assert_eq!(
            row.kind,
            RowKind::Display,
            "the store {:?} is {:?}; a store is inspected, never fired",
            row.key,
            row.kind
        );
        assert!(
            row.selectable(),
            "the cursor cannot land on the store {:?}, so Space can never inspect it",
            row.key
        );
        assert!(
            !row.heavy,
            "a store {:?} is marked heavy; there is nothing to confirm",
            row.key
        );
    }

    let first = positions[0];
    let last = *positions.last().expect("checked non-empty");
    assert_eq!(
        last - first + 1,
        positions.len(),
        "the stores rows are scattered through home rather than gathered into one group"
    );
    assert!(first > 0, "the stores group has no header above it");
    assert!(
        matches!(list[first - 1].kind, RowKind::Group),
        "the row above the first store is {:?}, not a group header",
        list[first - 1].kind
    );
}

/// One place holds the complete table. Duplicating the stores into another
/// section would give the same resource two rows, two cursor identities, and
/// two places to keep in step.
///
/// @spec TUI-NAV-049
#[test]
fn the_complete_resource_table_lives_in_home_and_nowhere_else() {
    let state = maximal();

    for section in Section::ALL {
        if section == home() {
            continue;
        }
        let strays = resource_rows(&state, section);
        assert!(
            strays.is_empty(),
            "the {} section lists {} store rows; the complete table belongs to home",
            section.title(),
            strays.len()
        );
    }

    let listed = resource_rows(&state, home());
    let unique: HashSet<ResourceId> = listed.iter().copied().collect();
    assert_eq!(listed.len(), unique.len(), "home lists a resource twice");
}

// ---- which stores are listed: the core's answer, and nothing of the shell's ----
//
// When a resource is revealed, and that a revealed one stays so, is the core's
// rule (SIM-ENG-053). The stores group asks it and holds no rule of its own:
// no stock, cap, rate, or price decides whether a store has a row. The tests
// below build their revealed-and-spent meadows by *playing* — commands and
// ticks, which is where the core records a reveal — because a stock set by
// hand and taken away by hand was never recorded and is, rightly, forgotten.

/// A new meadow that gathered ten times and foraged once: everything gathered
/// is spent to exactly nothing, with no tick between.
fn gathered_and_spent() -> (GameState, Vec<ResourceId>) {
    let mut s = GameState::new(49);
    for _ in 0..10 {
        s.apply(Command::GatherCatnip)
            .expect("fixture: a new meadow can gather");
    }
    let gathered: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| s.stock(*r) > 0.0)
        .collect();
    s.apply(Command::Forage)
        .expect("fixture: ten gathers pay for a forage");
    assert!(
        !gathered.is_empty(),
        "fixture: gathering leaves something in store"
    );
    for r in &gathered {
        assert_eq!(
            s.stock(*r),
            0.0,
            "fixture: the forage spends {r:?} to exactly nothing"
        );
    }
    (s, gathered)
}

/// The maximal meadow a tick on — so the core has recorded every store it
/// holds — with every stock then spent to nothing.
fn maximal_met_and_spent() -> GameState {
    let mut s = maximal();
    s.tick();
    for r in ResourceId::ALL {
        assert!(
            s.stock(r) > 0.0,
            "fixture: the maximal meadow holds {r:?} after a tick"
        );
    }
    with_every_stock(&s, 0.0)
}

/// The maximal meadow with nothing in store, after a command: every price it
/// shows has been recorded by the core, and none of what they name is held.
fn poor_with_its_prices_recorded() -> GameState {
    let mut s = poor_but_unlocked();
    s.apply(Command::SetSpeed(1.0))
        .expect("fixture: setting the speed is always accepted");
    s
}

/// A bare meadow with a whale visiting, after a command: the whale's price is
/// recorded, and the store it names has neither stock nor room.
fn a_whale_visiting_a_bare_meadow() -> GameState {
    let mut s = GameState::new(49);
    s.whale_visit_years = 3;
    s.apply(Command::SetSpeed(1.0))
        .expect("fixture: setting the speed is always accepted");
    s
}

/// A kitten at work on something the meadow has nowhere to keep: a positive
/// rate, a zero cap, and a stock that never rises.
fn making_what_it_cannot_keep() -> (GameState, ResourceId) {
    let mut s = GameState::new(49);
    s.kittens = vec![kitten("Ember", Some(JobId::Sunwarden))];
    let made = ResourceId::Warmth;
    for _ in 0..3 {
        s.tick();
    }
    assert!(
        s.net_rate(made) > 0.0,
        "fixture: the kitten's work should produce {made:?}"
    );
    assert_eq!(s.cap(made), 0.0, "fixture: with nowhere to keep it");
    assert_eq!(s.stock(made), 0.0, "fixture: so none is ever held");
    (s, made)
}

/// The resources a price on screen names: the price of every content row any
/// section lists, of every count-taking row the core prices per unit, and of
/// the nap while its row is listed.
fn resources_named_on_screen(state: &GameState) -> BTreeSet<ResourceId> {
    let mut named = BTreeSet::new();
    for (_, row) in all_rows(state) {
        if let RowKey::Content(id, _) = row.key {
            named.extend(state.cost_of_next(id, 1).into_iter().map(|(r, _)| r));
        }
        for c in commands_of(&row) {
            if let Some(price) = state.unit_price(&c) {
                named.extend(price.into_iter().map(|(r, _)| r));
            }
            if matches!(c, Command::DeepNap(_)) {
                named.insert(ResourceId::Starshards);
            }
        }
    }
    named
}

/// The stores group is the core's answer and nothing else: one row for each
/// resource the core reports revealed, for no other, in the core's order —
/// over the whole grouping spread and over meadows built to tempt a rule of
/// the shell's own, each asked as it stands, emptied, and filled. A store at
/// nothing the core calls revealed is listed; a store with room, or a rising
/// rate, or a price on screen naming it, that the core does not call revealed,
/// is not.
///
/// @spec TUI-NAV-049
#[test]
fn the_stores_group_lists_exactly_the_resources_the_core_reports_revealed_in_the_cores_order() {
    let mut spread: Vec<(&'static str, GameState)> = grouping_spread();
    spread.push((
        "a meadow that gathered and spent it all",
        gathered_and_spent().0,
    ));
    spread.push(("the maximal meadow, met and spent", maximal_met_and_spent()));
    spread.push((
        "the maximal meadow, poor, its prices recorded",
        poor_with_its_prices_recorded(),
    ));
    spread.push((
        "a meadow making what it cannot keep",
        making_what_it_cannot_keep().0,
    ));
    spread.push((
        "a bare meadow with a whale visiting",
        a_whale_visiting_a_bare_meadow(),
    ));

    let mut listed_at_nothing = 0;
    let mut room_and_no_row = 0;
    let mut rate_and_no_row = 0;
    let mut price_and_no_row = 0;
    let mut held_and_listed = 0;
    let mut listed_without_room = 0;

    for (what, base) in &spread {
        for (stocks, state) in [
            ("as it stands", base.clone()),
            ("emptied", with_every_stock(base, 0.0)),
            ("filled", with_every_stock(base, 1.0e12)),
        ] {
            let core = state.resources_revealed();
            let listed = resource_rows(&state, home());
            assert_eq!(
                listed, core,
                "in {what}, {stocks}: the stores group (left) is not the resources the core reports \
                 revealed, in the core's order (right)"
            );
            for r in ResourceId::ALL {
                assert_eq!(
                    listed.contains(&r),
                    state.resource_revealed(r),
                    "in {what}, {stocks}: {r:?}'s row and the core's answer for it disagree"
                );
            }

            let rates = state.net_rates();
            let named = resources_named_on_screen(&state);
            for r in ResourceId::ALL {
                let row = listed.contains(&r);
                if row && state.stock(r) == 0.0 {
                    listed_at_nothing += 1;
                }
                if row && state.stock(r) > 0.0 {
                    held_and_listed += 1;
                }
                if row && state.cap(r) == 0.0 {
                    listed_without_room += 1;
                }
                if !row && state.cap(r) > 0.0 {
                    room_and_no_row += 1;
                }
                if !row && rates[r.ix()] > 0.0 {
                    rate_and_no_row += 1;
                }
                if !row && named.contains(&r) {
                    price_and_no_row += 1;
                }
            }
        }
    }

    assert!(
        held_and_listed > 0,
        "fixture: some held store should be listed"
    );
    assert!(
        listed_at_nothing > 0,
        "fixture: some store standing at nothing should be listed"
    );
    assert!(
        listed_without_room > 0,
        "fixture: some store with no room to keep any should be listed"
    );
    assert!(
        room_and_no_row > 0,
        "fixture: some store with room should be unrevealed, and unlisted"
    );
    assert!(
        rate_and_no_row > 0,
        "fixture: some store with a rising rate should be unrevealed, and unlisted"
    );
    assert!(
        price_and_no_row > 0,
        "fixture: some store a price on screen names should be unrevealed by the core, and unlisted"
    );
}

/// The player's own report: a store spent to exactly nothing keeps its row,
/// reading zero, where it stood — the cursor resting on it stays put, and its
/// refill is watched from the row it was spent from.
///
/// @spec TUI-NAV-049
#[test]
fn a_store_spent_to_exactly_nothing_keeps_its_row_where_it_stood() {
    // By a command, with no tick between the gain and the spend.
    let mut s = GameState::new(49);
    for _ in 0..10 {
        s.apply(Command::GatherCatnip)
            .expect("fixture: a new meadow can gather");
    }
    let before = resource_rows(&s, home());
    assert!(!before.is_empty(), "fixture: the gathered store is listed");
    let spent = before[0];
    let key = RowKey::Resource(spent);
    let list = rows(&s, home());
    let stood =
        nav::rows::cursor_index(&list, Some(&key)).expect("the cursor finds the store's row");
    assert_eq!(
        list[stood].key, key,
        "fixture: the cursor rests on the store"
    );

    s.apply(Command::Forage)
        .expect("fixture: ten gathers pay for a forage");
    assert_eq!(
        s.stock(spent),
        0.0,
        "fixture: the forage spends {spent:?} to exactly nothing"
    );

    let after = resource_rows(&s, home());
    assert!(
        after.contains(&spent),
        "{spent:?} was spent to nothing and its row left the stores group: {after:?}"
    );
    assert_eq!(
        &after[..before.len()],
        &before[..],
        "the stores listed before the spend moved"
    );
    let list = rows(&s, home());
    let rests = nav::rows::cursor_index(&list, Some(&key)).expect("the cursor finds a row");
    assert_eq!(
        list[rests].key, key,
        "the cursor resting on {spent:?} was thrown to {:?} when the store was spent",
        list[rests].key
    );
    assert_eq!(
        list[rests].kind,
        RowKind::Display,
        "and its row is still a store's row"
    );

    // It stays while the meadow waits for more.
    for _ in 0..40 {
        s.tick();
        assert!(
            resource_rows(&s, home()).contains(&spent),
            "{spent:?} left the stores group at tick {} while it stood at nothing",
            s.ticks
        );
    }

    // And every store there is: met, then all spent at once, the whole home
    // list stands exactly as it stood.
    let mut met = maximal();
    met.tick();
    let listed = resource_rows(&met, home());
    assert_eq!(
        listed,
        ResourceId::ALL.to_vec(),
        "fixture: the maximal meadow lists every store after a tick"
    );
    let home_before = rows(&met, home());

    let mut spent_out = with_every_stock(&met, 0.0);
    assert_eq!(
        rows(&spent_out, home()),
        home_before,
        "spending every store to nothing changed the home list: a store at zero keeps its row \
         where it stood"
    );
    for _ in 0..5 {
        spent_out.tick();
        assert_eq!(
            resource_rows(&spent_out, home()),
            listed,
            "a store left, or moved in, the stores group as the spent meadow played on"
        );
    }
}

/// A resource the meadow has never met has no row, and the group no header:
/// the stores never name what the player has not met — not for the room to
/// keep it, and not for a rate that never lands.
///
/// @spec TUI-NAV-049
#[test]
fn a_resource_never_met_has_no_stores_row() {
    let mut s = GameState::new(49);
    for tick in 0..=60 {
        let list = rows(&s, home());
        assert!(
            resource_rows(&s, home()).is_empty(),
            "a new meadow lists stores at tick {tick}: {:?}",
            resource_rows(&s, home())
        );
        assert!(
            !list.iter().any(|r| r.key == RowKey::Fixed("stores")),
            "a new meadow draws the stores header above nothing at tick {tick}"
        );
        s.tick();
    }
    assert!(
        ResourceId::ALL.iter().any(|r| s.cap(*r) > 0.0),
        "fixture: the new meadow has room for stores it has not met"
    );

    // One gather: exactly what was gathered, and nothing else.
    s.apply(Command::GatherCatnip)
        .expect("fixture: a new meadow can gather");
    let held: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| s.stock(*r) > 0.0)
        .collect();
    assert_eq!(
        resource_rows(&s, home()),
        held,
        "after one gather the stores are exactly what was gathered"
    );

    // Made every tick and never kept: never met.
    let (working, made) = making_what_it_cannot_keep();
    assert!(
        !resource_rows(&working, home()).contains(&made),
        "{made:?} has a stores row though the meadow has never held any"
    );
}

/// A store a stated price names is listed before the meadow holds any, so
/// the need a row states can be read against the store it speaks of — its
/// stock, its cap, its rate. For every row on screen that states a price, in
/// a meadow holding nothing.
///
/// @spec TUI-NAV-049
#[test]
fn a_store_a_stated_price_names_is_listed_before_any_is_held() {
    let state = poor_with_its_prices_recorded();
    for r in ResourceId::ALL {
        assert_eq!(state.stock(r), 0.0, "fixture: nothing is in store");
    }
    let named = resources_named_on_screen(&state);
    assert!(
        named.len() > 10,
        "fixture: the rows on screen should name a good many stores"
    );

    let listed = resource_rows(&state, home());
    for r in &named {
        assert!(
            listed.contains(r),
            "a row on screen states a price naming {r:?}, and the stores group has no {r:?} row \
             to read it against"
        );
    }
    let list = rows(&state, home());
    for r in &listed {
        let row = list
            .iter()
            .find(|row| row.key == RowKey::Resource(*r))
            .expect("listed");
        assert!(
            row.selectable() && row.kind == RowKind::Display,
            "{r:?}'s row at zero is inspectable"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-097, TUI-NAV-098, TUI-NAV-099 — groups within a section
//
// A row's kind is read off its identity alone: a fixed row (one about no
// content item) is the section's own verb, and every other row is about a
// structure, a job, a kitten, a resource, a colony, a site, an adventure, or a
// content item of some other kind. A group header is the one row with no kind.
// The one exception is the Ideas section's done group (TUI-NAV-110): a row of
// that section the core names already the player's stands there, whatever its
// kind, so its group is read as `Done` rather than off its identity.
// The design's table is restated here as data — which kinds each section
// lists, in what order, and the order of its verbs — and every assertion below
// is a comparison between a built list and that table.
// ---------------------------------------------------------------------------

/// The kind of thing a row is about. A group of a section's list holds the
/// rows of exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum GroupKind {
    ThingsToDo,
    Structures,
    Stores,
    Jobs,
    Kittens,
    Ideas,
    Refinements,
    Rules,
    /// The tinkerers' bench (TUI-NAV-156).
    Tinkerers,
    /// The Meadow's energy and mess readouts (TUI-NAV-167).
    EnergyMess,
    Crafts,
    Rites,
    Arches,
    Colonies,
    Sites,
    LoomUpgrades,
    Traditions,
    Adventures,
    /// The Ideas section's done group: every finished row of its three kinds.
    Done,
}

/// The group `row` stands in on `section`'s list: the Ideas section's done
/// group for a row of that section the core names already the player's
/// (TUI-NAV-097, TUI-NAV-110), and otherwise the kind of thing it is about —
/// `None` for a group header.
fn kind_in(state: &GameState, section: Section, row: &Row) -> Option<GroupKind> {
    if section == Section::Ideas && finished(state, row) {
        return Some(GroupKind::Done);
    }
    group_kind(row)
}

/// The kind of thing `row` is about, or `None` for a group header.
fn group_kind(row: &Row) -> Option<GroupKind> {
    if matches!(row.kind, RowKind::Group) {
        return None;
    }
    Some(match &row.key {
        RowKey::Fixed(_) => GroupKind::ThingsToDo,
        RowKey::Content(ContentId::Structure(_), _) => GroupKind::Structures,
        RowKey::Content(ContentId::Job(_), _) => GroupKind::Jobs,
        RowKey::Content(ContentId::Idea(_), _) => GroupKind::Ideas,
        RowKey::Content(ContentId::Refinement(_), _) => GroupKind::Refinements,
        RowKey::Content(ContentId::Rule(_), _) => GroupKind::Rules,
        RowKey::Content(ContentId::Craft(_), _) => GroupKind::Crafts,
        RowKey::Content(ContentId::Rite(_), _) => GroupKind::Rites,
        RowKey::Content(ContentId::Arch(_), _) => GroupKind::Arches,
        RowKey::Content(ContentId::Tradition(_), _) => GroupKind::Traditions,
        RowKey::Content(ContentId::DreamLoom(_), _) => GroupKind::LoomUpgrades,
        RowKey::Resource(_) => GroupKind::Stores,
        RowKey::Kitten(..) => GroupKind::Kittens,
        RowKey::Colony(..) => GroupKind::Colonies,
        RowKey::Site(_) => GroupKind::Sites,
        RowKey::Adventure(_) => GroupKind::Adventures,
        RowKey::Bench => GroupKind::Tinkerers,
        RowKey::Energy | RowKey::Mess => GroupKind::EnergyMess,
    })
}

/// The design's groups for `section`, in the order they stand (TUI-NAV-098).
fn design_groups(section: Section) -> Vec<GroupKind> {
    use GroupKind::*;
    match section {
        Section::Meadow => vec![ThingsToDo, Structures, EnergyMess, Stores],
        Section::Clowder => vec![Jobs, ThingsToDo, Kittens],
        Section::Ideas => vec![Ideas, Refinements, Rules, Done],
        Section::Crafts => vec![Tinkerers, Crafts],
        Section::Sunbeams => vec![ThingsToDo, Rites],
        Section::Wonders => vec![ThingsToDo, Arches],
        Section::Friends => vec![Colonies, ThingsToDo],
        Section::Sky => vec![Sites, ThingsToDo],
        Section::Dreams => vec![ThingsToDo, LoomUpgrades],
        Section::Legacy => vec![ThingsToDo, Traditions, Adventures],
    }
}

/// The section's own verbs, by the command each fires, in the order its
/// "things to do" group stands them (TUI-NAV-098).
fn design_verbs(section: Section) -> Vec<&'static str> {
    match section {
        Section::Meadow => vec!["GatherCatnip", "Forage", "Rummage"],
        Section::Clowder => vec!["HostJamboree", "Pounce"],
        Section::Ideas | Section::Crafts => vec![],
        Section::Sunbeams => vec!["Bask", "Reverie", "Enlighten"],
        Section::Wonders => vec![
            "HoldParade",
            "RefineStarshard",
            "RefineEcho",
            "CondenseJar",
            "MakePromise",
        ],
        Section::Friends => vec!["ScoutPicnic"],
        Section::Sky => vec!["SummonStarWhale", "FeedWhale"],
        Section::Dreams => vec!["DeepNap"],
        Section::Legacy => vec!["GreatMeadowMove", "PassItDown"],
    }
}

fn position_in<T: PartialEq>(all: &[T], x: &T) -> usize {
    all.iter()
        .position(|y| y == x)
        .expect("every id is in its kind's ALL table")
}

/// Where the core lists a content item among its kind.
fn core_position(id: ContentId) -> usize {
    match id {
        ContentId::Structure(x) => position_in(&StructureId::ALL, &x),
        ContentId::Idea(x) => position_in(&IdeaId::ALL, &x),
        ContentId::Craft(x) => position_in(&CraftId::ALL, &x),
        ContentId::Job(x) => position_in(&JobId::ALL, &x),
        ContentId::Rite(x) => position_in(&RiteId::ALL, &x),
        ContentId::Arch(x) => position_in(&ArchId::ALL, &x),
        ContentId::Refinement(x) => position_in(&RefinementId::ALL, &x),
        ContentId::Rule(x) => position_in(&RuleId::ALL, &x),
        ContentId::Tradition(x) => position_in(&TraditionId::ALL, &x),
        ContentId::DreamLoom(x) => position_in(&DreamLoomId::ALL, &x),
    }
}

/// Where a row's item stands within its group, as the spec orders it: a verb
/// by the design's order, the no-adventure row before every adventure, a kitten
/// by roster position, and everything else by the order the core lists it. An
/// item's several rows share one position, so a non-decreasing run is one in
/// which they stand adjacent. `None` for a verb the design does not name for
/// this section.
fn place_in_group(state: &GameState, section: Section, row: &Row) -> Option<usize> {
    // The done group stands its finished ideas first, then refinements, then
    // rules, each kind in the order the core lists it (TUI-NAV-110).
    if kind_in(state, section, row) == Some(GroupKind::Done) {
        let rank = match &row.key {
            RowKey::Content(ContentId::Idea(_), _) => 0,
            RowKey::Content(ContentId::Refinement(_), _) => 1,
            _ => 2,
        };
        let RowKey::Content(id, _) = &row.key else {
            return None;
        };
        return Some(rank * 10_000 + core_position(*id));
    }
    match &row.key {
        RowKey::Fixed(_) => {
            let fired = commands_of(row);
            let name = variant(fired.first()?);
            design_verbs(section).iter().position(|v| *v == name)
        }
        RowKey::Content(id, _) => Some(core_position(*id)),
        RowKey::Resource(r) => Some(position_in(&ResourceId::ALL, r)),
        RowKey::Kitten(ix, _) => Some(*ix),
        RowKey::Colony(c, _) => Some(position_in(&ColonyId::ALL, c)),
        RowKey::Site(s) => Some(position_in(&SiteId::ALL, s)),
        RowKey::Adventure(None) => Some(0),
        RowKey::Adventure(Some(a)) => Some(1 + position_in(&clowder_core::AdventureId::ALL, a)),
        RowKey::Bench | RowKey::Energy => Some(0),
        RowKey::Mess => Some(1),
    }
}

/// A meadow holding a little catnip and nothing else: home lists its verbs and
/// its stores, and no structure has been revealed.
fn verbs_and_stores_only() -> GameState {
    let mut s = GameState::new(19);
    s.resources[ResourceId::Catnip.ix()] = 10.0;
    s
}

/// The open meadow with no whale visiting and no colony found: a section's
/// verbs listed with its content group empty, and a verb whose row is absent.
fn maximal_no_whale_no_friends() -> GameState {
    let mut s = maximal();
    s.whale_visit_years = 0;
    s.discovered = BTreeSet::new();
    s
}

/// The open meadow whose two Clowder verbs are both held back by the gates the
/// core reports for them, while the jobs and the roster stay listed: the verbs'
/// group has rows the builder offers and lists none of them.
fn maximal_clowder_verbs_shut() -> GameState {
    let mut s = maximal();
    s.ideas.remove(&IdeaId::TeaCeremonies);
    s.ideas.remove(&IdeaId::PouncingPractice);
    assert!(
        !s.command_gate_open(&Command::HostJamboree) && !s.command_gate_open(&Command::Pounce(1)),
        "fixture: both Clowder verbs must be gated shut"
    );
    s
}

/// The spread every group claim is quantified over: between them these lay out
/// every group of every section, every verb, sections with only some of their
/// groups, a group whose only row is teased, and groups whose rows are all held
/// back by a gate the core reports.
fn grouping_spread() -> Vec<(&'static str, GameState)> {
    vec![
        ("a new meadow", GameState::new(7)),
        ("a new meadow with unlimited stock", rich_but_locked()),
        (
            "a meadow with only verbs and stores",
            verbs_and_stores_only(),
        ),
        (
            "a meadow with one teased structure",
            teased_and_half_covered(),
        ),
        ("the maximal meadow", maximal()),
        ("the maximal meadow, poor", poor_but_unlocked()),
        ("the maximal meadow, no roster", maximal_without_a_roster()),
        (
            "the maximal meadow, first generation",
            maximal_first_generation(),
        ),
        (
            "the maximal meadow, every hand busy",
            maximal_all_hands_busy(),
        ),
        (
            "the maximal meadow, a small named roster",
            every_multi_row_item_listed(),
        ),
        (
            "the maximal meadow, no whale and no friends",
            maximal_no_whale_no_friends(),
        ),
        (
            "the maximal meadow, its Clowder verbs gated shut",
            maximal_clowder_verbs_shut(),
        ),
        ("a meadow mid-game, its ideas part done", mid_game()),
        ("a meadow whose every idea is done", all_ideas_done()),
    ]
}

fn describe_list(list: &[Row]) -> String {
    list.iter()
        .map(|r| match group_kind(r) {
            None => format!("  [header {:?}]", r.key),
            Some(k) => format!("  {k:?}: {:?}", r.key),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every row of a kind stands in its kind's one group — contiguous, never split
/// by a header or by a row of another kind — and every row a section lists is
/// of a kind its design gives it. Within a group, verbs stand in the order
/// TUI-NAV-098 names them, the no-adventure row first, kittens by roster
/// position, and every other kind in the order the core lists it; an item's
/// several rows (a craft's two, a colony's two) stand adjacent. Quantified
/// over every section of every meadow in the spread, which between them list
/// every kind and every verb the design names — checked below.
///
/// @spec TUI-NAV-097
#[test]
fn every_row_of_a_kind_stands_contiguous_in_its_kinds_one_group_in_the_specs_order() {
    let mut kinds_seen: HashSet<(Section, GroupKind)> = HashSet::new();
    let mut verbs_seen: HashSet<(Section, &'static str)> = HashSet::new();

    for (label, state) in grouping_spread() {
        for section in Section::ALL {
            let list = rows(&state, section);
            let design = design_groups(section);

            let mut positions: HashMap<GroupKind, Vec<usize>> = HashMap::new();
            for (i, row) in list.iter().enumerate() {
                let Some(kind) = kind_in(&state, section, row) else {
                    continue;
                };
                assert!(
                    design.contains(&kind),
                    "in {label}, {} lists {:?}, a row of kind {kind:?}, which is not one of its \
                     groups {design:?}. List:\n{}",
                    section.title(),
                    row.key,
                    describe_list(&list)
                );
                positions.entry(kind).or_default().push(i);
                kinds_seen.insert((section, kind));
                if kind == GroupKind::ThingsToDo {
                    if let Some(c) = commands_of(row).first() {
                        verbs_seen.insert((section, variant(c)));
                    }
                }
            }

            for (kind, at) in &positions {
                let (first, last) = (at[0], *at.last().expect("non-empty"));
                assert_eq!(
                    last - first + 1,
                    at.len(),
                    "in {label}, {}'s {kind:?} rows are split — something stands between \
                     them (rows {first}..={last}), so the kind is not one group. List:\n{}",
                    section.title(),
                    describe_list(&list)
                );

                let mut previous: Option<usize> = None;
                for i in at {
                    let row = &list[*i];
                    let place = place_in_group(&state, section, row).unwrap_or_else(|| {
                        panic!(
                            "in {label}, {} lists the verb row {:?}, which its things-to-do \
                             group does not name",
                            section.title(),
                            row.key
                        )
                    });
                    if let Some(p) = previous {
                        assert!(
                            place >= p,
                            "in {label}, {}'s {kind:?} group stands {:?} after a row the spec \
                             orders behind it (verbs in their named order, the no-adventure row \
                             first, kittens by roster, everything else in the core's order, an \
                             item's rows adjacent). List:\n{}",
                            section.title(),
                            row.key,
                            describe_list(&list)
                        );
                    }
                    previous = Some(place);
                }
            }
        }
    }

    for section in Section::ALL {
        for kind in design_groups(section) {
            assert!(
                kinds_seen.contains(&(section, kind)),
                "fixture: no meadow in the spread lists {}'s {kind:?} group, so the claim is \
                 not quantified over it",
                section.title()
            );
        }
        for verb in design_verbs(section) {
            assert!(
                verbs_seen.contains(&(section, verb)),
                "fixture: no meadow in the spread lists {}'s {verb} row",
                section.title()
            );
        }
    }
}

/// A section's groups stand in the design's fixed order, whichever of them are
/// listed now: the kinds a list holds, read top to bottom, are exactly the
/// design's groups for that section with the absent ones left out — so no
/// group moves as another comes or goes. Quantified over every section of
/// every meadow in the spread, which lists some sections whole and others with
/// only some of their groups.
///
/// @spec TUI-NAV-098
#[test]
fn a_sections_groups_stand_in_the_designs_fixed_order_whichever_are_listed() {
    let mut partial_seen = false;
    for (label, state) in grouping_spread() {
        for section in Section::ALL {
            let list = rows(&state, section);
            let mut order: Vec<GroupKind> = Vec::new();
            for kind in list.iter().filter_map(|r| kind_in(&state, section, r)) {
                if order.last() != Some(&kind) {
                    order.push(kind);
                }
            }
            let expected: Vec<GroupKind> = design_groups(section)
                .into_iter()
                .filter(|k| order.contains(k))
                .collect();
            assert_eq!(
                order,
                expected,
                "in {label}, {}'s groups stand out of the design's order. List:\n{}",
                section.title(),
                describe_list(&list)
            );
            if !order.is_empty() && order.len() < design_groups(section).len() {
                partial_seen = true;
            }
        }
    }
    assert!(
        partial_seen,
        "fixture: the spread never lists a section with only some of its groups"
    );
}

/// Each group a several-group section lists — judged on the rows it lists
/// after every gate the core reports, a teased row counting — is headed by
/// exactly one header directly above its first row, the first group included;
/// no header stands above an empty group, so none stands alone, at the list's
/// end, or above another header; and a one-group section, were there one, lists no
/// header at all. A section that lists no row the cursor can land on lists
/// nothing. Quantified over every section of every meadow in the spread — which
/// reaches a group listing only a teased row and a group whose rows are all
/// held back by a gate, both checked below.
///
/// @spec TUI-NAV-099
#[test]
fn every_listed_group_of_a_several_group_section_and_no_other_wears_one_header_directly_above_it() {
    let mut teased_only_group_seen = false;
    let mut gated_group_seen = false;

    for (label, state) in grouping_spread() {
        for section in Section::ALL {
            let list = rows(&state, section);
            let headers: Vec<usize> = list
                .iter()
                .enumerate()
                .filter(|(_, r)| kind_in(&state, section, r).is_none())
                .map(|(i, _)| i)
                .collect();

            if !list.iter().any(|r| r.selectable()) {
                assert!(
                    list.is_empty(),
                    "in {label}, {} lists no selectable row but still lists {} line(s) — a \
                     header alone. List:\n{}",
                    section.title(),
                    list.len(),
                    describe_list(&list)
                );
                continue;
            }

            if design_groups(section).len() == 1 {
                assert!(
                    headers.is_empty(),
                    "in {label}, {} has one group by design and must list no header, but lists \
                     {}. List:\n{}",
                    section.title(),
                    headers.len(),
                    describe_list(&list)
                );
                continue;
            }

            // Each listed group's first row, and the kinds listed.
            let mut firsts: Vec<(GroupKind, usize)> = Vec::new();
            for (i, row) in list.iter().enumerate() {
                if let Some(kind) = kind_in(&state, section, row) {
                    if !firsts.iter().any(|(k, _)| *k == kind) {
                        firsts.push((kind, i));
                    }
                }
            }

            for (kind, first) in &firsts {
                assert!(
                    *first > 0 && kind_in(&state, section, &list[first - 1]).is_none(),
                    "in {label}, {}'s {kind:?} group is listed with no header directly above its \
                     first row. List:\n{}",
                    section.title(),
                    describe_list(&list)
                );
                assert!(
                    *first < 2 || kind_in(&state, section, &list[first - 2]).is_some(),
                    "in {label}, {}'s {kind:?} group has two headers above it. List:\n{}",
                    section.title(),
                    describe_list(&list)
                );
            }
            assert_eq!(
                headers.len(),
                firsts.len(),
                "in {label}, {} lists {} group(s) but {} header(s): every listed group wears \
                 exactly one, and an empty group none. List:\n{}",
                section.title(),
                firsts.len(),
                headers.len(),
                describe_list(&list)
            );
            for h in &headers {
                assert!(
                    list.get(h + 1)
                        .is_some_and(|r| kind_in(&state, section, r).is_some()),
                    "in {label}, {}'s header at line {h} stands above nothing — at the list's \
                     end or above another header. List:\n{}",
                    section.title(),
                    describe_list(&list)
                );
            }

            // Reach: a group whose only rows are teased content.
            for (kind, first) in &firsts {
                let members: Vec<&Row> = list[*first..]
                    .iter()
                    .take_while(|r| kind_in(&state, section, r) == Some(*kind))
                    .collect();
                if members.iter().all(|r| match r.key {
                    RowKey::Content(id, _) => state.availability(id) == Availability::Teased,
                    _ => false,
                }) {
                    teased_only_group_seen = true;
                }
            }
            // Reach: a design group with rows the builder offers, every one
            // of them held back by a gate the core reports — here, a verb the
            // design names that the core reports closed while its section
            // lists other groups.
            if design_groups(section).contains(&GroupKind::ThingsToDo)
                && !firsts.iter().any(|(k, _)| *k == GroupKind::ThingsToDo)
            {
                gated_group_seen = true;
            }
        }
    }

    assert!(
        teased_only_group_seen,
        "fixture: the spread never lists a group whose only rows are teased"
    );
    assert!(
        gated_group_seen,
        "fixture: the spread never lists a several-group section with its verbs all held back by a gate"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-110, TUI-NAV-113, TUI-NAV-114, TUI-NAV-099 — the done group (Ideas)
//
// What counts as finished is the core's answer alone: a row whose item the
// core's lock reason names already the player's, the answer behind its `✓`
// (TUI-NAV-053). These tests read nothing else — no set of known ideas, bought
// refinements or adopted rules — so a shell that judged by such a set would
// disagree with them exactly where the set and the core disagree, which one
// fixture below is built to make happen.
//
// Folding is how the list is drawn, never what it lists: `rows` is what the
// section lists, folded rows included, and `drawn_rows` is what the list draws
// and the cursor visits under the view's fold.
// ---------------------------------------------------------------------------

/// Whether the core names `row`'s item already the player's — the one thing
/// TUI-NAV-110 calls finished.
fn finished(state: &GameState, row: &Row) -> bool {
    match &row.key {
        RowKey::Content(id, _) => {
            state.lock_reason(*id) == Some(clowder_core::LockReason::AlreadyYours)
        }
        _ => false,
    }
}

/// `d` pressed as the player presses it: resolved against the view, then
/// folded in. The repeat clock is cleared after, so a later press in the same
/// test reads as a distinct one.
fn press_done_key(state: &GameState, view: &mut ViewState) {
    let action = nav::keys::resolve(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), view);
    nav::update(action, view, state);
    view.last_repeat = None;
}

/// Where `view`'s focused row is listed but folded into the Ideas section's
/// done group, open the group by its key — the one way a player reaches such a
/// row (TUI-NAV-113) — keeping the focus on that row.
fn unfold_focus(state: &GameState, view: &mut ViewState) {
    let Some(section) = view.section else { return };
    let Some(key) = view.focus[section.index()].clone() else {
        return;
    };
    if drawn_rows(state, section, view)
        .iter()
        .any(|r| r.key == key)
        || !rows(state, section).iter().any(|r| r.key == key)
    {
        return;
    }
    press_done_key(state, view);
    view.focus[section.index()] = Some(key);
}

/// The Ideas section in focus, its done group as every sitting opens it.
fn ideas_view() -> ViewState {
    ViewState {
        section: Some(Section::Ideas),
        ..Default::default()
    }
}

/// The Ideas section in focus with its done group opened by its key.
fn ideas_open(state: &GameState) -> ViewState {
    let mut v = ideas_view();
    press_done_key(state, &mut v);
    v
}

/// Which of the Ideas section's three kinds a row is about: ideas, then
/// refinements, then rules — the order its groups stand in, and the order the
/// done group gathers them in.
fn ideas_rank(row: &Row) -> Option<usize> {
    match &row.key {
        RowKey::Content(ContentId::Idea(_), _) => Some(0),
        RowKey::Content(ContentId::Refinement(_), _) => Some(1),
        RowKey::Content(ContentId::Rule(_), _) => Some(2),
        _ => None,
    }
}

/// A row's place in the Ideas section with nothing gathered: its kind's rank,
/// then the order the core lists that kind.
fn ideas_place(row: &Row) -> (usize, usize) {
    let RowKey::Content(id, _) = &row.key else {
        panic!("an Ideas row is about a content item: {:?}", row.key)
    };
    (
        ideas_rank(row).expect("an Ideas row is an idea, a refinement, or a rule"),
        core_position(*id),
    )
}

/// A meadow mid-game: the open meadow with every other idea that no other idea
/// needs left unresearched, every other refinement on offer bought, and one
/// rule adopted from each of two of the four pairs — so each of the Ideas
/// section's three kinds lists several rows finished and several not.
fn mid_game() -> GameState {
    use clowder_core::content::{idea_def, refinement_def, rule_def};
    let mut s = maximal();
    let leaves: Vec<IdeaId> = IdeaId::ALL
        .iter()
        .copied()
        .filter(|i| !IdeaId::ALL.iter().any(|j| idea_def(*j).prereqs.contains(i)))
        .collect();
    for (n, i) in leaves.iter().enumerate() {
        if n % 2 == 0 {
            s.ideas.remove(i);
        }
    }
    let mut n = 0;
    for r in RefinementId::ALL {
        if s.ideas.contains(&refinement_def(r).gate) {
            if n % 2 == 0 {
                s.refinements.insert(r);
            }
            n += 1;
        }
    }
    let mut paired: Vec<RuleId> = Vec::new();
    let mut pair = 0;
    for r in RuleId::ALL {
        if paired.contains(&r) {
            continue;
        }
        paired.push(r);
        paired.push(rule_def(r).blocks);
        if pair % 2 == 0 {
            s.rules.insert(r);
        }
        pair += 1;
    }

    let list = rows(&s, Section::Ideas);
    for rank in 0..3 {
        let of_kind: Vec<&Row> = list
            .iter()
            .filter(|r| ideas_rank(r) == Some(rank))
            .collect();
        let done = of_kind.iter().filter(|r| finished(&s, r)).count();
        assert!(
            done >= 2 && of_kind.len() - done >= 2,
            "fixture: mid-game, Ideas kind {rank} lists {done} finished and {} unfinished rows; \
             several of each are needed",
            of_kind.len() - done
        );
    }
    s
}

/// The open meadow with every idea researched, every refinement bought, and
/// one rule of every pair adopted: every row the Ideas section lists is
/// finished.
fn all_ideas_done() -> GameState {
    use clowder_core::content::rule_def;
    let mut s = maximal();
    s.refinements = RefinementId::ALL.iter().copied().collect();
    let mut paired: Vec<RuleId> = Vec::new();
    for r in RuleId::ALL {
        if !paired.contains(&r) {
            paired.push(r);
            paired.push(rule_def(r).blocks);
            s.rules.insert(r);
        }
    }
    let list = rows(&s, Section::Ideas);
    assert!(
        list.iter().any(|r| r.selectable()),
        "fixture: the Ideas section lists rows"
    );
    assert!(
        list.iter()
            .filter(|r| r.selectable())
            .all(|r| finished(&s, r)),
        "fixture: every row the Ideas section lists is finished"
    );
    s
}

/// A meadow holding an idea whose prerequisite it does not hold: the idea sits
/// in the state's set of known ideas, yet the core's lock reason for it is not
/// ownership, so its row wears `○`, not `✓`. A shell that read the set would
/// call it finished; the core does not.
fn an_idea_held_past_a_missing_prerequisite() -> (GameState, IdeaId) {
    use clowder_core::content::idea_def;
    let mut s = maximal();
    let held = IdeaId::ALL
        .iter()
        .copied()
        .find(|i| !idea_def(*i).prereqs.is_empty())
        .expect("fixture: some idea has a prerequisite");
    s.ideas.remove(&idea_def(held).prereqs[0]);
    assert!(
        s.ideas.contains(&held),
        "fixture: the idea is in the state's set of known ideas"
    );
    assert_ne!(
        s.lock_reason(ContentId::Idea(held)),
        Some(clowder_core::LockReason::AlreadyYours),
        "fixture: the core does not name the idea already the player's"
    );
    assert!(
        rows(&s, Section::Ideas)
            .iter()
            .any(|r| r.key == RowKey::Content(ContentId::Idea(held), "")),
        "fixture: the idea's row is listed"
    );
    (s, held)
}

/// Every meadow the done group's claims are quantified over.
fn done_spread() -> Vec<(&'static str, GameState)> {
    let mut spread = grouping_spread();
    spread.push((
        "a meadow holding an idea past a missing prerequisite",
        an_idea_held_past_a_missing_prerequisite().0,
    ));
    spread
}

/// A list broken at its group headers into each group's rows, headers set
/// aside. A list that does not open with a header is one group.
fn groups_of(list: &[Row]) -> Vec<Vec<Row>> {
    let mut out: Vec<Vec<Row>> = Vec::new();
    for row in list {
        if matches!(row.kind, RowKind::Group) {
            out.push(Vec::new());
        } else {
            match out.last_mut() {
                Some(g) => g.push(row.clone()),
                None => out.push(vec![row.clone()]),
            }
        }
    }
    out
}

fn keys_of(groups: &[Vec<Row>]) -> Vec<Vec<RowKey>> {
    groups
        .iter()
        .map(|g| g.iter().map(|r| r.key.clone()).collect())
        .collect()
}

/// In the Ideas section, a row is finished exactly where the core's lock reason
/// names its item already the player's, and every finished row stands in one
/// done group after the ideas, refinements and rules groups and in no other:
/// the section lists its unfinished ideas, its unfinished refinements, its
/// unfinished rules, and then every finished row — finished ideas first, then
/// refinements, then rules — each in the order the core lists its kind, each
/// group under its own header, and no group listed empty. Quantified over a
/// spread of meadows that list every kind finished and unfinished, every row
/// finished, and none; one holds an idea the state's set calls known while the
/// core does not, and its row stays among the ideas.
///
/// @spec TUI-NAV-110
#[test]
fn every_finished_ideas_row_stands_in_one_done_group_at_the_sections_foot() {
    let mut reached = [[false; 2]; 3];
    for (label, state) in done_spread() {
        let list = rows(&state, Section::Ideas);
        if list.is_empty() {
            continue;
        }
        assert!(
            matches!(list[0].kind, RowKind::Group),
            "in {label}, the Ideas list opens with {:?} rather than a group header",
            list[0].key
        );
        let all: Vec<Row> = list.iter().filter(|r| r.selectable()).cloned().collect();
        let mut expected: Vec<Vec<Row>> = Vec::new();
        for (rank, seen) in reached.iter_mut().enumerate() {
            let mut g: Vec<Row> = all
                .iter()
                .filter(|r| ideas_rank(r) == Some(rank) && !finished(&state, r))
                .cloned()
                .collect();
            g.sort_by_key(ideas_place);
            if !g.is_empty() {
                seen[0] = true;
                expected.push(g);
            }
        }
        let mut done: Vec<Row> = all
            .iter()
            .filter(|r| finished(&state, r))
            .cloned()
            .collect();
        done.sort_by_key(ideas_place);
        for r in &done {
            reached[ideas_rank(r).expect("an Ideas row")][1] = true;
        }
        if !done.is_empty() {
            expected.push(done);
        }
        assert_eq!(
            keys_of(&groups_of(&list)),
            keys_of(&expected),
            "in {label}, the Ideas section's groups are not its unfinished ideas, refinements and \
             rules followed by one done group of every finished row. List:\n{}",
            describe_list(&list)
        );
    }
    for (rank, kind) in ["ideas", "refinements", "rules"].iter().enumerate() {
        assert!(
            reached[rank][0],
            "fixture: no meadow lists an unfinished row among the {kind}"
        );
        assert!(
            reached[rank][1],
            "fixture: no meadow lists a finished row among the {kind}"
        );
    }

    let (state, held) = an_idea_held_past_a_missing_prerequisite();
    let groups = groups_of(&rows(&state, Section::Ideas));
    let key = RowKey::Content(ContentId::Idea(held), "");
    assert!(
        groups
            .first()
            .is_some_and(|g| g.iter().any(|r| r.key == key)),
        "an idea the state's set calls known, whose lock reason the core does not name ownership, \
         must stand among the unfinished ideas, not in the done group: the shell judges by the \
         core's answer alone. Groups: {:?}",
        keys_of(&groups)
    );
}

/// No section but Ideas gathers its finished rows: every other section's
/// groups each hold one kind, no kind is split across two groups, and a row
/// the core names already the player's stands in its own kind's group beside
/// the rest. Quantified over the spread, which lists finished rows outside the
/// Ideas section — a held rite and a bought arch among them — checked below.
///
/// @spec TUI-NAV-110
#[test]
fn no_section_but_ideas_has_a_done_group() {
    let mut finished_elsewhere = 0;
    for (label, state) in done_spread() {
        for section in Section::ALL.into_iter().filter(|s| *s != Section::Ideas) {
            let list = rows(&state, section);
            let groups = groups_of(&list);
            let mut kinds: Vec<GroupKind> = Vec::new();
            for g in &groups {
                let mut here: Vec<GroupKind> = g.iter().filter_map(group_kind).collect();
                here.dedup();
                assert_eq!(
                    here.len(),
                    1,
                    "in {label}, a group of {} holds rows of {} kinds ({here:?}); outside the Ideas \
                     section a group holds one kind. List:\n{}",
                    section.title(),
                    here.len(),
                    describe_list(&list)
                );
                assert!(
                    !kinds.contains(&here[0]),
                    "in {label}, {} lists its {:?} rows in two groups — a done group of its own. \
                     List:\n{}",
                    section.title(),
                    here[0],
                    describe_list(&list)
                );
                kinds.push(here[0]);
            }
            finished_elsewhere += list.iter().filter(|r| finished(&state, r)).count();
        }
    }
    assert!(
        finished_elsewhere > 0,
        "fixture: no section but Ideas lists a finished row"
    );
}

/// While the done group is folded the list draws its header and none of its
/// rows, every other row and header exactly as listed; while it is open the
/// list draws every row the section lists, in list order. No other section's
/// drawing is touched by the fold either way. Quantified over the spread.
///
/// @spec TUI-NAV-113
#[test]
fn a_folded_done_group_draws_only_its_header_and_an_open_one_draws_every_listed_row() {
    let mut folded_seen = 0;
    for (label, state) in done_spread() {
        let listed = rows(&state, Section::Ideas);
        let folded = drawn_rows(&state, Section::Ideas, &ideas_view());
        let expected: Vec<Row> = listed
            .iter()
            .filter(|r| !finished(&state, r))
            .cloned()
            .collect();
        assert_eq!(
            folded.iter().map(|r| r.key.clone()).collect::<Vec<_>>(),
            expected.iter().map(|r| r.key.clone()).collect::<Vec<_>>(),
            "in {label}, the folded Ideas list does not draw exactly the listed rows less the done \
             group's, its header kept. Listed:\n{}\nDrawn:\n{}",
            describe_list(&listed),
            describe_list(&folded)
        );
        if listed.iter().any(|r| finished(&state, r)) {
            folded_seen += 1;
        }

        let open = ideas_open(&state);
        assert_eq!(
            drawn_rows(&state, Section::Ideas, &open),
            listed,
            "in {label}, the open Ideas list does not draw every listed row in list order"
        );

        for section in Section::ALL.into_iter().filter(|s| *s != Section::Ideas) {
            for (how, view) in [("folded", ideas_view()), ("open", open.clone())] {
                let view = ViewState {
                    section: Some(section),
                    ..view
                };
                assert_eq!(
                    drawn_rows(&state, section, &view),
                    rows(&state, section),
                    "in {label}, with the Ideas done group {how}, {} draws other than it lists",
                    section.title()
                );
            }
        }
    }
    assert!(
        folded_seen > 0,
        "fixture: no meadow lists a done group to fold"
    );
}

/// The done header is listed exactly while its group holds a row, and while
/// the group is folded it stands alone, the last line of the drawn list, the
/// one header with no row beneath it; every other header still stands
/// directly above a drawn row. Quantified over the spread.
///
/// @spec TUI-NAV-099
#[test]
fn while_folded_the_done_header_stands_alone_as_the_lists_last_line() {
    let mut alone_seen = 0;
    for (label, state) in done_spread() {
        let has_done = rows(&state, Section::Ideas)
            .iter()
            .any(|r| finished(&state, r));
        let drawn = drawn_rows(&state, Section::Ideas, &ideas_view());
        for (i, row) in drawn.iter().enumerate() {
            if !matches!(row.kind, RowKind::Group) {
                continue;
            }
            let last = i + 1 == drawn.len();
            if has_done && last {
                alone_seen += 1;
                continue;
            }
            assert!(
                drawn.get(i + 1).is_some_and(|r| r.selectable()),
                "in {label}, the folded Ideas list draws a header at line {i} above no row, which only \
                 the done header may do, and only as the last line while its group holds a row. \
                 Drawn:\n{}",
                describe_list(&drawn)
            );
        }
        if has_done {
            assert!(
                drawn
                    .last()
                    .is_some_and(|r| matches!(r.kind, RowKind::Group)),
                "in {label}, the done group holds rows but the folded list does not end with its \
                 header standing alone. Drawn:\n{}",
                describe_list(&drawn)
            );
        }
    }
    assert!(alone_seen > 0, "fixture: no meadow folds a done group");
}

/// A row folded in the done group counts toward its section's tab exactly as a
/// drawn row does: a section whose every row is folded still lists rows the
/// cursor could land on, so it keeps its tab whichever section holds focus,
/// and the tab bar is the same whether its group is folded or open.
///
/// @spec TUI-NAV-114
#[test]
fn a_section_whose_every_row_is_folded_keeps_its_tab() {
    let state = all_ideas_done();
    assert!(
        !drawn_rows(&state, Section::Ideas, &ideas_view())
            .iter()
            .any(|r| r.selectable()),
        "with its group folded, the Ideas section must draw no row the cursor can land on, every \
         row it lists being finished (TUI-NAV-113)"
    );
    assert!(
        lists_selectable(&state, Section::Ideas),
        "the Ideas section's folded rows are still listed, and count as selectable for its tab"
    );
    for focus in [None, Some(Section::Meadow), Some(Section::Ideas)] {
        assert!(
            tab_sections(&state, focus).contains(&Section::Ideas),
            "with focus on {focus:?}, the Ideas section lost its tab although every row it lists \
             is only folded"
        );
    }
}

// ---------------------------------------------------------------------------
// Coats on a partial row — TUI-THEME-019, TUI-THEME-020
// ---------------------------------------------------------------------------

/// A resource's stores-row name — the variant spaced at each capital.
fn resource_name(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// A content item's flavour name, as its row spells it.
fn content_name(id: ContentId) -> String {
    use clowder_core::content;
    match id {
        ContentId::Structure(s) => content::structure_def(s).name.to_string(),
        ContentId::Idea(i) => content::idea_def(i).name.to_string(),
        ContentId::Craft(c) => content::craft_def(c).name.to_string(),
        ContentId::Job(j) => content::job_def(j).name.to_string(),
        ContentId::Rite(r) => content::rite_def(r).name.to_string(),
        ContentId::Arch(a) => content::arch_def(a).name.to_string(),
        ContentId::Refinement(r) => content::refinement_def(r).name.to_string(),
        ContentId::Rule(r) => content::rule_def(r).name.to_string(),
        ContentId::Tradition(t) => resource_name_like(&format!("{t:?}")),
        ContentId::DreamLoom(l) => content::dream_loom_def(l).name.to_string(),
    }
}

fn resource_name_like(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Draw `section` wide with `focus` under the cursor, and find the line whose
/// name column — five cells in, after the list's divider, the cursor column,
/// and the marker — holds `name` whole: the buffer, the list, and the line.
fn drawn_line_named(
    state: &GameState,
    section: Section,
    focus: &RowKey,
    name: &str,
) -> Option<(ratatui::buffer::Buffer, ratatui::layout::Rect, u16, String)> {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let (w, h) = (150u16, 60u16);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    let mut view = nav::ViewState {
        section: Some(section),
        ..Default::default()
    };
    view.focus[section.index()] = Some(focus.clone());
    unfold_focus(state, &mut view);
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, &view, None)))
        .expect("TestBackend is infallible");
    let list = match layout.expect("frame returns its layout").body {
        clowder_tui::render::layout::Body::Wide { list, .. } => list,
        other => panic!("fixture: {w}x{h} lays out a list pane, not {other:?}"),
    };
    let buf = term.backend().buffer().clone();
    for y in list.y..list.bottom() {
        let line: String = (list.x..list.right())
            .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
            .collect();
        let column: String = line.chars().skip(5).collect();
        if column.starts_with(&format!("{name}  ")) {
            return Some((buf, list, y, line));
        }
    }
    None
}

/// A row of a kind with no reveal ladder, short of its price with some of it
/// in hand, wears `◐` in `partial` — and on that line each name in its cost
/// field wears its coat, while the figure before it keeps the row's colour.
///
/// @spec TUI-THEME-019, TUI-THEME-020
#[test]
fn a_partial_row_wears_coats_on_its_cost_names() {
    use clowder_core::LockReason;
    use clowder_tui::theme::{coat, coat_colours, Role};

    let mut state = poor_but_unlocked();
    for r in ResourceId::ALL {
        state.resources[r.ix()] = 1.0;
    }
    let partial = Role::Partial.color().expect("a state colour");

    let mut verified = 0usize;
    let mut tried: Vec<String> = Vec::new();
    for (section, row) in all_rows(&state) {
        let RowKey::Content(id, tag) = &row.key else {
            continue;
        };
        if *tag == "tinker" || id.is_reveal_tracked() || !row.selectable() {
            continue;
        }
        if !matches!(state.lock_reason(*id), Some(LockReason::ShortBy { .. })) {
            continue;
        }
        let covered = state.affordability(*id);
        if covered <= 0.0 || covered >= 1.0 {
            continue;
        }
        let cost = state.cost_of_next(*id, 1);
        if cost.is_empty() {
            continue;
        }
        // The cursor rests on the row beside this one, so this one is drawn
        // unfocused and in view.
        let listed = rows(&state, section);
        let at = listed
            .iter()
            .position(|r| r.key == row.key)
            .expect("the row is listed");
        let Some(beside) = listed[..at]
            .iter()
            .rev()
            .chain(listed[at + 1..].iter())
            .find(|r| r.selectable())
        else {
            continue;
        };
        let name = content_name(*id);
        let Some((buf, list, y, line)) = drawn_line_named(&state, section, &beside.key, &name)
        else {
            tried.push(format!("{name} in {} (not drawn in view)", section.title()));
            continue;
        };
        let fg = |x: usize| {
            buf.cell((list.x + x as u16, y))
                .map(|c| c.fg)
                .unwrap_or_default()
        };
        let marker = buf
            .cell((list.x + 3, y))
            .map(|c| c.symbol().to_string())
            .unwrap_or_default();
        assert_eq!(
            marker,
            "◐",
            "fixture: {name} in {} is short with some coverage: {line:?}",
            section.title()
        );
        assert_eq!(
            fg(3),
            partial,
            "fixture: a two-state kind's `◐` wears partial: {line:?}"
        );

        let mut names_checked = 0usize;
        for (r, _) in &cost {
            let rname = resource_name(*r);
            let Some(at) = line.rfind(&format!(" {rname}")) else {
                continue;
            };
            let cx = line[..at].chars().count() + 1;
            if cx < 5 + name.chars().count() {
                continue;
            }
            for (k, want) in coat_colours(coat(*r), rname.chars().count())
                .iter()
                .enumerate()
            {
                let got = fg(cx + k);
                assert_eq!(
                    &got, want,
                    "the partial row {name}'s cost name {rname}, character {k}, wears {got:?}, not \
                     its coat's {want:?}: {line:?}"
                );
            }
            assert_eq!(
                fg(cx - 2),
                partial,
                "the figure before {rname} wears the row's colour: {line:?}"
            );
            names_checked += 1;
        }
        if names_checked == 0 {
            tried.push(format!(
                "{name} in {} (no cost name drawn)",
                section.title()
            ));
            continue;
        }
        verified += 1;
        break;
    }
    assert!(
        verified > 0,
        "fixture: no partial row of a two-state kind was drawn with a priced cost field; tried {tried:?}"
    );
}
