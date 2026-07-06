//! Depth Wave 2 tests: economy depth (RES/BLD/CRAFT/NRG/MESS facets), the
//! banded idea tree, and sky events. Written tests-first from the specs —
//! they compile against the Depth Wave 2 API contract and fail until the
//! content lands.

mod common;

use clowder_core::*;
use common::*;

// ---- Local band map (transcribed from the curiosity design's tree tables) ----
//
// Band 0: ids 0–17 plus 56 (Pouncing Practice, appended late — ids never
// reorder). Band 1: ids 18–27. Band 2: 28–35. Band 3: 36–43. Band 4: 44–55.
fn idea_band(i: IdeaId) -> u32 {
    match i as u32 {
        0..=17 | 56 => 0,
        18..=27 => 1,
        28..=35 => 2,
        36..=43 => 3,
        44..=55 | 57..=58 => 4,
        other => panic!("unmapped idea id {other}"),
    }
}

/// A structure's band is the band of its gate idea; ungated = band 0.
fn structure_band(s: StructureId) -> u32 {
    match content::structure_def(s).gate {
        StructureGate::Idea(i) => idea_band(i),
        _ => 0,
    }
}

/// The crafted resource each recipe outputs (1:1 by the RES/CRAFT tables).
fn craft_output(c: CraftId) -> ResourceId {
    match c {
        CraftId::Wands => ResourceId::Wands,
        CraftId::Tiles => ResourceId::Tiles,
        CraftId::Badges => ResourceId::Badges,
        CraftId::Brass => ResourceId::Brass,
        CraftId::Cogs => ResourceId::Cogs,
        CraftId::Moonsteel => ResourceId::Moonsteel,
        CraftId::Lattices => ResourceId::Lattices,
        CraftId::Pavers => ResourceId::Pavers,
        CraftId::Quills => ResourceId::Quills,
        CraftId::Stories => ResourceId::Stories,
        CraftId::Anthologies => ResourceId::Anthologies,
        CraftId::Schemes => ResourceId::Schemes,
        CraftId::Cairns => ResourceId::Cairns,
        CraftId::Kites => ResourceId::Kites,
        CraftId::Balloons => ResourceId::Balloons,
        CraftId::Moonmilk => ResourceId::Moonmilk,
        CraftId::Dreamweave => ResourceId::Dreamweave,
        CraftId::Cometsilk => ResourceId::Cometsilk,
    }
}

/// Step off the day boundary and quiet any adoption-day zoomies so a single
/// tick measures exactly one production/consumption step.
fn off_boundary(s: &mut GameState) {
    s.ticks += 1;
    s.zoomies_until = 0;
}

// =====================================================================
// RES — Resources & Tiers
// =====================================================================

// @spec SIM-ECO-001
#[test]
fn res_table_has_46_appended_ids_with_tabled_base_caps() {
    assert_eq!(ResourceId::ALL.len(), 46);
    // Append-only id spots across the table.
    assert_eq!(ResourceId::Catnip.ix(), 0);
    assert_eq!(ResourceId::Curiosity.ix(), 3);
    assert_eq!(ResourceId::Hushfelt.ix(), 11);
    assert_eq!(ResourceId::Pep.ix(), 12);
    assert_eq!(ResourceId::Feathers.ix(), 16);
    assert_eq!(ResourceId::JarsOfPurrs.ix(), 25);
    assert_eq!(ResourceId::Reverie.ix(), 26);
    assert_eq!(ResourceId::Wands.ix(), 27);
    assert_eq!(ResourceId::Kites.ix(), 40);
    assert_eq!(ResourceId::Cometsilk.ix(), 44);
    assert_eq!(ResourceId::Cleverness.ix(), 45);

    // A new meadow holds nothing, across the whole widened array.
    let s = GameState::new(101);
    assert_eq!(s.resources, [0.0; 46]);

    // Base caps per the RES table; ∞ = uncapped.
    let cap_of = |r: ResourceId| content::BASE_CAPS[r.ix()];
    assert_eq!(cap_of(ResourceId::Catnip), 200.0);
    assert_eq!(cap_of(ResourceId::Twigs), 50.0);
    assert_eq!(cap_of(ResourceId::Fluff), 25.0);
    assert_eq!(cap_of(ResourceId::Curiosity), 0.0);
    assert_eq!(cap_of(ResourceId::Pebbles), 60.0);
    assert_eq!(cap_of(ResourceId::Buttons), 40.0);
    assert_eq!(cap_of(ResourceId::Embers), 30.0);
    assert_eq!(cap_of(ResourceId::Shinies), 15.0);
    assert_eq!(cap_of(ResourceId::Dew), 0.0);
    assert_eq!(cap_of(ResourceId::Moonstone), 8.0);
    assert_eq!(cap_of(ResourceId::Stardust), 0.0);
    assert_eq!(cap_of(ResourceId::Hushfelt), 0.0);
    assert_eq!(cap_of(ResourceId::Pep), 100.0);
    assert_eq!(cap_of(ResourceId::Charm), 80.0);
    assert_eq!(cap_of(ResourceId::Warmth), 0.0);
    assert_eq!(cap_of(ResourceId::Skymaps), 0.0);
    assert_eq!(cap_of(ResourceId::JarsOfPurrs), 0.0);
    assert!(cap_of(ResourceId::Feathers).is_infinite());
    assert!(cap_of(ResourceId::Mint).is_infinite());
    assert!(cap_of(ResourceId::Unicorns).is_infinite());
    assert!(cap_of(ResourceId::Reverie).is_infinite());
    assert!(cap_of(ResourceId::Wands).is_infinite());
    assert!(cap_of(ResourceId::Cleverness).is_infinite());
}

// @spec SIM-ECO-001, SIM-ECO-025
#[test]
fn uncapped_resources_never_clamp() {
    let mut s = GameState::new(102);
    s.ticks = 3; // mid-day: no boundary processing in the measured ticks
    give(&mut s, ResourceId::Feathers, 1e12); // no kittens, so no luxury drain
    give(&mut s, ResourceId::Rainbows, 5e11);
    give(&mut s, ResourceId::Wands, 7e11);
    give(&mut s, ResourceId::Cleverness, 1e10);
    run_ticks(&mut s, 2);
    assert_eq!(s.stock(ResourceId::Feathers), 1e12, "luxuries are uncapped");
    assert_eq!(s.stock(ResourceId::Rainbows), 5e11, "mystics are uncapped");
    assert_eq!(s.stock(ResourceId::Wands), 7e11, "crafted goods are uncapped");
    assert_eq!(s.stock(ResourceId::Cleverness), 1e10, "cleverness has no ceiling");
    assert!(s.cap(ResourceId::Feathers).is_infinite());
}

// @spec SIM-ECO-001, SIM-ECO-002
#[test]
fn cap_zero_resources_hold_nothing_until_their_extender_exists() {
    // Dew, warmth, skymaps, stardust all start at cap 0: any stock is
    // trimmed away by the clamp phase until an extender structure exists.
    let mut s = GameState::new(103);
    s.ticks = 3;
    for r in [ResourceId::Dew, ResourceId::Warmth, ResourceId::Skymaps, ResourceId::Stardust] {
        give(&mut s, r, 5.0);
    }
    run_ticks(&mut s, 1);
    for r in [ResourceId::Dew, ResourceId::Warmth, ResourceId::Skymaps, ResourceId::Stardust] {
        assert_eq!(s.stock(r), 0.0, "{r:?} accrues nothing at cap 0");
    }

    // With the extender built, the stock survives (and the Collector trickles).
    let mut s = GameState::new(104);
    s.ticks = 3;
    grant(&mut s, StructureId::DewCollector, 1);
    give(&mut s, ResourceId::Dew, 5.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Dew) - 5.02).abs() < 1e-9, "cap 1200 holds; +0.02/t trickles");

    let mut s = GameState::new(105);
    s.ticks = 3;
    grant(&mut s, StructureId::StarBasin, 1);
    give(&mut s, ResourceId::Stardust, 5.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Stardust) - 5.0003).abs() < 1e-9, "cap 60 holds; +0.0003/t");
}

// =====================================================================
// BLD — Structures
// =====================================================================

// @spec SIM-ECO-002
#[test]
fn new_structures_extend_caps_including_multi_resource_extenders() {
    let mut s = GameState::new(106);
    grant(&mut s, StructureId::Shed, 2);
    grant(&mut s, StructureId::Porch, 1); // kite high-water 0 ⇒ Porch multiplier ×1
    grant(&mut s, StructureId::StudyNook, 1);
    grant(&mut s, StructureId::SunRoom, 2);
    grant(&mut s, StructureId::SunbeamAltar, 1);
    grant(&mut s, StructureId::StarBasin, 1);
    grant(&mut s, StructureId::DewCollector, 1);
    grant(&mut s, StructureId::StargazingLedge, 2);
    grant(&mut s, StructureId::BoxLabyrinth, 1);

    assert!((s.cap(ResourceId::Twigs) - (50.0 + 2.0 * 150.0 + 400.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Pebbles) - (60.0 + 2.0 * 200.0 + 500.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Buttons) - (40.0 + 2.0 * 25.0 + 80.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Embers) - (30.0 + 2.0 * 30.0 + 50.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Moonstone) - (8.0 + 2.0 * 10.0 + 25.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Shinies) - (15.0 + 2.0 * 5.0 + 12.0 + 100.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Catnip) - (200.0 + 1000.0)).abs() < 1e-9);
    // Curiosity: Study Nook 200 + Ledges 2×800 + Labyrinth 1500.
    assert!((s.cap(ResourceId::Curiosity) - 3300.0).abs() < 1e-9);
    // Warmth: Sun Rooms 2×75 + Sunbeam Altar 120.
    assert!((s.cap(ResourceId::Warmth) - 270.0).abs() < 1e-9);
    // Charm: base 80 + Sun Rooms 2×60 + Sunbeam Altar 100.
    assert!((s.cap(ResourceId::Charm) - 300.0).abs() < 1e-9);
    assert!((s.cap(ResourceId::Stardust) - 60.0).abs() < 1e-9);
    assert!((s.cap(ResourceId::Dew) - 1200.0).abs() < 1e-9);
    assert!((s.cap(ResourceId::Skymaps) - 20.0).abs() < 1e-9);
}

// @spec SIM-ECO-028
#[test]
fn structure_table_spot_anchors_across_bands() {
    assert_eq!(StructureId::ALL.len(), 53, "exactly the BLD table, no staged rebuilds");
    assert_eq!(GameState::new(107).structures, [0u32; 53]);
    assert_eq!(StructureId::CatnipPatch.ix(), 0);
    assert_eq!(StructureId::WindowPerch.ix(), 7);
    assert_eq!(StructureId::Porch.ix(), 14);
    assert_eq!(StructureId::UnicornMeadow.ix(), 37);

    // Band 0.
    let d = content::structure_def(StructureId::HerbGarden);
    assert_eq!(d.base_costs, &[(ResourceId::Pebbles, 60.0)]);
    assert_eq!(d.ratio, 1.12);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::Horticulture));

    let d = content::structure_def(StructureId::Hearth);
    assert_eq!(d.base_costs, &[(ResourceId::Pebbles, 45.0), (ResourceId::Twigs, 25.0)]);
    assert_eq!(d.ratio, 1.15);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::Emberkeeping));

    // Band 1.
    let d = content::structure_def(StructureId::StargazingLedge);
    assert_eq!(
        d.base_costs,
        &[
            (ResourceId::Buttons, 250.0),
            (ResourceId::Curiosity, 800.0),
            (ResourceId::Tiles, 20.0),
            (ResourceId::Lattices, 40.0),
        ]
    );
    assert_eq!(d.ratio, 1.10);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::SkyCharts));

    let d = content::structure_def(StructureId::Porch);
    assert_eq!(
        d.base_costs,
        &[(ResourceId::Tiles, 40.0), (ResourceId::Badges, 60.0), (ResourceId::Lattices, 4.0)]
    );
    assert_eq!(d.ratio, 1.15);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::PorchLogistics));

    let d = content::structure_def(StructureId::TeaHouse);
    assert_eq!(
        d.base_costs,
        &[
            (ResourceId::Twigs, 800.0),
            (ResourceId::Charm, 600.0),
            (ResourceId::Mint, 4.0),
            (ResourceId::Quills, 300.0),
        ]
    );
    assert_eq!(d.ratio, 1.5, "a deliberate wall");
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::TeaCeremonies));

    // Band 2.
    let d = content::structure_def(StructureId::Kiln);
    assert_eq!(
        d.base_costs,
        &[(ResourceId::Moonstone, 4.0), (ResourceId::Dew, 150.0), (ResourceId::Brass, 30.0)]
    );
    assert_eq!(d.ratio, 1.15);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::CeramicArts));

    // Band 3.
    let d = content::structure_def(StructureId::DynamoWheel);
    assert_eq!(
        d.base_costs,
        &[(ResourceId::Cogs, 4.0), (ResourceId::Moonsteel, 8.0), (ResourceId::Schemes, 1.0)]
    );
    assert_eq!(d.ratio, 1.25);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::Dynamos));

    let d = content::structure_def(StructureId::YarnComputer);
    assert_eq!(d.base_costs, &[(ResourceId::Stardust, 30.0), (ResourceId::Curiosity, 60000.0)]);
    assert_eq!(d.ratio, 1.15);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::YarnComputing));

    // Band 0 wall (mystic-priced).
    let d = content::structure_def(StructureId::UnicornMeadow);
    assert_eq!(d.base_costs, &[(ResourceId::Unicorns, 2.0)]);
    assert_eq!(d.ratio, 1.75);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::UnicornLore));

    // Gate behavior: locked until the idea, unlocked after.
    let mut s = GameState::new(108);
    assert!(!s.structure_unlocked(StructureId::Kiln));
    assert!(!s.structure_unlocked(StructureId::DynamoWheel));
    s.ideas.insert(IdeaId::CeramicArts);
    s.ideas.insert(IdeaId::Dynamos);
    assert!(s.structure_unlocked(StructureId::Kiln));
    assert!(s.structure_unlocked(StructureId::DynamoWheel));
}

// @spec SIM-ECO-024
#[test]
fn banded_reachability_first_copies_fit_achievable_caps() {
    // Achievable caps: every cap-extending structure at 20 copies, plus the
    // Porch ratchet at an arbitrarily-earned kite high-water (kites are
    // uncapped and accumulate, so any high-water is achievable).
    let mut world = GameState::new(109);
    for st in StructureId::ALL {
        grant(&mut world, st, 20);
    }
    world.kites_high_water = 1e9;

    for st in StructureId::ALL {
        let def = content::structure_def(st);
        // Copies that must stay reachable, by the gate idea's band: bands 0–2
        // keep 20, band 3 keeps 5, band 4 keeps its first copy. Declared
        // walls (Tea House, Treat Bakery — deliberate few-copy structures per
        // the price-band vocabulary) use the 5-copy rule instead of their
        // band's horizon.
        let wall = matches!(st, StructureId::TeaHouse | StructureId::TreatBakery);
        let monument = matches!(st, StructureId::YarnComputer); // first-copy horizon (SIM-ECO-024 amended)
        let copies: u32 = if monument {
            1
        } else if wall {
            5
        } else {
            match structure_band(st) {
                0..=2 => 20,
                3 => 5,
                _ => 1,
            }
        };
        for k in 0..copies {
            for (r, base) in def.base_costs {
                if *r == ResourceId::Hushfelt {
                    // Exempt: the hushfelt cap extender is owed by the cosmic
                    // leaf and absent this wave (design's explicit exemption).
                    continue;
                }
                let cap = world.cap(*r);
                if !cap.is_finite() {
                    continue; // uncapped components (crafted goods etc.) accumulate
                }
                let cost = (base * def.ratio.powi(k as i32)).ceil();
                assert!(
                    cost <= cap,
                    "{}: copy {} needs {} {:?}, but the max achievable cap is {}",
                    def.name,
                    k + 1,
                    cost,
                    r,
                    cap
                );
            }
        }
    }
}

// =====================================================================
// RES — Luxury drain & manual actions
// =====================================================================

// @spec SIM-ECO-026
#[test]
fn luxuries_drain_per_home_kitten_and_lapse_gently() {
    let mut s = GameState::new(110);
    adopt(&mut s, 3);
    for k in &mut s.kittens {
        k.job = None;
    }
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 150.0);
    give(&mut s, ResourceId::Feathers, 2.0);
    give(&mut s, ResourceId::Bells, 2.0);
    give(&mut s, ResourceId::Mint, 2.0);
    run_ticks(&mut s, 1);
    // Three Home kittens: feathers 0.001, bells 0.0007, mint 0.0001 each.
    assert!((s.stock(ResourceId::Feathers) - (2.0 - 3.0 * 0.001)).abs() < 1e-9);
    assert!((s.stock(ResourceId::Bells) - (2.0 - 3.0 * 0.0007)).abs() < 1e-9);
    assert!((s.stock(ResourceId::Mint) - (2.0 - 3.0 * 0.0001)).abs() < 1e-9);

    // A Gift Stand eases demand by 4%.
    grant(&mut s, StructureId::GiftStand, 1);
    give(&mut s, ResourceId::Feathers, 2.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Feathers) - (2.0 - 3.0 * 0.001 * 0.96)).abs() < 1e-9);

    // The drain floors at zero, and an empty shelf is never a famine: the
    // bonus lapses, nobody gets Grumpy.
    give(&mut s, ResourceId::Feathers, 0.0005); // less than one tick's demand
    give(&mut s, ResourceId::Bells, 0.0);
    give(&mut s, ResourceId::Catnip, 150.0);
    run_ticks(&mut s, 1);
    assert_eq!(s.stock(ResourceId::Feathers), 0.0, "drain floors at zero");
    assert_eq!(s.stock(ResourceId::Bells), 0.0);
    assert!(!s.grumpy, "empty luxuries never trigger Grumpiness");
}

// @spec SIM-ECO-027
#[test]
fn rummage_is_idea_gated_pebble_foraging() {
    let mut s = GameState::new(111);
    assert_eq!(s.apply(Command::Rummage).unwrap_err(), CommandError::Locked);
    assert_eq!(s.stock(ResourceId::Pebbles), 0.0);

    s.ideas.insert(IdeaId::CouchSpelunking);
    s.apply(Command::Rummage).unwrap();
    assert!((s.stock(ResourceId::Pebbles) - 0.5).abs() < 1e-9);

    give(&mut s, ResourceId::Pebbles, 60.0); // base pebble cap
    assert_eq!(s.apply(Command::Rummage).unwrap_err(), CommandError::StorageFull);
    assert_eq!(s.stock(ResourceId::Pebbles), 60.0);
}

// =====================================================================
// BLD — Converters
// =====================================================================

// @spec SIM-ECO-029
#[test]
fn hearth_converter_runs_full_and_starves_proportionally() {
    // Full supply: −0.05 twigs −0.10 pebbles → +0.02 buttons +0.001 embers.
    let mut s = GameState::new(112);
    s.ticks = 3;
    grant(&mut s, StructureId::Hearth, 1);
    give(&mut s, ResourceId::Twigs, 10.0);
    give(&mut s, ResourceId::Pebbles, 20.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Twigs) - 9.95).abs() < 1e-9);
    assert!((s.stock(ResourceId::Pebbles) - 19.90).abs() < 1e-9);
    assert!((s.stock(ResourceId::Buttons) - 0.02).abs() < 1e-9);
    assert!((s.stock(ResourceId::Embers) - 0.001).abs() < 1e-9);

    // Worst-supplied input at 50% throttles ALL flows to 50% — the plentiful
    // input is drawn at half rate too, and outputs halve. No error, no spam.
    let mut s = GameState::new(113);
    s.ticks = 3;
    grant(&mut s, StructureId::Hearth, 1);
    give(&mut s, ResourceId::Twigs, 0.025); // half of the 0.05 demand
    give(&mut s, ResourceId::Pebbles, 20.0);
    let events = run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Twigs) - 0.0).abs() < 1e-9);
    assert!((s.stock(ResourceId::Pebbles) - 19.95).abs() < 1e-9);
    assert!((s.stock(ResourceId::Buttons) - 0.01).abs() < 1e-9);
    assert!((s.stock(ResourceId::Embers) - 0.0005).abs() < 1e-9);
    assert!(events.is_empty(), "a starving converter idles quietly");
}

// @spec SIM-ECO-029
#[test]
fn kiln_full_supply_rates() {
    // −0.5 pebbles −0.024 dew → +0.15 buttons +0.0005 moonstone per tick.
    let mut s = GameState::new(114);
    s.ticks = 3;
    grant(&mut s, StructureId::Kiln, 1);
    give(&mut s, ResourceId::Pebbles, 20.0);
    give(&mut s, ResourceId::Dew, 1.0); // stocked for the tick; clamps later (cap 0)
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Pebbles) - 19.5).abs() < 1e-9);
    assert!((s.stock(ResourceId::Buttons) - 0.15).abs() < 1e-9);
    assert!((s.stock(ResourceId::Moonstone) - 0.0005).abs() < 1e-9);
}

// @spec SIM-ECO-030
#[test]
fn treat_bakery_output_scales_with_the_pep_cap_not_the_pep_stock() {
    // mpr = pep_cap × 0.007 / 100 = 100 × 0.007 / 100 = 0.007 at base cap.
    // Feathers ×1.25·mpr, bells ×0.3·mpr; inputs −0.004 shinies −0.6 pep.
    let mut s = GameState::new(115);
    s.ticks = 3;
    grant(&mut s, StructureId::TreatBakery, 1);
    give(&mut s, ResourceId::Shinies, 15.0);
    give(&mut s, ResourceId::Pep, 100.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Feathers) - 1.25 * 0.007).abs() < 1e-9);
    assert!((s.stock(ResourceId::Bells) - 0.3 * 0.007).abs() < 1e-9);
    assert!((s.stock(ResourceId::Shinies) - (15.0 - 0.004)).abs() < 1e-9);
    assert!((s.stock(ResourceId::Pep) - (100.0 - 0.6)).abs() < 1e-9);

    // Storage feeds production: half the pep *stock*, same cap, same output.
    let mut half = GameState::new(116);
    half.ticks = 3;
    grant(&mut half, StructureId::TreatBakery, 1);
    give(&mut half, ResourceId::Shinies, 15.0);
    give(&mut half, ResourceId::Pep, 50.0);
    run_ticks(&mut half, 1);
    assert!((half.stock(ResourceId::Feathers) - 1.25 * 0.007).abs() < 1e-9);
}

// @spec SIM-ECO-031
#[test]
fn porch_caps_ratchet_on_the_kite_high_water_and_never_shrink() {
    let mut s = GameState::new(117);
    s.ticks = 3;
    grant(&mut s, StructureId::Porch, 1);
    assert!((s.cap(ResourceId::Catnip) - 1200.0).abs() < 1e-9, "no kites yet: ×1");

    // Holding 100 kites: multiplier = 1 + limited_dr(0.01×100, 2.25) = 2.
    give(&mut s, ResourceId::Kites, 100.0);
    run_ticks(&mut s, 1);
    assert!((s.kites_high_water - 100.0).abs() < 1e-9);
    assert!((s.cap(ResourceId::Catnip) - (200.0 + 1000.0 * 2.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Twigs) - (50.0 + 400.0 * 2.0)).abs() < 1e-9);
    assert!((s.cap(ResourceId::Shinies) - (15.0 + 12.0 * 2.0)).abs() < 1e-9);

    // Spending every kite (direct stock reduction) never confiscates storage:
    // the high-water is a ratchet.
    give(&mut s, ResourceId::Kites, 0.0);
    run_ticks(&mut s, 1);
    assert!((s.kites_high_water - 100.0).abs() < 1e-9, "high-water persists");
    assert!((s.cap(ResourceId::Catnip) - 2200.0).abs() < 1e-9, "caps never shrink");

    // A lower later stock doesn't lower it either.
    give(&mut s, ResourceId::Kites, 50.0);
    run_ticks(&mut s, 1);
    assert!((s.kites_high_water - 100.0).abs() < 1e-9);
}

// =====================================================================
// NRG/ENG — the global-production layer (layer 6) and tier exemptions
// =====================================================================

// @spec SIM-ECO-032, SIM-ENG-021
#[test]
fn dynamo_and_star_basin_boost_material_production() {
    let mut base = GameState::new(118);
    adopt(&mut base, 1);
    base.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    off_boundary(&mut base);
    give(&mut base, ResourceId::Catnip, 100.0);
    give(&mut base, ResourceId::Dew, 10.0); // dynamo fuel

    let mut plain = base.clone();
    let mut dynamo = base.clone();
    grant(&mut dynamo, StructureId::DynamoWheel, 1);
    let mut amplified = dynamo.clone();
    grant(&mut amplified, StructureId::Cogworks, 1);
    let mut basin = base.clone();
    grant(&mut basin, StructureId::StarBasin, 1);
    let mut cogs_only = base.clone();
    grant(&mut cogs_only, StructureId::Cogworks, 1);
    let mut starved = base.clone();
    grant(&mut starved, StructureId::DynamoWheel, 1);
    give(&mut starved, ResourceId::Dew, 0.0);

    // Effect-key view: +2%/active dynamo, ×(1+0.15×Cogworks); basin +5%.
    assert!((plain.effect(Effect::GlobalProductionRatio) - 0.0).abs() < 1e-9);
    assert!((dynamo.effect(Effect::GlobalProductionRatio) - 0.02).abs() < 1e-9);
    assert!((amplified.effect(Effect::GlobalProductionRatio) - 0.023).abs() < 1e-9);
    assert!((basin.effect(Effect::GlobalProductionRatio) - 0.05).abs() < 1e-9);

    run_ticks(&mut plain, 1);
    run_ticks(&mut dynamo, 1);
    run_ticks(&mut amplified, 1);
    run_ticks(&mut basin, 1);
    run_ticks(&mut cogs_only, 1);
    run_ticks(&mut starved, 1);

    // Gardener 0.15/t at mood 1.0; the global layer multiplies it.
    let gain = |s: &GameState| s.stock(ResourceId::Catnip);
    assert!((gain(&dynamo) - gain(&plain) - 0.15 * 0.02).abs() < 1e-9);
    assert!((gain(&amplified) - gain(&plain) - 0.15 * 0.023).abs() < 1e-9);
    assert!((gain(&basin) - gain(&plain) - 0.15 * 0.05).abs() < 1e-9);
    // Cogworks alone amplifies nothing (it boosts dynamos, not the world).
    assert!((gain(&cogs_only) - gain(&plain)).abs() < 1e-9);
    // A starved wheel spins down entirely: no fuel, no boost, no energy.
    assert!((gain(&starved) - gain(&plain)).abs() < 1e-9);
    assert!((starved.energy_production() - 0.0).abs() < 1e-9);
}

// @spec SIM-ECO-025, SIM-ECO-032, SIM-ENG-021
#[test]
fn global_layers_never_touch_transient_or_mystic_income() {
    // The key tier-exemption stack test: a fueled Dynamo Wheel boosts material
    // flows but must never change curiosity, pep, or mystic income.
    let mut base = GameState::new(119);
    grant(&mut base, StructureId::CardboardBox, 1); // Observer gate + curiosity cap
    grant(&mut base, StructureId::UnicornMeadow, 1); // +0.001 unicorns/t (mystic)
    adopt(&mut base, 2);
    base.ideas.insert(IdeaId::PouncingPractice);
    base.apply(Command::AssignJob(JobId::Observer)).unwrap();
    base.apply(Command::AssignJob(JobId::Pouncer)).unwrap();
    off_boundary(&mut base);
    give(&mut base, ResourceId::Catnip, 100.0);
    give(&mut base, ResourceId::Dew, 10.0);

    let mut plain = base.clone();
    let mut boosted = base.clone();
    grant(&mut boosted, StructureId::DynamoWheel, 1);
    grant(&mut boosted, StructureId::StarBasin, 1);
    run_ticks(&mut plain, 1);
    run_ticks(&mut boosted, 1);

    for r in [ResourceId::Curiosity, ResourceId::Pep, ResourceId::Unicorns] {
        assert!(plain.stock(r) > 0.0, "{r:?} income must exist for the test to bite");
        assert_eq!(
            plain.stock(r),
            boosted.stock(r),
            "{r:?} is exempt from global layers 6–8"
        );
    }
}

// =====================================================================
// CRAFT — Crafting
// =====================================================================

// @spec SIM-ECO-033
#[test]
fn craft_command_is_gated_all_or_nothing_and_ratio_boosted() {
    // Locked until the granting idea, even with inputs in paw.
    let mut s = GameState::new(120);
    give(&mut s, ResourceId::Twigs, 15.0);
    assert_eq!(s.apply(Command::Craft(CraftId::Wands)).unwrap_err(), CommandError::Locked);
    assert_eq!(s.stock(ResourceId::Twigs), 15.0);

    // Unlocked: one batch spends the recipe and yields 1 × (1 + craft ratio).
    s.ideas.insert(IdeaId::Tinkering);
    s.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert_eq!(s.stock(ResourceId::Twigs), 0.0);
    assert!((s.stock(ResourceId::Wands) - 1.0).abs() < 1e-9);

    // Insufficient inputs: all-or-nothing, nothing spent.
    give(&mut s, ResourceId::Twigs, 14.0);
    let err = s.apply(Command::Craft(CraftId::Wands)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Twigs), 14.0);
    assert!((s.stock(ResourceId::Wands) - 1.0).abs() < 1e-9);

    // Multi-input recipes spend together or not at all.
    s.ideas.insert(IdeaId::Excavation);
    give(&mut s, ResourceId::Buttons, 10.0);
    give(&mut s, ResourceId::Embers, 5.0);
    let err = s.apply(Command::Craft(CraftId::Brass)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Buttons), 10.0);
    assert_eq!(s.stock(ResourceId::Embers), 5.0);
    give(&mut s, ResourceId::Embers, 10.0 - 1e-10); // epsilon-tolerant spend
    s.apply(Command::Craft(CraftId::Brass)).unwrap();
    assert!((s.stock(ResourceId::Brass) - 1.0).abs() < 1e-9);
    assert_eq!(s.stock(ResourceId::Buttons), 0.0);
    assert!(s.stock(ResourceId::Embers) >= 0.0, "deduction floors at zero");

    // Craft ratio: 2 Craft Corners (+6% each) + 1 Toy Workshop (+5%) = +17%.
    let mut s = GameState::new(121);
    s.ideas.insert(IdeaId::Tinkering);
    grant(&mut s, StructureId::CraftCorner, 2);
    grant(&mut s, StructureId::ToyWorkshop, 1);
    assert!((s.effect(Effect::CraftRatio) - 0.17).abs() < 1e-9);
    give(&mut s, ResourceId::Twigs, 15.0);
    s.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!((s.stock(ResourceId::Wands) - 1.17).abs() < 1e-9);
}

// @spec SIM-ECO-034
#[test]
fn craft_recipe_table_spot_anchors() {
    assert_eq!(CraftId::ALL.len(), 18, "exactly the CRAFT table");
    assert_eq!(CraftId::Wands as usize, 0);
    assert_eq!(CraftId::Cometsilk as usize, 17);

    let d = content::craft_def(CraftId::Wands);
    assert_eq!(d.inputs, &[(ResourceId::Twigs, 15.0)]);
    assert_eq!(d.gate, Some(IdeaId::Tinkering));

    let d = content::craft_def(CraftId::Brass);
    assert_eq!(d.inputs, &[(ResourceId::Buttons, 10.0), (ResourceId::Embers, 10.0)]);
    assert_eq!(d.gate, Some(IdeaId::Excavation));

    let d = content::craft_def(CraftId::Pavers);
    assert_eq!(d.inputs, &[(ResourceId::Tiles, 250.0), (ResourceId::Brass, 3.0)]);
    assert_eq!(d.gate, Some(IdeaId::Moonmetallurgy));

    let d = content::craft_def(CraftId::Quills);
    assert_eq!(d.inputs, &[(ResourceId::Feathers, 18.0)]);
    assert_eq!(d.gate, Some(IdeaId::OralTradition));

    let d = content::craft_def(CraftId::Kites);
    assert_eq!(
        d.inputs,
        &[(ResourceId::Skymaps, 3.0), (ResourceId::Badges, 15.0), (ResourceId::Lattices, 10.0)]
    );
    assert_eq!(d.gate, Some(IdeaId::PorchLogistics));

    let d = content::craft_def(CraftId::Anthologies);
    assert_eq!(d.inputs, &[(ResourceId::Curiosity, 1000.0), (ResourceId::Stories, 5.0)]);
    assert_eq!(d.gate, Some(IdeaId::AnthologyBinding));

    let d = content::craft_def(CraftId::Schemes);
    assert_eq!(d.inputs, &[(ResourceId::Curiosity, 2500.0), (ResourceId::Anthologies, 3.0)]);
    assert_eq!(d.gate, Some(IdeaId::GrandSchemes));

    let d = content::craft_def(CraftId::Dreamweave);
    assert_eq!(d.inputs, &[(ResourceId::Hushfelt, 100.0), (ResourceId::Moonsteel, 250.0)]);
    assert_eq!(d.gate, Some(IdeaId::Dreamweaving));
}

// @spec SIM-ECO-034
#[test]
fn no_recipe_is_dead_every_input_is_producible_by_its_band() {
    // Earliest band at which each non-crafted resource has a producer
    // (structure production, converter output, job yield, or manual action),
    // per the BLD/jobs/actions tables. None = expedition/colony faucet,
    // exempted by the spec's explicit list.
    let raw_producer_band = |r: ResourceId| -> Option<u32> {
        match r {
            // Owed by the society/wonder waves: expedition & colony faucets.
            ResourceId::Feathers
            | ResourceId::Bells
            | ResourceId::Mint
            | ResourceId::Shinies
            | ResourceId::Unicorns => None,
            // Band 0: patches/gather (catnip), forage/collectors (twigs),
            // groomers (fluff), boxes (curiosity), rummage/couch mines
            // (pebbles), hearths (buttons, embers), pouncers (pep), story
            // circles (charm).
            ResourceId::Catnip
            | ResourceId::Twigs
            | ResourceId::Fluff
            | ResourceId::Curiosity
            | ResourceId::Pebbles
            | ResourceId::Buttons
            | ResourceId::Embers
            | ResourceId::Pep
            | ResourceId::Charm => Some(0),
            // Band 1: sun rooms (warmth), sky events via ledges (skymaps).
            ResourceId::Warmth | ResourceId::Skymaps => Some(1),
            // Band 2: dew collectors, kilns (native moonstone trickle).
            ResourceId::Dew | ResourceId::Moonstone => Some(2),
            // Band 3: star basins / prism spinners.
            ResourceId::Stardust => Some(3),
            // Band 4: hushfelt sources open with Hushfelt Seeking.
            ResourceId::Hushfelt => Some(4),
            other => panic!("{other:?} appears as a craft input with no producer mapping"),
        }
    };

    for c in CraftId::ALL {
        let def = content::craft_def(c);
        let gate = def.gate.expect("every craft rides the idea tree");
        let band = idea_band(gate);
        for (r, amount) in def.inputs {
            assert!(*amount > 0.0, "{c:?}: degenerate input amount");
            // Crafted inputs: producible at their own recipe's gate band.
            let producer = match CraftId::ALL.into_iter().find(|k| craft_output(*k) == *r) {
                Some(input_craft) => Some(idea_band(
                    content::craft_def(input_craft).gate.expect("crafts are idea-gated"),
                )),
                None => raw_producer_band(*r),
            };
            match producer {
                None => (), // exempt faucet, by the spec's explicit list
                Some(pb) => assert!(
                    pb <= band,
                    "{c:?} unlocks in band {band} but needs {r:?}, first producible in band {pb}"
                ),
            }
        }
    }
}

// =====================================================================
// NRG — Energy
// =====================================================================

// @spec SIM-ECO-035
#[test]
fn energy_accounting_sums_active_structures() {
    // An empty grid: nothing produced, nothing consumed, no deficit.
    let s = GameState::new(122);
    assert_eq!(s.energy_production(), 0.0);
    assert_eq!(s.energy_consumption(), 0.0);
    assert_eq!(s.energy_deficit_ratio(), 1.0, "no consumption, no brownout");

    // The Sunning Deck breathes with the year: +2, winter ×0.75, summer ×1.25.
    let mut s = GameState::new(123);
    grant(&mut s, StructureId::SunningDeck, 1);
    s.ticks = 105; // spring
    assert!((s.energy_production() - 2.0).abs() < 1e-9);
    s.ticks = 751; // winter
    assert!((s.energy_production() - 1.5).abs() < 1e-9);
    s.ticks = 1251; // summer
    assert!((s.energy_production() - 2.5).abs() < 1e-9);

    // Yarn Computers consume 2n + n²/2 in total.
    let mut s = GameState::new(124);
    grant(&mut s, StructureId::YarnComputer, 1);
    assert!((s.energy_consumption() - 2.5).abs() < 1e-9);
    grant(&mut s, StructureId::YarnComputer, 3);
    assert!((s.energy_consumption() - 16.0).abs() < 1e-9);

    // The Dynamo Wheel produces 5 × its fuel fraction.
    let mut s = GameState::new(125);
    s.ticks = 105;
    grant(&mut s, StructureId::DynamoWheel, 1);
    assert_eq!(s.energy_production(), 0.0, "unfueled");
    give(&mut s, ResourceId::Dew, 0.025); // half the 0.05 demand
    assert!((s.energy_production() - 2.5).abs() < 1e-9);
    give(&mut s, ResourceId::Dew, 10.0);
    assert!((s.energy_production() - 5.0).abs() < 1e-9);

    // A composite grid, summed per the BLD table.
    let mut s = GameState::new(126);
    s.ticks = 1251; // summer
    grant(&mut s, StructureId::SunningDeck, 1); // +2.5
    grant(&mut s, StructureId::StreamWheel, 1); // +5
    grant(&mut s, StructureId::StarBasin, 1); // +10
    grant(&mut s, StructureId::Cogworks, 1); // +1
    grant(&mut s, StructureId::WhiskerLab, 2); // −2
    grant(&mut s, StructureId::ToyWorkshop, 1); // −2
    grant(&mut s, StructureId::PrismSpinner, 1); // −2
    grant(&mut s, StructureId::BoxLabyrinth, 1); // −2
    grant(&mut s, StructureId::MemoryChest, 1); // −20
    grant(&mut s, StructureId::YarnComputer, 3); // −10.5
    assert!((s.energy_production() - 18.5).abs() < 1e-9);
    assert!((s.energy_consumption() - 38.5).abs() < 1e-9);
    assert!((s.energy_deficit_ratio() - 18.5 / 38.5).abs() < 1e-9);
}

// @spec SIM-ECO-036, SIM-ENG-021
#[test]
fn energy_deficit_browns_out_only_the_global_layers() {
    let mut base = GameState::new(127);
    adopt(&mut base, 1);
    base.apply(Command::AssignJob(JobId::Gardener)).unwrap();
    off_boundary(&mut base);
    give(&mut base, ResourceId::Catnip, 100.0);
    give(&mut base, ResourceId::Dew, 10.0);

    let mut plain = base.clone();
    let mut deficit = base.clone(); // brownout, but no global layer to dim
    grant(&mut deficit, StructureId::MemoryChest, 2);
    let mut dimmed = deficit.clone(); // brownout dimming a fueled dynamo
    grant(&mut dimmed, StructureId::DynamoWheel, 1);

    // Production 5, consumption 40: 0.125 clamps up to the 0.25 floor.
    assert!((dimmed.energy_deficit_ratio() - 0.25).abs() < 1e-9);

    run_ticks(&mut plain, 1);
    run_ticks(&mut deficit, 1);
    run_ticks(&mut dimmed, 1);

    // Base gardener yield is untouched by the brownout (layers 1–5 exempt)…
    assert_eq!(
        plain.stock(ResourceId::Catnip),
        deficit.stock(ResourceId::Catnip),
        "a brownout never dims base production"
    );
    // …while the dynamo's +2% shrinks to +2% × 0.25 — a brownout, no blackout.
    assert!(
        (dimmed.stock(ResourceId::Catnip) - plain.stock(ResourceId::Catnip) - 0.15 * 0.02 * 0.25)
            .abs()
            < 1e-9
    );

    // A mild deficit dims proportionally: production 5, consumption 8.
    let mut mild = GameState::new(128);
    grant(&mut mild, StructureId::DynamoWheel, 1);
    grant(&mut mild, StructureId::WhiskerLab, 8);
    give(&mut mild, ResourceId::Dew, 10.0);
    assert!((mild.energy_deficit_ratio() - 5.0 / 8.0).abs() < 1e-9);
}

// @spec SIM-ECO-036
#[test]
fn moonlight_stage_charm_rides_the_surplus_and_darkens_off_grid() {
    // No grid at all: surplus ratio 0, the stage is dark by design.
    let mut dark = GameState::new(129);
    dark.ticks = 3;
    grant(&mut dark, StructureId::MoonlightStage, 1);
    assert_eq!(dark.energy_surplus_ratio(), 0.0);
    run_ticks(&mut dark, 1);
    assert_eq!(dark.stock(ResourceId::Charm), 0.0, "an unpowered stage is dark");

    // Surplus clamps at 1.75: production 10, consumption 0.
    let mut bright = GameState::new(130);
    bright.ticks = 3;
    grant(&mut bright, StructureId::MoonlightStage, 1);
    grant(&mut bright, StructureId::StreamWheel, 2);
    assert!((bright.energy_surplus_ratio() - 1.75).abs() < 1e-9);
    run_ticks(&mut bright, 1);
    assert!((bright.stock(ResourceId::Charm) - 0.25 * 1.75).abs() < 1e-9);

    // In between: production 5, consumption 4 ⇒ ×1.25.
    let mut cozy = GameState::new(131);
    cozy.ticks = 3;
    grant(&mut cozy, StructureId::MoonlightStage, 1);
    grant(&mut cozy, StructureId::StreamWheel, 1);
    grant(&mut cozy, StructureId::ToyWorkshop, 2);
    assert!((cozy.energy_surplus_ratio() - 1.25).abs() < 1e-9);
    run_ticks(&mut cozy, 1);
    assert!((cozy.stock(ResourceId::Charm) - 0.25 * 1.25).abs() < 1e-9);
}

// =====================================================================
// MESS — Mess
// =====================================================================

// @spec SIM-ECO-037
#[test]
fn mess_accrues_from_dirty_structures_and_dissipates_toward_equilibrium() {
    // Pure dissipation: with no dirty structures, mess decays by 1e-4 × mess.
    let mut s = GameState::new(132);
    s.ticks = 3;
    s.mess = 10_000.0;
    run_ticks(&mut s, 1);
    assert!((s.mess - 9_999.0).abs() < 1e-9, "dissipation is 1e-4 × mess per tick");

    // Accrual: 2 Gravel Pits (0.25 each) + 1 fed Kiln (1.0) = 1.5/tick.
    let mut s = GameState::new(133);
    s.ticks = 3;
    grant(&mut s, StructureId::GravelPit, 2);
    grant(&mut s, StructureId::Kiln, 1);
    give(&mut s, ResourceId::Pebbles, 20.0);
    give(&mut s, ResourceId::Dew, 1.0);
    run_ticks(&mut s, 1);
    assert!(
        (s.mess - 1.5).abs() <= 2e-4,
        "one tick of accrual (±the tick's own dissipation): {}",
        s.mess
    );

    // The flow pushes toward the analytic equilibrium (accrual / 1e-4).
    let mut high = GameState::new(134);
    high.ticks = 3;
    grant(&mut high, StructureId::GravelPit, 2);
    grant(&mut high, StructureId::Kiln, 1);
    give(&mut high, ResourceId::Pebbles, 20.0);
    give(&mut high, ResourceId::Dew, 1.0);
    let mut low = high.clone();
    high.mess = 20_000.0; // above the 15,000 equilibrium
    low.mess = 10_000.0; // below it
    run_ticks(&mut high, 1);
    run_ticks(&mut low, 1);
    assert!(high.mess < 20_000.0, "above equilibrium, mess falls");
    assert!(low.mess > 10_000.0, "below equilibrium, mess rises");

    // Levels: floor(log10(mess/1e4)) at or above 1e4, and -1 below it.
    let mut s = GameState::new(135);
    s.mess = 0.0;
    assert_eq!(s.mess_level(), -1);
    s.mess = 9_999.9;
    assert_eq!(s.mess_level(), -1);
    s.mess = 10_000.0;
    assert_eq!(s.mess_level(), 0);
    s.mess = 99_999.0;
    assert_eq!(s.mess_level(), 0);
    s.mess = 100_000.0;
    assert_eq!(s.mess_level(), 1);
    s.mess = 1e6;
    assert_eq!(s.mess_level(), 2);
}

// @spec SIM-ECO-038
#[test]
fn mess_levels_slow_catnip_arrivals_and_cheer_but_never_confiscate() {
    // Catnip growth ×0.8 per level step, floored at ×0.25. One patch in
    // summer produces 0.125/tick before the mess layer.
    let patch_gain = |mess: f64| {
        let mut s = GameState::new(136);
        grant(&mut s, StructureId::CatnipPatch, 1);
        s.ticks = 1251; // summer, mid-day
        s.mess = mess;
        run_ticks(&mut s, 1);
        s.stock(ResourceId::Catnip)
    };
    assert!((patch_gain(0.0) - 0.125).abs() < 1e-9);
    assert!((patch_gain(1e4) - 0.125 * 0.8).abs() < 1e-9, "level 0");
    assert!((patch_gain(1e5) - 0.125 * 0.8 * 0.8).abs() < 1e-9, "level 1");
    assert!((patch_gain(1e6) - 0.125 * 0.8 * 0.8 * 0.8).abs() < 1e-9, "level 2");
    assert!((patch_gain(1e15) - 0.125 * 0.25).abs() < 1e-9, "deep mess floors at ×0.25");

    // Arrivals get shy at level 1+ (chance ×0.75); level 0 leaves them alone.
    let mut s = GameState::new(137);
    give(&mut s, ResourceId::Catnip, 50.0); // base 15% chance
    s.mess = 1e4;
    assert!((s.arrival_chance() - 0.15).abs() < 1e-9, "level 0 never deters arrivals");
    s.mess = 1e5;
    assert!((s.arrival_chance() - 0.15 * 0.75).abs() < 1e-9, "level 1 makes arrivals shy");

    // Happiness: −5 points per level above 1, starting at level 2.
    let mut s = GameState::new(138);
    adopt(&mut s, 1);
    s.kittens[0].job = None;
    s.mess = 0.0;
    let h0 = s.happiness();
    s.mess = 1e5; // level 1: no happiness effect yet
    assert!((s.happiness() - h0).abs() < 1e-9);
    s.mess = 1e6; // level 2
    assert!((s.happiness() - (h0 - 5.0)).abs() < 1e-9);
    s.mess = 1e7; // level 3
    assert!((s.happiness() - (h0 - 10.0)).abs() < 1e-9);

    // Cozy invariant: mess slows gains; it never destroys stock.
    let mut s = GameState::new(139);
    s.ticks = 3;
    give(&mut s, ResourceId::Twigs, 10.0);
    give(&mut s, ResourceId::Catnip, 20.0);
    s.mess = 1e15;
    run_ticks(&mut s, 1);
    assert_eq!(s.stock(ResourceId::Twigs), 10.0);
    assert_eq!(s.stock(ResourceId::Catnip), 20.0);
}

// =====================================================================
// Jobs — the extended roster
// =====================================================================

// @spec SIM-ECO-040
#[test]
fn rummager_is_idea_gated_and_yields_pebbles() {
    assert_eq!(JobId::ALL.len(), 8, "the extended roster");
    let mut s = GameState::new(140);
    adopt(&mut s, 1);
    assert_eq!(s.apply(Command::AssignJob(JobId::Rummager)).unwrap_err(), CommandError::Locked);
    s.ideas.insert(IdeaId::CouchSpelunking);
    s.apply(Command::AssignJob(JobId::Rummager)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Pebbles) - 0.02).abs() < 1e-9);
}

// @spec SIM-ECO-041
#[test]
fn pouncer_is_idea_gated_and_yields_pep() {
    let mut s = GameState::new(141);
    adopt(&mut s, 1);
    assert_eq!(s.apply(Command::AssignJob(JobId::Pouncer)).unwrap_err(), CommandError::Locked);
    s.ideas.insert(IdeaId::PouncingPractice);
    s.apply(Command::AssignJob(JobId::Pouncer)).unwrap();
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Pep) - 0.06).abs() < 1e-9);
}

// @spec SIM-ECO-042
#[test]
fn sunwarden_is_idea_gated_and_yields_warmth() {
    let mut s = GameState::new(142);
    adopt(&mut s, 1);
    assert_eq!(s.apply(Command::AssignJob(JobId::Sunwarden)).unwrap_err(), CommandError::Locked);
    s.ideas.insert(IdeaId::SolarDevotion);
    s.apply(Command::AssignJob(JobId::Sunwarden)).unwrap();
    grant(&mut s, StructureId::SunRoom, 1); // warmth cap 75, plus its own 0.004/t
    off_boundary(&mut s);
    give(&mut s, ResourceId::Catnip, 100.0);
    run_ticks(&mut s, 1);
    assert!((s.stock(ResourceId::Warmth) - (0.004 + 0.0015)).abs() < 1e-9);
}

// =====================================================================
// CUR — The banded idea tree
// =====================================================================

// @spec SIM-CUR-005
#[test]
fn the_tree_has_57_ideas_with_append_only_ids() {
    assert_eq!(IdeaId::ALL.len(), 59);
    // The founding nine keep their ids.
    assert_eq!(IdeaId::PantryOrganization as usize, 0);
    assert_eq!(IdeaId::ScratchingArts as usize, 1);
    assert_eq!(IdeaId::NeighborhoodWatch as usize, 8);
    // Band spots across the ladder.
    assert_eq!(IdeaId::CouchSpelunking as usize, 9);
    assert_eq!(IdeaId::OralTradition as usize, 17);
    assert_eq!(IdeaId::GiftEconomy as usize, 18);
    assert_eq!(IdeaId::PorchLogistics as usize, 27);
    assert_eq!(IdeaId::AnthologyBinding as usize, 28);
    assert_eq!(IdeaId::GrandSchemes as usize, 35);
    assert_eq!(IdeaId::Dynamos as usize, 36);
    assert_eq!(IdeaId::YarnComputing as usize, 43);
    assert_eq!(IdeaId::ParadeRoutes as usize, 44);
    assert_eq!(IdeaId::DeepNapTheory as usize, 55);
    assert_eq!(IdeaId::PouncingPractice as usize, 56, "appended late; ids never reorder");
    // Whimsy with clarity: every idea names itself and explains its effect.
    for i in IdeaId::ALL {
        assert!(!content::idea_def(i).name.is_empty());
        assert!(!content::idea_def(i).blurb.is_empty(), "{i:?} must explain itself");
    }
}

/// Research an idea from a hand-built state and assert the exact spend.
fn research_spends_exactly(
    seed: u64,
    idea: IdeaId,
    prereqs: &[IdeaId],
    stocks: &[(ResourceId, f64)],
    spends: &[(ResourceId, f64)],
) {
    let mut s = GameState::new(seed);
    grant(&mut s, StructureId::CardboardBox, 1); // curiosity cap > 0 opens research
    for p in prereqs {
        s.ideas.insert(*p);
    }
    for (r, amount) in stocks {
        give(&mut s, *r, *amount);
    }
    let events = s.apply(Command::ResearchIdea(idea)).unwrap();
    assert!(events.contains(&Event::IdeaResearched { id: idea }));
    assert!(s.ideas.contains(&idea));
    for (r, spent) in spends {
        let held = stocks.iter().find(|(sr, _)| sr == r).map(|(_, v)| *v).unwrap_or(0.0);
        assert!(
            (s.stock(*r) - (held - spent)).abs() < 1e-9,
            "{idea:?} must spend exactly {spent} {r:?} (held {held}, left {})",
            s.stock(*r)
        );
    }
}

// @spec SIM-CUR-005, SIM-CUR-007
#[test]
fn idea_cost_spot_anchors_spend_curiosity_and_gate_crafts_together() {
    use IdeaId::*;
    use ResourceId::{Anthologies, Curiosity, Echoes, Schemes, Starshards, Stories};
    // Band 0: curiosity only.
    research_spends_exactly(
        143,
        CouchSpelunking,
        &[TwigArchitecture],
        &[(Curiosity, 200.0)],
        &[(Curiosity, 120.0)],
    );
    research_spends_exactly(
        144,
        PouncingPractice,
        &[PantryOrganization],
        &[(Curiosity, 250.0)],
        &[(Curiosity, 200.0)],
    );
    // Band 1: + stories.
    research_spends_exactly(
        145,
        SunSalutations,
        &[Storycraft],
        &[(Curiosity, 2000.0), (Stories, 7.0)],
        &[(Curiosity, 1500.0), (Stories, 5.0)],
    );
    // Band 2 entry still gated on stories, then anthologies.
    research_spends_exactly(
        146,
        AnthologyBinding,
        &[SkyCharts],
        &[(Curiosity, 8000.0), (Stories, 45.0)],
        &[(Curiosity, 7000.0), (Stories, 40.0)],
    );
    research_spends_exactly(
        147,
        GrandSchemes,
        &[DeepCuriosity],
        &[(Curiosity, 25000.0), (Anthologies, 41.0)],
        &[(Curiosity, 20000.0), (Anthologies, 40.0)],
    );
    // Band 3: + schemes.
    research_spends_exactly(
        148,
        Dynamos,
        &[GrandSchemes],
        &[(Curiosity, 30000.0), (Schemes, 10.0)],
        &[(Curiosity, 25000.0), (Schemes, 8.0)],
    );
    // Band 4: mystic gate currencies.
    research_spends_exactly(
        149,
        DreamTending,
        &[ParadeRoutes],
        &[(Curiosity, 100_000.0), (Starshards, 3.0)],
        &[(Curiosity, 90_000.0), (Starshards, 3.0)],
    );
    research_spends_exactly(
        150,
        DeepNapTheory,
        &[DreamLoomcraft],
        &[(Curiosity, 300_000.0), (Echoes, 10.0)],
        &[(Curiosity, 300_000.0), (Echoes, 10.0)],
    );
}

// @spec SIM-CUR-007
#[test]
fn gate_craft_spend_is_all_or_nothing() {
    let mut s = GameState::new(151);
    grant(&mut s, StructureId::CardboardBox, 1);
    s.ideas.insert(IdeaId::Storycraft);
    give(&mut s, ResourceId::Curiosity, 1500.0);
    give(&mut s, ResourceId::Stories, 4.9); // one story short, beyond epsilon
    let err = s.apply(Command::ResearchIdea(IdeaId::SunSalutations)).unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
    assert_eq!(s.stock(ResourceId::Curiosity), 1500.0, "nothing spent on a refusal");
    assert_eq!(s.stock(ResourceId::Stories), 4.9);

    // Epsilon-tolerant affordability, and the deduction floors at zero.
    give(&mut s, ResourceId::Stories, 5.0 - 1e-10);
    s.apply(Command::ResearchIdea(IdeaId::SunSalutations)).unwrap();
    assert_eq!(s.stock(ResourceId::Curiosity), 0.0);
    assert_eq!(s.stock(ResourceId::Stories), 0.0);
    assert!(s.ideas.contains(&IdeaId::SunSalutations));
}

// @spec SIM-CUR-008
#[test]
fn the_idea_graph_is_acyclic_and_fully_reachable() {
    // Walk the unlock query to a fixpoint: starting from a bare Cardboard Box
    // (the physical gate), every idea must eventually unlock. A prerequisite
    // cycle or an orphaned idea would strand the walk below 57.
    let mut s = GameState::new(152);
    grant(&mut s, StructureId::CardboardBox, 1);
    loop {
        let mut changed = false;
        for i in IdeaId::ALL {
            if !s.ideas.contains(&i) && s.idea_unlocked(i) {
                s.ideas.insert(i);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    assert_eq!(
        s.ideas.len(),
        59,
        "unreachable or cyclic ideas: {:?}",
        IdeaId::ALL.iter().filter(|i| !s.ideas.contains(i)).collect::<Vec<_>>()
    );
}

// @spec SIM-CUR-008
#[test]
fn multi_prerequisite_ideas_require_all_of_them() {
    // Parade Routes needs Unicorn Lore AND Grand Schemes.
    let mut s = GameState::new(153);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 50_000.0);
    give(&mut s, ResourceId::Schemes, 20.0);

    s.ideas.insert(IdeaId::UnicornLore);
    assert!(!s.idea_unlocked(IdeaId::ParadeRoutes), "one prerequisite is not enough");
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::ParadeRoutes)).unwrap_err(),
        CommandError::Locked
    );

    let mut t = GameState::new(154);
    grant(&mut t, StructureId::CardboardBox, 1);
    t.ideas.insert(IdeaId::GrandSchemes);
    assert!(!t.idea_unlocked(IdeaId::ParadeRoutes), "the other alone is not enough either");

    s.ideas.insert(IdeaId::GrandSchemes);
    assert!(s.idea_unlocked(IdeaId::ParadeRoutes));
    s.apply(Command::ResearchIdea(IdeaId::ParadeRoutes)).unwrap();
    assert!((s.stock(ResourceId::Curiosity) - 20_000.0).abs() < 1e-9, "30000 spent");
    assert!((s.stock(ResourceId::Schemes) - 5.0).abs() < 1e-9, "15 schemes spent");
}

// @spec SIM-CUR-009
#[test]
fn ideas_grant_their_tabled_unlocks_as_pure_lookups() {
    let mut s = GameState::new(155);
    // Before any research: everything downstream is closed.
    assert!(!s.structure_unlocked(StructureId::CraftCorner));
    assert!(!s.structure_unlocked(StructureId::CouchMine));
    assert!(!s.structure_unlocked(StructureId::Porch));
    assert!(!s.structure_unlocked(StructureId::StargazingLedge));
    assert!(!s.structure_unlocked(StructureId::SunbeamAltar));
    assert!(!s.structure_unlocked(StructureId::UnicornMeadow));
    assert!(!s.job_unlocked(JobId::Rummager));
    assert!(!s.job_unlocked(JobId::Sunwarden));
    give(&mut s, ResourceId::Twigs, 15.0);
    assert_eq!(s.apply(Command::Craft(CraftId::Wands)).unwrap_err(), CommandError::Locked);
    assert_eq!(s.apply(Command::Rummage).unwrap_err(), CommandError::Locked);

    // Each idea opens exactly its tabled doors.
    s.ideas.insert(IdeaId::Tinkering);
    assert!(s.structure_unlocked(StructureId::CraftCorner));
    s.apply(Command::Craft(CraftId::Wands)).unwrap();
    assert!((s.stock(ResourceId::Wands) - 1.0).abs() < 1e-9);

    s.ideas.insert(IdeaId::CouchSpelunking);
    assert!(s.structure_unlocked(StructureId::CouchMine));
    assert!(s.job_unlocked(JobId::Rummager));
    s.apply(Command::Rummage).unwrap();
    assert!((s.stock(ResourceId::Pebbles) - 0.5).abs() < 1e-9);

    s.ideas.insert(IdeaId::SolarDevotion);
    assert!(s.structure_unlocked(StructureId::SunbeamAltar));
    assert!(s.job_unlocked(JobId::Sunwarden));

    s.ideas.insert(IdeaId::PorchLogistics);
    assert!(s.structure_unlocked(StructureId::Porch));
    s.ideas.insert(IdeaId::SkyCharts);
    assert!(s.structure_unlocked(StructureId::StargazingLedge));
    s.ideas.insert(IdeaId::UnicornLore);
    assert!(s.structure_unlocked(StructureId::UnicornMeadow));
}

// =====================================================================
// ENG — Sky events
// =====================================================================

// @spec SIM-ENG-025
#[test]
fn sky_events_roll_daily_per_ledge_and_sometimes_self_observe() {
    // 10 Ledges: 2% event chance per day close, 10% auto-observe. Over 6000
    // days expect ~120 events, ~12 auto-observed (~108 visible). Bounds stay
    // generous against seed luck.
    let mut s = GameState::new(156);
    grant(&mut s, StructureId::StargazingLedge, 10);
    let events = run_days(&mut s, 6000);
    let visible = events.iter().filter(|e| matches!(e, Event::SkyEventVisible)).count();
    assert!(
        (50..=170).contains(&visible),
        "visible sky events over 6000 days: {visible}"
    );
    // Auto-observed events each bank 1 skymap and 25 curiosity × (1 + the
    // curiosity ratios; 10 Ledges give +250%): the stocks stay in lockstep.
    let skymaps = s.stock(ResourceId::Skymaps);
    assert!(
        (2.0..=40.0).contains(&skymaps),
        "auto-observed skymaps over 6000 days: {skymaps}"
    );
    assert!(
        (s.stock(ResourceId::Curiosity) - 25.0 * 3.5 * skymaps).abs() < 1e-6,
        "auto-observe grants ride the curiosity ratios"
    );
}

// @spec SIM-ENG-025
#[test]
fn observe_sky_works_in_the_window_and_expired_events_fade() {
    // No event pending: nothing to observe, and the refusal is a perfect no-op.
    let mut fresh = GameState::new(157);
    let snapshot = fresh.clone();
    assert_eq!(fresh.apply(Command::ObserveSky).unwrap_err(), CommandError::Locked);
    assert_eq!(fresh, snapshot);

    // Hunt a visible (not auto-observed) sky event.
    let mut s = GameState::new(158);
    grant(&mut s, StructureId::StargazingLedge, 10);
    let mut found = false;
    for _ in 0..20_000 {
        let events = run_days(&mut s, 1);
        if events.iter().any(|e| matches!(e, Event::SkyEventVisible)) {
            found = true;
            break;
        }
    }
    assert!(found, "a 2%-per-day event must fire within 20000 days");
    assert!(s.pending_sky_event.is_some(), "the visible event is pending");

    // Observing within the window: 25 curiosity × (1 + 2.5 from ten Ledges),
    // plus one skymap; the pending event is consumed.
    let mut observed = s.clone();
    let curiosity_before = observed.stock(ResourceId::Curiosity);
    let skymaps_before = observed.stock(ResourceId::Skymaps);
    observed.apply(Command::ObserveSky).unwrap();
    assert!((observed.stock(ResourceId::Curiosity) - curiosity_before - 87.5).abs() < 1e-9);
    assert!((observed.stock(ResourceId::Skymaps) - skymaps_before - 1.0).abs() < 1e-9);
    assert_eq!(observed.pending_sky_event, None, "observing consumes the event");
    assert_eq!(observed.apply(Command::ObserveSky).unwrap_err(), CommandError::Locked);

    // Left alone past the end of the next day, the event simply fades. Run
    // forward until a 30-day stretch passes with no fresh visible event, so
    // the only candidate is long expired.
    let mut faded = s.clone();
    loop {
        let events = run_days(&mut faded, 30);
        if !events.iter().any(|e| matches!(e, Event::SkyEventVisible)) {
            break;
        }
    }
    let err = faded.apply(Command::ObserveSky).unwrap_err();
    assert_eq!(err, CommandError::Locked, "an expired event has faded");
}

// =====================================================================
// Persistence — v1 saves zero-fill the depth-wave state
// =====================================================================

// @spec SAVE-013, SIM-ECO-001
#[test]
fn v1_saves_zero_fill_the_widened_arrays_and_new_fields() {
    let fixture = include_str!("fixtures/save_v1.json");
    let loaded = load_string(fixture, 1_751_700_000).expect("v1 fixture still loads");
    let s = &loaded.state;
    // The v1 fixture carries 4 resources and 7 structures; the rest zero-fill.
    assert_eq!(s.resources[45], 0.0, "appended resources start empty");
    assert_eq!(s.stock(ResourceId::Cleverness), 0.0);
    assert_eq!(s.stock(ResourceId::Pebbles), 0.0);
    assert_eq!(s.structures[StructureId::UnicornMeadow.ix()], 0);
    // New scalar fields get new-game defaults.
    assert_eq!(s.mess, 0.0);
    assert_eq!(s.kites_high_water, 0.0);
    assert_eq!(s.pending_sky_event, None);
    // And the old meadow is still exactly itself.
    assert!((s.stock(ResourceId::Catnip) - 127.90187499999998).abs() < 1e-12);
    assert_eq!(s.structures[StructureId::CatnipPatch.ix()], 1);
}
