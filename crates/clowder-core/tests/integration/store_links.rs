//! What a command does to the stores: the core's read-only answer to which
//! resources a command raises, lowers, or both, for a shell that holds no rule
//! per kind of item.

use clowder_core::content;
use clowder_core::*;

fn link(s: &GameState, cmd: &Command, r: ResourceId) -> Option<StoreLink> {
    s.store_links(cmd)
        .into_iter()
        .find(|(x, _)| *x == r)
        .map(|(_, l)| l)
}

fn lowered(l: Option<StoreLink>) -> bool {
    matches!(l, Some(StoreLink::Lowers | StoreLink::Both))
}

fn raised(l: Option<StoreLink>) -> bool {
    matches!(l, Some(StoreLink::Raises | StoreLink::Both))
}

/// A meadow with every idea, every store well stocked, and room for all of it.
fn rich() -> GameState {
    let mut s = GameState::new(11);
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().collect();
    s.revealed = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .chain(IdeaId::ALL.into_iter().map(ContentId::Idea))
        .collect();
    s.generation = 1;
    s.peak_devotion = 1.0e12;
    s.arches_bought.insert(ArchId::EverdreamArch);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e7;
    }
    s
}

/// Links stand in the core's resource order, each store once, and reading them
/// changes nothing.
/// @spec SIM-ENG-079
#[test]
fn links_stand_in_resource_order_once_each_and_change_nothing() {
    let s = rich();
    let before = serde_json::to_string(&s).expect("a state serializes");
    let mut seen = 0;
    for b in StructureId::ALL {
        let links = s.store_links(&Command::Build(b));
        seen += links.len();
        for pair in links.windows(2) {
            assert!(pair[0].0.ix() < pair[1].0.ix(), "{b:?}: {links:?}");
        }
    }
    assert!(seen > 0, "fixture: structures touch stores");
    assert_eq!(
        before,
        serde_json::to_string(&s).expect("a state serializes"),
        "reading links is read-only"
    );
}

/// A command that moves no store by its own rule links nothing, and neither
/// does a Move, whose armed summary says what becomes of the stores.
/// @spec SIM-ENG-079
#[test]
fn a_command_that_moves_no_store_and_a_move_link_nothing() {
    let s = rich();
    for cmd in [
        Command::SetSpeed(2.0),
        Command::DeclareAdventure(None),
        Command::AppointElder(0),
        Command::GreatMeadowMove,
        Command::PassItDown,
    ] {
        assert!(s.store_links(&cmd).is_empty(), "{cmd:?}");
    }
}

/// Every structure: its price lowers, its converter inputs and year-craft raws
/// lower, and what it makes, stores more of, or multiplies raises, a store on
/// both sides reading as both.
/// @spec SIM-ENG-080, SIM-ENG-081
#[test]
fn a_structures_links_read_its_price_and_its_definition() {
    let s = rich();
    for b in StructureId::ALL {
        let d = content::structure_def(b);
        let cmd = Command::Build(b);
        let price = s.cost_of_next(ContentId::Structure(b), 1);
        for (r, _) in &price {
            assert!(lowered(link(&s, &cmd, *r)), "{b:?} is priced in {r:?}");
        }
        for (r, _) in d.productions {
            assert!(raised(link(&s, &cmd, *r)), "{b:?} produces {r:?}");
        }
        for (r, _) in d.cap_bonuses {
            assert!(raised(link(&s, &cmd, *r)), "{b:?} stores more {r:?}");
        }
        for (r, ratio) in d.resource_ratios {
            if *ratio > 0.0 {
                assert!(raised(link(&s, &cmd, *r)), "{b:?} multiplies {r:?}");
            }
        }
        if let Some(c) = &d.converter {
            for (r, _) in c.inputs {
                assert!(lowered(link(&s, &cmd, *r)), "{b:?} draws {r:?}");
            }
            for (r, _) in c.outputs {
                assert!(raised(link(&s, &cmd, *r)), "{b:?} converts to {r:?}");
            }
        }
        if let Some(y) = &d.year_craft {
            for (raw, craft) in y.pairs {
                assert!(lowered(link(&s, &cmd, *raw)), "{b:?} takes {raw:?}");
                let out = content::craft_output(*craft);
                assert!(raised(link(&s, &cmd, out)), "{b:?} makes {out:?}");
            }
        }
        if d.parade_rainbows > 0.0 {
            assert!(raised(link(&s, &cmd, ResourceId::Rainbows)), "{b:?}");
        }
        if d.prism_descent > 0.0 {
            assert!(raised(link(&s, &cmd, ResourceId::Prisms)), "{b:?}");
        }
        if d.deja_vu_chance > 0.0 {
            assert!(raised(link(&s, &cmd, ResourceId::Reverie)), "{b:?}");
        }
        if d.dream_drift > 0.0 {
            assert!(lowered(link(&s, &cmd, ResourceId::Prisms)), "{b:?}");
            assert!(raised(link(&s, &cmd, ResourceId::Dreamcorns)), "{b:?}");
        }
        for (r, l) in s.store_links(&cmd) {
            let spends = price.iter().any(|(x, _)| *x == r)
                || d.converter
                    .as_ref()
                    .is_some_and(|c| c.inputs.iter().any(|(x, _)| *x == r))
                || d.year_craft
                    .as_ref()
                    .is_some_and(|y| y.pairs.iter().any(|(x, _)| *x == r))
                || (d.dream_drift > 0.0 && r == ResourceId::Prisms);
            assert_eq!(
                matches!(l, StoreLink::Lowers | StoreLink::Both),
                spends,
                "{b:?} and {r:?}"
            );
        }
    }
}

/// A recipe's make row spends its inputs and raises its output; a tinkerer's
/// bind draws the same, with no price of its own.
/// @spec SIM-ENG-080, SIM-ENG-081
#[test]
fn a_recipes_rows_lower_its_inputs_and_raise_its_output() {
    let s = rich();
    for c in CraftId::ALL {
        let out = content::craft_output(c);
        for cmd in [Command::Craft(c), Command::AssignTinkerer(c)] {
            assert!(raised(link(&s, &cmd, out)), "{cmd:?} makes {out:?}");
            for (r, _) in content::craft_def(c).inputs {
                assert!(lowered(link(&s, &cmd, *r)), "{cmd:?} takes {r:?}");
            }
        }
    }
}

/// A job raises what it yields, and costs nothing.
/// @spec SIM-ENG-081
#[test]
fn a_job_raises_what_it_yields() {
    let s = rich();
    for j in JobId::ALL {
        let cmd = Command::AssignJob(j);
        let links = s.store_links(&cmd);
        match content::job_def(j).yield_per_tick {
            Some((r, _)) => assert_eq!(links, vec![(r, StoreLink::Raises)], "{j:?}"),
            None => assert!(links.is_empty(), "{j:?}: {links:?}"),
        }
    }
}

/// An item held once has no price left to pay, while what it does still stands.
/// @spec SIM-ENG-080
#[test]
fn an_item_already_held_has_no_price_left_to_lower() {
    let f = RefinementId::ALL
        .into_iter()
        .find(|f| {
            content::refinement_def(*f)
                .effects
                .iter()
                .any(|(e, v)| matches!(e, Effect::ResourceRatio(_) | Effect::MaxCap(_)) && *v > 0.0)
        })
        .expect("fixture: a refinement grows a store");
    let cmd = Command::BuyRefinement(f);
    let grown = content::refinement_def(f)
        .effects
        .iter()
        .find_map(|(e, _)| match e {
            Effect::ResourceRatio(r) | Effect::MaxCap(r) => Some(*r),
            _ => None,
        })
        .expect("fixture: the effect names a store");

    let mut s = rich();
    s.refinements.remove(&f);
    let price = s.cost_of_next(ContentId::Refinement(f), 1);
    assert!(!price.is_empty(), "fixture: the refinement has a price");
    for (r, _) in &price {
        assert!(lowered(link(&s, &cmd, *r)), "{f:?} is priced in {r:?}");
    }

    s.refinements.insert(f);
    assert_eq!(
        s.lock_reason(ContentId::Refinement(f)),
        Some(LockReason::AlreadyYours),
        "fixture: the refinement is held"
    );
    for (r, l) in s.store_links(&cmd) {
        assert_ne!(l, StoreLink::Lowers, "{f:?} has no price left, yet {r:?}");
    }
    assert!(raised(link(&s, &cmd, grown)), "what it grows still stands");
}

/// Effects count alike whichever kind holds them, in tables or in rules, and
/// whether they name a store directly or through the one item they name.
/// @spec SIM-ENG-081
#[test]
fn an_effect_naming_a_store_raises_it_whichever_kind_holds_it() {
    let s = rich();
    for (cmd, r) in [
        (Command::BuyRite(RiteId::MorningStretch), ResourceId::Warmth),
        (Command::BuyRite(RiteId::SunWardensOath), ResourceId::Pep),
        (Command::BuyRite(RiteId::StainedSunlight), ResourceId::Charm),
        (
            Command::BuyRite(RiteId::SharedSunpuddle),
            ResourceId::Curiosity,
        ),
        (Command::BuyRite(RiteId::GoldenHour), ResourceId::Warmth),
        (
            Command::BuyRite(RiteId::EverwarmFrescoes),
            ResourceId::Warmth,
        ),
        (
            Command::BuyArch(ArchId::EverdreamArch),
            ResourceId::JarsOfPurrs,
        ),
        (Command::BuyArch(ArchId::RibbonArch), ResourceId::Unicorns),
        (
            Command::BuyArch(ArchId::DreamingArch),
            ResourceId::Starshards,
        ),
        (
            Command::BuyDreamLoom(DreamLoomId::ReverieNet),
            ResourceId::Reverie,
        ),
        (
            Command::BuyDreamLoom(DreamLoomId::LucidNap),
            ResourceId::Reverie,
        ),
    ] {
        assert!(raised(link(&s, &cmd, r)), "{cmd:?} grows {r:?}");
    }
}

/// A table's effects read the same way, the one item a key names included: every
/// refinement and rule whose effects name a store or an item that makes one
/// reports that store by the effect's sign.
/// @spec SIM-ENG-081
#[test]
fn every_tabled_effect_links_the_store_it_grows_or_shrinks() {
    let s = rich();
    let check = |cmd: Command, effects: &[(Effect, f64)]| -> usize {
        let mut seen = 0;
        for (key, size) in effects {
            let named: Vec<ResourceId> = match key {
                Effect::ResourceRatio(r) | Effect::MaxCap(r) | Effect::MaxCapRatio(r) => vec![*r],
                Effect::JobRatio(j) => content::job_def(*j)
                    .yield_per_tick
                    .iter()
                    .map(|(r, _)| *r)
                    .collect(),
                Effect::CraftRatioFor(c) => vec![content::craft_output(*c)],
                Effect::StructureRatio(b) => content::structure_def(*b)
                    .productions
                    .iter()
                    .map(|(r, _)| *r)
                    .collect(),
                _ => Vec::new(),
            };
            for r in named {
                seen += 1;
                let l = link(&s, &cmd, r);
                if *size > 0.0 {
                    assert!(raised(l), "{cmd:?} grows {r:?}: {l:?}");
                } else {
                    assert!(lowered(l), "{cmd:?} shrinks {r:?}: {l:?}");
                }
            }
        }
        seen
    };
    let mut seen = 0;
    for f in RefinementId::ALL {
        seen += check(
            Command::BuyRefinement(f),
            content::refinement_def(f).effects,
        );
    }
    let mut rules = 0;
    for r in RuleId::ALL {
        rules += check(Command::AdoptRule(r), content::rule_def(r).effects);
    }
    assert!(seen > 0, "fixture: refinements name stores");
    assert!(rules > 0, "fixture: rules name stores");
}

/// An effect on every store at once names none: the rule that grows every cap
/// marks only what it costs, and a structure multiplying all production names
/// no store for that.
/// @spec SIM-ENG-081
#[test]
fn an_effect_on_every_store_names_none() {
    let s = rich();
    let rule = RuleId::PersonalStashes;
    let cmd = Command::AdoptRule(rule);
    assert!(
        content::rule_def(rule)
            .effects
            .iter()
            .any(|(k, _)| matches!(k, Effect::CapRatio)),
        "fixture: the rule grows every cap"
    );
    let price = s.cost_of_next(ContentId::Rule(rule), 1);
    for (r, l) in s.store_links(&cmd) {
        assert!(
            price.iter().any(|(x, _)| *x == r) && l == StoreLink::Lowers,
            "{rule:?} names {r:?}: {l:?}"
        );
    }
    let mut seen = 0;
    for b in StructureId::ALL {
        let d = content::structure_def(b);
        if d.global_ratio != 0.0 && d.productions.is_empty() && d.converter.is_none() {
            seen += 1;
            for (r, l) in s.store_links(&Command::Build(b)) {
                assert!(
                    l == StoreLink::Lowers || d.cap_bonuses.iter().any(|(x, _)| *x == r),
                    "{b:?} names {r:?}: {l:?}"
                );
            }
        }
    }
    assert!(seen > 0, "fixture: a structure multiplies all production");
}

/// A year-close craft links only the recipes it can run now, so a recipe not
/// yet learned names neither what it would take nor what it would make.
/// @spec SIM-ENG-080, SIM-ENG-081
#[test]
fn a_year_close_craft_links_only_what_it_can_run_now() {
    let mut checked = 0;
    for b in StructureId::ALL {
        let Some(y) = &content::structure_def(b).year_craft else {
            continue;
        };
        let cmd = Command::Build(b);
        for (raw, craft) in y.pairs {
            let mut s = rich();
            s.ideas = IdeaId::ALL
                .into_iter()
                .filter(|i| content::craft_def(*craft).gate != Some(*i))
                .collect();
            if s.craft_unlocked(*craft) {
                continue;
            }
            checked += 1;
            let out = content::craft_output(*craft);
            let d = content::structure_def(b);
            let elsewhere = |r: ResourceId| {
                s.cost_of_next(ContentId::Structure(b), 1)
                    .iter()
                    .any(|(x, _)| *x == r)
                    || y.pairs.iter().any(|(o, c)| {
                        c != craft
                            && s.craft_unlocked(*c)
                            && (*o == r || content::craft_output(*c) == r)
                    })
                    || d.cap_bonuses.iter().any(|(x, _)| *x == r)
                    || d.productions.iter().any(|(x, _)| *x == r)
            };
            if !elsewhere(*raw) {
                assert_eq!(link(&s, &cmd, *raw), None, "{b:?} can't take {raw:?} yet");
            }
            if !elsewhere(out) {
                assert_eq!(link(&s, &cmd, out), None, "{b:?} can't make {out:?} yet");
            }
        }
    }
    assert!(checked > 0, "fixture: some recipe can be held back");
}

/// While the adventure under way sends squads home with nothing, a pounce still
/// costs what it costs and names no goods.
/// @spec SIM-ENG-083
#[test]
fn a_pounce_that_brings_nothing_names_no_goods() {
    let mut s = rich();
    assert!(raised(link(&s, &Command::Pounce(1), ResourceId::Feathers)));
    s.active_adventure = Some(AdventureId::GentlePaws);
    assert_eq!(
        s.store_links(&Command::Pounce(1)),
        vec![(ResourceId::Pep, StoreLink::Lowers)]
    );
}

/// A nap with its loom held brings reverie by its own rule, and says so.
/// @spec SIM-ENG-081
#[test]
fn a_nap_with_its_loom_raises_what_the_loom_brings() {
    let mut s = rich();
    assert_eq!(link(&s, &Command::DeepNap(1), ResourceId::Reverie), None);
    s.dream_looms.insert(DreamLoomId::LucidNap);
    assert!(raised(link(&s, &Command::DeepNap(1), ResourceId::Reverie)));
}

/// A neighbour's good stays unmarked until a caravan has brought it home, so
/// the links spend no discovery the sources query keeps back; its price shows.
/// @spec SIM-ENG-081
#[test]
fn a_neighbours_goods_link_only_once_learned() {
    let mut s = rich();
    let c = ColonyId::ALL
        .into_iter()
        .find(|c| *c != ColonyId::StarWhales && !content::colony_goods(*c).is_empty())
        .expect("fixture: a colony with goods");
    let good = content::colony_goods(c)[0].0;
    s.discovered.insert(c);
    s.learned_goods.remove(&c);
    let cmd = Command::Trade(c, 1);
    let priced: Vec<ResourceId> = s
        .unit_price(&cmd)
        .expect("fixture: the colony is reached")
        .into_iter()
        .map(|(r, _)| r)
        .collect();
    for r in &priced {
        assert!(lowered(link(&s, &cmd, *r)), "the caravan costs {r:?}");
    }
    if !priced.contains(&good) {
        assert_eq!(link(&s, &cmd, good), None, "unlearned, {good:?} is unnamed");
    }
    s.learned_goods.entry(c).or_default().insert(good);
    assert!(raised(link(&s, &cmd, good)), "learned, {good:?} is named");
}

/// The fixed acts have no definition, so what each spends sits beside its rule
/// in the core: applied, each moves the stores it says it does and no others.
/// @spec SIM-ENG-082
#[test]
fn a_fixed_acts_links_hold_true_to_what_it_moves() {
    let mut whale = rich();
    whale.whale_visit_years = 5;
    let site = SiteId::ALL[0];
    let cases: Vec<(Command, GameState)> = vec![
        (Command::GatherCatnip, rich()),
        (Command::Forage, rich()),
        (Command::Rummage, rich()),
        (Command::HostJamboree, rich()),
        (Command::Bask, rich()),
        (Command::HoldParade, rich()),
        (Command::RefineStarshard, rich()),
        (Command::RefineEcho, rich()),
        (Command::CondenseJar, rich()),
        (Command::MakePromise, rich()),
        (Command::SendPenPal(ColonyId::ALL[0]), {
            let mut s = rich();
            s.discovered.insert(ColonyId::ALL[0]);
            s
        }),
        (Command::ScoutPicnic, rich()),
        (Command::LaunchExpedition(site), {
            let mut s = rich();
            s.sites_reached.remove(&site);
            s
        }),
        (Command::SummonStarWhale, rich()),
        (Command::FeedWhale, whale),
        (Command::Pounce(1), rich()),
    ];
    let mut applied = 0;
    for (cmd, mut s) in cases {
        // Room in the capped store the act fills, so it can rise when it should.
        for r in [
            ResourceId::Catnip,
            ResourceId::Twigs,
            ResourceId::Pebbles,
            ResourceId::JarsOfPurrs,
        ] {
            let cap = s.cap(r);
            s.resources[r.ix()] = if cap.is_finite() { cap * 0.5 } else { 1.0 };
        }
        assert!(
            s.command_gate_open(&cmd),
            "fixture: {cmd:?}'s row is listed"
        );
        let links = s.store_links(&cmd);
        let before = s.resources;
        if let Err(e) = s.apply(cmd) {
            panic!("fixture: {cmd:?} should apply, refused with {e:?}");
        }
        applied += 1;
        for r in ResourceId::ALL {
            let (was, now) = (before[r.ix()], s.resources[r.ix()]);
            let l = links.iter().find(|(x, _)| *x == r).map(|(_, l)| *l);
            if now < was - 1e-9 {
                assert!(lowered(l), "{cmd:?} lowered {r:?}, which it doesn't report");
            }
            if now > was + 1e-9 {
                assert!(raised(l), "{cmd:?} raised {r:?}, which it doesn't report");
            }
        }
        for (r, l) in links {
            if l == StoreLink::Lowers {
                assert!(
                    s.resources[r.ix()] < before[r.ix()],
                    "{cmd:?} reports lowering {r:?}, which didn't fall"
                );
            }
        }
    }
    assert_eq!(applied, 16, "every case applied");
}

/// A nap says what it spends, and applied, spends it; what its catch-up makes
/// is the whole meadow running on, so the raises aren't held to its links.
/// @spec SIM-ENG-080, SIM-ENG-082
#[test]
fn a_nap_lowers_what_it_spends() {
    let mut s = rich();
    let cmd = Command::DeepNap(1);
    assert!(lowered(link(&s, &cmd, ResourceId::Starshards)));
    assert!(
        s.command_gate_open(&cmd),
        "fixture: the nap's row is listed"
    );
    let (was, cost) = (s.stock(ResourceId::Starshards), s.starshard_cost(1));
    assert!(cost > 0.0, "fixture: a nap has a price");
    s.apply(cmd).expect("fixture: the nap applies");
    assert!(s.stock(ResourceId::Starshards) <= was + 1e-9);
}

/// A promotion spends its shinies, which applied, fall.
/// @spec SIM-ENG-080, SIM-ENG-082
#[test]
fn a_promotion_lowers_the_shinies_it_spends() {
    let mut s = rich();
    s.kittens.push(Kitten {
        name: "Pip".to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 1,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e9,
        rank: 0,
        trait_id: KittenTrait::Brassy,
        tinkering: None,
        profile: None,
    });
    let ix = s.kittens.len() - 1;
    let cmd = Command::Promote(ix);
    let links = s.store_links(&cmd);
    assert_eq!(links, vec![(ResourceId::Shinies, StoreLink::Lowers)]);
    let was = s.stock(ResourceId::Shinies);
    s.apply(cmd).expect("fixture: the promotion applies");
    assert!(s.stock(ResourceId::Shinies) < was);
}
