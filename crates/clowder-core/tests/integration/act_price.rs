//! The price of a fixed act: what applying it now takes, read from what it does.
//!
//! @spec SIM-ENG-087

use clowder_core::*;

/// A meadow where every fixed act can be taken: everything learned and built, and
/// plenty in the stores but where an act needs room for what it brings.
fn rich() -> GameState {
    let mut s = GameState::new(46);
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.arches_bought = ArchId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().take(3).collect();
    s.discovered = ColonyId::ALL.into_iter().collect();
    s.whale_visit_years = 3;
    for st in StructureId::ALL {
        s.structures[st.ix()] = 8;
    }
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e7;
    }
    s.resources[ResourceId::Twigs.ix()] = 0.0;
    s.resources[ResourceId::JarsOfPurrs.ix()] = 0.0;
    s
}

fn fixed_acts() -> Vec<Command> {
    vec![
        Command::Forage,
        Command::HostJamboree,
        Command::Bask,
        Command::HoldParade,
        Command::RefineStarshard,
        Command::RefineEcho,
        Command::CondenseJar,
        Command::MakePromise,
        Command::ScoutPicnic,
        Command::SummonStarWhale,
        Command::FeedWhale,
        Command::LaunchExpedition(SiteId::ALL[3]),
    ]
}

/// Whether the query prices a command. Exhaustive on purpose: a new command stops
/// this compiling until it is decided.
fn priced(c: &Command) -> bool {
    use Command as C;
    match c {
        C::Forage
        | C::HostJamboree
        | C::Bask
        | C::HoldParade
        | C::RefineStarshard
        | C::RefineEcho
        | C::CondenseJar
        | C::MakePromise
        | C::ScoutPicnic
        | C::SummonStarWhale
        | C::FeedWhale
        | C::LaunchExpedition(_) => true,

        // Priced by a content item.
        C::Build(_)
        | C::ResearchIdea(_)
        | C::Craft(_)
        | C::CraftAll(_)
        | C::BuyRefinement(_)
        | C::AdoptRule(_)
        | C::BuyRite(_)
        | C::BuyArch(_)
        | C::BuyTradition(_)
        | C::BuyDreamLoom(_) => false,
        // Priced per unit, or by a query of their own.
        C::Pounce(_) | C::Trade(..) | C::DeepNap(_) | C::SendPenPal(_) => false,
        // A kitten's promotion moves with the kitten (#48).
        C::Promote(_) => false,
        // These spend no store.
        C::GatherCatnip
        | C::Rummage
        | C::ObserveSky
        | C::SetSpeed(_)
        | C::Raze(_)
        | C::AssignJob(_)
        | C::UnassignJob(_)
        | C::AppointElder(_)
        | C::AssignTinkerer(_)
        | C::UnassignTinkerer(_)
        | C::Reverie
        | C::Enlighten
        | C::GreatMeadowMove
        | C::PassItDown
        | C::DeclareAdventure(_) => false,
    }
}

/// Each act's price is what applying it takes: each component falls by just that,
/// and no other store falls.
///
/// @spec SIM-ENG-087
#[test]
fn a_fixed_acts_price_is_what_applying_it_takes() {
    for act in fixed_acts() {
        assert!(priced(&act), "fixture: {act:?}");
        let before = rich();
        let price = before
            .act_price(&act)
            .unwrap_or_else(|| panic!("{act:?} is priced here"));
        assert!(!price.is_empty(), "{act:?} names what it takes");

        let mut after = before.clone();
        after
            .apply(act)
            .unwrap_or_else(|e| panic!("{act:?}: {e:?}"));
        for r in ResourceId::ALL {
            let fell = before.stock(r) - after.stock(r);
            match price.iter().find(|(x, _)| *x == r) {
                // A picnic that finds nobody hands most of its pep back afterwards.
                Some((_, need)) if act == Command::ScoutPicnic && fell < *need => {
                    assert!(fell > 0.0, "{act:?} took no {r:?}");
                }
                Some((_, need)) => assert!(
                    (fell - need).abs() < 1e-6,
                    "{act:?} took {fell} of {r:?}, priced at {need}"
                ),
                None => assert!(fell <= 1e-6, "{act:?} took {fell} of {r:?}, unpriced"),
            }
        }
    }
}

/// Each fixed amount is the act's own line: the meadow holding exactly that takes
/// it, and one short of any component is refused.
///
/// @spec SIM-ENG-087
#[test]
fn a_fixed_amount_is_where_the_act_is_refused() {
    for act in fixed_acts() {
        if act == Command::Bask {
            continue;
        }
        let price = rich().act_price(&act).expect("priced");
        let holding = |short: Option<ResourceId>| {
            let mut s = rich();
            for (r, need) in &price {
                s.resources[r.ix()] = if short == Some(*r) { need - 1.0 } else { *need };
            }
            s
        };
        assert!(
            holding(None).apply(act).is_ok(),
            "{act:?} refused holding its price"
        );
        for (r, _) in &price {
            assert!(
                matches!(
                    holding(Some(*r)).apply(act),
                    Err(CommandError::InsufficientResources { .. })
                ),
                "{act:?} took its price short of {r:?}"
            );
        }
    }
}

/// The act that spends a whole store is priced at that store, whatever it holds,
/// and at nothing once it holds nothing.
///
/// @spec SIM-ENG-087
#[test]
fn the_act_that_spends_a_whole_store_is_priced_at_its_stock() {
    for stock in [3.5, 1_000.0] {
        let mut s = rich();
        s.resources[ResourceId::Warmth.ix()] = stock;
        assert_eq!(
            s.act_price(&Command::Bask),
            Some(vec![(ResourceId::Warmth, stock)])
        );
    }
    let mut empty = rich();
    empty.resources[ResourceId::Warmth.ix()] = 0.0;
    assert_eq!(empty.act_price(&Command::Bask), None);
}

/// With its gate closed, no act is priced: nothing not yet reached is stated.
///
/// @spec SIM-ENG-087
#[test]
fn an_act_whose_gate_is_closed_is_not_priced() {
    let s = GameState::new(47);
    let mut closed = 0;
    for act in fixed_acts() {
        if !s.command_gate_open(&act) {
            closed += 1;
            assert_eq!(
                s.act_price(&act),
                None,
                "{act:?} was priced behind its gate"
            );
        }
    }
    assert!(closed >= 5, "fixture: most wait on something ({closed})");
}

/// Every other command is answered with nothing.
///
/// @spec SIM-ENG-087
#[test]
fn every_other_command_is_not_priced() {
    let s = rich();
    let others = [
        Command::GatherCatnip,
        Command::Rummage,
        Command::ObserveSky,
        Command::SetSpeed(1.0),
        Command::Raze(StructureId::ALL[0]),
        Command::Build(StructureId::ALL[0]),
        Command::ResearchIdea(IdeaId::ALL[0]),
        Command::Craft(CraftId::ALL[0]),
        Command::Pounce(1),
        Command::Trade(ColonyId::ALL[0], 1),
        Command::DeepNap(1),
        Command::SendPenPal(ColonyId::ALL[0]),
        Command::Promote(0),
        Command::Reverie,
        Command::Enlighten,
        Command::GreatMeadowMove,
        Command::PassItDown,
    ];
    for c in others {
        assert!(!priced(&c), "fixture: {c:?}");
        assert_eq!(s.act_price(&c), None, "{c:?}");
    }
}
