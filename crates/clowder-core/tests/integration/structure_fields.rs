//! What a structure does per copy is a field of its definition, and the rule
//! that applies it reads that field: the rule and the inspector's statement of
//! it then read one value. Each table test pins the field's holders and states
//! that no other structure carries it; each rule test holds the rule to the
//! field, reading the expected figure from the table.

use crate::common;

use clowder_core::content::{structure_def, StructureDef};
use clowder_core::*;
use common::*;

/// Every structure whose definition sets a per-copy rate, with that rate.
fn holders(field: fn(&StructureDef) -> f64) -> Vec<(StructureId, f64)> {
    StructureId::ALL
        .into_iter()
        .map(|s| (s, field(structure_def(s))))
        .filter(|(_, v)| *v != 0.0)
        .collect()
}

/// Every structure whose definition sets a mark.
fn marked(field: fn(&StructureDef) -> bool) -> Vec<StructureId> {
    StructureId::ALL
        .into_iter()
        .filter(|s| field(structure_def(*s)))
        .collect()
}

// ---- The tables ----

/// @spec SIM-UNI-013
#[test]
fn only_the_rainbow_arch_brings_rainbows_to_a_parade() {
    assert_eq!(
        holders(|d| d.parade_rainbows),
        vec![(StructureId::RainbowArch, 2500.0)]
    );
}

/// @spec SIM-UNI-014
#[test]
fn only_the_rainbow_arch_adds_to_a_days_prism_descent() {
    assert_eq!(
        holders(|d| d.prism_descent),
        vec![(StructureId::RainbowArch, 0.0002)]
    );
}

/// @spec SIM-UNI-015
#[test]
fn only_the_rainbow_arch_accrues_dream_drift() {
    assert_eq!(
        holders(|d| d.dream_drift),
        vec![(StructureId::RainbowArch, 0.000001)]
    );
}

/// @spec SIM-DREAM-013
#[test]
fn only_the_memory_chest_adds_to_a_seasons_deja_vu_roll() {
    assert_eq!(
        holders(|d| d.deja_vu_chance),
        vec![(StructureId::MemoryChest, 0.01)]
    );
}

/// @spec SIM-MOVE-016
#[test]
fn only_the_memory_chest_carries_a_share_through_a_move() {
    assert_eq!(
        holders(|d| d.move_carry),
        vec![(StructureId::MemoryChest, 0.015)]
    );
}

/// @spec SIM-SUN-010
#[test]
fn only_the_sunstone_adds_long_sun_cap_per_enlightenment_tier() {
    assert_eq!(
        holders(|d| d.long_sun_tier_cap),
        vec![(StructureId::Sunstone, 0.05)]
    );
}

/// The effect sum reads this field from the Far Beacon alone, rather than scan
/// every structure each tick, so a second holder would have to change that read.
/// @spec SIM-SKY-011
#[test]
fn only_the_far_beacon_grows_every_storage_cap() {
    assert_eq!(
        holders(|d| d.cap_ratio),
        vec![(StructureId::FarBeacon, 0.10)]
    );
}

/// @spec SIM-ENG-077
#[test]
fn only_the_stargazing_ledge_adds_to_a_days_sky_event() {
    assert_eq!(
        holders(|d| d.sky_event_chance),
        vec![(StructureId::StargazingLedge, 0.002)]
    );
}

/// @spec SIM-ENG-078
#[test]
fn only_the_stargazing_ledge_adds_to_auto_observing_a_sky_event() {
    assert_eq!(
        holders(|d| d.sky_auto_observe),
        vec![(StructureId::StargazingLedge, 0.01)]
    );
}

/// @spec SIM-ECO-053
#[test]
fn only_the_gift_stand_adds_a_trade_ratio() {
    assert_eq!(
        holders(|d| d.trade_ratio),
        vec![(StructureId::GiftStand, 0.015)]
    );
}

/// @spec SIM-ECO-054
#[test]
fn only_the_cogworks_carries_a_year_close_craft() {
    let holders: Vec<StructureId> = StructureId::ALL
        .into_iter()
        .filter(|s| structure_def(*s).year_craft.is_some())
        .collect();
    assert_eq!(holders, vec![StructureId::Cogworks]);
    let craft = structure_def(StructureId::Cogworks)
        .year_craft
        .as_ref()
        .unwrap();
    assert_eq!(craft.share, 0.02);
    assert_eq!(craft.share_max, 0.5);
    assert_eq!(craft.threshold, 0.98);
    assert_eq!(
        craft.pairs,
        &[
            (ResourceId::Twigs, CraftId::Wands),
            (ResourceId::Pebbles, CraftId::Tiles),
            (ResourceId::Buttons, CraftId::Badges),
            (ResourceId::Dew, CraftId::Moonmilk),
            (ResourceId::Stardust, CraftId::Cometsilk),
        ]
    );
}

/// @spec SIM-ECO-055
#[test]
fn only_the_cogworks_boosts_a_fuel_scaled_global_ratio() {
    assert_eq!(
        holders(|d| d.fuel_global_boost),
        vec![(StructureId::Cogworks, 0.15)]
    );
}

/// @spec SIM-ECO-056
#[test]
fn only_the_treat_bakery_has_cap_fed_outputs() {
    let holders: Vec<StructureId> = StructureId::ALL
        .into_iter()
        .filter(|s| {
            structure_def(*s)
                .converter
                .as_ref()
                .is_some_and(|c| c.cap_fed.is_some())
        })
        .collect();
    assert_eq!(holders, vec![StructureId::TreatBakery]);
    let fed = structure_def(StructureId::TreatBakery)
        .converter
        .as_ref()
        .unwrap()
        .cap_fed
        .as_ref()
        .unwrap();
    assert_eq!(fed.cap, ResourceId::Pep);
    assert_eq!(fed.per_cap, 0.007 / 100.0);
    assert_eq!(
        fed.outputs,
        &[(ResourceId::Feathers, 1.25), (ResourceId::Bells, 0.3)]
    );
}

/// @spec SIM-ECO-057
#[test]
fn only_the_yarn_computer_escalates_its_energy_per_copy_squared() {
    assert_eq!(
        holders(|d| d.energy_cons_sq),
        vec![(StructureId::YarnComputer, 0.5)]
    );
    assert_eq!(structure_def(StructureId::YarnComputer).energy_cons, 2.0);
}

/// @spec SIM-ECO-058
#[test]
fn only_the_porch_ratchets_its_caps_on_kites() {
    assert_eq!(marked(|d| d.caps_kite_ratcheted), vec![StructureId::Porch]);
}

/// @spec SIM-ECO-059
#[test]
fn only_the_dynamo_wheel_is_fuel_scaled() {
    assert_eq!(marked(|d| d.fuel_scaled), vec![StructureId::DynamoWheel]);
}

/// @spec SIM-ECO-060
#[test]
fn only_the_sunning_deck_and_solar_array_make_seasonal_energy() {
    assert_eq!(
        marked(|d| d.energy_seasonal),
        vec![StructureId::SunningDeck, StructureId::SolarArray]
    );
}

// ---- The rules ----

/// A Parade brings the field's rainbows per copy, read before any ratio.
/// @spec SIM-UNI-001
#[test]
fn a_parade_brings_each_copys_field_of_rainbows() {
    let per_copy = structure_def(StructureId::RainbowArch).parade_rainbows;
    let mut s = GameState::new(1);
    s.ideas.insert(IdeaId::ParadeRoutes);
    grant(&mut s, StructureId::RainbowArch, 3);
    give(&mut s, ResourceId::Unicorns, content::PARADE_UNICORN_COST);
    let before = s.stock(ResourceId::Rainbows);
    s.apply(Command::HoldParade).unwrap();
    assert_eq!(s.stock(ResourceId::Rainbows) - before, per_copy * 3.0);
}

/// @spec SIM-UNI-006
#[test]
fn a_days_prism_descent_is_each_copys_field() {
    let per_copy = structure_def(StructureId::RainbowArch).prism_descent;
    let mut s = GameState::new(2);
    grant(&mut s, StructureId::RainbowArch, 4);
    assert_eq!(s.prism_descent_chance(), per_copy * 4.0);
}

/// @spec SIM-DREAM-013
#[test]
fn a_seasons_deja_vu_chance_is_each_copys_field() {
    let per_copy = structure_def(StructureId::MemoryChest).deja_vu_chance;
    let mut s = GameState::new(3);
    grant(&mut s, StructureId::MemoryChest, 2);
    assert_eq!(s.effect(Effect::DejaVuChance), per_copy * 2.0);
}

/// @spec SIM-MOVE-003
#[test]
fn a_moves_carry_share_is_each_copys_field() {
    let per_copy = structure_def(StructureId::MemoryChest).move_carry;
    let mut s = GameState::new(4);
    grant(&mut s, StructureId::MemoryChest, 5);
    assert_eq!(s.memory_chest_ratio(), per_copy * 5.0);
}

/// @spec SIM-ECO-053
#[test]
fn the_trade_ratio_effect_sums_each_copys_field() {
    let per_copy = structure_def(StructureId::GiftStand).trade_ratio;
    let mut s = GameState::new(5);
    assert_eq!(s.effect(Effect::TradeRatio), 0.0);
    grant(&mut s, StructureId::GiftStand, 7);
    assert_eq!(s.effect(Effect::TradeRatio), per_copy * 7.0);
}

/// The Yarn Computer's use at n copies is energy_cons·n + energy_cons_sq·n²,
/// and the hot-path pair agrees with the query.
/// @spec SIM-ECO-035
#[test]
fn energy_used_escalates_from_the_definitions_two_fields() {
    let d = structure_def(StructureId::YarnComputer);
    let mut s = GameState::new(6);
    grant(&mut s, StructureId::YarnComputer, 5);
    assert_eq!(
        s.energy_consumption(),
        d.energy_cons * 5.0 + d.energy_cons_sq * 25.0
    );
}

/// A marked structure's energy made breathes with the season; an unmarked one
/// holds its rate all year. @spec SIM-ECO-035
#[test]
fn energy_made_is_seasonal_exactly_where_the_definition_marks_it() {
    for season_day in [0, 3 * content::DAYS_PER_SEASON] {
        for b in StructureId::ALL {
            let d = structure_def(b);
            if d.energy_prod == 0.0 || d.fuel_scaled {
                continue;
            }
            let mut s = GameState::new(7);
            s.ticks = season_day * content::TICKS_PER_DAY;
            grant(&mut s, b, 1);
            let factor = if d.energy_seasonal {
                content::sunning_deck_season_factor(s.season())
            } else {
                1.0
            };
            assert_eq!(s.energy_production(), d.energy_prod * factor, "{b:?}");
        }
    }
}

/// A fuel-scaled structure's fuel fraction is the throttle of its converter's
/// inputs over its copies, and it scales both figures the mark covers.
/// @spec SIM-ECO-059
#[test]
fn a_fuel_scaled_structures_figures_follow_the_throttle_of_its_inputs() {
    let b = StructureId::DynamoWheel;
    let d = structure_def(b);
    let (fuel, per_copy) = d.converter.as_ref().unwrap().inputs[0];
    let mut s = GameState::new(8);
    grant(&mut s, b, 2);
    // Half of what two copies draw in a tick.
    s.resources[fuel.ix()] = per_copy;
    let fraction = s.throttle_fraction(&[(fuel, per_copy * 2.0)]);
    assert!(fraction > 0.0 && fraction < 1.0, "fixture: partly starved");
    assert_eq!(s.energy_production(), d.energy_prod * 2.0 * fraction);
    assert_eq!(
        s.effect(Effect::GlobalProductionRatio),
        d.global_ratio * 2.0 * fraction
    );
}

/// Each copy's share joins the term every cap is multiplied by, whatever that
/// cap is made of: its base alone, a flat extension, or the structure's own.
/// @spec SIM-SKY-011
#[test]
fn each_copy_of_the_far_beacon_grows_every_storage_cap_by_its_share() {
    let beacon = StructureId::FarBeacon;
    let share = structure_def(beacon).cap_ratio;
    assert!(share > 0.0, "fixture: the field is set");
    let copies = 3;

    let mut before = GameState::new(8);
    grant(&mut before, StructureId::Pantry, 1);
    let mut after = before.clone();
    grant(&mut after, beacon, copies);

    let term = 1.0 + share * copies as f64;
    for r in ResourceId::ALL {
        let flat = clowder_core::content::BASE_CAPS[r.ix()] + after.effect(Effect::MaxCap(r));
        assert_eq!(after.cap(r), flat * term, "{r:?}");
    }
    // A base with a flat extension on it, and a base alone.
    let catnip = before.cap(ResourceId::Catnip);
    assert!(catnip > clowder_core::content::BASE_CAPS[ResourceId::Catnip.ix()]);
    assert_eq!(after.cap(ResourceId::Catnip), catnip * term);
    assert_eq!(
        after.cap(ResourceId::Twigs),
        before.cap(ResourceId::Twigs) * term
    );
    // The structure's own extension is grown with the rest.
    let own = structure_def(beacon)
        .cap_bonuses
        .iter()
        .find(|(r, _)| *r == ResourceId::JarsOfPurrs)
        .unwrap()
        .1;
    assert_eq!(
        after.cap(ResourceId::JarsOfPurrs),
        (clowder_core::content::BASE_CAPS[ResourceId::JarsOfPurrs.ix()] + own * copies as f64)
            * term
    );
}

/// A percentage of nothing opens no gate, and a percentage of no limit sets none.
/// @spec SIM-SKY-011
#[test]
fn the_far_beacons_share_leaves_zero_and_infinite_caps_as_they_are() {
    let mut s = GameState::new(8);
    grant(&mut s, StructureId::FarBeacon, 5);
    assert!(
        s.effect(Effect::CapRatio) > 0.0,
        "fixture: the share counts"
    );
    assert_eq!(s.cap(ResourceId::Curiosity), 0.0);
    assert!(s.cap(ResourceId::Feathers).is_infinite());
}

/// The share sums with the other cap-ratio effects: it does not compound on
/// them, or on itself.
/// @spec SIM-SKY-011
#[test]
fn the_far_beacons_share_sums_with_the_other_cap_ratio_effects() {
    let beacon = StructureId::FarBeacon;
    let share = structure_def(beacon).cap_ratio;
    let rule = RuleId::PersonalStashes;
    let other = clowder_core::content::rule_def(rule)
        .effects
        .iter()
        .find(|(k, _)| matches!(k, Effect::CapRatio))
        .unwrap()
        .1;
    let mut s = GameState::new(8);
    grant(&mut s, beacon, 2);
    s.rules.insert(rule);
    assert_eq!(s.effect(Effect::CapRatio), share * 2.0 + other);
    let base = clowder_core::content::BASE_CAPS[ResourceId::Twigs.ix()];
    let summed = base * (1.0 + share * 2.0 + other);
    let compounded = base * (1.0 + share).powi(2) * (1.0 + other);
    assert!(share > 0.0 && (summed - compounded).abs() > 1e-6, "fixture");
    assert!((s.cap(ResourceId::Twigs) - summed).abs() < 1e-9);
}

/// The wider cap is the one the clamp holds a stock to.
/// @spec SIM-SKY-011
#[test]
fn a_stock_the_far_beacon_makes_room_for_stands_through_the_clamp() {
    let r = ResourceId::Twigs;
    let base = clowder_core::content::BASE_CAPS[r.ix()];
    let over = base * (1.0 + structure_def(StructureId::FarBeacon).cap_ratio);
    assert!(over > base, "fixture: the field is set");

    let mut without = GameState::new(8);
    without.resources[r.ix()] = over;
    run_ticks(&mut without, 1);
    assert_eq!(without.stock(r), base);

    let mut with = GameState::new(8);
    grant(&mut with, StructureId::FarBeacon, 1);
    with.resources[r.ix()] = over;
    run_ticks(&mut with, 1);
    assert_eq!(with.stock(r), over);
}
