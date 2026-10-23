//! Tests-first for where a store can come from:
//!
//!   * SIM-ENG-074 — every source that can raise a resource, in a fixed
//!     order, fixed by the content, a source for every path the core raises a
//!     stock through.
//!   * SIM-ENG-075 — whether the player knows each source yet.

mod common;

use std::collections::BTreeSet;

use clowder_core::*;
use common::*;

fn listed(s: &GameState, r: ResourceId) -> Vec<Source> {
    s.sources_of(r).into_iter().map(|(src, _)| src).collect()
}

/// A meadow with everything open, owned, bought, and met.
fn everything() -> GameState {
    let mut s = GameState::new(8080);
    s.ideas = IdeaId::ALL.into_iter().collect();
    for b in StructureId::ALL {
        s.structures[b.ix()] = 3;
        s.revealed.insert(ContentId::Structure(b));
    }
    s.discovered = ColonyId::ALL.into_iter().collect();
    s.penpal = [10; 8];
    s.whale_visit_years = 5;
    s.arches_bought = ArchId::ALL.into_iter().collect();
    s.refinements = RefinementId::ALL.into_iter().collect();
    for r in RiteId::ALL {
        s.rites.insert(r);
        s.rite_counts[r.ix()] = 1;
    }
    s.peak_devotion = 1.0e12;
    s.dream_looms = DreamLoomId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().collect();
    s.generation = 1;
    adopt(&mut s, 8);
    for (i, j) in JobId::ALL.into_iter().enumerate() {
        if let Some(k) = s.kittens.get_mut(i) {
            k.job = Some(j);
        }
    }
    s
}

fn fill(s: &mut GameState) {
    for r in ResourceId::ALL {
        let cap = s.cap(r);
        give(
            s,
            r,
            if cap.is_finite() && cap > 0.0 {
                cap * 0.5
            } else {
                1.0e6
            },
        );
    }
}

fn risen(before: &GameState, after: &GameState) -> BTreeSet<ResourceId> {
    ResourceId::ALL
        .into_iter()
        .filter(|r| after.stock(*r) > before.stock(*r) + 1e-9)
        .collect()
}

// ---------------------------------------------------------------------------
// SIM-ENG-074 — what is listed
// ---------------------------------------------------------------------------

/// Every table that names what something makes is listed: structure
/// productions and converter outputs, job yields, recipe outputs, every good
/// a colony offers, and the whale's exchange.
/// @spec SIM-ENG-074
#[test]
fn every_tabled_maker_is_listed_as_a_source() {
    let s = GameState::new(1);
    for b in StructureId::ALL {
        let d = content::structure_def(b);
        let mut makes: Vec<ResourceId> = d.productions.iter().map(|(r, _)| *r).collect();
        if let Some(conv) = &d.converter {
            makes.extend(conv.outputs.iter().map(|(r, _)| *r));
        }
        for r in makes {
            assert!(
                listed(&s, r).contains(&Source::Structure(b)),
                "{b:?} makes {r:?}"
            );
        }
    }
    for j in JobId::ALL {
        if let Some((r, _)) = content::job_def(j).yield_per_tick {
            assert!(
                listed(&s, r).contains(&Source::Job(j)),
                "{j:?} yields {r:?}"
            );
        }
    }
    for c in CraftId::ALL {
        let r = content::craft_output(c);
        assert!(
            listed(&s, r).contains(&Source::Craft(c)),
            "{c:?} makes {r:?}"
        );
    }
    for c in ColonyId::ALL {
        for (r, _) in content::colony_goods(c) {
            assert!(
                listed(&s, r).contains(&Source::Colony(c)),
                "{c:?} trades {r:?}"
            );
        }
    }
    for r in [
        ResourceId::Skymaps,
        ResourceId::Starshards,
        ResourceId::Echoes,
    ] {
        assert!(
            listed(&s, r).contains(&Source::Colony(ColonyId::StarWhales)),
            "the whale's exchange brings {r:?}"
        );
    }
}

/// The paths decided in rules rather than tables are listed too.
/// @spec SIM-ENG-074
#[test]
fn the_ruled_paths_are_listed() {
    use ResourceId as R;
    let s = GameState::new(2);
    let cases: Vec<(Source, Vec<ResourceId>)> = vec![
        (
            Source::Structure(StructureId::TreatBakery),
            vec![R::Feathers, R::Bells],
        ),
        (
            Source::Structure(StructureId::StargazingLedge),
            vec![R::Curiosity, R::Skymaps],
        ),
        (Source::Structure(StructureId::RainbowArch), vec![R::Prisms]),
        (
            Source::Structure(StructureId::MemoryChest),
            vec![R::Reverie],
        ),
        (Source::Act(Act::GatherCatnip), vec![R::Catnip]),
        (Source::Act(Act::Forage), vec![R::Twigs]),
        (Source::Act(Act::Rummage), vec![R::Pebbles]),
        (
            Source::Act(Act::Pounce),
            vec![R::Feathers, R::Bells, R::Shinies, R::Unicorns],
        ),
        (Source::Act(Act::ObserveSky), vec![R::Curiosity, R::Skymaps]),
        (Source::Act(Act::HoldParade), vec![R::Rainbows]),
        (Source::Act(Act::RefineStarshard), vec![R::Starshards]),
        (Source::Act(Act::RefineEcho), vec![R::Echoes]),
        (Source::Act(Act::CondenseJar), vec![R::JarsOfPurrs]),
        (Source::Arch(ArchId::SkyArch), vec![R::Prisms]),
        (Source::Arch(ArchId::MemoryArch), vec![R::Dreamcorns]),
        (Source::Arch(ArchId::CrystalArch), vec![R::Prisms]),
        (Source::Arch(ArchId::PrismArch), vec![R::Rainbows]),
        (
            Source::Refinement(RefinementId::StoryPress),
            vec![R::Stories],
        ),
        (Source::Refinement(RefinementId::Bellows), vec![R::Embers]),
        (Source::Refinement(RefinementId::EmberBeds), vec![R::Embers]),
        (Source::Rite(RiteId::StainedSunlight), vec![R::Charm]),
        (Source::Rite(RiteId::GrandBasking), vec![R::Charm]),
        (Source::Loom(DreamLoomId::DreamCatcher), vec![R::Reverie]),
    ];
    for (src, makes) in cases {
        for r in makes {
            assert!(listed(&s, r).contains(&src), "{src:?} raises {r:?}");
        }
    }
}

/// Fixed by the content: the same list in a fresh meadow and a full one,
/// each source once, in the kinds' order and each kind in its order.
/// @spec SIM-ENG-074
#[test]
fn the_list_is_fixed_ordered_and_without_repeats() {
    let fresh = GameState::new(3);
    let full = everything();
    for r in ResourceId::ALL {
        let a = listed(&fresh, r);
        assert_eq!(a, listed(&full, r), "{r:?}: fixed by the content");
        let mut sorted = a.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(a, sorted, "{r:?}: in order, each source once");
    }
}

/// Every command that raises a stock raises only stocks that list it, and
/// every flow the pipeline credits is a listed source — a path added
/// without a source fails here.
/// @spec SIM-ENG-074
#[test]
fn every_path_the_core_raises_a_stock_through_is_listed() {
    let mut base = everything();
    fill(&mut base);
    let mut commands: Vec<(Command, Source)> = vec![
        (Command::GatherCatnip, Source::Act(Act::GatherCatnip)),
        (Command::Forage, Source::Act(Act::Forage)),
        (Command::Rummage, Source::Act(Act::Rummage)),
        (Command::Pounce(3), Source::Act(Act::Pounce)),
        (Command::HoldParade, Source::Act(Act::HoldParade)),
        (Command::RefineStarshard, Source::Act(Act::RefineStarshard)),
        (Command::RefineEcho, Source::Act(Act::RefineEcho)),
        (Command::CondenseJar, Source::Act(Act::CondenseJar)),
    ];
    for c in CraftId::ALL {
        commands.push((Command::Craft(c), Source::Craft(c)));
    }
    for c in ColonyId::ALL {
        commands.push((Command::Trade(c, 5), Source::Colony(c)));
    }
    let mut raised_any = 0;
    for (cmd, src) in commands {
        let mut s = base.clone();
        let before = s.clone();
        if s.apply(cmd).is_err() {
            continue;
        }
        for r in risen(&before, &s) {
            raised_any += 1;
            assert!(
                listed(&base, r).contains(&src),
                "{cmd:?} raised {r:?}, but {src:?} is not listed for it"
            );
        }
    }
    assert!(
        raised_any > 20,
        "fixture: commands raised stocks ({raised_any})"
    );

    // Observing a waiting sky event.
    let mut s = base.clone();
    s.pending_sky_event = Some(s.ticks + 1000);
    let before = s.clone();
    s.apply(Command::ObserveSky).unwrap();
    for r in risen(&before, &s) {
        assert!(
            listed(&base, r).contains(&Source::Act(Act::ObserveSky)),
            "{r:?}"
        );
    }

    // The pipeline's flows: each source credited with raising a stock is
    // listed for it — a structure's output an effect grants listed as the
    // refinement or rite that grants it.
    let mut credited = 0;
    for r in ResourceId::ALL {
        let have = listed(&base, r);
        for (flow, rate) in base.flows(r) {
            if rate <= 0.0 {
                continue;
            }
            credited += 1;
            let ok = match flow {
                FlowSource::Structure(b) => {
                    have.contains(&Source::Structure(b))
                        || have
                            .iter()
                            .any(|x| matches!(x, Source::Refinement(_) | Source::Rite(_)))
                }
                FlowSource::Job(j) => have.contains(&Source::Job(j)),
                FlowSource::Craft(c) => have.contains(&Source::Craft(c)),
                FlowSource::Arch(a) => have.contains(&Source::Arch(a)),
                FlowSource::Kittens => false,
            };
            assert!(ok, "{flow:?} raises {r:?} but is not listed");
        }
    }
    assert!(
        credited > 10,
        "fixture: the pipeline raises stocks ({credited})"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-075 — what the player knows
// ---------------------------------------------------------------------------

fn known(s: &GameState, r: ResourceId, src: Source) -> bool {
    s.sources_of(r)
        .into_iter()
        .find(|(x, _)| *x == src)
        .map(|(_, k)| k)
        .unwrap_or_else(|| panic!("{src:?} is listed for {r:?}"))
}

/// Each kind is known by its own gate, judged on the state at the asking.
/// @spec SIM-ENG-075
#[test]
fn each_source_is_known_by_its_own_gate() {
    let mut s = GameState::new(5);
    // An act: gathering is always open; rummaging waits on its idea.
    assert!(known(
        &s,
        ResourceId::Catnip,
        Source::Act(Act::GatherCatnip)
    ));
    assert!(!known(&s, ResourceId::Pebbles, Source::Act(Act::Rummage)));
    s.ideas.insert(IdeaId::CouchSpelunking);
    assert!(known(&s, ResourceId::Pebbles, Source::Act(Act::Rummage)));

    // A structure: known once the core reports it present.
    let kiln = Source::Structure(StructureId::Kiln);
    assert!(!known(&s, ResourceId::Moonstone, kiln));
    s.revealed.insert(ContentId::Structure(StructureId::Kiln));
    assert!(known(&s, ResourceId::Moonstone, kiln));

    // A job and a recipe: once unlocked.
    assert!(!known(&s, ResourceId::Fluff, Source::Job(JobId::Groomer)));
    s.ideas.insert(IdeaId::GroomedToPurrfection);
    assert!(known(&s, ResourceId::Fluff, Source::Job(JobId::Groomer)));
    let wands = Source::Craft(CraftId::Wands);
    assert!(!known(&s, ResourceId::Wands, wands));
    s.ideas.insert(IdeaId::Tinkering);
    assert!(known(&s, ResourceId::Wands, wands));

    // A colony: met is not enough; only a good it has brought home.
    let deer = Source::Colony(ColonyId::Deer);
    assert!(!known(&s, ResourceId::Moonstone, deer));
    s.discovered.insert(ColonyId::Deer);
    s.learned_goods.insert(ColonyId::Deer, BTreeSet::new());
    assert!(
        !known(&s, ResourceId::Moonstone, deer),
        "met, but not seen to trade it"
    );
    s.learned_goods.insert(
        ColonyId::Deer,
        [ResourceId::Moonstone].into_iter().collect(),
    );
    assert!(known(&s, ResourceId::Moonstone, deer));

    // The whale: once its summoning's gate is open.
    let whale = Source::Colony(ColonyId::StarWhales);
    assert!(!known(&s, ResourceId::Starshards, whale));
    s.structures[StructureId::SkyDock.ix()] = 1;
    s.ideas.insert(IdeaId::SkywardSong);
    assert!(known(&s, ResourceId::Starshards, whale));

    // An arch upgrade, a refinement, a rite, a loom: once present.
    for (r, src, id) in [
        (
            ResourceId::Prisms,
            Source::Arch(ArchId::SkyArch),
            ContentId::Arch(ArchId::SkyArch),
        ),
        (
            ResourceId::Stories,
            Source::Refinement(RefinementId::StoryPress),
            ContentId::Refinement(RefinementId::StoryPress),
        ),
        (
            ResourceId::Charm,
            Source::Rite(RiteId::StainedSunlight),
            ContentId::Rite(RiteId::StainedSunlight),
        ),
        (
            ResourceId::Reverie,
            Source::Loom(DreamLoomId::DreamCatcher),
            ContentId::DreamLoom(DreamLoomId::DreamCatcher),
        ),
    ] {
        assert_eq!(
            known(&s, r, src),
            s.availability(id) != Availability::Hidden,
            "{src:?} is known exactly while present"
        );
    }
    let full = everything();
    for (r, src) in [
        (ResourceId::Prisms, Source::Arch(ArchId::SkyArch)),
        (
            ResourceId::Stories,
            Source::Refinement(RefinementId::StoryPress),
        ),
        (ResourceId::Charm, Source::Rite(RiteId::StainedSunlight)),
        (ResourceId::Reverie, Source::Loom(DreamLoomId::DreamCatcher)),
    ] {
        assert!(known(&full, r, src), "{src:?} is known in a full meadow");
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-076 — no price names a store nothing raises
// ---------------------------------------------------------------------------

/// Every resource a price, a recipe's inputs, or a colony's buy-price names
/// has at least one source, so nothing can be priced in a store the meadow
/// can never raise.
/// @spec SIM-ENG-076
#[test]
fn every_priced_store_has_a_source() {
    let s = everything();
    let mut ids: Vec<ContentId> = Vec::new();
    ids.extend(StructureId::ALL.map(ContentId::Structure));
    ids.extend(IdeaId::ALL.map(ContentId::Idea));
    ids.extend(RefinementId::ALL.map(ContentId::Refinement));
    ids.extend(RuleId::ALL.map(ContentId::Rule));
    ids.extend(RiteId::ALL.map(ContentId::Rite));
    ids.extend(ArchId::ALL.map(ContentId::Arch));
    ids.extend(DreamLoomId::ALL.map(ContentId::DreamLoom));
    ids.extend(CraftId::ALL.map(ContentId::Craft));
    let mut priced: BTreeSet<ResourceId> = BTreeSet::new();
    for id in ids {
        priced.extend(s.cost_of_next(id, 1).into_iter().map(|(r, _)| r));
    }
    for c in ColonyId::ALL {
        if c != ColonyId::StarWhales {
            priced.insert(content::colony_def(c).buy.0);
        }
    }
    assert!(priced.len() > 15, "fixture: many stores are priced");
    for r in priced {
        assert!(
            !s.sources_of(r).is_empty(),
            "{r:?} is named in a price, but nothing raises it"
        );
    }
}

/// The Rabbits sell mint: half their caravans, spring-favoured.
/// @spec SIM-TRADE-003
#[test]
fn the_rabbits_sell_mint() {
    let mint = content::colony_def(ColonyId::Rabbits)
        .sells
        .iter()
        .find(|g| g.resource == ResourceId::Mint)
        .expect("the Rabbits sell mint");
    assert!((mint.amount - 20.0).abs() < 1e-9);
    assert!((mint.chance - 0.5).abs() < 1e-9);
    assert_eq!(mint.season, Some(Season::Spring));
    assert!((mint.season_bonus - 0.25).abs() < 1e-9);
    assert_eq!(mint.min_penpal, 0);
}
