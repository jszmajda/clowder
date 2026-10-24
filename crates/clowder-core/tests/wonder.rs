//! Depth Wave 4 tests: the wonder segments — ceremonies (SIM-SUN), mystic
//! (SIM-UNI), and colonies (SIM-TRADE) — plus the wave-4 seams in economy
//! (SIM-ECO-028), the engine (SIM-ENG-021 REVISED), and curiosity
//! (SIM-CUR-005). Written tests-first from the design/spec docs: they compile
//! against the Depth Wave 4 API contract and fail until the content lands.
//!
//! ---- Contract shapes this suite pins (for the implementer) ----
//!
//! New enums (all Copy + Ord for BTreeSet membership, each with an `ALL`):
//!   * `ColonyId`  = Otters, Raccoons, Foxes, Hedgehogs, Rabbits, Deer, Owls,
//!                   StarWhales                    (8; indexed via `as usize`)
//!   * `RiteId`    = MorningStretch, SharedSunpuddle, GoldenHour, WarmWelcome,
//!                   StainedSunlight, TheLongSun, GrandBasking, SunWardensOath,
//!                   TheReverie, Enlightenment, EverwarmFrescoes          (11)
//!   * `ArchId`    = RibbonArch, CrystalArch, PrismArch, SkyArch, DreamingArch,
//!                   StarlitArch, MemoryArch, EverdreamArch                (8)
//!
//! `StructureId` gains RainbowArch (38) and Sunstone (39); STRUCTURE_COUNT = 40,
//! so `structures: [u32; 40]`.  `IdeaId` gains LongSunMasonry (57) and
//! SkywardSong (58); IDEA_COUNT = 59.
//!
//! New `GameState` fields (all `pub`, all `#[serde(default)]`):
//!   devotion: f64, serenity: f64, peak_devotion: f64 (monotone high-water),
//!   enlightenment_tier: u32, dream_drift: f64,
//!   rites: BTreeSet<RiteId>, arches_bought: BTreeSet<ArchId>,
//!   promises: Vec<Promise> (Promise has a `pub quiet: bool`; this suite never
//!     constructs a Promise literal — MakePromise builds them — so extra
//!     Promise fields stay the implementer's business),
//!   penpal: [u32; 8] (per-colony level, colony-indexed),
//!   discovered: BTreeSet<ColonyId>.
//!
//! NOTE on rite stacking (SIM-SUN-006): `rites: BTreeSet<RiteId>` records only
//! *which* rites are bought. Re-buyable-after-Enlightenment stacking needs a
//! companion per-rite count (e.g. BTreeMap<RiteId,u32> or [u32;11]); this suite
//! observes stacking through BuyRite results, effect() sums, and spend amounts
//! only, so it never reads that counter.
//!
//! Content tables: `content::rite_def(RiteId) -> &RiteDef {name, cost:
//! &[(ResourceId,f64)], devotion_gate: f64}`, `content::arch_def(ArchId) ->
//! &ArchDef {name, cost: &[(ResourceId,f64)]}`, `content::colony_def(ColonyId)
//! -> &ColonyDef {name, start_standing: f64, buy: (ResourceId,f64), sells:
//! &[ColonySell {resource, amount, chance}]}`.
//!
//! Derived queries pinned: `reverence()`, `long_sun_bonus()` (the layer-8 bonus
//! itself, 0 with no Long Sun rite), `standing(ColonyId)`.
#![allow(clippy::type_complexity, clippy::doc_overindented_list_items)]

mod common;

use clowder_core::*;
use common::*;

/// Step off the day boundary and quiet any adoption-day zoomies so a single
/// tick measures exactly one production/consumption step. (Mirrors the helper
/// in economy_depth.rs — structures never take mood/zoomies anyway, but this
/// keeps job-bearing scenarios clean too.)
fn off_boundary(s: &mut GameState) {
    s.ticks += 1;
    s.zoomies_until = 0;
}

/// The Enlightenment cumulative cost: total_cost(t) = inverse_unlimited_dr(
/// e^t / 10, 0.1). inverse_unlimited_dr(y, s) = s·y·(y+1)/2 (the algebraic
/// inverse of unlimited_dr). @spec SIM-SUN-008
fn enlighten_total_cost(t: u32) -> f64 {
    let y = (t as f64).exp() / 10.0;
    0.1 * y * (y + 1.0) / 2.0
}

// =====================================================================
// SIM-SUN — Ceremonies: currencies & offerings
// =====================================================================

// @spec SIM-SUN-001
#[test]
fn bask_offers_warmth_up_into_devotion_scaled_by_reverence() {
    // With no serenity, reverence is zero: devotion gains exactly the warmth.
    let mut s = GameState::new(301);
    give(&mut s, ResourceId::Warmth, 100.0);
    assert!(
        (s.reverence() - 0.0).abs() < 1e-9,
        "no serenity, no reverence"
    );
    let events = s.apply(Command::Bask).unwrap();
    assert!(
        (s.devotion - 100.0).abs() < 1e-9,
        "devotion += warmth × (1 + 0)"
    );
    assert_eq!(
        s.stock(ResourceId::Warmth),
        0.0,
        "warmth is offered up, zeroed"
    );
    assert!(
        (s.peak_devotion - 100.0).abs() < 1e-9,
        "peak devotion is a high-water"
    );
    assert!(events.iter().any(|e| matches!(e, Event::Basked { .. })));

    // Reverence comes from serenity: reverence = unlimited_dr(serenity, 0.1)×0.1,
    // and every Basking is multiplied by (1 + reverence) — permanent growth.
    let mut s = GameState::new(302);
    s.serenity = 10.0;
    let expected_rev = unlimited_dr(10.0, 0.1) * 0.1;
    assert!((s.reverence() - expected_rev).abs() < 1e-9);
    give(&mut s, ResourceId::Warmth, 100.0);
    s.apply(Command::Bask).unwrap();
    assert!((s.devotion - 100.0 * (1.0 + expected_rev)).abs() < 1e-9);

    // An empty sunpuddle offers nothing: Basking with no warmth is rejected and
    // changes nothing.
    let mut s = GameState::new(303);
    let before = s.clone();
    assert!(matches!(
        s.apply(Command::Bask).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    assert_eq!(s, before, "a rejected Bask mutates nothing");
}

// @spec SIM-SUN-002
#[test]
fn reverie_settles_devotion_into_permanent_serenity() {
    // Before the Reverie rite, Sunrise Reverie is Locked even with devotion.
    let mut s = GameState::new(304);
    s.devotion = 5.0e6;
    assert_eq!(s.apply(Command::Reverie).unwrap_err(), CommandError::Locked);

    // With the rite: serenity += devotion/1e6 × (tier+1)² × 1.01, devotion
    // resets to a 0.01 spark, and the peak-devotion high-water is untouched.
    let mut s = GameState::new(305);
    s.rites.insert(RiteId::TheReverie);
    s.devotion = 5.0e6;
    s.peak_devotion = 5.0e6;
    let ser0 = s.serenity;
    let events = s.apply(Command::Reverie).unwrap();
    let expected = 5.0e6 / 1.0e6 * 1.0 * 1.01; // tier 0 → (0+1)² = 1
    assert!((s.serenity - (ser0 + expected)).abs() < 1e-9);
    assert!(
        (s.devotion - 0.01).abs() < 1e-9,
        "devotion resets to a spark"
    );
    assert!(
        (s.peak_devotion - 5.0e6).abs() < 1e-9,
        "peak devotion preserved"
    );
    assert!(events.iter().any(|e| matches!(e, Event::Reveried { .. })));

    // The enlightenment tier squares the conversion: (tier+1)².
    let mut s = GameState::new(306);
    s.rites.insert(RiteId::TheReverie);
    s.enlightenment_tier = 2;
    s.devotion = 5.0e6;
    s.apply(Command::Reverie).unwrap();
    assert!(
        (s.serenity - (5.0e6 / 1.0e6 * 9.0 * 1.01)).abs() < 1e-9,
        "(2+1)² = 9"
    );

    // Negligible devotion is nothing to offer: InsufficientResources.
    let mut s = GameState::new(307);
    s.rites.insert(RiteId::TheReverie);
    s.devotion = 0.0;
    assert!(matches!(
        s.apply(Command::Reverie).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
}

// @spec SIM-SUN-004, SIM-ENG-021
#[test]
fn the_long_sun_multiplies_material_production_only() {
    // The bonus itself: long_sun_bonus = limited_dr(unlimited_dr(devotion,
    // 1000)/100, cap), cap = 10 (base; +0.05/tier per Sunstone, none here). It
    // is applied as (1 + bonus) at layer 8 to raw-tier flows — structures too,
    // per the SIM-ENG-021 revision. Before the Long Sun rite the bonus is 0.
    let devotion = 1.0e6;
    let expected_bonus = limited_dr(unlimited_dr(devotion, 1000.0) / 100.0, 10.0);

    let mut no_rite = GameState::new(309);
    no_rite.devotion = devotion;
    assert!(
        (no_rite.long_sun_bonus() - 0.0).abs() < 1e-9,
        "no rite → no bonus"
    );

    let mut lit = GameState::new(310);
    lit.rites.insert(RiteId::TheLongSun);
    lit.devotion = devotion;
    assert!((lit.long_sun_bonus() - expected_bonus).abs() < 1e-9);

    // Measure a Catnip Patch: base 0.125/t × season, then × (1 + long_sun).
    // (Spring at day 0 = ×1.5; both worlds share it, so the ratio is the bonus.)
    let mut plain = GameState::new(311);
    grant(&mut plain, StructureId::CatnipPatch, 1);
    off_boundary(&mut plain);
    let mut sunlit = plain.clone();
    sunlit.rites.insert(RiteId::TheLongSun);
    sunlit.devotion = devotion;
    run_ticks(&mut plain, 1);
    run_ticks(&mut sunlit, 1);
    let plain_gain = plain.stock(ResourceId::Catnip);
    let sunlit_gain = sunlit.stock(ResourceId::Catnip);
    assert!(plain_gain > 0.0);
    assert!(
        (sunlit_gain - plain_gain * (1.0 + expected_bonus)).abs() < 1e-9,
        "the Long Sun brightens the meadow's material flows (structures included)"
    );

    // But transient warmth is exempt from layers 6–8: a Sun Room's warmth is
    // unmoved by the Long Sun, however deep the devotion.
    let mut plain_w = GameState::new(312);
    grant(&mut plain_w, StructureId::SunRoom, 1);
    off_boundary(&mut plain_w);
    let mut sunlit_w = plain_w.clone();
    sunlit_w.rites.insert(RiteId::TheLongSun);
    sunlit_w.devotion = devotion;
    run_ticks(&mut plain_w, 1);
    run_ticks(&mut sunlit_w, 1);
    assert_eq!(
        plain_w.stock(ResourceId::Warmth),
        sunlit_w.stock(ResourceId::Warmth),
        "warmth is transient — the Long Sun never touches it"
    );
}

// @spec SIM-SUN-005
#[test]
fn the_rite_ladder_matches_the_design_table() {
    use ResourceId::{Charm, Shinies, Warmth};
    // Costs (warmth, shinies, [charm]) and devotion gates, exactly.
    let table: [(RiteId, &[(ResourceId, f64)], f64); 11] = [
        (RiteId::MorningStretch, &[(Warmth, 100.0)], 150.0),
        (
            RiteId::SharedSunpuddle,
            &[(Warmth, 250.0), (Shinies, 150.0)],
            300.0,
        ),
        (
            RiteId::GoldenHour,
            &[(Warmth, 350.0), (Shinies, 250.0)],
            500.0,
        ),
        (
            RiteId::WarmWelcome,
            &[(Warmth, 500.0), (Shinies, 250.0)],
            750.0,
        ),
        (
            RiteId::StainedSunlight,
            &[(Warmth, 500.0), (Shinies, 250.0)],
            750.0,
        ),
        (
            RiteId::TheLongSun,
            &[(Warmth, 750.0), (Shinies, 500.0)],
            1000.0,
        ),
        (
            RiteId::GrandBasking,
            &[(Warmth, 1250.0), (Shinies, 750.0)],
            10000.0,
        ),
        (
            RiteId::SunWardensOath,
            &[(Warmth, 3500.0), (Shinies, 1500.0)],
            75000.0,
        ),
        (
            RiteId::TheReverie,
            &[(Warmth, 5000.0), (Shinies, 1800.0)],
            100000.0,
        ),
        (
            RiteId::Enlightenment,
            &[(Warmth, 7500.0), (Shinies, 2000.0)],
            125000.0,
        ),
        (
            RiteId::EverwarmFrescoes,
            &[(Warmth, 4000.0), (Shinies, 1800.0), (Charm, 8000.0)],
            200000.0,
        ),
    ];
    assert_eq!(RiteId::ALL.len(), 11);
    for (rite, cost, gate) in table {
        let def = content::rite_def(rite);
        assert!(!def.name.is_empty(), "{rite:?} names itself");
        assert_eq!(def.cost, cost, "{rite:?} cost");
        assert!((def.devotion_gate - gate).abs() < 1e-9, "{rite:?} gate");
    }

    // The gate reads devotion's persisted high-water, not live devotion: a Bask
    // to the gate then a Reverie that resets live devotion leaves the rite still
    // buyable (the worship-gate never re-locks).
    let mut s = GameState::new(313);
    s.rites.insert(RiteId::TheReverie); // so we can Reverie the live pool away
    give(&mut s, ResourceId::Warmth, 300.0);
    s.apply(Command::Bask).unwrap(); // devotion 300 → peak_devotion 300
    assert!((s.peak_devotion - 300.0).abs() < 1e-9);
    s.apply(Command::Reverie).unwrap(); // devotion resets to 0.01
    assert!(s.devotion < 1.0, "live devotion was reset");
    give(&mut s, ResourceId::Warmth, 250.0);
    give(&mut s, ResourceId::Shinies, 150.0);
    s.apply(Command::BuyRite(RiteId::SharedSunpuddle)).unwrap(); // gate 300 ≤ peak
    assert!(s.rites.contains(&RiteId::SharedSunpuddle));

    // Locked before the gate is ever reached; InsufficientResources when the
    // gate is met but the warmth/shinies aren't in paw.
    let mut s = GameState::new(314);
    give(&mut s, ResourceId::Warmth, 100.0);
    assert_eq!(
        s.apply(Command::BuyRite(RiteId::MorningStretch))
            .unwrap_err(),
        CommandError::Locked,
        "gate 150 unreached"
    );
    s.peak_devotion = 200.0; // gate now met
    give(&mut s, ResourceId::Warmth, 0.0);
    assert!(matches!(
        s.apply(Command::BuyRite(RiteId::MorningStretch))
            .unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));

    // Four rite effects, observed through the effect system / caps:
    let mut s = GameState::new(315);
    s.peak_devotion = 300000.0; // above every gate
    grant(&mut s, StructureId::SunbeamAltar, 1);
    grant(&mut s, StructureId::SunRoom, 1); // +75 warmth cap, so GoldenHour bites
    let warmth_before = s.effect(Effect::ResourceRatio(ResourceId::Warmth));
    let happy_before = s.effect(Effect::HappinessPoints);
    // Morning Stretch: warmth production +10%.
    buy_rite_free(&mut s, RiteId::MorningStretch);
    assert!(
        (s.effect(Effect::ResourceRatio(ResourceId::Warmth)) - warmth_before - 0.10).abs() < 1e-9
    );
    // Shared Sunpuddle: Altars +0.02 curiosity cap-ratio each (1 altar → 0.02).
    buy_rite_free(&mut s, RiteId::SharedSunpuddle);
    assert!((s.effect(Effect::MaxCapRatio(ResourceId::Curiosity)) - 0.02).abs() < 1e-9);
    // Golden Hour: the whole warmth reservoir × (1.4 + 0.1 per Golden Hour) —
    // 1 copy → ×1.5, over every warmth-cap source (Sun Room 75 + Altar 120),
    // never shrinking it (Tenet 1: buying a rite is never a confiscation).
    buy_rite_free(&mut s, RiteId::GoldenHour);
    assert!(
        (s.cap(ResourceId::Warmth) - (75.0 + 120.0) * 1.5).abs() < 1e-9,
        "warmth cap ×1.5"
    );
    // Warm Welcome: Altars +0.4 happiness points each (1 altar → +0.4).
    buy_rite_free(&mut s, RiteId::WarmWelcome);
    assert!((s.effect(Effect::HappinessPoints) - happy_before - 0.4).abs() < 1e-9);
}

/// Buy a rite with cost handed over first — asserts the purchase succeeds so a
/// caller can focus on the rite's effect. (peak_devotion must already clear the
/// gate.)
fn buy_rite_free(s: &mut GameState, rite: RiteId) {
    for (r, amount) in content::rite_def(rite).cost {
        let held = s.stock(*r);
        give(s, *r, held + *amount);
    }
    s.apply(Command::BuyRite(rite))
        .expect("rite affordable and past its gate");
}

// @spec SIM-SUN-003
#[test]
fn serenity_and_tier_are_legacy_and_never_decrease() {
    let mut s = GameState::new(308);
    s.serenity = 42.5;
    s.enlightenment_tier = 3;
    s.peak_devotion = 1.0e5;

    // They ride out ordinary play untouched — legacy currencies only grow.
    run_days(&mut s, 200);
    assert!(
        (s.serenity - 42.5).abs() < 1e-9,
        "serenity never decreases passively"
    );
    assert_eq!(s.enlightenment_tier, 3, "the tier never decreases");

    // And they survive the save file (they persist across the Great Meadow Move,
    // which is serenity-bought; here we pin the serialization half).
    let json = save_string(&s, 500_000);
    let loaded = load_string(&json, 500_000).unwrap();
    assert!((loaded.state.serenity - 42.5).abs() < 1e-9);
    assert_eq!(loaded.state.enlightenment_tier, 3);
    assert!((loaded.state.peak_devotion - 1.0e5).abs() < 1e-9);
}

// @spec SIM-SUN-006
#[test]
fn enlightenment_makes_early_rites_restackable_at_a_2_5_ramp() {
    // Without the Enlightenment rite, every rite is single-purchase.
    let mut s = GameState::new(316);
    s.peak_devotion = 300000.0;
    give(&mut s, ResourceId::Warmth, 100.0);
    s.apply(Command::BuyRite(RiteId::MorningStretch)).unwrap();
    give(&mut s, ResourceId::Warmth, 1000.0);
    assert_eq!(
        s.apply(Command::BuyRite(RiteId::MorningStretch))
            .unwrap_err(),
        CommandError::AlreadyResearched,
        "single-purchase before Enlightenment"
    );

    // With Enlightenment, rites 0–7 become re-buyable; effects stack and the
    // price ramps ×2.5 per additional copy.
    let mut s = GameState::new(317);
    s.peak_devotion = 300000.0;
    s.rites.insert(RiteId::Enlightenment);
    give(&mut s, ResourceId::Warmth, 100.0);
    s.apply(Command::BuyRite(RiteId::MorningStretch)).unwrap(); // 1st copy: 100
    assert!((s.effect(Effect::ResourceRatio(ResourceId::Warmth)) - 0.10).abs() < 1e-9);

    // 2nd copy costs 100 × 2.5 = 250 warmth: 249 is short, 250 buys it.
    give(&mut s, ResourceId::Warmth, 249.0);
    assert!(matches!(
        s.apply(Command::BuyRite(RiteId::MorningStretch))
            .unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    give(&mut s, ResourceId::Warmth, 250.0); // now 250 (give sets, not adds)
    s.apply(Command::BuyRite(RiteId::MorningStretch)).unwrap();
    assert_eq!(s.stock(ResourceId::Warmth), 0.0, "2nd copy cost 250 warmth");
    assert!(
        (s.effect(Effect::ResourceRatio(ResourceId::Warmth)) - 0.20).abs() < 1e-9,
        "stacked warmth ratio"
    );

    // Rites 5, 8, 9, 10 stay single-purchase even under Enlightenment.
    give(&mut s, ResourceId::Warmth, 5000.0);
    give(&mut s, ResourceId::Shinies, 5000.0);
    s.apply(Command::BuyRite(RiteId::TheLongSun)).unwrap();
    give(&mut s, ResourceId::Warmth, 5000.0);
    give(&mut s, ResourceId::Shinies, 5000.0);
    assert_eq!(
        s.apply(Command::BuyRite(RiteId::TheLongSun)).unwrap_err(),
        CommandError::AlreadyResearched,
        "rite 5 is single even with Enlightenment"
    );
}

// @spec SIM-SUN-007
#[test]
fn rite_effects_flow_through_the_effect_system_and_are_never_removed() {
    let mut s = GameState::new(318);
    s.peak_devotion = 300000.0;
    buy_rite_free(&mut s, RiteId::MorningStretch);
    assert!((s.effect(Effect::ResourceRatio(ResourceId::Warmth)) - 0.10).abs() < 1e-9);

    // No mechanic removes a bought rite's effect: it survives long play …
    run_days(&mut s, 400);
    assert!((s.effect(Effect::ResourceRatio(ResourceId::Warmth)) - 0.10).abs() < 1e-9);
    assert!(s.rites.contains(&RiteId::MorningStretch));

    // … and the save file.
    let json = save_string(&s, 600_000);
    let loaded = load_string(&json, 600_000).unwrap();
    assert!(loaded.state.rites.contains(&RiteId::MorningStretch));
    assert!(
        (loaded
            .state
            .effect(Effect::ResourceRatio(ResourceId::Warmth))
            - 0.10)
            .abs()
            < 1e-9
    );
}

// @spec SIM-SUN-008
#[test]
fn enlighten_spends_the_marginal_serenity_and_raises_the_tier() {
    // total_cost(t) = inverse_unlimited_dr(e^t/10, 0.1); each Enlighten spends
    // total_cost(t) − total_cost(t−1) and increments the tier.
    let mut s = GameState::new(319);
    s.rites.insert(RiteId::Enlightenment); // the rite enables the ladder
    s.serenity = 1.0;
    let step0 = enlighten_total_cost(1) - enlighten_total_cost(0);
    let events = s.apply(Command::Enlighten).unwrap();
    assert_eq!(s.enlightenment_tier, 1);
    assert!(
        (s.serenity - (1.0 - step0)).abs() < 1e-9,
        "spent the marginal cost to tier 1"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Enlightened { .. })));

    // The next tier's marginal is total_cost(2) − total_cost(1) — steeper.
    let step1 = enlighten_total_cost(2) - enlighten_total_cost(1);
    assert!(step1 > step0, "the ladder is brutally exponential");
    let ser1 = s.serenity;
    s.serenity = ser1 + step1; // just enough
    s.apply(Command::Enlighten).unwrap();
    assert_eq!(s.enlightenment_tier, 2);
    assert!((s.serenity - ser1).abs() < 1e-9);

    // Too little serenity for the next tier rejects with InsufficientResources.
    let mut s = GameState::new(320);
    s.rites.insert(RiteId::Enlightenment);
    s.serenity = 0.0001; // far below total_cost(1) − total_cost(0)
    assert!(matches!(
        s.apply(Command::Enlighten).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    assert_eq!(
        s.enlightenment_tier, 0,
        "a rejected Enlighten changes no tier"
    );
}

// =====================================================================
// SIM-UNI — Mystic: the currency chain
// =====================================================================

// @spec SIM-UNI-001
#[test]
fn hold_parade_trades_unicorns_for_arch_scaled_rainbows() {
    // Locked before the Parade Routes idea, even with unicorns marshalled.
    let mut s = GameState::new(321);
    give(&mut s, ResourceId::Unicorns, 2500.0);
    assert_eq!(
        s.apply(Command::HoldParade).unwrap_err(),
        CommandError::Locked
    );

    // After the idea, with ≥ 2500 unicorns: spend 2500, gain 2500 × arches ×
    // parade_ratio rainbows (base parade_ratio = 1).
    let mut s = GameState::new(322);
    s.ideas.insert(IdeaId::ParadeRoutes);
    grant(&mut s, StructureId::RainbowArch, 2);
    give(&mut s, ResourceId::Unicorns, 2500.0);
    let events = s.apply(Command::HoldParade).unwrap();
    assert_eq!(
        s.stock(ResourceId::Unicorns),
        0.0,
        "2500 unicorns march off"
    );
    assert!(
        (s.stock(ResourceId::Rainbows) - 2500.0 * 2.0).abs() < 1e-9,
        "2500 × arches"
    );
    assert!(events.iter().any(|e| matches!(e, Event::ParadeHeld { .. })));

    // Below 2500 unicorns rejects with InsufficientResources.
    let mut s = GameState::new(323);
    s.ideas.insert(IdeaId::ParadeRoutes);
    grant(&mut s, StructureId::RainbowArch, 1);
    give(&mut s, ResourceId::Unicorns, 2000.0);
    assert!(matches!(
        s.apply(Command::HoldParade).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
}

// @spec SIM-UNI-011
#[test]
fn researching_parade_routes_grants_the_first_rainbow_arch_free() {
    let mut s = GameState::new(324);
    grant(&mut s, StructureId::CardboardBox, 1); // curiosity cap > 0 opens research
    s.ideas.insert(IdeaId::UnicornLore);
    s.ideas.insert(IdeaId::GrandSchemes);
    give(&mut s, ResourceId::Curiosity, 30000.0);
    give(&mut s, ResourceId::Schemes, 15.0);
    assert_eq!(s.structures[StructureId::RainbowArch.ix()], 0);
    s.apply(Command::ResearchIdea(IdeaId::ParadeRoutes))
        .unwrap();
    assert_eq!(
        s.structures[StructureId::RainbowArch.ix()],
        1,
        "the first arch arrives free, so the loop bootstraps without deadlock"
    );
}

// @spec SIM-UNI-002
#[test]
fn unicorn_meadow_scales_by_the_mystic_unicorns_ratio_not_the_global_layers() {
    // A Ribbon Arch grants +5% unicorns via the mystic tier's own multiplier.
    let mut plain = GameState::new(325);
    grant(&mut plain, StructureId::UnicornMeadow, 1);
    off_boundary(&mut plain);
    let mut ribboned = plain.clone();
    ribboned.arches_bought.insert(ArchId::RibbonArch);
    assert!((ribboned.effect(Effect::UnicornsRatio) - 0.05).abs() < 1e-9);

    run_ticks(&mut plain, 1);
    run_ticks(&mut ribboned, 1);
    // Meadow base 0.001/t; Ribbon Arch → ×1.05.
    assert!((plain.stock(ResourceId::Unicorns) - 0.001).abs() < 1e-9);
    assert!((ribboned.stock(ResourceId::Unicorns) - 0.001 * 1.05).abs() < 1e-9);

    // A fueled Dynamo Wheel (global layer 6) must NOT double-multiply unicorns —
    // mystic income is exempt from the global layers.
    let mut dynamo = plain.clone();
    grant(&mut dynamo, StructureId::DynamoWheel, 1);
    give(&mut dynamo, ResourceId::Dew, 10.0);
    run_ticks(&mut dynamo, 1);
    run_ticks(&mut plain, 1); // match dynamo's tick count (dynamo was cloned post-tick)
    assert_eq!(
        dynamo.stock(ResourceId::Unicorns),
        plain.stock(ResourceId::Unicorns),
        "unicorns are mystic — the Dynamo's global ratio never reaches them"
    );
}

// @spec SIM-UNI-003
#[test]
fn refining_condenses_prisms_and_starshards_upward() {
    // 25 prisms → 1 × (1 + starshard-refine) starshards. Base refine = 0 → 1.0.
    let mut s = GameState::new(326);
    give(&mut s, ResourceId::Prisms, 25.0);
    s.apply(Command::RefineStarshard).unwrap();
    assert_eq!(s.stock(ResourceId::Prisms), 0.0);
    assert!((s.stock(ResourceId::Starshards) - 1.0).abs() < 1e-9);

    // A Dreaming Arch adds +5% starshard refine, observed on the yield.
    let mut s = GameState::new(327);
    s.arches_bought.insert(ArchId::DreamingArch);
    assert!((s.effect(Effect::StarshardRefine) - 0.05).abs() < 1e-9);
    give(&mut s, ResourceId::Prisms, 25.0);
    s.apply(Command::RefineStarshard).unwrap();
    assert!((s.stock(ResourceId::Starshards) - 1.05).abs() < 1e-9);

    // Under 25 prisms rejects.
    let mut s = GameState::new(328);
    give(&mut s, ResourceId::Prisms, 24.0);
    assert!(matches!(
        s.apply(Command::RefineStarshard).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));

    // 25 starshards → 1 × (1 + echo-refine) echoes; base echo-refine 0 → 1.0.
    let mut s = GameState::new(329);
    give(&mut s, ResourceId::Starshards, 25.0);
    s.apply(Command::RefineEcho).unwrap();
    assert_eq!(s.stock(ResourceId::Starshards), 0.0);
    assert!((s.stock(ResourceId::Echoes) - 1.0).abs() < 1e-9);
    // Under 25 starshards rejects.
    give(&mut s, ResourceId::Starshards, 24.0);
    assert!(matches!(
        s.apply(Command::RefineEcho).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
}

// @spec SIM-UNI-004
#[test]
fn condensing_a_jar_needs_the_everdream_arch_and_clamps_to_cap() {
    // Before the Everdream Arch (the Jar-condensing arch), CondenseJar is Locked.
    let mut s = GameState::new(330);
    give(&mut s, ResourceId::Rainbows, 10000.0);
    assert_eq!(
        s.apply(Command::CondenseJar).unwrap_err(),
        CommandError::Locked
    );

    // The Everdream Arch raises the Jar cap (+2 each) and unlocks condensing.
    let mut s = GameState::new(331);
    s.arches_bought.insert(ArchId::EverdreamArch);
    assert!(
        (s.cap(ResourceId::JarsOfPurrs) - 2.0).abs() < 1e-9,
        "cap +2 per Everdream Arch"
    );

    // Under 10000 rainbows rejects; at 10000, one Jar condenses (within cap).
    give(&mut s, ResourceId::Rainbows, 9999.0);
    assert!(matches!(
        s.apply(Command::CondenseJar).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    give(&mut s, ResourceId::Rainbows, 10000.0);
    s.apply(Command::CondenseJar).unwrap();
    assert_eq!(
        s.stock(ResourceId::Rainbows),
        0.0,
        "10000 rainbows condense"
    );
    assert!((s.stock(ResourceId::JarsOfPurrs) - 1.0).abs() < 1e-9);
    assert!(s.stock(ResourceId::JarsOfPurrs) <= s.cap(ResourceId::JarsOfPurrs));
}

// @spec SIM-UNI-005
#[test]
fn the_rainbow_arch_ladder_matches_the_design_table() {
    use ResourceId::{Cairns, Dreamcorns, Prisms, Rainbows, Shinies, Stardust};
    let table: [(ArchId, &[(ResourceId, f64)]); 8] = [
        (ArchId::RibbonArch, &[(Rainbows, 500.0)]),
        (ArchId::CrystalArch, &[(Rainbows, 25000.0), (Prisms, 25.0)]),
        (ArchId::PrismArch, &[(Rainbows, 50000.0), (Prisms, 50.0)]),
        (ArchId::SkyArch, &[(Rainbows, 125000.0), (Prisms, 500.0)]),
        (
            ArchId::DreamingArch,
            &[(Shinies, 500.0), (Rainbows, 1_000_000.0), (Prisms, 5000.0)],
        ),
        (
            ArchId::StarlitArch,
            &[(Shinies, 1250.0), (Rainbows, 750_000.0), (Prisms, 25000.0)],
        ),
        (ArchId::MemoryArch, &[(Stardust, 2500.0), (Prisms, 50000.0)]),
        (
            ArchId::EverdreamArch,
            &[(Dreamcorns, 100.0), (Cairns, 1000.0)],
        ),
    ];
    assert_eq!(ArchId::ALL.len(), 8);
    for (arch, cost) in table {
        let def = content::arch_def(arch);
        assert!(!def.name.is_empty(), "{arch:?} names itself");
        assert_eq!(def.cost, cost, "{arch:?} cost");
    }

    // BuyArch is Locked before the Parade Routes idea, buyable once after.
    let mut s = GameState::new(332);
    give(&mut s, ResourceId::Rainbows, 500.0);
    assert_eq!(
        s.apply(Command::BuyArch(ArchId::RibbonArch)).unwrap_err(),
        CommandError::Locked
    );
    s.ideas.insert(IdeaId::ParadeRoutes);
    s.apply(Command::BuyArch(ArchId::RibbonArch)).unwrap();
    assert!(s.arches_bought.contains(&ArchId::RibbonArch));
    give(&mut s, ResourceId::Rainbows, 500.0);
    assert_eq!(
        s.apply(Command::BuyArch(ArchId::RibbonArch)).unwrap_err(),
        CommandError::AlreadyResearched,
        "each arch upgrade is bought once"
    );
}

// @spec SIM-UNI-006, SIM-ENG-018
#[test]
fn prisms_descend_from_rainbow_arches_over_the_days() {
    // Effect view: each Rainbow Arch structure is +0.02%/day, plus Arch-upgrade
    // bonuses (a Crystal Arch adds +0.05%/day).
    let mut s = GameState::new(333);
    grant(&mut s, StructureId::RainbowArch, 100);
    assert!(
        (s.effect(Effect::PrismDescentChance) - 0.02).abs() < 1e-9,
        "100 arches × 0.0002"
    );
    s.arches_bought.insert(ArchId::CrystalArch);
    assert!(
        (s.effect(Effect::PrismDescentChance) - 0.0205).abs() < 1e-9,
        "+0.0005 from Crystal"
    );

    // Statistical accrual: 100 arches over 2000 days is a mean of ~40 prisms
    // (Binomial(2000, 0.02)); generous bounds absorb the RNG.
    let mut s = GameState::new(334);
    grant(&mut s, StructureId::RainbowArch, 100);
    run_days(&mut s, 2000);
    let prisms = s.stock(ResourceId::Prisms);
    assert!(
        (10.0..=90.0).contains(&prisms),
        "expected ~40 prisms to descend over 2000 days, got {prisms}"
    );
}

// @spec SIM-UNI-007
#[test]
fn dreaming_drifts_prisms_into_dreamcorns_but_never_the_last_one() {
    // The Memory Arch upgrade turns the Rainbow Arch *structures* into the
    // dreamcorn engine; drift scales with the arch count. Throttled ×0.25 once
    // any dreamcorn already dreams. Same arches, same span: the throttled run
    // drifts a quarter as fast (a slowdown, never a loss).
    let mut awake = GameState::new(335);
    awake.arches_bought.insert(ArchId::MemoryArch);
    grant(&mut awake, StructureId::RainbowArch, 100); // the engine scales with structures
    let mut dreaming = awake.clone();
    give(&mut dreaming, ResourceId::Dreamcorns, 1.0); // a dreamcorn already asleep
    run_days(&mut awake, 1);
    run_days(&mut dreaming, 1);
    assert!(awake.dream_drift > 0.0, "Memory Arches feed the drift");
    assert!(
        (dreaming.dream_drift / awake.dream_drift - 0.25).abs() < 1e-6,
        "an existing dreamcorn throttles the drift to ×0.25"
    );

    // When drift ≥ 1, one prism falls asleep: prisms −1, dreamcorns +1.
    let mut s = GameState::new(336);
    s.arches_bought.insert(ArchId::MemoryArch);
    s.dream_drift = 1.5;
    give(&mut s, ResourceId::Prisms, 2.0);
    run_days(&mut s, 1);
    assert!(
        (s.stock(ResourceId::Prisms) - 1.0).abs() < 1e-9,
        "one prism drifted off"
    );
    assert!((s.stock(ResourceId::Dreamcorns) - 1.0).abs() < 1e-9);

    // The cozy floor: the last prism must stay awake to keep the dream going.
    let mut s = GameState::new(337);
    s.arches_bought.insert(ArchId::MemoryArch);
    s.dream_drift = 1.5;
    give(&mut s, ResourceId::Prisms, 1.0);
    run_days(&mut s, 1);
    assert!(
        (s.stock(ResourceId::Prisms) - 1.0).abs() < 1e-9,
        "never the last prism"
    );
    assert_eq!(
        s.stock(ResourceId::Dreamcorns),
        0.0,
        "so no dreamcorn forms"
    );
}

// @spec SIM-UNI-008
#[test]
fn dreamcorns_are_the_softest_resource_never_spent_but_by_a_promise() {
    // With no Promises and no Memory Arches, the dreamcorn stock is inviolate —
    // nothing in ordinary play reduces it.
    let mut s = GameState::new(338);
    give(&mut s, ResourceId::Dreamcorns, 10.0);
    run_days(&mut s, 10);
    assert_eq!(
        s.stock(ResourceId::Dreamcorns),
        10.0,
        "dreamcorns only ever nap, never leave"
    );

    // Dreaming only ever adds dreamcorns (never removes), and never a prism
    // below one — the floor from SIM-UNI-007 holds across a long dream.
    let mut s = GameState::new(339);
    s.arches_bought.insert(ArchId::MemoryArch);
    s.dream_drift = 1.5;
    give(&mut s, ResourceId::Prisms, 3.0);
    run_days(&mut s, 50);
    assert!(
        s.stock(ResourceId::Prisms) >= 1.0,
        "a prism always stays awake"
    );
    assert!(s.stock(ResourceId::Dreamcorns) >= 0.0);
}

/// A state with a free Promise slot (from Everdream content), the Promises idea,
/// and generous echoes + dreamcorns — ready for MakePromise.
fn promise_ready(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    s.arches_bought.insert(ArchId::EverdreamArch); // slots come from Everdream content
    s.ideas.insert(IdeaId::NightCourtEtiquette); // the Promises surface
    give(&mut s, ResourceId::Echoes, 1000.0);
    give(&mut s, ResourceId::Dreamcorns, 10.0);
    s
}

// @spec SIM-UNI-009
#[test]
fn making_a_promise_spends_echoes_fills_a_slot_and_grants_a_global_effect() {
    let mut s = promise_ready(340);
    let echoes_before = s.stock(ResourceId::Echoes);
    let global_before = s.effect(Effect::GlobalProductionRatio);
    s.apply(Command::MakePromise).unwrap();
    assert_eq!(s.promises.len(), 1, "a slot is filled");
    assert!(
        s.stock(ResourceId::Echoes) < echoes_before,
        "echoes are spent"
    );
    assert!(
        s.effect(Effect::GlobalProductionRatio) > global_before,
        "an active Promise grants its small global effect"
    );

    // Upkeep draws 0.0005 dreamcorn/day while the Promise is kept.
    let corns_before = s.stock(ResourceId::Dreamcorns);
    run_days(&mut s, 1);
    assert!(
        (corns_before - s.stock(ResourceId::Dreamcorns) - 0.0005).abs() < 1e-9,
        "0.0005 dreamcorn/day gentle upkeep, per Promise"
    );
}

// @spec SIM-UNI-010
#[test]
fn a_neglected_promise_goes_quiet_and_wakes_again_never_shattering() {
    let mut s = promise_ready(341);
    s.apply(Command::MakePromise).unwrap();
    let active_effect = s.effect(Effect::GlobalProductionRatio);
    assert!(active_effect > 0.0);

    // Drain the dreamcorns: with no upkeep to pay, the Promise naps — its effect
    // suspends but its slot is held and nothing is destroyed.
    give(&mut s, ResourceId::Dreamcorns, 0.0);
    run_days(&mut s, 1);
    assert_eq!(s.promises.len(), 1, "the slot is held, not lost");
    assert!(s.promises[0].quiet, "the Promise goes quiet");
    assert!(
        s.effect(Effect::GlobalProductionRatio) < active_effect,
        "a quiet Promise's effect is suspended"
    );

    // Refill the dreamcorns: the Promise wakes automatically next upkeep.
    give(&mut s, ResourceId::Dreamcorns, 10.0);
    run_days(&mut s, 1);
    assert!(!s.promises[0].quiet, "upkeep resumes → the Promise wakes");
    assert!((s.effect(Effect::GlobalProductionRatio) - active_effect).abs() < 1e-9);
}

// =====================================================================
// SIM-TRADE — Colonies: trade & diplomacy
// =====================================================================

// @spec SIM-TRADE-001
#[test]
fn trade_costs_pep_shinies_and_the_buy_resource_per_caravan() {
    // An undiscovered colony rejects with Locked.
    let mut s = GameState::new(342);
    give(&mut s, ResourceId::Pep, 100.0);
    give(&mut s, ResourceId::Shinies, 100.0);
    give(&mut s, ResourceId::Pebbles, 2000.0);
    assert_eq!(
        s.apply(Command::Trade(ColonyId::Otters, 1)).unwrap_err(),
        CommandError::Locked
    );

    // Discovered and affordable: one caravan spends 50 pep + 15 shinies + the
    // Otters' buy resource (1000 pebbles), whatever the outcome roll returns.
    let mut s = GameState::new(343);
    s.discovered.insert(ColonyId::Otters);
    give(&mut s, ResourceId::Pep, 50.0);
    give(&mut s, ResourceId::Shinies, 15.0);
    give(&mut s, ResourceId::Pebbles, 1000.0);
    s.apply(Command::Trade(ColonyId::Otters, 1)).unwrap();
    assert_eq!(s.stock(ResourceId::Pep), 0.0, "50 pep per caravan");
    assert_eq!(s.stock(ResourceId::Shinies), 0.0, "15 shinies per caravan");
    assert_eq!(
        s.stock(ResourceId::Pebbles),
        0.0,
        "the colony's buy resource"
    );

    // Unaffordable rejects — and affordability is checked BEFORE any RNG draw,
    // so a rejected trade leaves the whole state (RNG included) untouched.
    let mut s = GameState::new(344);
    s.discovered.insert(ColonyId::Otters);
    give(&mut s, ResourceId::Pep, 40.0); // short of 50
    give(&mut s, ResourceId::Shinies, 15.0);
    give(&mut s, ResourceId::Pebbles, 1000.0);
    let before = s.clone();
    assert!(matches!(
        s.apply(Command::Trade(ColonyId::Otters, 1)).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    assert_eq!(s, before, "a rejected trade draws nothing");
}

// @spec SIM-TRADE-002
#[test]
fn caravans_resolve_against_standing_success_yields_a_basket_a_nap_still_pays() {
    // A warm colony (Otters, +0.25) trades well: over many caravans its
    // guaranteed twig basket piles up (generous bounds absorb the RNG).
    let mut s = GameState::new(345);
    s.discovered.insert(ColonyId::Otters);
    give(&mut s, ResourceId::Pep, 50_000.0);
    give(&mut s, ResourceId::Shinies, 15_000.0);
    give(&mut s, ResourceId::Pebbles, 1_000_000.0);
    s.apply(Command::Trade(ColonyId::Otters, 1000)).unwrap();
    let twigs = s.stock(ResourceId::Twigs);
    assert!(
        (100_000.0..=1_500_000.0).contains(&twigs),
        "a warm colony's successful caravans yield baskets, got {twigs} twigs"
    );

    // A nap still spends the caravan (the caravan travelled): a shy colony's
    // cost is fully paid regardless of how many caravans napped.
    let mut s = GameState::new(346);
    s.discovered.insert(ColonyId::Owls); // shy, −0.10
    give(&mut s, ResourceId::Pep, 5_000.0);
    give(&mut s, ResourceId::Shinies, 1_500.0);
    give(&mut s, ResourceId::Mint, 5_000.0);
    s.apply(Command::Trade(ColonyId::Owls, 100)).unwrap();
    assert_eq!(
        s.stock(ResourceId::Pep),
        0.0,
        "every caravan's pep is spent"
    );
    assert_eq!(s.stock(ResourceId::Shinies), 0.0);
    assert_eq!(
        s.stock(ResourceId::Mint),
        0.0,
        "even napped caravans pay the buy resource"
    );
}

// @spec SIM-TRADE-003
#[test]
fn the_seven_tradeable_colonies_match_the_design_table() {
    use ResourceId::*;
    assert_eq!(
        ColonyId::ALL.len(),
        8,
        "eight colonies; the eighth is the visitor slot"
    );

    // (colony, start standing, buy, primary sell: resource/amount/chance)
    let table: [(ColonyId, f64, (ResourceId, f64), (ResourceId, f64, f64)); 7] = [
        (
            ColonyId::Otters,
            0.25,
            (Pebbles, 1000.0),
            (Twigs, 500.0, 1.0),
        ),
        (
            ColonyId::Raccoons,
            0.0,
            (Buttons, 100.0),
            (Catnip, 35000.0, 1.0),
        ),
        (
            ColonyId::Foxes,
            -0.15,
            (Twigs, 500.0),
            (Buttons, 250.0, 1.0),
        ),
        (
            ColonyId::Hedgehogs,
            0.0,
            (Feathers, 500.0),
            (Pebbles, 1000.0, 1.0),
        ),
        (ColonyId::Rabbits, 0.15, (Tiles, 50.0), (Embers, 350.0, 1.0)),
        (
            ColonyId::Deer,
            0.0,
            (Shinies, 250.0),
            (Moonstone, 1.0, 0.95),
        ),
        (
            ColonyId::Owls,
            -0.10,
            (Mint, 50.0),
            (Curiosity, 2500.0, 1.0),
        ),
    ];
    for (colony, standing, buy, (sell_r, sell_amt, sell_chance)) in table {
        let def = content::colony_def(colony);
        assert!(!def.name.is_empty(), "{colony:?} names itself");
        assert!(
            (def.start_standing - standing).abs() < 1e-9,
            "{colony:?} start standing"
        );
        assert_eq!(def.buy, buy, "{colony:?} buy resource");
        let first = &def.sells[0];
        assert_eq!(first.resource, sell_r, "{colony:?} primary sell");
        assert!(
            (first.amount - sell_amt).abs() < 1e-9,
            "{colony:?} sell amount"
        );
        assert!(
            (first.chance - sell_chance).abs() < 1e-9,
            "{colony:?} sell chance"
        );
    }

    // The Star Whale (colony 7) reserves a visitor slot only — its basket is
    // deferred [D] to the cosmic wave.
    assert!(
        content::colony_def(ColonyId::StarWhales).sells.is_empty(),
        "the Star Whale's trade table is deferred"
    );
}

// @spec SIM-TRADE-004
#[test]
fn foxes_moonstone_scales_with_kites() {
    // chance 0.15 + 0.0035 × Kites, amount (1.5 + 0.03 × Kites) × (1 + fox
    // standing bonus): more Kites, more moonstone. Compared statistically over
    // many caravans; a Shed grants moonstone cap headroom so nothing clamps.
    let trade_moonstone = |seed: u64, kites: f64| -> f64 {
        let mut s = GameState::new(seed);
        s.discovered.insert(ColonyId::Foxes);
        grant(&mut s, StructureId::Shed, 100); // +1000 moonstone cap
        give(&mut s, ResourceId::Kites, kites);
        give(&mut s, ResourceId::Pep, 10_000.0);
        give(&mut s, ResourceId::Shinies, 3_000.0);
        give(&mut s, ResourceId::Twigs, 100_000.0);
        s.apply(Command::Trade(ColonyId::Foxes, 200)).unwrap();
        s.stock(ResourceId::Moonstone)
    };
    let no_kites = trade_moonstone(347, 0.0);
    let many_kites = trade_moonstone(348, 100.0);
    assert!(
        many_kites > no_kites,
        "Kites scale Foxes' moonstone up ({many_kites} vs {no_kites})"
    );
    assert!(many_kites > 5.0, "100 Kites open a real early-metal supply");
}

// @spec SIM-TRADE-005
#[test]
fn standing_starts_from_the_table_warms_with_pen_pals_and_never_turns_enemy() {
    // Standing = start + Pen Pal contribution + dedicated standing effects.
    let s = GameState::new(349);
    assert!((s.standing(ColonyId::Otters) - 0.25).abs() < 1e-9);
    assert!((s.standing(ColonyId::Raccoons) - 0.0).abs() < 1e-9);
    assert!((s.standing(ColonyId::Foxes) - (-0.15)).abs() < 1e-9);
    assert!((s.standing(ColonyId::Rabbits) - 0.15).abs() < 1e-9);
    assert!((s.standing(ColonyId::Owls) - (-0.10)).abs() < 1e-9);

    // The StandingFor(colony) effect key exists and is dormant at zero baseline.
    assert!((s.effect(Effect::StandingFor(ColonyId::Otters)) - 0.0).abs() < 1e-9);

    // A shy colony only ever warms toward neutral: with deep Pen Pal ties the
    // Foxes' standing rises above its −0.15 floor but is capped at 0 (neutral).
    // It can never sink below the floor (never becomes an enemy).
    let mut s = GameState::new(350);
    s.penpal[ColonyId::Foxes as usize] = 1000;
    let warmed = s.standing(ColonyId::Foxes);
    assert!(
        warmed >= -0.15 - 1e-9,
        "never more negative than the start floor"
    );
    assert!(
        warmed <= 1e-9,
        "a shy colony only warms to neutral, never past it"
    );
    assert!(warmed > -0.15 + 1e-9, "Pen Pals do warm the shy colony");
}

// @spec SIM-TRADE-006
#[test]
fn sending_pen_pals_costs_ramping_charm_deepens_ties_and_whales_refuse() {
    let mut s = GameState::new(351);
    s.ideas.insert(IdeaId::PenPalNetworks);
    s.discovered.insert(ColonyId::Otters);
    give(&mut s, ResourceId::Charm, 1_000_000.0);

    // charm cost = base × 1.15^level. Level 0 → base; level 1 → base × 1.15.
    let c0 = s.stock(ResourceId::Charm);
    s.apply(Command::SendPenPal(ColonyId::Otters)).unwrap();
    assert_eq!(
        s.penpal[ColonyId::Otters as usize],
        1,
        "the correspondence deepens by a level"
    );
    let d1 = c0 - s.stock(ResourceId::Charm);

    let c1 = s.stock(ResourceId::Charm);
    s.apply(Command::SendPenPal(ColonyId::Otters)).unwrap();
    assert_eq!(s.penpal[ColonyId::Otters as usize], 2);
    let d2 = c1 - s.stock(ResourceId::Charm);
    assert!(d1 > 0.0);
    assert!((d2 / d1 - 1.15).abs() < 1e-9, "cost ramps ×1.15 per level");

    // Star Whales take no Pen Pals — the correspondence is refused.
    let mut s = GameState::new(352);
    s.ideas.insert(IdeaId::PenPalNetworks);
    s.discovered.insert(ColonyId::StarWhales);
    give(&mut s, ResourceId::Charm, 1_000_000.0);
    assert_eq!(
        s.apply(Command::SendPenPal(ColonyId::StarWhales))
            .unwrap_err(),
        CommandError::Locked,
        "the Star Whale drifts on, taking no Pen Pal"
    );
}

// @spec SIM-TRADE-007
#[test]
fn scout_picnics_discover_the_next_colony_by_deterministic_priority() {
    // Locked before the trade surface (the Gift Economy idea) opens.
    let mut s = GameState::new(353);
    give(&mut s, ResourceId::Pep, 1000.0);
    assert_eq!(
        s.apply(Command::ScoutPicnic).unwrap_err(),
        CommandError::Locked
    );

    // With the surface open and 1000 pep, a picnic either finds someone (full
    // 1000 pep spent) or comes home quiet (950 pep refunded — they mostly
    // stayed home).
    let mut s = GameState::new(354);
    s.ideas.insert(IdeaId::GiftEconomy);
    give(&mut s, ResourceId::Pep, 1000.0);
    let before = s.discovered.len();
    s.apply(Command::ScoutPicnic).unwrap();
    if s.discovered.len() == before {
        assert!(
            (s.stock(ResourceId::Pep) - 950.0).abs() < 1e-9,
            "quiet day: 950 pep refunded"
        );
    } else {
        assert!(
            (s.stock(ResourceId::Pep) - 0.0).abs() < 1e-9,
            "a discovery costs the full 1000"
        );
        assert_eq!(s.discovered.len(), before + 1, "exactly one new colony");
    }

    // Deterministic priority: with every first-tier colony already known, Foxes
    // eligible (charm ≥ 1500) and no Kites (Deer/Owls ineligible), any
    // successful picnic must find the Foxes.
    let mut s = GameState::new(355);
    s.ideas.insert(IdeaId::GiftEconomy);
    for c in [
        ColonyId::Otters,
        ColonyId::Raccoons,
        ColonyId::Hedgehogs,
        ColonyId::Rabbits,
    ] {
        s.discovered.insert(c);
    }
    give(&mut s, ResourceId::Charm, 1500.0); // Foxes' prerequisite
    let mut found = false;
    for _ in 0..1000 {
        give(&mut s, ResourceId::Pep, 1000.0); // top up (quiet days refund 950)
        s.apply(Command::ScoutPicnic).unwrap();
        if s.discovered.contains(&ColonyId::Foxes) {
            found = true;
            break;
        }
        assert!(!s.discovered.contains(&ColonyId::Deer), "Deer need a Kite");
        assert!(
            !s.discovered.contains(&ColonyId::Owls),
            "Owls need 100 Kites"
        );
    }
    assert!(
        found,
        "a successful picnic deterministically discovers the Foxes"
    );
}

// @spec SIM-TRADE-008
#[test]
fn the_trade_surface_hands_over_the_first_colony_free_and_forever() {
    let mut s = GameState::new(356);
    grant(&mut s, StructureId::CardboardBox, 1); // curiosity cap > 0 opens research
    s.ideas.insert(IdeaId::OralTradition); // Gift Economy's prerequisite
    give(&mut s, ResourceId::Curiosity, 1100.0);
    assert!(s.discovered.is_empty());
    s.apply(Command::ResearchIdea(IdeaId::GiftEconomy)).unwrap();
    assert!(
        !s.discovered.is_empty(),
        "the first colony arrives free when trade opens"
    );

    // A colony is never undiscovered — it survives long play and the save file.
    run_days(&mut s, 300);
    assert!(!s.discovered.is_empty());
    let json = save_string(&s, 700_000);
    let loaded = load_string(&json, 700_000).unwrap();
    assert_eq!(
        loaded.state.discovered, s.discovered,
        "discovery is permanent"
    );
}

// =====================================================================
// Cross-wave seams: engine (SIM-ENG-021 REVISED), economy (SIM-ECO-028),
// curiosity (SIM-CUR-005), and save compatibility.
// =====================================================================

// @spec SIM-ENG-021
#[test]
fn the_global_layer_now_reaches_structure_raw_production() {
    // SIM-ENG-021 REVISED: the global layers (6–8) apply to ALL raw-tier material
    // production — structures and converters, not only jobs. A Catnip Patch's
    // output is multiplied by a built-and-fueled Dynamo Wheel's global ratio.
    //
    // CASCADE FLAG (implementer): this CHANGES prior behavior — produce() must
    // fold `global_mult` into the STRUCTURE loop for raw-tier flows, not only
    // the jobs loop. (Existing economy_depth.rs SIM-ECO-032 tests measure a
    // Gardener job and a mystic UnicornMeadow, and the mess/patch test builds no
    // global structure, so none asserts structure-exemption — but re-run them.)
    let mut plain = GameState::new(357);
    grant(&mut plain, StructureId::CatnipPatch, 1);
    off_boundary(&mut plain);
    let mut dynamo = plain.clone();
    grant(&mut dynamo, StructureId::DynamoWheel, 1);
    give(&mut dynamo, ResourceId::Dew, 10.0); // fuel it → global ratio 2%
    assert!((dynamo.effect(Effect::GlobalProductionRatio) - 0.02).abs() < 1e-9);

    run_ticks(&mut plain, 1);
    run_ticks(&mut dynamo, 1);
    let plain_gain = plain.stock(ResourceId::Catnip);
    let dynamo_gain = dynamo.stock(ResourceId::Catnip);
    assert!(plain_gain > 0.0);
    assert!(
        (dynamo_gain - plain_gain * 1.02).abs() < 1e-9,
        "the patch's raw output IS boosted by the global layer (previously job-only)"
    );
}

// @spec SIM-ECO-028
#[test]
fn the_structure_table_is_forty_with_the_two_cross_leaf_arches() {
    use ResourceId::{Cairns, Echoes, Moonstone, Rainbows};
    assert_eq!(StructureId::ALL.len(), 53, "40 structures, append-only");
    assert_eq!(StructureId::RainbowArch.ix(), 38);
    assert_eq!(StructureId::Sunstone.ix(), 39);
    let s = GameState::new(358);
    assert_eq!(
        s.structures, [0u32; 53],
        "a new meadow across the widened array"
    );

    // The cross-leaf structures carry costs/ramps here; effects live in mystic
    // (Rainbow Arch) and ceremonies (Sunstone) respectively.
    let d = content::structure_def(StructureId::RainbowArch);
    assert_eq!(d.base_costs, &[(Rainbows, 5000.0), (Cairns, 25.0)]);
    assert_eq!(d.ratio, 1.15);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::ParadeRoutes));

    let d = content::structure_def(StructureId::Sunstone);
    assert_eq!(
        d.base_costs,
        &[(Moonstone, 8.0), (Cairns, 50.0), (Echoes, 1.0)]
    );
    assert_eq!(d.ratio, 1.15);
    assert_eq!(d.gate, StructureGate::Idea(IdeaId::LongSunMasonry));
}

// @spec SIM-ECO-024
#[test]
fn the_two_new_structures_first_copies_are_reachable() {
    // Band-4 first-copy horizon. Every capped cost component of the first copy
    // must fit the maximum achievable cap (all cap structures at 20 + the Porch
    // kite ratchet). Rainbow Arch has no capped components (rainbows/cairns are
    // uncapped) AND its first copy is granted free (SIM-UNI-011) — trivially
    // reachable. Sunstone's echo cost is mystic-currency-gated and exempt like
    // other mystic-gated components; its only capped cost is moonstone (8).
    let mut world = GameState::new(359);
    for st in StructureId::ALL {
        grant(&mut world, st, 20);
    }
    world.kites_high_water = 1e9;

    for st in [StructureId::RainbowArch, StructureId::Sunstone] {
        let def = content::structure_def(st);
        for (r, base) in def.base_costs {
            let cap = world.cap(*r);
            if !cap.is_finite() {
                continue; // uncapped mystic/crafted components (rainbows, cairns, echoes)
            }
            assert!(
                *base <= cap,
                "{}: first copy needs {} {:?}, achievable cap {}",
                def.name,
                base,
                r,
                cap
            );
        }
    }
}

// @spec SIM-CUR-005
#[test]
fn the_idea_tree_grows_to_fifty_nine_with_the_two_wonder_ideas() {
    use ResourceId::Echoes;
    assert_eq!(IdeaId::ALL.len(), 59, "59 ideas, ids append-only");
    assert_eq!(IdeaId::LongSunMasonry as usize, 57);
    assert_eq!(IdeaId::SkywardSong as usize, 58);
    for i in [IdeaId::LongSunMasonry, IdeaId::SkywardSong] {
        assert!(!content::idea_def(i).name.is_empty());
        assert!(
            !content::idea_def(i).blurb.is_empty(),
            "{i:?} explains itself"
        );
    }

    // Costs and echo gate-crafts.
    let d = content::idea_def(IdeaId::LongSunMasonry);
    assert!((d.cost - 120000.0).abs() < 1e-9);
    assert_eq!(d.gate, &[(Echoes, 3.0)]);
    let d = content::idea_def(IdeaId::SkywardSong);
    assert!((d.cost - 180000.0).abs() < 1e-9);
    assert_eq!(d.gate, &[(Echoes, 8.0)]);

    // Long Sun Masonry needs BOTH Solar Devotion AND Skyward Longing.
    let prereqs = content::idea_def(IdeaId::LongSunMasonry).prereqs;
    assert_eq!(prereqs.len(), 2);
    assert!(prereqs.contains(&IdeaId::SolarDevotion));
    assert!(prereqs.contains(&IdeaId::SkywardLonging));

    let mut s = GameState::new(360);
    grant(&mut s, StructureId::CardboardBox, 1); // curiosity cap > 0
    s.ideas.insert(IdeaId::SolarDevotion);
    assert!(
        !s.idea_unlocked(IdeaId::LongSunMasonry),
        "one prerequisite is not enough"
    );
    s.ideas.insert(IdeaId::SkywardLonging);
    assert!(
        s.idea_unlocked(IdeaId::LongSunMasonry),
        "both prerequisites open it"
    );
}

// @spec SAVE-013
#[test]
fn v1_saves_still_load_with_wave_4_fields_defaulted() {
    let fixture = include_str!("fixtures/save_v1.json");
    let loaded = load_string(fixture, 1_751_700_000).expect("v1 fixture still loads");
    let st = &loaded.state;
    assert_eq!(st.ticks, 45, "the old meadow is preserved exactly");
    assert_eq!(
        st.structures.len(),
        53,
        "structures zero-filled forward to 53"
    );
    assert_eq!(st.structures[StructureId::CatnipPatch.ix()], 1);
    // Every wave-4 field gets its new-game default.
    assert_eq!(st.devotion, 0.0);
    assert_eq!(st.serenity, 0.0);
    assert_eq!(st.peak_devotion, 0.0);
    assert_eq!(st.enlightenment_tier, 0);
    assert_eq!(st.dream_drift, 0.0);
    assert!(st.rites.is_empty());
    assert!(st.arches_bought.is_empty());
    assert!(st.promises.is_empty());
    assert_eq!(st.penpal, [0u32; 8]);
    assert!(st.discovered.is_empty());
}

// @spec SAVE-013
#[test]
fn a_played_wonder_state_round_trips_losslessly() {
    let mut s = GameState::new(361);
    s.devotion = 123.5;
    s.serenity = 7.25;
    s.peak_devotion = 900.0;
    s.enlightenment_tier = 4;
    s.dream_drift = 0.42;
    s.rites.insert(RiteId::MorningStretch);
    s.rites.insert(RiteId::TheLongSun);
    s.arches_bought.insert(ArchId::RibbonArch);
    s.penpal[ColonyId::Foxes as usize] = 5;
    s.penpal[ColonyId::Otters as usize] = 2;
    s.discovered.insert(ColonyId::Otters);
    s.discovered.insert(ColonyId::Foxes);

    // now == saved_at → zero catch-up ticks, so the state persists verbatim.
    let json = save_string(&s, 800_000);
    let st = load_string(&json, 800_000).unwrap().state;
    assert!((st.devotion - 123.5).abs() < 1e-9);
    assert!((st.serenity - 7.25).abs() < 1e-9);
    assert!((st.peak_devotion - 900.0).abs() < 1e-9);
    assert_eq!(st.enlightenment_tier, 4);
    assert!((st.dream_drift - 0.42).abs() < 1e-9);
    assert_eq!(st.rites, s.rites);
    assert_eq!(st.arches_bought, s.arches_bought);
    assert_eq!(st.penpal, s.penpal);
    assert_eq!(st.discovered, s.discovered);
}
