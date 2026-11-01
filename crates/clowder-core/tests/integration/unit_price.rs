//! Tests-first for SIM-ENG-047 — the per-unit price of a count-taking command.
//!
//! A command that takes its count as an argument and is priced a fixed amount
//! per unit (a pounce per squad; a trade per caravan to the colony it names,
//! the whale's exchange by its own rate) has a price a caller can show before
//! any count is chosen, with the promise that n units cost exactly n times it.
//! The query answers nothing:
//!
//!   * for a count-taking command priced otherwise — a deep nap's price
//!     escalates with its length;
//!   * for a command that takes no count;
//!   * for a command the command layer would refuse as locked, decided by the
//!     same checks that command makes before it spends.
//!
//! The suite pins the query as
//!   `GameState::unit_price(&self, cmd: &Command) -> Option<Vec<(ResourceId, f64)>>`
//! — the command names what is priced (a trade's colony is in its argument),
//! and the count it carries is not part of the question: the answer is the
//! price of ONE unit whatever count the command happens to hold.
//!
//! "n units cost exactly n times the price" is tested where it can be observed
//! without reading the command's own pricing rule: at the command layer's
//! all-or-nothing boundary. A meadow holding exactly n times each reported
//! component, and nothing else, must accept n units; the same meadow short of
//! any one component must refuse them and name that component. A price that
//! left a component out, or reported one the command does not charge per unit,
//! fails one side of that boundary.

use clowder_core::*;

/// Everything this suite quantifies over, with one representative of every
/// `Command` variant, is classified here with no wildcard arm: a new command
/// stops this compiling until someone decides whether it takes a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Takes its count as an argument, priced a fixed amount per unit.
    FixedPerUnit,
    /// Takes its count as an argument, priced otherwise.
    PricedOtherwise,
    /// Takes no count.
    NoCount,
}

fn shape(c: &Command) -> Shape {
    use Command as C;
    match c {
        C::Pounce(_) | C::Trade(..) => Shape::FixedPerUnit,
        C::DeepNap(_) => Shape::PricedOtherwise,
        C::GatherCatnip
        | C::Forage
        | C::Rummage
        | C::Build(_)
        | C::AssignJob(_)
        | C::UnassignJob(_)
        | C::ResearchIdea(_)
        | C::Craft(_)
        | C::ObserveSky
        | C::SetSpeed(_)
        | C::HostJamboree
        | C::Promote(_)
        | C::AppointElder(_)
        | C::BuyRefinement(_)
        | C::AdoptRule(_)
        | C::AssignTinkerer(_)
        | C::UnassignTinkerer(_)
        | C::Bask
        | C::Reverie
        | C::BuyRite(_)
        | C::Enlighten
        | C::HoldParade
        | C::BuyArch(_)
        | C::RefineStarshard
        | C::RefineEcho
        | C::CondenseJar
        | C::MakePromise
        | C::SendPenPal(_)
        | C::ScoutPicnic
        | C::GreatMeadowMove
        | C::PassItDown
        | C::BuyTradition(_)
        | C::DeclareAdventure(_)
        | C::LaunchExpedition(_)
        | C::SummonStarWhale
        | C::FeedWhale
        | C::BuyDreamLoom(_) => Shape::NoCount,
    }
}

/// How many variants `Command` has. Paired with `shape`'s exhaustive match, so
/// `every_command` cannot silently miss a variant.
const COMMAND_VARIANTS: usize = 40;

/// The same command carrying a different count, where it carries one.
fn counted(c: Command, n: u32) -> Command {
    match c {
        Command::Pounce(_) => Command::Pounce(n),
        Command::Trade(colony, _) => Command::Trade(colony, n),
        Command::DeepNap(_) => Command::DeepNap(n),
        other => other,
    }
}

/// Every command there is: each variant, over every id it can name.
fn every_command() -> Vec<Command> {
    let mut out = vec![
        Command::GatherCatnip,
        Command::Forage,
        Command::Rummage,
        Command::ObserveSky,
        Command::SetSpeed(2.0),
        Command::Pounce(1),
        Command::HostJamboree,
        Command::Promote(0),
        Command::AppointElder(0),
        Command::Bask,
        Command::Reverie,
        Command::Enlighten,
        Command::HoldParade,
        Command::RefineStarshard,
        Command::RefineEcho,
        Command::CondenseJar,
        Command::MakePromise,
        Command::ScoutPicnic,
        Command::GreatMeadowMove,
        Command::PassItDown,
        Command::DeclareAdventure(None),
        Command::SummonStarWhale,
        Command::FeedWhale,
        Command::DeepNap(1),
    ];
    out.extend(StructureId::ALL.map(Command::Build));
    out.extend(JobId::ALL.map(Command::AssignJob));
    out.extend(JobId::ALL.map(Command::UnassignJob));
    out.extend(IdeaId::ALL.map(Command::ResearchIdea));
    out.extend(CraftId::ALL.map(Command::Craft));
    out.extend(CraftId::ALL.map(Command::AssignTinkerer));
    out.extend(CraftId::ALL.map(Command::UnassignTinkerer));
    out.extend(RefinementId::ALL.map(Command::BuyRefinement));
    out.extend(RuleId::ALL.map(Command::AdoptRule));
    out.extend(RiteId::ALL.map(Command::BuyRite));
    out.extend(ArchId::ALL.map(Command::BuyArch));
    out.extend(ColonyId::ALL.map(|c| Command::Trade(c, 1)));
    out.extend(ColonyId::ALL.map(Command::SendPenPal));
    out.extend(TraditionId::ALL.map(Command::BuyTradition));
    out.extend(AdventureId::ALL.map(|a| Command::DeclareAdventure(Some(a))));
    out.extend(SiteId::ALL.map(Command::LaunchExpedition));
    out.extend(DreamLoomId::ALL.map(Command::BuyDreamLoom));

    let mut variants: Vec<std::mem::Discriminant<Command>> = Vec::new();
    for c in &out {
        let d = std::mem::discriminant(c);
        if !variants.contains(&d) {
            variants.push(d);
        }
    }
    assert_eq!(
        variants.len(),
        COMMAND_VARIANTS,
        "fixture: every_command must name every Command variant"
    );
    out
}

/// Every command of the fixed-per-unit shape: a pounce, and a trade to every
/// colony (the whale's exchange included).
fn fixed_per_unit_commands() -> Vec<Command> {
    every_command()
        .into_iter()
        .filter(|c| shape(c) == Shape::FixedPerUnit)
        .collect()
}

/// A meadow in which every count-taking command is open: every idea known,
/// every colony discovered, and a whale visiting — stores full, a small roster.
fn open_meadow() -> GameState {
    let mut s = GameState::new(47);
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.discovered = ColonyId::ALL.into_iter().collect();
    s.whale_visit_years = 5;
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    s
}

/// Meadows spanning each lock the fixed-per-unit commands have: a fresh one
/// (everything locked), an open one, and open ones with exactly one door shut.
fn meadows() -> Vec<(&'static str, GameState)> {
    let open = open_meadow();

    let mut no_pounce = open.clone();
    no_pounce.ideas.retain(|i| *i != IdeaId::PouncingPractice);

    let mut one_colony = open.clone();
    let first = ColonyId::ALL
        .into_iter()
        .find(|c| *c != ColonyId::StarWhales)
        .unwrap();
    one_colony.discovered = [first].into_iter().collect();

    let mut no_whale = open.clone();
    no_whale.whale_visit_years = 0;

    vec![
        ("a fresh meadow", GameState::new(47)),
        ("an open meadow", open),
        ("an open meadow that never learnt to pounce", no_pounce),
        ("an open meadow that knows one colony", one_colony),
        ("an open meadow with no whale visiting", no_whale),
    ]
}

/// The price of one unit of each fixed-per-unit command, where the command
/// layer would not refuse it as locked, is a non-empty list of positive
/// amounts; where it would refuse it as locked, the query answers nothing. The
/// two are decided by the same checks, so they are compared state by state.
///
/// @spec SIM-ENG-047
#[test]
fn a_fixed_per_unit_command_is_priced_exactly_where_the_command_layer_would_not_lock_it() {
    let mut priced = 0;
    let mut withheld = 0;
    for (what, state) in meadows() {
        for c in fixed_per_unit_commands() {
            let locked = matches!(state.clone().apply(c), Err(CommandError::Locked));
            let price = state.unit_price(&c);
            if locked {
                withheld += 1;
                assert_eq!(
                    price, None,
                    "in {what}, {c:?} is refused as locked, yet a price was stated for it; no \
                     price of something not yet reached is stated"
                );
            } else {
                priced += 1;
                let price = price.unwrap_or_else(|| {
                    panic!("in {what}, {c:?} is not locked and is priced per unit, yet no price was stated")
                });
                assert!(!price.is_empty(), "in {what}, {c:?} was priced at nothing");
                for (r, amount) in &price {
                    assert!(
                        amount.is_finite() && *amount > 0.0,
                        "in {what}, {c:?} was priced at {amount} {r:?} per unit"
                    );
                }
                let mut seen: Vec<ResourceId> = price.iter().map(|(r, _)| *r).collect();
                seen.sort_by_key(|r| r.ix());
                seen.dedup();
                assert_eq!(
                    seen.len(),
                    price.len(),
                    "in {what}, {c:?}'s price names a component twice: {price:?}"
                );
            }
        }
    }
    assert!(
        priced > 0,
        "fixture: no meadow opens a fixed-per-unit command"
    );
    assert!(
        withheld > 0,
        "fixture: no meadow locks a fixed-per-unit command"
    );
}

/// The price is of ONE unit, whatever count the command carries.
///
/// @spec SIM-ENG-047
#[test]
fn the_price_is_of_one_unit_whatever_count_the_command_carries() {
    let state = open_meadow();
    for c in fixed_per_unit_commands() {
        let one = state.unit_price(&counted(c, 1));
        for n in [2, 7, 40] {
            assert_eq!(
                state.unit_price(&counted(c, n)),
                one,
                "{c:?} carrying a count of {n} was priced differently from carrying one"
            );
        }
    }
}

/// n units cost exactly n times the price reported, per component: a meadow
/// holding exactly that, and nothing else, can take n units; the same meadow
/// short of any single component cannot, and is told which.
///
/// @spec SIM-ENG-047
#[test]
fn n_units_cost_exactly_n_times_the_price_reported() {
    let open = open_meadow();
    for c in fixed_per_unit_commands() {
        let price = open
            .unit_price(&c)
            .unwrap_or_else(|| panic!("{c:?} is open in this meadow, yet no price was stated"));

        for n in [1u32, 2, 7, 40] {
            let mut exact = open.clone();
            for r in ResourceId::ALL {
                exact.resources[r.ix()] = 0.0;
            }
            for (r, unit) in &price {
                exact.resources[r.ix()] = unit * n as f64;
            }

            let taken = exact.clone().apply(counted(c, n));
            assert!(
                taken.is_ok(),
                "{n} units of {c:?} were refused ({taken:?}) by a meadow holding exactly {n} times \
                 the price reported per unit ({price:?}) and nothing else"
            );

            for (r, unit) in &price {
                let mut short = exact.clone();
                let total = unit * n as f64;
                short.resources[r.ix()] = total - total * 0.01;
                match short.apply(counted(c, n)) {
                    Err(CommandError::InsufficientResources { missing }) => assert!(
                        missing.iter().any(|(m, _)| m == r),
                        "{n} units of {c:?} short of {r:?} were refused, but the refusal named \
                         {missing:?}"
                    ),
                    other => panic!(
                        "{n} units of {c:?} were not refused as unaffordable ({other:?}) by a \
                         meadow one percent short of {n} times the reported {r:?} price"
                    ),
                }
            }
        }
    }
}

/// A count-taking command priced otherwise — the deep nap, whose price
/// escalates with its length — answers nothing, open or locked: the promise
/// that n units cost n times one would not hold for it.
///
/// @spec SIM-ENG-047
#[test]
fn a_count_taking_command_priced_otherwise_is_answered_with_nothing() {
    for (what, state) in meadows() {
        for c in every_command()
            .into_iter()
            .filter(|c| shape(c) == Shape::PricedOtherwise)
        {
            for n in [1, 3] {
                assert_eq!(
                    state.unit_price(&counted(c, n)),
                    None,
                    "in {what}, {:?} was given a per-unit price; its price is not fixed per unit",
                    counted(c, n)
                );
            }
        }
    }
}

/// A command that takes no count answers nothing — every one of them, in every
/// meadow: its price, if it has one, is a content item's, asked for elsewhere.
///
/// @spec SIM-ENG-047
#[test]
fn a_command_that_takes_no_count_is_answered_with_nothing() {
    let mut asked = 0;
    for (what, state) in meadows() {
        for c in every_command()
            .into_iter()
            .filter(|c| shape(c) == Shape::NoCount)
        {
            asked += 1;
            assert_eq!(
                state.unit_price(&c),
                None,
                "in {what}, {c:?} takes no count, yet a per-unit price was stated for it"
            );
        }
    }
    assert!(
        asked > 0,
        "fixture: no command without a count was asked about"
    );
}
