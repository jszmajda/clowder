//! Depth Wave 5 tests: the legacy segments — prestige / the Great Meadow Move
//! (SIM-MOVE) and adventures (SIM-ADV) — plus the wave-5 seams in kittens
//! (SIM-KIT-011 REVISED, Warm Memories) and the engine (SIM-ENG-021 REVISED,
//! Elder Wisdom layer 7 and Long Sun layer 8 both energy-deficit-dimmed).
//! Written tests-first from the design/spec docs: they compile against the
//! Depth Wave 5 API contract and fail until the content lands.
//!
//! ---- Contract shapes this suite pins (for the implementer) ----
//!
//! New enums (Copy + Ord for set membership / `as usize` indexing, each `ALL`):
//!   * `TraditionId` (30, prestige shop-table order):
//!       CarefulPacking(0), TravelingBasketsI(1), TravelingBasketsII(2),
//!       TravelingBasketsIII(3), MemoryChestI(4), MemoryChestII(5),
//!       WarmWelcomeHome(6), ElderStories(7), GoldenRatio(8),
//!       GrandProportion(9), VitruvianCat(10), Renaissance(11),
//!       NineLivesI..NineLivesX(12..21), GoodNeighbours(22), WellRested(23),
//!       BrightEyed(24), FestivalSpirit(25), SecondWind(26), Forecaster(27),
//!       PrismSight(28), AdventurousSpirit(29).
//!   * `AdventureId` (7, catalogue order): QuietRetreat(0), LongWinter(1),
//!       NapTime(2), FrugalMeadow(3), SilentSun(4), GentlePaws(5),
//!       TidyMeadow(6).
//!
//! Content tables:
//!   * `content::tradition_def(TraditionId) -> &TraditionDef { cost_wisdom: f64, .. }`
//!   * `content::adventure_def(AdventureId) -> &AdventureDef { .. twist/goal/reward .. }`
//!
//! New `GameState` fields (all `pub`, all `#[serde(default)]`):
//!   elder_wisdom: f64, warm_memories: f64, passed_down_wisdom: f64,
//!   generation: u32, traditions: BTreeSet<TraditionId>,
//!   family_album: Vec<ArchivedKitten>, armed_adventure: Option<AdventureId>,
//!   active_adventure: Option<AdventureId>, adventure_completions: [u32; 7],
//!   plus a minimal adventure reward pool (shape is the implementer's business;
//!   this suite only observes rewards through `effect()` / behavior) and a
//!   Tidy-Meadow year timer (observed only through completions here).
//!   ArchivedKitten { name: String, trait_id: KittenTrait, rank: u8,
//!   generation: u32 } — slim, NOT a full Kitten (no skills/job).
//!
//! New commands: GreatMeadowMove, PassItDown, BuyTradition(TraditionId),
//!   DeclareAdventure(Option<AdventureId>).
//! New events: MeadowMoved { generation }, Passed { passed_down_wisdom },
//!   TraditionBought { id }, AdventureDeclared { id: Option<AdventureId> },
//!   AdventureCompleted { id }.
//! New Effect variants: GlobalPriceRatioDelta, KittenGrowthRatio,
//!   StandingRatio (global), MasterSkillMultiplier, EnergyConsumptionRatio,
//!   SeasonResourceRatio(Season, ResourceId).
//! Derived queries pinned: nine_lives_ratio(), basket_capacity(),
//!   memory_chest_ratio(). Elder Wisdom / Passed-Down production & storage are
//!   tested through observable production and cap(), never a named query.
#![allow(clippy::type_complexity, clippy::doc_overindented_list_items)]

mod common;

use clowder_core::*;
use common::*;

const TOL: f64 = 1e-9;

/// Step off the day boundary and quiet any zoomies so a single tick measures
/// exactly one production step (mirrors the helper in wonder.rs).
fn off_boundary(s: &mut GameState) {
    s.ticks += 1;
    s.zoomies_until = 0;
}

/// A plain Home kitten with a distinct name and seniority.
fn kitten(name: &str, arrived_day: u64) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: None,
        location: Location::Home,
        arrived_day,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
    }
}

/// A meadow with `n` uniquely-named kittens, seniority == push order
/// (kitten 0 is the most senior, arrived_day 0). Direct state construction, as
/// the other suites do — the Move needs only roster size, not housing.
fn meadow_with(seed: u64, n: usize) -> GameState {
    let mut s = GameState::new(seed);
    for i in 0..n {
        s.kittens.push(kitten(&format!("Kit{i:03}"), i as u64));
    }
    s
}

/// karma_gain(k) — the piecewise Warm Memories gain (SIM-MOVE-008).
fn karma_gain(k: f64) -> f64 {
    (k - 35.0)
        + 3.0 * (k - 60.0).max(0.0)
        + 4.0 * (k - 100.0).max(0.0)
        + 5.0 * (k - 150.0).max(0.0)
        + 10.0 * (k - 300.0).max(0.0)
        + 15.0 * (k - 750.0).max(0.0)
}

// =====================================================================
// SIM-MOVE-001 — the Move needs ≥71 kittens and archives the generation
// =====================================================================

// @spec SIM-MOVE-001
#[test]
fn a_move_needs_seventy_one_kittens_and_archives_the_whole_roster() {
    // 70 kittens: below the paragon threshold — rejected, nothing mutated.
    let mut small = meadow_with(401, 70);
    let before = small.clone();
    assert!(matches!(
        small.apply(Command::GreatMeadowMove).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    assert_eq!(small, before, "a rejected Move mutates nothing");
    assert!(small.family_album.is_empty());

    // 71 kittens — some Away: Away still counts toward the threshold and still
    // archives. Every current kitten lands in the Album, none is ever lost.
    let mut s = meadow_with(402, 71);
    s.kittens[3].location = Location::NeighborsYard;
    s.kittens[50].location = Location::NeighborsYard;
    let names: Vec<String> = s.kittens.iter().map(|k| k.name.clone()).collect();
    let album0 = s.family_album.len();
    let events = s.apply(Command::GreatMeadowMove).unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::MeadowMoved { .. })));
    assert_eq!(
        s.family_album.len(),
        album0 + 71,
        "the Album grows by the whole roster (Home and Away)"
    );
    for name in &names {
        assert!(
            s.family_album.iter().any(|a| &a.name == name),
            "no kitten name is ever lost from the Album: {name}"
        );
    }
    // A single generation stamps this whole batch.
    let gen = s.family_album[album0].generation;
    assert!(
        s.family_album[album0..].iter().all(|a| a.generation == gen),
        "one Move archives its roster under a single generation number"
    );
}

// @spec SIM-MOVE-001
#[test]
fn archived_kittens_are_slim_and_keep_name_trait_rank() {
    let mut s = meadow_with(403, 71);
    s.kittens[0].trait_id = KittenTrait::Bold;
    s.kittens[0].rank = 4;
    let events = s.apply(Command::GreatMeadowMove).unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::MeadowMoved { .. })));
    let bold = s
        .family_album
        .iter()
        .find(|a| a.name == "Kit000")
        .expect("the senior kitten is archived");
    // ArchivedKitten is slim: name, trait, rank, generation — no skills/job.
    assert_eq!(
        bold.trait_id,
        KittenTrait::Bold,
        "trait preserved in the Album"
    );
    assert_eq!(bold.rank, 4, "rank preserved in the Album");
}

// =====================================================================
// SIM-MOVE-002 — Traveling Baskets
// =====================================================================

// @spec SIM-MOVE-002
#[test]
fn traveling_baskets_carry_the_most_senior_kittens_normalized() {
    // Default basket_capacity is 0: a Move carries no kittens forward.
    let mut empty = meadow_with(404, 71);
    assert_eq!(empty.basket_capacity(), 0);
    empty.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        empty.kittens.is_empty(),
        "no baskets → the new meadow starts empty"
    );

    // Traveling Baskets I + II + III raise capacity to 1 + 2 + 4 = 7.
    let mut s = meadow_with(405, 71);
    s.traditions.insert(TraditionId::TravelingBasketsI);
    s.traditions.insert(TraditionId::TravelingBasketsII);
    s.traditions.insert(TraditionId::TravelingBasketsIII);
    assert_eq!(s.basket_capacity(), 7);
    // Each kitten gets a job, a seq stamp, and a preserved skill/rank; the 7
    // most-senior (lowest arrived_day) ride along.
    for (i, k) in s.kittens.iter_mut().enumerate() {
        k.job = Some(JobId::Gardener);
        k.job_seq = Some(i as u64);
        k.away_seq = Some(i as u64);
    }
    s.kittens[0].rank = 3;
    s.kittens[0].skills[JobId::Gardener as usize] = 500.0;
    let senior: Vec<String> = s.kittens.iter().take(7).map(|k| k.name.clone()).collect();
    s.apply(Command::GreatMeadowMove).unwrap();
    assert_eq!(s.kittens.len(), 7, "capacity kittens are carried");
    let carried: Vec<String> = s.kittens.iter().map(|k| k.name.clone()).collect();
    for name in &senior {
        assert!(carried.contains(name), "most-senior carried first: {name}");
    }
    // Every carried kitten is normalized to the fresh meadow (no Second Wind).
    for k in &s.kittens {
        assert_eq!(k.location, Location::Home, "carried kittens arrive Home");
        assert_eq!(
            k.arrived_day, 0,
            "arrived_day normalized to the fresh meadow"
        );
        assert_eq!(k.job, None, "job cleared without Second Wind");
        assert_eq!(k.job_seq, None, "job_seq cleared");
        assert_eq!(k.away_seq, None, "away_seq cleared");
    }
    // …but names, traits, ranks, and skills survive the trip.
    let veteran = s.kittens.iter().find(|k| k.name == "Kit000").unwrap();
    assert_eq!(veteran.rank, 3, "rank rides along in the basket");
    assert_eq!(
        veteran.skills[JobId::Gardener as usize],
        500.0,
        "skills ride along in the basket"
    );
}

// @spec SIM-MOVE-002
#[test]
fn second_wind_lets_exactly_one_carried_kitten_keep_its_job() {
    let mut s = meadow_with(406, 71);
    s.traditions.insert(TraditionId::TravelingBasketsII); // capacity 2
    s.traditions.insert(TraditionId::SecondWind);
    for (i, k) in s.kittens.iter_mut().enumerate() {
        k.job = Some(JobId::Gardener);
        k.job_seq = Some(i as u64);
    }
    s.apply(Command::GreatMeadowMove).unwrap();
    assert_eq!(s.kittens.len(), 2);
    let with_job = s.kittens.iter().filter(|k| k.job.is_some()).count();
    assert_eq!(
        with_job, 1,
        "Second Wind keeps exactly one carried kitten's job"
    );
}

// =====================================================================
// SIM-MOVE-003 — the Memory Chest
// =====================================================================

// @spec SIM-MOVE-003
#[test]
fn the_memory_chest_carries_a_fraction_of_each_stock() {
    // No chest traditions, no chest structures: ratio 0, nothing carried.
    let bare = meadow_with(407, 71);
    assert!((bare.memory_chest_ratio() - 0.0).abs() < TOL);

    // memory_chest_ratio = traditions (I +1.5%, II +3%) + 1.5% per Memory Chest
    // structure owned in the OLD meadow (read before the reset).
    let mut s = meadow_with(408, 71);
    s.traditions.insert(TraditionId::MemoryChestI);
    s.traditions.insert(TraditionId::MemoryChestII);
    grant(&mut s, StructureId::MemoryChest, 2);
    let ratio = 0.015 + 0.03 + 0.015 * 2.0; // 0.075
    assert!((s.memory_chest_ratio() - ratio).abs() < TOL);
    // Carry uncapped crafted stocks so nothing clamps on the fresh meadow.
    give(&mut s, ResourceId::Wands, 1000.0);
    give(&mut s, ResourceId::Brass, 400.0);
    s.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        (s.stock(ResourceId::Wands) - 1000.0 * ratio).abs() < 1e-6,
        "the Memory Chest carries ratio × each stock"
    );
    assert!((s.stock(ResourceId::Brass) - 400.0 * ratio).abs() < 1e-6);
    // The Memory Chest structures themselves reset (the ratio was read first).
    assert_eq!(s.structures[StructureId::MemoryChest.ix()], 0);
}

/// A slim Album entry for pre-seeding the Family Album.
fn archived(name: &str, generation: u32) -> ArchivedKitten {
    ArchivedKitten {
        name: name.to_string(),
        trait_id: KittenTrait::None,
        rank: 0,
        generation,
    }
}

// =====================================================================
// SIM-MOVE-004 — exact persist / reset
// =====================================================================

// @spec SIM-MOVE-004
#[test]
fn a_move_preserves_the_legacy_set_and_resets_everything_else() {
    let mut s = meadow_with(409, 71);
    // ---- Legacies to PRESERVE ----
    s.elder_wisdom = 12.0;
    s.warm_memories = 34.0;
    s.passed_down_wisdom = 5.0;
    s.serenity = 7.0;
    s.enlightenment_tier = 3;
    s.traditions.insert(TraditionId::CarefulPacking);
    s.traditions.insert(TraditionId::WellRested);
    s.family_album.push(archived("Ancestor", 0));
    s.active_adventure = Some(AdventureId::LongWinter);
    s.speed = 2.0;
    let gen0 = s.generation;
    // ---- Meadow state to RESET ----
    grant(&mut s, StructureId::CatnipPatch, 5);
    s.ideas.insert(IdeaId::PantryOrganization);
    s.refinements.insert(RefinementId::PebbleCombs);
    s.rules.insert(RuleId::EarlyBirds);
    s.devotion = 500.0;
    s.peak_devotion = 900.0;
    s.rites.insert(RiteId::MorningStretch);
    s.arches_bought.insert(ArchId::RibbonArch);
    s.dream_drift = 0.5;
    s.promises.push(Promise { quiet: false });
    give(&mut s, ResourceId::Rainbows, 100.0);
    give(&mut s, ResourceId::Prisms, 50.0);
    s.discovered.insert(ColonyId::Otters);
    s.penpal[ColonyId::Otters.ix()] = 3;
    s.elder = Some(0);
    s.mess = 50_000.0;
    s.pity = 3;
    s.famine_days = 2;
    s.recovery_days = 1;
    s.next_seq = 99;
    s.zoomies_until = 999;
    s.jamboree_until = 888;
    s.kites_high_water = 50.0;
    s.pending_sky_event = Some(100);
    s.ticks = 12_345;

    s.apply(Command::GreatMeadowMove).unwrap();

    // ---- Preserved ----
    assert!(
        s.elder_wisdom >= 12.0,
        "Elder Wisdom persists (and grows) — never reset"
    );
    assert!(s.warm_memories >= 34.0, "Warm Memories persists");
    assert!(
        (s.passed_down_wisdom - 5.0).abs() < TOL,
        "Passed-Down Wisdom persists"
    );
    assert!((s.serenity - 7.0).abs() < TOL, "serenity persists");
    assert_eq!(s.enlightenment_tier, 3, "enlightenment tier persists");
    assert!(s.traditions.contains(&TraditionId::CarefulPacking));
    assert!(
        s.traditions.contains(&TraditionId::WellRested),
        "traditions persist"
    );
    assert!(
        s.family_album.iter().any(|a| a.name == "Ancestor"),
        "the Family Album keeps its prior entries"
    );
    assert_eq!(
        s.active_adventure,
        Some(AdventureId::LongWinter),
        "active adventure persists"
    );
    assert_eq!(s.speed, 2.0, "the game-speed setting persists");
    assert_eq!(
        s.generation,
        gen0 + 1,
        "generation increments by exactly one"
    );

    // ---- Reset to new-game defaults ----
    assert!(s.structures.iter().all(|&c| c == 0), "structures reset");
    assert!(s.ideas.is_empty(), "ideas reset");
    assert!(s.refinements.is_empty(), "refinements reset");
    assert!(s.rules.is_empty(), "house rules reset");
    assert_eq!(s.devotion, 0.0, "devotion resets");
    assert_eq!(s.peak_devotion, 0.0, "peak devotion resets");
    assert!(s.rites.is_empty(), "rites reset");
    assert!(s.arches_bought.is_empty(), "the mystic chain resets");
    assert_eq!(s.dream_drift, 0.0, "dream drift resets");
    assert!(s.promises.is_empty(), "Promises reset");
    assert!(s.discovered.is_empty(), "colony discovery resets");
    assert_eq!(s.penpal, [0u32; 8], "Pen Pals reset");
    assert!(
        s.resources.iter().all(|&x| x == 0.0),
        "resources reset (no Memory Chest bought → nothing carried)"
    );
    assert_eq!(s.elder, None, "the Elder appointment resets");
    assert_eq!(s.mess, 0.0, "mess resets");
    assert_eq!(s.pity, 0, "pity clock resets");
    assert_eq!(s.famine_days, 0, "famine clock resets");
    assert_eq!(s.recovery_days, 0, "recovery clock resets");
    assert_eq!(s.next_seq, 0, "the seq counter resets");
    assert_eq!(s.zoomies_until, 0, "zoomies reset");
    assert_eq!(s.jamboree_until, 0, "jamboree resets");
    assert_eq!(s.kites_high_water, 0.0, "the kite high-water resets");
    assert_eq!(s.pending_sky_event, None, "the pending sky event resets");
    assert_eq!(s.ticks, 0, "the tick counter resets");
}

// @spec SIM-MOVE-004
#[test]
fn a_move_never_reseeds_the_rng_and_rebuilds_the_reveal_worklists() {
    // The RNG continues across a Move — the Move itself draws no randomness.
    let mut s = meadow_with(410, 71);
    s.revealed
        .insert(ContentId::Structure(StructureId::CatnipPatch));
    let rng_before = s.rng.clone();
    s.apply(Command::GreatMeadowMove).unwrap();
    assert_eq!(s.rng, rng_before, "the RNG is carried over, never reseeded");

    // Two states that Move identically stay bit-identical afterward.
    let a = meadow_with(411, 71);
    let b = meadow_with(411, 71);
    let mut a = a;
    let mut b = b;
    a.apply(Command::GreatMeadowMove).unwrap();
    b.apply(Command::GreatMeadowMove).unwrap();
    assert_eq!(
        save_string(&a, 0),
        save_string(&b, 0),
        "the Move is deterministic"
    );

    // The reveal worklists are rebuilt from the emptied `revealed` set: a fresh
    // meadow still reveals content on a tick.
    assert!(
        s.revealed.is_empty(),
        "the reveal set is emptied by the Move"
    );
    give(&mut s, ResourceId::Catnip, 100.0);
    run_ticks(&mut s, 1);
    assert!(
        !s.revealed.is_empty(),
        "a post-Move meadow can still reveal (worklists were rebuilt)"
    );
}

// =====================================================================
// SIM-MOVE-005 — a Move never decreases a legacy; the Album only grows
// =====================================================================

// @spec SIM-MOVE-005
#[test]
fn a_move_never_decreases_a_legacy_and_the_album_only_grows() {
    let mut s = meadow_with(412, 71);
    s.elder_wisdom = 20.0;
    s.warm_memories = 40.0;
    s.passed_down_wisdom = 8.0;

    let ew0 = s.elder_wisdom;
    let wm0 = s.warm_memories;
    let pd0 = s.passed_down_wisdom;
    let album0 = s.family_album.len();
    s.apply(Command::GreatMeadowMove).unwrap();
    assert!(s.elder_wisdom >= ew0, "Elder Wisdom never falls on a Move");
    assert!(
        s.warm_memories >= wm0,
        "Warm Memories never falls on a Move"
    );
    assert!(
        (s.passed_down_wisdom - pd0).abs() < TOL,
        "Passed-Down Wisdom untouched by a Move"
    );
    assert_eq!(s.family_album.len(), album0 + 71, "the Album grew");

    // Re-populate and Move again: still monotone, the Album still only grows.
    let ew1 = s.elder_wisdom;
    let wm1 = s.warm_memories;
    let album1 = s.family_album.len();
    for i in 0..71 {
        s.kittens.push(kitten(&format!("New{i:03}"), i as u64));
    }
    s.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        s.elder_wisdom >= ew1,
        "legacies stay monotone across many Moves"
    );
    assert!(s.warm_memories >= wm1);
    assert_eq!(
        s.family_album.len(),
        album1 + 71,
        "the Album is append-only"
    );
}

// =====================================================================
// SIM-MOVE-006 — Elder Wisdom earned on a Move
// =====================================================================

// @spec SIM-MOVE-006
#[test]
fn a_move_adds_kittens_minus_seventy_plus_millennia_to_elder_wisdom() {
    // First Move at exactly 71, at game start: += max(0, 71 − 70) = +1, no
    // millennia elapsed.
    let mut s = meadow_with(413, 71);
    let ew0 = s.elder_wisdom;
    s.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        (s.elder_wisdom - (ew0 + 1.0)).abs() < TOL,
        "71 kittens → +1 Elder Wisdom"
    );

    // A bigger clowder: += max(0, k − 70).
    let mut big = meadow_with(414, 85);
    let ew0 = big.elder_wisdom;
    big.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        (big.elder_wisdom - (ew0 + 15.0)).abs() < TOL,
        "85 kittens → +15 Elder Wisdom"
    );

    // The slow trickle: +1 per game-millennium (1000 game-years) since the last
    // Move. ticks = 2_000_000 → day 200_000 → year 2000 → 2 millennia. (ASSUMES
    // a game-millennium is 1000 years — see the report's ambiguities.)
    let mut old = meadow_with(415, 71);
    old.ticks = 2_000_000;
    let ew0 = old.elder_wisdom;
    old.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        (old.elder_wisdom - (ew0 + 1.0 + 2.0)).abs() < TOL,
        "+1 (size) +2 (two millennia elapsed)"
    );
}

// =====================================================================
// SIM-MOVE-007 — Elder Wisdom production & storage contributions
// =====================================================================

// @spec SIM-MOVE-007
#[test]
fn elder_wisdom_boosts_production_capped_by_nine_lives_and_storage() {
    // Layer 7: +1% production per point, limited_dr-capped at 2.0 ×
    // nine_lives_ratio. Measure a Catnip Patch (structure raw production).
    let mut plain = GameState::new(416);
    grant(&mut plain, StructureId::CatnipPatch, 1);
    off_boundary(&mut plain);
    let mut base = plain.clone();
    run_ticks(&mut base, 1);
    let p = base.stock(ResourceId::Catnip);
    assert!(p > 0.0);

    // elder_wisdom 100 → 0.01×100 = 1.0, below the 0.75×2.0 knee → +100%.
    let mut wise = plain.clone();
    wise.elder_wisdom = 100.0;
    run_ticks(&mut wise, 1);
    assert!(
        (wise.stock(ResourceId::Catnip) - p * 2.0).abs() < 1e-9,
        "+1% production per Elder Wisdom point"
    );

    // The cap is 2.0 × nine_lives_ratio. elder_wisdom 500 → 0.01×500 = 5.0,
    // deep in the asymptotic regime, so the cap governs.
    let mut capped = plain.clone();
    capped.elder_wisdom = 500.0;
    run_ticks(&mut capped, 1);
    let bonus0 = limited_dr(5.0, 2.0 * 1.0);
    assert!((capped.stock(ResourceId::Catnip) - p * (1.0 + bonus0)).abs() < 1e-9);

    // Nine Lives raises the cap (and the ratio query): I → ratio 1.05, cap 2.1.
    let mut nine = plain.clone();
    nine.elder_wisdom = 500.0;
    nine.traditions.insert(TraditionId::NineLivesI);
    assert!((nine.nine_lives_ratio() - 1.05).abs() < TOL);
    run_ticks(&mut nine, 1);
    let bonus9 = limited_dr(5.0, 2.0 * 1.05);
    assert!((nine.stock(ResourceId::Catnip) - p * (1.0 + bonus9)).abs() < 1e-9);
    assert!(
        nine.stock(ResourceId::Catnip) > capped.stock(ResourceId::Catnip),
        "Nine Lives lifts the capped Elder-Wisdom bonus"
    );

    // Storage: +0.1% per point (linear). Catnip base cap 200 → ×1.1 at 100 pts.
    let mut store = GameState::new(417);
    let base_cap = store.cap(ResourceId::Catnip);
    store.elder_wisdom = 100.0;
    assert!(
        (store.cap(ResourceId::Catnip) - base_cap * 1.1).abs() < 1e-6,
        "+0.1% storage per Elder Wisdom point"
    );
}

// =====================================================================
// SIM-MOVE-008 — Warm Memories earned on a Move; happiness legacy
// =====================================================================

// @spec SIM-MOVE-008
#[test]
fn a_move_accumulates_warm_memories_by_the_karma_formula() {
    // 71 kittens → karma_gain(71) = (71−35) + 3·11 = 69.
    let mut s = meadow_with(418, 71);
    s.warm_memories = 10.0;
    s.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        (s.warm_memories - (10.0 + karma_gain(71.0))).abs() < TOL,
        "Warm Memories += karma_gain(k), accumulated"
    );

    // A bigger clowder pays the steeper piecewise tiers.
    let mut big = meadow_with(419, 105);
    let wm0 = big.warm_memories;
    big.apply(Command::GreatMeadowMove).unwrap();
    assert!((big.warm_memories - (wm0 + karma_gain(105.0))).abs() < TOL);
}

// @spec SIM-MOVE-008, SIM-KIT-011
#[test]
fn warm_memories_add_sqrt_diminishing_happiness() {
    // One loafing Home kitten: base happiness = 100 + 1 loafer point = 101.
    let mut s = GameState::new(420);
    s.kittens.push(kitten("Solo", 0));
    let base = s.happiness();
    assert!((base - 101.0).abs() < TOL);

    // warm_memory = unlimited_dr(Warm Memories, 5) adds +1 happiness point each.
    s.warm_memories = 45.0;
    let expected = base + unlimited_dr(45.0, 5.0);
    assert!(
        (s.happiness() - expected).abs() < TOL,
        "Warm Memories brighten the meadow (SIM-KIT-011 term)"
    );
}

// =====================================================================
// SIM-MOVE-009 — Pass It Down (Elder Wisdom → Passed-Down Wisdom)
// =====================================================================

// @spec SIM-MOVE-009
#[test]
fn pass_it_down_converts_all_elder_wisdom_one_way() {
    let mut s = GameState::new(421);
    s.elder_wisdom = 30.0;
    s.passed_down_wisdom = 5.0;
    let events = s.apply(Command::PassItDown).unwrap();
    assert_eq!(s.elder_wisdom, 0.0, "all Elder Wisdom is converted");
    assert!(
        (s.passed_down_wisdom - 35.0).abs() < TOL,
        "…into Passed-Down Wisdom"
    );
    assert!(events.iter().any(
        |e| matches!(e, Event::Passed { passed_down_wisdom } if (*passed_down_wisdom - 35.0).abs() < TOL)
    ));
}

// @spec SIM-MOVE-009
#[test]
fn passed_down_wisdom_is_a_second_capped_production_and_storage_pool() {
    let mut plain = GameState::new(422);
    grant(&mut plain, StructureId::CatnipPatch, 1);
    off_boundary(&mut plain);
    let mut base = plain.clone();
    run_ticks(&mut base, 1);
    let p = base.stock(ResourceId::Catnip);

    // +1% per point, capped at +100%. passed_down 50 → limited_dr(0.5, 1.0) = 0.5.
    let mut second = plain.clone();
    second.passed_down_wisdom = 50.0;
    run_ticks(&mut second, 1);
    assert!((second.stock(ResourceId::Catnip) - p * 1.5).abs() < 1e-9);

    // …and it is a SECOND pool, capped at +100% (never runs away).
    let mut deep = plain.clone();
    deep.passed_down_wisdom = 100_000.0;
    run_ticks(&mut deep, 1);
    assert!(
        deep.stock(ResourceId::Catnip) < p * 2.0,
        "the +100% cap holds however deep the wisdom"
    );

    // Storage: +0.05% per point. Catnip base cap 200 → ×1.05 at 100 points.
    let mut store = GameState::new(423);
    let base_cap = store.cap(ResourceId::Catnip);
    store.passed_down_wisdom = 100.0;
    assert!((store.cap(ResourceId::Catnip) - base_cap * 1.05).abs() < 1e-6);
}

// =====================================================================
// SIM-MOVE-010 / 011 / 012 — the Traditions shop
// =====================================================================

// @spec SIM-MOVE-010
#[test]
fn the_traditions_table_is_thirty_in_shop_order_with_wisdom_costs() {
    assert_eq!(
        TraditionId::ALL.len(),
        30,
        "exactly 30 traditions, append-only"
    );
    // Shop-table ordering (indices are load-bearing: the Nine Lives ladder and
    // the derived nine_lives_ratio both count owned traditions).
    assert_eq!(TraditionId::CarefulPacking as usize, 0);
    assert_eq!(TraditionId::MemoryChestI as usize, 4);
    assert_eq!(TraditionId::Renaissance as usize, 11);
    assert_eq!(TraditionId::NineLivesI as usize, 12);
    assert_eq!(TraditionId::NineLivesX as usize, 21);
    assert_eq!(TraditionId::GoodNeighbours as usize, 22);
    assert_eq!(TraditionId::AdventurousSpirit as usize, 29);
    // Elder-Wisdom costs from the shop table.
    let cost = |t| content::tradition_def(t).cost_wisdom;
    assert!((cost(TraditionId::CarefulPacking) - 5.0).abs() < TOL);
    assert!((cost(TraditionId::TravelingBasketsIII) - 300.0).abs() < TOL);
    assert!((cost(TraditionId::MemoryChestII) - 100.0).abs() < TOL);
    assert!((cost(TraditionId::Renaissance) - 750.0).abs() < TOL);
    assert!((cost(TraditionId::NineLivesX) - 60_000.0).abs() < TOL);
    assert!((cost(TraditionId::AdventurousSpirit) - 40.0).abs() < TOL);
}

// @spec SIM-MOVE-010, SIM-MOVE-012
#[test]
fn buy_tradition_spends_wisdom_once_each_and_survives_a_move() {
    let mut s = GameState::new(424);
    s.elder_wisdom = 10.0;
    let events = s
        .apply(Command::BuyTradition(TraditionId::CarefulPacking))
        .unwrap();
    assert!(
        (s.elder_wisdom - 5.0).abs() < TOL,
        "Elder Wisdom is spent (cost 5)"
    );
    assert!(s.traditions.contains(&TraditionId::CarefulPacking));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::TraditionBought { id } if *id == TraditionId::CarefulPacking)));

    // Once each: a second buy is refused and spends nothing more.
    assert_eq!(
        s.apply(Command::BuyTradition(TraditionId::CarefulPacking))
            .unwrap_err(),
        CommandError::AlreadyResearched
    );
    assert!((s.elder_wisdom - 5.0).abs() < TOL, "no double-spend");

    // Unaffordable → InsufficientResources, all-or-nothing.
    let mut poor = GameState::new(425);
    poor.elder_wisdom = 2.0;
    let before = poor.clone();
    assert!(matches!(
        poor.apply(Command::BuyTradition(TraditionId::CarefulPacking))
            .unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    assert_eq!(poor, before, "a rejected buy mutates nothing");

    // Permanent across a Move: build a clowder, keep the tradition through it.
    let mut moved = meadow_with(426, 71);
    moved.traditions.insert(TraditionId::CarefulPacking);
    moved.apply(Command::GreatMeadowMove).unwrap();
    assert!(
        moved.traditions.contains(&TraditionId::CarefulPacking),
        "no Tradition is ever un-bought or reset by a Move"
    );
}

// @spec SIM-MOVE-010, SIM-MOVE-011, SIM-MOVE-012
#[test]
fn tradition_effects_flow_through_the_effect_system() {
    // Careful Packing → a −1% global price delta (consumed in next_cost).
    let mut s = GameState::new(427);
    s.traditions.insert(TraditionId::CarefulPacking);
    assert!((s.effect(Effect::GlobalPriceRatioDelta) - 0.01).abs() < TOL);

    // Traveling Baskets I → basket_capacity +1.
    let mut s = GameState::new(428);
    assert_eq!(s.basket_capacity(), 0);
    s.traditions.insert(TraditionId::TravelingBasketsI);
    assert_eq!(s.basket_capacity(), 1);

    // Nine Lives → +0.05 to nine_lives_ratio each (default 1.0).
    let mut s = GameState::new(429);
    assert!((s.nine_lives_ratio() - 1.0).abs() < TOL);
    s.traditions.insert(TraditionId::NineLivesI);
    s.traditions.insert(TraditionId::NineLivesII);
    assert!((s.nine_lives_ratio() - 1.10).abs() < TOL);

    // Well-Rested → kitten arrival ×1.75 (a +75% KittenGrowthRatio).
    let mut s = GameState::new(430);
    give(&mut s, ResourceId::Catnip, 50.0);
    let base = s.arrival_chance();
    assert!((base - 0.15).abs() < TOL);
    s.traditions.insert(TraditionId::WellRested);
    assert!((s.effect(Effect::KittenGrowthRatio) - 0.75).abs() < TOL);
    assert!(
        (s.arrival_chance() - base * 1.75).abs() < TOL,
        "arrival speed +75% multiplies the arrival chance"
    );

    // Good Neighbours → +10% trade standing everywhere (a global StandingRatio).
    let plain = GameState::new(431);
    let base_standing = plain.standing(ColonyId::Otters);
    let mut s = GameState::new(432);
    s.traditions.insert(TraditionId::GoodNeighbours);
    assert!((s.effect(Effect::StandingRatio) - 0.10).abs() < TOL);
    assert!(
        s.standing(ColonyId::Otters) > base_standing,
        "Good Neighbours warms standing everywhere"
    );
}

// =====================================================================
// SIM-MOVE-013 — GlobalPriceRatioDelta is DR-clamped in next_cost
// =====================================================================

// @spec SIM-MOVE-013
#[test]
fn stacked_price_reductions_are_dr_clamped_and_never_reach_zero() {
    // The five structure-price-reduction traditions and their deltas.
    let mut s = GameState::new(433);
    for (t, _) in [
        (TraditionId::CarefulPacking, 0.01),
        (TraditionId::GoldenRatio, 0.016),
        (TraditionId::GrandProportion, 0.018),
        (TraditionId::VitruvianCat, 0.02),
        (TraditionId::Renaissance, 0.0225),
    ] {
        s.traditions.insert(t);
    }
    let sum = 0.01 + 0.016 + 0.018 + 0.02 + 0.0225; // 0.0865
    assert!(
        (s.effect(Effect::GlobalPriceRatioDelta) - sum).abs() < TOL,
        "the price deltas sum in the effect system"
    );

    // cost = base × ratio^owned × (1 − limited_dr(price_delta, 0.5)).
    let plain = GameState::new(434);
    let c0 = plain.next_cost(StructureId::HerbGarden)[0].1; // 60 pebbles, ceil'd
    let c1 = s.next_cost(StructureId::HerbGarden)[0].1;
    let mult = 1.0 - limited_dr(sum, 0.5);
    assert!(c1 < c0, "stacked reductions lower the price");
    assert!(
        (c1 - c0 * mult).abs() < 1.0,
        "next_cost applies (1 − limited_dr(delta, 0.5)) within ceil rounding"
    );

    // The DR clamp guarantees the multiplier can never reach zero — even an
    // absurd stack keeps a price above half of its raw value.
    assert!(
        1.0 - limited_dr(1.0e9, 0.5) > 0.5,
        "limited_dr(_, 0.5) < 0.5 always, so the price multiplier stays > 0"
    );
}

// =====================================================================
// SIM-ADV-001 — declare / arm / disarm
// =====================================================================

// @spec SIM-ADV-001
#[test]
fn declare_adventure_arms_only_once_the_surface_is_unlocked() {
    // Before the surface is unlocked: Locked.
    let mut s = GameState::new(435);
    assert_eq!(
        s.apply(Command::DeclareAdventure(Some(AdventureId::LongWinter)))
            .unwrap_err(),
        CommandError::Locked
    );

    // The Adventurous Spirit tradition opens the surface.
    let mut s = GameState::new(436);
    s.traditions.insert(TraditionId::AdventurousSpirit);
    let events = s
        .apply(Command::DeclareAdventure(Some(AdventureId::LongWinter)))
        .unwrap();
    assert_eq!(s.armed_adventure, Some(AdventureId::LongWinter));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::AdventureDeclared {
            id: Some(AdventureId::LongWinter)
        }
    )));
    // Re-declaring replaces the armed adventure.
    s.apply(Command::DeclareAdventure(Some(AdventureId::NapTime)))
        .unwrap();
    assert_eq!(
        s.armed_adventure,
        Some(AdventureId::NapTime),
        "re-declare replaces"
    );
    // None disarms.
    s.apply(Command::DeclareAdventure(None)).unwrap();
    assert_eq!(s.armed_adventure, None, "None disarms");

    // The same-named curiosity idea also opens the surface.
    let mut s = GameState::new(437);
    s.ideas.insert(IdeaId::AdventurousSpirit);
    s.apply(Command::DeclareAdventure(Some(AdventureId::QuietRetreat)))
        .unwrap();
    assert_eq!(s.armed_adventure, Some(AdventureId::QuietRetreat));
}

// =====================================================================
// SIM-ADV-002 — armed → active on a Move; the twists
// =====================================================================

// @spec SIM-ADV-002
#[test]
fn a_move_activates_the_armed_adventure_and_long_winter_forces_winter() {
    let mut s = meadow_with(438, 71);
    s.traditions.insert(TraditionId::AdventurousSpirit);
    s.apply(Command::DeclareAdventure(Some(AdventureId::LongWinter)))
        .unwrap();
    s.apply(Command::GreatMeadowMove).unwrap();
    assert_eq!(
        s.active_adventure,
        Some(AdventureId::LongWinter),
        "armed → active"
    );
    assert_eq!(s.armed_adventure, None, "the armed slot is cleared");
    // Long Winter forces season() to Winter regardless of the tick counter.
    assert_eq!(
        s.season(),
        Season::Winter,
        "a fresh (Spring) meadow reads Winter"
    );
    s.ticks = 300; // day 30 → would be Summer
    assert_eq!(
        s.season(),
        Season::Winter,
        "…still Winter, whatever the ticks say"
    );
}

// @spec SIM-ADV-002
#[test]
fn nap_time_halves_job_production_but_not_structures() {
    let mut base = GameState::new(439);
    base.kittens.push(kitten("Napper", 0));
    base.kittens[0].job = Some(JobId::TwigCollector);
    base.kittens[0].job_seq = Some(0);
    grant(&mut base, StructureId::CardboardBox, 1); // a structure (Curiosity)
    give(&mut base, ResourceId::Catnip, 100.0); // no shortfall → mood 1.0
    off_boundary(&mut base);
    let mut nap = base.clone();
    nap.active_adventure = Some(AdventureId::NapTime);
    run_ticks(&mut base, 1);
    run_ticks(&mut nap, 1);

    let bt = base.stock(ResourceId::Twigs);
    assert!(bt > 0.0);
    assert!(
        (nap.stock(ResourceId::Twigs) - bt * 0.5).abs() < 1e-9,
        "Nap Time halves job (kitten) production"
    );
    assert!(
        (nap.stock(ResourceId::Curiosity) - base.stock(ResourceId::Curiosity)).abs() < 1e-9,
        "structure production is untouched by Nap Time"
    );
}

// @spec SIM-ADV-002
#[test]
fn silent_sun_and_quiet_retreat_remove_options() {
    // Silent Sun makes ceremony commands Locked.
    let mut s = GameState::new(440);
    s.active_adventure = Some(AdventureId::SilentSun);
    give(&mut s, ResourceId::Warmth, 100.0);
    assert_eq!(
        s.apply(Command::Bask).unwrap_err(),
        CommandError::Locked,
        "Bask is Locked"
    );
    s.peak_devotion = 500.0; // gate met, so only the twist can block the rite
    give(&mut s, ResourceId::Shinies, 500.0);
    assert_eq!(
        s.apply(Command::BuyRite(RiteId::MorningStretch))
            .unwrap_err(),
        CommandError::Locked,
        "BuyRite is Locked under Silent Sun"
    );

    // Quiet Retreat blocks jamborees.
    let mut s = GameState::new(441);
    s.ideas.insert(IdeaId::TeaCeremonies);
    give(&mut s, ResourceId::Pep, 800.0);
    give(&mut s, ResourceId::Charm, 1500.0);
    give(&mut s, ResourceId::Quills, 40.0);
    s.active_adventure = Some(AdventureId::QuietRetreat);
    assert_eq!(
        s.apply(Command::HostJamboree).unwrap_err(),
        CommandError::Locked
    );
}

// @spec SIM-ADV-002
#[test]
fn gentle_paws_zeroes_pounce_yields() {
    let mut s = GameState::new(442);
    s.ideas.insert(IdeaId::PouncingPractice);
    give(&mut s, ResourceId::Pep, 500.0);
    s.active_adventure = Some(AdventureId::GentlePaws);
    let events = s.apply(Command::Pounce(5)).unwrap();
    let ret = events
        .iter()
        .find_map(|e| match e {
            Event::PounceReturned {
                feathers,
                bells,
                shinies,
                unicorns,
                ..
            } => Some((*feathers, *bells, *shinies, *unicorns)),
            _ => None,
        })
        .expect("a pounce returns");
    assert!(ret.0.abs() < TOL, "no feathers");
    assert!(ret.1.abs() < TOL, "no bells");
    assert!(ret.2.abs() < TOL, "no shinies");
    assert_eq!(
        ret.3, 0,
        "no unicorns — pounces yield nothing under Gentle Paws"
    );
}

// =====================================================================
// SIM-ADV-003 — the catalogue; twists never delete the player's things
// =====================================================================

// @spec SIM-ADV-003
#[test]
fn seven_adventures_are_defined_and_no_twist_deletes_the_roster() {
    assert_eq!(AdventureId::ALL.len(), 7, "exactly 7 adventures");
    for a in AdventureId::ALL {
        let _ = content::adventure_def(a); // each is defined
    }

    // Declaring + Moving with an adventure armed drops nothing beyond the
    // ordinary Move: the carried roster is identical with or without a twist.
    let mut plain = meadow_with(443, 71);
    plain.traditions.insert(TraditionId::TravelingBasketsIII); // capacity 4
    plain.traditions.insert(TraditionId::AdventurousSpirit);
    let mut twisted = plain.clone();
    twisted
        .apply(Command::DeclareAdventure(Some(AdventureId::LongWinter)))
        .unwrap();
    plain.apply(Command::GreatMeadowMove).unwrap();
    twisted.apply(Command::GreatMeadowMove).unwrap();
    let plain_roster: Vec<String> = plain.kittens.iter().map(|k| k.name.clone()).collect();
    let twisted_roster: Vec<String> = twisted.kittens.iter().map(|k| k.name.clone()).collect();
    assert_eq!(plain_roster.len(), 4);
    assert_eq!(
        plain_roster, twisted_roster,
        "an armed adventure never costs the player kittens"
    );
}

// =====================================================================
// SIM-ADV-004 — goal met → completed; never-completed = no penalty
// =====================================================================

// @spec SIM-ADV-004
#[test]
fn an_active_adventure_completes_when_its_goal_is_met() {
    // Nap Time's goal is to build a Yarn Computer.
    let mut s = GameState::new(444);
    s.active_adventure = Some(AdventureId::NapTime);
    assert_eq!(s.adventure_completions[AdventureId::NapTime as usize], 0);
    assert!(
        (s.effect(Effect::MasterSkillMultiplier) - 0.0).abs() < TOL,
        "no reward yet"
    );
    grant(&mut s, StructureId::YarnComputer, 1); // the goal is met
    let events = run_ticks(&mut s, 15); // cross a day boundary
    assert_eq!(
        s.adventure_completions[AdventureId::NapTime as usize],
        1,
        "the goal is completed exactly once"
    );
    assert!(events.iter().any(|e| matches!(
        e,
        Event::AdventureCompleted {
            id: AdventureId::NapTime
        }
    )));
    // The reward pool grows and flows through the effect system (+0.2 per run).
    assert!(
        (s.effect(Effect::MasterSkillMultiplier) - 0.2).abs() < TOL,
        "one completion → +0.2 master-skill multiplier reward"
    );
}

// @spec SIM-ADV-004
#[test]
fn a_never_completed_adventure_carries_no_penalty() {
    // Quiet Retreat's goal is 50 kittens; with only 3 it never completes.
    let mut s = GameState::new(445);
    for i in 0..3 {
        s.kittens.push(kitten(&format!("Q{i}"), i as u64));
    }
    give(&mut s, ResourceId::Catnip, 200.0);
    s.active_adventure = Some(AdventureId::QuietRetreat);
    let roster0 = s.kittens.len();
    run_ticks(&mut s, 30);
    assert_eq!(
        s.adventure_completions[AdventureId::QuietRetreat as usize],
        0,
        "an unmet goal never completes"
    );
    assert_eq!(s.kittens.len(), roster0, "no penalty: the roster is intact");
    assert_eq!(
        s.active_adventure,
        Some(AdventureId::QuietRetreat),
        "it just stays active"
    );
}

// @spec SIM-ADV-004
#[test]
fn tidy_meadow_completes_after_a_full_year_of_zero_mess() {
    // A fresh meadow has no mess sources: mess stays 0. Tidy Meadow's goal is a
    // full year (100 days = 1000 ticks) of zero mess.
    let mut clean = GameState::new(446);
    clean.active_adventure = Some(AdventureId::TidyMeadow);
    run_ticks(&mut clean, 1010);
    assert!(
        clean.adventure_completions[AdventureId::TidyMeadow as usize] >= 1,
        "a full tidy year completes the adventure"
    );

    // Deep clutter never clears within the year: no completion.
    let mut messy = GameState::new(447);
    messy.active_adventure = Some(AdventureId::TidyMeadow);
    messy.mess = 100_000.0;
    run_ticks(&mut messy, 1010);
    assert_eq!(
        messy.adventure_completions[AdventureId::TidyMeadow as usize],
        0,
        "a year that never reached zero mess does not complete"
    );
}

// =====================================================================
// SIM-ADV-005 — rewards are permanent, cumulative, and survive Moves
// =====================================================================

// @spec SIM-ADV-005
#[test]
fn adventure_rewards_are_applied_through_effects_and_survive_a_move() {
    let mut s = GameState::new(448);
    s.active_adventure = Some(AdventureId::NapTime);
    grant(&mut s, StructureId::YarnComputer, 1);
    run_ticks(&mut s, 15);
    assert_eq!(s.adventure_completions[AdventureId::NapTime as usize], 1);
    assert!((s.effect(Effect::MasterSkillMultiplier) - 0.2).abs() < TOL);

    // Move with nothing armed: the twist lifts (active clears), but the earned
    // reward pool and completion count persist — even though the Yarn Computer
    // (and every structure) is reset by the Move.
    for i in 0..71 {
        s.kittens.push(kitten(&format!("Gen{i:03}"), i as u64));
    }
    s.apply(Command::GreatMeadowMove).unwrap();
    assert_eq!(s.active_adventure, None, "nothing armed → no active twist");
    assert_eq!(
        s.structures[StructureId::YarnComputer.ix()],
        0,
        "structures reset"
    );
    assert_eq!(
        s.adventure_completions[AdventureId::NapTime as usize],
        1,
        "completion counts persist across a Move"
    );
    assert!(
        (s.effect(Effect::MasterSkillMultiplier) - 0.2).abs() < TOL,
        "the reward is permanent and survives the Move"
    );
}

// A reward pool is inert on the effect key until it is EARNED, then it must
// actually change simulated behavior — not merely grow the pool. The tests
// below drive each consumer end to end.

// @spec SIM-ADV-005
#[test]
fn quiet_retreat_reward_measurably_raises_spring_catnip() {
    // A seasonal catnip source, measured in spring (fresh meadow, day 0).
    let mut base = GameState::new(460);
    grant(&mut base, StructureId::CatnipPatch, 1);
    off_boundary(&mut base);
    assert_eq!(base.season(), Season::Spring, "day 0 is spring");

    // One Quiet Retreat completion → +5% spring catnip through the effect key.
    let mut rewarded = base.clone();
    rewarded.adventure_completions[AdventureId::QuietRetreat as usize] = 1;
    rewarded.adventure_rewards[AdventureId::QuietRetreat as usize] = 0.05;
    run_ticks(&mut base, 1);
    run_ticks(&mut rewarded, 1);

    let b = base.stock(ResourceId::Catnip);
    let r = rewarded.stock(ResourceId::Catnip);
    assert!(b > 0.0, "the patch produced catnip");
    assert!(
        (r - b * 1.05).abs() < TOL,
        "Quiet Retreat adds +5% spring catnip, got {r} vs {b}"
    );

    // A second completion stacks (still under the LDR cap): strictly more.
    let mut twice = rewarded.clone();
    twice.adventure_rewards[AdventureId::QuietRetreat as usize] = 0.10;
    twice.resources[ResourceId::Catnip.ix()] = 0.0;
    run_ticks(&mut twice, 1);
    assert!(
        twice.stock(ResourceId::Catnip) > r,
        "rewards stack across completions"
    );

    // Out of season the spring reward does nothing: winter catnip is unchanged.
    let mut winter_base = GameState::new(461);
    grant(&mut winter_base, StructureId::CatnipPatch, 1);
    winter_base.ticks = 76 * content::TICKS_PER_DAY; // deep winter
    off_boundary(&mut winter_base);
    assert_eq!(winter_base.season(), Season::Winter);
    let mut winter_reward = winter_base.clone();
    winter_reward.adventure_rewards[AdventureId::QuietRetreat as usize] = 0.05;
    run_ticks(&mut winter_base, 1);
    run_ticks(&mut winter_reward, 1);
    assert!(
        (winter_reward.stock(ResourceId::Catnip) - winter_base.stock(ResourceId::Catnip)).abs()
            < TOL,
        "a spring reward never touches winter catnip"
    );
}

// @spec SIM-ADV-005
#[test]
fn long_winter_reward_measurably_raises_winter_catnip() {
    let mut base = GameState::new(462);
    grant(&mut base, StructureId::CatnipPatch, 1);
    base.ticks = 76 * content::TICKS_PER_DAY; // deep winter
    off_boundary(&mut base);
    assert_eq!(base.season(), Season::Winter);

    let mut rewarded = base.clone();
    rewarded.adventure_rewards[AdventureId::LongWinter as usize] = 0.05;
    run_ticks(&mut base, 1);
    run_ticks(&mut rewarded, 1);

    let b = base.stock(ResourceId::Catnip);
    assert!(b > 0.0);
    assert!(
        (rewarded.stock(ResourceId::Catnip) - b * 1.05).abs() < TOL,
        "Long Winter adds +5% winter catnip"
    );
}

// @spec SIM-ADV-005, SIM-ADV-002
#[test]
fn frugal_meadow_reward_lowers_energy_and_its_twist_raises_it() {
    // A single Yarn Computer draws 2n + n²/2 = 2.5 energy at n = 1.
    let mut base = GameState::new(463);
    grant(&mut base, StructureId::YarnComputer, 1);
    let draw = base.energy_consumption();
    assert!((draw - 2.5).abs() < TOL, "baseline consumption");

    // The reward (−2% per completion, a NEGATIVE ratio, LDR-capped) lowers the
    // draw. One completion adds reward_per (−0.02) to the pool.
    let reward_per = content::adventure_def(AdventureId::FrugalMeadow).reward_per;
    assert!(
        reward_per < 0.0,
        "the Frugal Meadow reward is a negative ratio"
    );
    let mut rewarded = base.clone();
    rewarded.adventure_rewards[AdventureId::FrugalMeadow as usize] = reward_per;
    let reward_delta = -limited_dr(reward_per.abs(), 0.02); // signed, LDR-capped
    assert!(
        (rewarded.energy_consumption() - draw * (1.0 + reward_delta)).abs() < TOL,
        "Frugal Meadow lowers energy consumption"
    );
    assert!(rewarded.energy_consumption() < draw, "measurably lower");

    // The twist (+10% while active) raises the draw through the same key.
    let mut twisted = base.clone();
    twisted.active_adventure = Some(AdventureId::FrugalMeadow);
    assert!(
        (twisted.energy_consumption() - draw * (1.0 + content::FRUGAL_MEADOW_TWIST)).abs() < TOL,
        "the Frugal Meadow twist raises consumption +10%"
    );

    // Both together sum in one multiplier: +10% twist plus the (negative) reward.
    let mut both = base.clone();
    both.active_adventure = Some(AdventureId::FrugalMeadow);
    both.adventure_rewards[AdventureId::FrugalMeadow as usize] = reward_per;
    assert!(
        (both.energy_consumption() - draw * (1.0 + content::FRUGAL_MEADOW_TWIST + reward_delta))
            .abs()
            < TOL,
        "twist and reward flow through one summed EnergyConsumptionRatio key"
    );
}

// @spec SIM-ADV-005, SIM-KIT-020
#[test]
fn nap_time_reward_lifts_only_the_master_tier_yield() {
    // A lone master-tier worker (≥ 9000 XP → +18.75%) producing twigs.
    let mut base = GameState::new(464);
    base.kittens.push(kitten("Sensei", 0));
    base.kittens[0].job = Some(JobId::TwigCollector);
    base.kittens[0].job_seq = Some(0);
    base.kittens[0].skills[JobId::TwigCollector as usize] = 9000.0;
    give(&mut base, ResourceId::Catnip, 100.0); // no shortfall → mood 1.0
    off_boundary(&mut base);

    let mut rewarded = base.clone();
    rewarded.adventure_rewards[AdventureId::NapTime as usize] = 0.2; // one completion
    run_ticks(&mut base, 1);
    run_ticks(&mut rewarded, 1);

    let b = base.stock(ResourceId::Twigs);
    let r = rewarded.stock(ResourceId::Twigs);
    assert!(b > 0.0);
    // The master tier's +18.75% becomes 0.1875 × (1 + 0.2); lower tiers untouched.
    let expected = (1.0 + 0.1875 * 1.2) / (1.0 + 0.1875);
    assert!(
        (r / b - expected).abs() < 1e-9,
        "Nap Time lifts the master-tier yield"
    );

    // A non-master worker (5000 XP → +12.5% tier) is unchanged by the reward.
    let mut jr_base = GameState::new(465);
    jr_base.kittens.push(kitten("Pupil", 0));
    jr_base.kittens[0].job = Some(JobId::TwigCollector);
    jr_base.kittens[0].job_seq = Some(0);
    jr_base.kittens[0].skills[JobId::TwigCollector as usize] = 5000.0;
    give(&mut jr_base, ResourceId::Catnip, 100.0);
    off_boundary(&mut jr_base);
    let mut jr_reward = jr_base.clone();
    jr_reward.adventure_rewards[AdventureId::NapTime as usize] = 0.2;
    run_ticks(&mut jr_base, 1);
    run_ticks(&mut jr_reward, 1);
    assert!(
        (jr_reward.stock(ResourceId::Twigs) - jr_base.stock(ResourceId::Twigs)).abs() < TOL,
        "the reward scales only the master tier, per SIM-KIT-020"
    );
}

// @spec SIM-ADV-005
#[test]
fn silent_sun_reward_now_flows_through_the_long_sun_cap_key() {
    // The Silent Sun reward was mapped to an unread key; it now raises the Long
    // Sun cap through the real LongSunCapBonus effect.
    let mut s = GameState::new(466);
    assert!(
        (s.effect(Effect::LongSunCapBonus) - 0.0).abs() < TOL,
        "no reward yet"
    );
    s.adventure_completions[AdventureId::SilentSun as usize] = 1;
    s.adventure_rewards[AdventureId::SilentSun as usize] = 0.10;
    assert!(
        (s.effect(Effect::LongSunCapBonus) - 0.10).abs() < TOL,
        "a Silent Sun completion raises the Long Sun cap"
    );
}

// =====================================================================
// SIM-ENG-021 REVISED — Elder Wisdom (layer 7) & Long Sun (layer 8) are
// real production multipliers, both dimmed by the energy deficit
// =====================================================================

// @spec SIM-ENG-021, SIM-MOVE-007
#[test]
fn elder_wisdom_multiplies_structure_production_and_is_deficit_dimmed() {
    let mut plain = GameState::new(449);
    grant(&mut plain, StructureId::CatnipPatch, 1);
    off_boundary(&mut plain);
    let mut base = plain.clone();
    run_ticks(&mut base, 1);
    let p = base.stock(ResourceId::Catnip);
    assert!(p > 0.0);

    // Layer 7 reaches raw-tier STRUCTURE production (not just jobs).
    let mut wise = plain.clone();
    wise.elder_wisdom = 100.0; // bonus limited_dr(1.0, 2.0) = 1.0 → +100%
    run_ticks(&mut wise, 1);
    assert!((wise.stock(ResourceId::Catnip) - p * 2.0).abs() < 1e-9);

    // A Yarn Computer draws energy with no supply → deficit ratio 0.25, which
    // dims the Elder-Wisdom bonus exactly as it dims layer 6.
    let mut dim = plain.clone();
    dim.elder_wisdom = 100.0;
    grant(&mut dim, StructureId::YarnComputer, 1);
    assert!((dim.energy_deficit_ratio() - 0.25).abs() < TOL);
    run_ticks(&mut dim, 1);
    assert!(
        (dim.stock(ResourceId::Catnip) - p * (1.0 + 1.0 * 0.25)).abs() < 1e-9,
        "layer 7 is dimmed by the energy deficit"
    );
}

// @spec SIM-ENG-021, SIM-SUN-004
#[test]
fn the_long_sun_layer_eight_is_now_deficit_dimmed_too() {
    let mut plain = GameState::new(450);
    grant(&mut plain, StructureId::CatnipPatch, 1);
    off_boundary(&mut plain);
    let mut base = plain.clone();
    run_ticks(&mut base, 1);
    let p = base.stock(ResourceId::Catnip);

    let lsb = limited_dr(unlimited_dr(1.0e6, 1000.0) / 100.0, 10.0);

    // No deficit: full Long Sun bonus (unchanged from wave 4).
    let mut lit = plain.clone();
    lit.rites.insert(RiteId::TheLongSun);
    lit.devotion = 1.0e6;
    run_ticks(&mut lit, 1);
    assert!((lit.stock(ResourceId::Catnip) - p * (1.0 + lsb)).abs() < 1e-9);

    // With an energy deficit (0.25), layer 8's bonus is dimmed — the SIM-ENG-021
    // revision: the Long Sun is now dimmed by the deficit, exactly as layer 6.
    let mut dim = plain.clone();
    dim.rites.insert(RiteId::TheLongSun);
    dim.devotion = 1.0e6;
    grant(&mut dim, StructureId::YarnComputer, 1);
    run_ticks(&mut dim, 1);
    assert!(
        (dim.stock(ResourceId::Catnip) - p * (1.0 + lsb * 0.25)).abs() < 1e-9,
        "layer 8 (the Long Sun) is now dimmed by the energy deficit"
    );
}

// =====================================================================
// Save compatibility — new fields default; a played+moved state round-trips
// =====================================================================

// @spec SAVE-013
#[test]
fn v1_saves_still_load_with_legacy_fields_defaulted() {
    let fixture = include_str!("fixtures/save_v1.json");
    let loaded = load_string(fixture, 1_751_700_000).expect("v1 fixture still loads");
    let st = &loaded.state;
    assert_eq!(st.ticks, 45, "the old meadow is preserved exactly");
    // Every wave-5 legacy field gets its new-game default.
    assert_eq!(st.elder_wisdom, 0.0);
    assert_eq!(st.warm_memories, 0.0);
    assert_eq!(st.passed_down_wisdom, 0.0);
    assert_eq!(st.generation, 0);
    assert!(st.traditions.is_empty());
    assert!(st.family_album.is_empty());
    assert_eq!(st.armed_adventure, None);
    assert_eq!(st.active_adventure, None);
    assert_eq!(st.adventure_completions, [0u32; 7]);
}

// @spec SAVE-013
#[test]
fn a_played_and_moved_state_round_trips_losslessly() {
    let mut s = meadow_with(451, 71);
    s.elder_wisdom = 40.0;
    s.warm_memories = 12.0;
    s.traditions.insert(TraditionId::TravelingBasketsI);
    s.family_album.push(archived("Forebear", 0));
    s.apply(Command::GreatMeadowMove).unwrap();
    // The Move archived the whole generation and bumped the counter.
    assert!(s.family_album.len() >= 72);
    assert_eq!(s.generation, 1);

    // now == saved_at → zero catch-up ticks, so the state persists verbatim.
    let json = save_string(&s, 900_000);
    let st = load_string(&json, 900_000).unwrap().state;
    assert_eq!(
        save_string(&st, 900_000),
        json,
        "save → load → save is byte-stable"
    );
    assert_eq!(
        st.family_album.len(),
        s.family_album.len(),
        "the Album survives the save"
    );
    assert!(
        st.family_album.iter().any(|a| a.name == "Forebear"),
        "ancestors persist"
    );
    assert!(
        (st.elder_wisdom - s.elder_wisdom).abs() < TOL,
        "Elder Wisdom persists"
    );
    assert_eq!(st.generation, s.generation, "generation persists");
    assert!(st.traditions.contains(&TraditionId::TravelingBasketsI));
}
