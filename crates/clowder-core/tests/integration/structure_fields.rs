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
