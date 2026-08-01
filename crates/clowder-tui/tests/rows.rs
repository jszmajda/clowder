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

use nav::rows::{rows, Row, RowKey, RowKind};
use nav::{CountSemantics, Section};

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
        RowKind::Count { increment, decrement } => vec![*increment, *decrement],
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
    assert_eq!(home, Section::Meadow, "home — where the stores live — is digit 1");
    assert_eq!(home.digit(), '1', "and home answers to that digit in both directions");
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
    for state in [maximal(), poor_but_unlocked(), rich_but_locked(), GameState::new(5)] {
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
            RowKind::Act { command: Command::DeclareAdventure(None), .. }
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

    assert!(found > 0, "fixture: build and craft rows must be present for this to mean anything");
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
        ("everything unlocked, nothing affordable", poor_but_unlocked()),
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
        ("everything unlocked, nothing affordable", poor_but_unlocked()),
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
        assert!(listed.contains(&r), "{r:?} is in the stores but has no row in home");
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
    assert!(grown.contains(&ResourceId::Pebbles), "a newly acquired resource gained no row");
    for r in &listed {
        assert!(grown.contains(r), "{r:?} lost its row when another resource appeared");
    }

    // And a meadow that holds everything lists everything.
    let full = resource_rows(&maximal(), home());
    for r in ResourceId::ALL {
        assert!(full.contains(&r), "{r:?} is held but has no row; the full truth lives in home");
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
        assert!(!row.heavy, "a store {:?} is marked heavy; there is nothing to confirm", row.key);
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
