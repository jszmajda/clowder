//! Depth Wave 6 tests: the frontier segments — cosmic (SIM-SKY) and dreamtime
//! (SIM-DREAM) — plus the wave-6 seams in economy (SIM-ECO-028 → 53 structures,
//! the Site gate; SIM-ECO-024 outpost exemption), the engine (SIM-ENG-021
//! outdoor carve-out), colonies (SIM-TRADE-001 whale routing), and prestige
//! (SIM-MOVE-004 cosmic/dreamtime legacies; SIM-MOVE-009 the Distant Dawn cap).
//! Written tests-first from the design/spec docs: they compile against the
//! Depth Wave 6 API contract and fail until the content lands.
//!
//! ---- Contract shapes this suite pins (for the implementer) ----
//!
//! New enums (Copy + Ord for BTreeSet membership / `as usize` indexing, each
//! with an `ALL` slice), exported from the crate root:
//!   * `SiteId` (13, ladder order): FenceTop(0), OldOak(1), Rooftops(2),
//!       SunnyHill(3), MeadowsEdge(4), TheBrook(5), WhisperingWoods(6),
//!       TheFoothills(7), CloudNine(8), TheAurora(9), TheMilkyWay(10),
//!       TheFarShore(11), TheFurthestStar(12).
//!   * `DreamLoomId` (9, ladder order): SoftPillow(0), WarmBlanket(1),
//!       DreamCatcher(2), DeepCushion(3), LucidNap(4), CozyCave(5),
//!       LongSlumber(6), ReverieNet(7), DistantDawn(8).
//!
//! `StructureId` gains 13 cosmic outposts (ids 40..=52): LookoutPost(40),
//! Treehouse(41), WeatherVane(42), SolarArray(43), RangerStation(44),
//! HushfeltGrotto(45), WhisperPost(46), CairnCircle(47), SkyDock(48),
//! AuroraLantern(49), StarNursery(50), FarBeacon(51), WishingStar(52).
//! STRUCTURE_COUNT = 53, so `structures: [u32; 53]`.
//!
//! `StructureDef.gate` becomes `StructureGate` (was `Option<IdeaId>`):
//!   `enum StructureGate { Always, Idea(IdeaId), Site(SiteId) }` — the outposts
//!   carry `Site(<their site>)`, everything else `Always`/`Idea` as before.
//!
//! Content tables:
//!   * `content::site_def(SiteId) -> &SiteDef { mission_fuel:
//!       &[(ResourceId, f64)], outpost: StructureId }`
//!   * `content::dream_loom_def(DreamLoomId) -> &DreamLoomDef {
//!       cost: &[(ResourceId, f64)], .. }` (effects observed through effect()).
//!
//! New `GameState` fields (all `pub`, all `#[serde(default)]`):
//!   sites_reached: BTreeSet<SiteId>, dream_looms: BTreeSet<DreamLoomId>,
//!   stirred_dreams: f64, distant_dawn: bool, whale_visit_years: u32
//!   (0 = not visiting), whale_contentment: u32, deja_vu_until: u64.
//!   (reverie is the existing resource RES 26, ResourceId::Reverie.)
//!
//! New commands: LaunchExpedition(SiteId), SummonStarWhale, FeedWhale,
//!   DeepNap(u32), BuyDreamLoom(DreamLoomId); Trade(ColonyId::StarWhales, n)
//!   routes to the visit-gated whale exchange.
//! New events: SiteReached { site }, WhaleSummoned, WhaleFed { contentment },
//!   Napped { years } (a SUMMARY, not a per-tick stream), DreamLoomBought { id },
//!   DejaVuDay, DistantDawn.
//! New Effect variants: OutdoorProductionRatio, LongSunCapBonus, DejaVuChance,
//!   ReverieYield, HeatEfficiency, HeatDissipation, NapYield.
//! Derived queries pinned (all GameState methods): site_reachable(SiteId) ->
//!   bool, max_nap_length() -> u32, starshard_cost(years: u32) -> f64,
//!   outdoor_transfer() -> f64 (a state-independent constant fraction, 0 < t < 1).
#![allow(clippy::type_complexity, clippy::doc_overindented_list_items)]

mod common;

use clowder_core::*;
use common::*;

const TOL: f64 = 1e-9;

/// Step off the day boundary and quiet any zoomies so a single tick measures
/// exactly one production/consumption step (mirrors the helper in the other
/// depth suites).
fn off_boundary(s: &mut GameState) {
    s.ticks += 1;
    s.zoomies_until = 0;
}

/// A plain Home kitten (direct construction, as the legacy suite does).
fn kitten(name: &str) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// A meadow with `n` uniquely-named Home kittens.
fn meadow_with(seed: u64, n: usize) -> GameState {
    let mut s = GameState::new(seed);
    for i in 0..n {
        s.kittens.push(kitten(&format!("Kit{i:03}")));
    }
    s
}

/// Fast-reach a site: unlock the cosmic surface (Skyward Longing), mark every
/// PRIOR site reached, and pour generous mission fuel across every reaching
/// resource. Leaves the target itself UNreached — the caller launches it.
fn fast_reach(s: &mut GameState, site: SiteId) {
    s.ideas.insert(IdeaId::SkywardLonging);
    for prior in SiteId::ALL {
        if prior == site {
            break;
        }
        s.sites_reached.insert(prior);
    }
    for r in [
        ResourceId::Curiosity,
        ResourceId::Skymaps,
        ResourceId::Moonmilk,
        ResourceId::Cometsilk,
        ResourceId::Echoes,
        ResourceId::JarsOfPurrs,
    ] {
        give(s, r, 1e12);
    }
}

/// Does a mission-fuel slice charge (at least) `amount` of `r`? The design
/// table lists *headline* fuels; this checks a headline component is present
/// without over-pinning any smaller escalating components the table omits.
fn fuel_has(fuel: &[(ResourceId, f64)], r: ResourceId, amount: f64) -> bool {
    fuel.iter()
        .any(|(fr, fa)| *fr == r && (*fa - amount).abs() < TOL)
}

// =====================================================================
// SIM-SKY-001 — LaunchExpedition: next-in-order, surface-gated, affordable
// =====================================================================

// @spec SIM-SKY-001
#[test]
fn launching_an_expedition_reaches_the_next_site_only() {
    // Before the Skyward Longing idea the whole ladder is Locked — even the
    // first rung, however much fuel is in paw.
    let mut cold = GameState::new(501);
    give(&mut cold, ResourceId::Curiosity, 1e9);
    give(&mut cold, ResourceId::Skymaps, 1e9);
    assert_eq!(
        cold.apply(Command::LaunchExpedition(SiteId::FenceTop))
            .unwrap_err(),
        CommandError::Locked,
        "the cosmic surface (Skyward Longing) gates every launch"
    );
    assert!(!cold.site_reachable(SiteId::FenceTop));

    // Surface unlocked: Fence Top is the one reachable site; a later rung is
    // out of order and rejects Locked (predecessor unreached).
    let mut s = GameState::new(502);
    s.ideas.insert(IdeaId::SkywardLonging);
    assert!(
        s.site_reachable(SiteId::FenceTop),
        "Fence Top is first after the idea"
    );
    assert!(
        !s.site_reachable(SiteId::OldOak),
        "the second rung waits on the first"
    );
    give(&mut s, ResourceId::Curiosity, 1e9);
    give(&mut s, ResourceId::Skymaps, 1e9);
    give(&mut s, ResourceId::Moonmilk, 1e9);
    assert_eq!(
        s.apply(Command::LaunchExpedition(SiteId::OldOak))
            .unwrap_err(),
        CommandError::Locked,
        "out-of-order launch is Locked, not merely unaffordable"
    );

    // Unaffordable next-in-order rejects with InsufficientResources.
    let mut broke = GameState::new(503);
    broke.ideas.insert(IdeaId::SkywardLonging);
    assert!(matches!(
        broke
            .apply(Command::LaunchExpedition(SiteId::FenceTop))
            .unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
    assert!(
        broke.sites_reached.is_empty(),
        "a rejected launch reaches nothing"
    );

    // Success: the exact mission fuel is spent, the site is marked reached, and
    // a SiteReached event fires.
    let fuel = content::site_def(SiteId::FenceTop).mission_fuel;
    for (r, amt) in fuel {
        give(&mut s, *r, *amt);
    }
    let events = s
        .apply(Command::LaunchExpedition(SiteId::FenceTop))
        .unwrap();
    assert!(s.sites_reached.contains(&SiteId::FenceTop));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::SiteReached { site } if *site == SiteId::FenceTop)),
        "a reached site announces itself"
    );
    for (r, _) in fuel {
        assert!(s.stock(*r) < 1e-6, "{r:?} fuel is spent to the bottom");
    }
    // Now the second rung has opened.
    assert!(
        s.site_reachable(SiteId::OldOak),
        "reaching Fence Top opens Old Oak"
    );

    // The ladder holds mid-way too: with every prior rung reached and fuel in
    // paw (the fast_reach helper), the next-in-order rung launches on demand.
    let mut deep = GameState::new(508);
    fast_reach(&mut deep, SiteId::TheBrook);
    assert!(
        deep.site_reachable(SiteId::TheBrook),
        "priors reached ⇒ the next rung is reachable"
    );
    deep.apply(Command::LaunchExpedition(SiteId::TheBrook))
        .unwrap();
    assert!(deep.sites_reached.contains(&SiteId::TheBrook));
}

// =====================================================================
// SIM-SKY-002 — the thirteen-site ladder, in order
// =====================================================================

// @spec SIM-SKY-002
#[test]
fn the_site_ladder_is_thirteen_in_order_with_fuels_and_outposts() {
    use ResourceId::{Cometsilk, Curiosity, Echoes, JarsOfPurrs, Moonmilk, Skymaps};
    assert_eq!(
        SiteId::ALL.len(),
        13,
        "exactly the thirteen sites, append-only"
    );

    // Each site's one outpost, in ladder order (economy ids 40..=52).
    let outposts: [(SiteId, StructureId); 13] = [
        (SiteId::FenceTop, StructureId::LookoutPost),
        (SiteId::OldOak, StructureId::Treehouse),
        (SiteId::Rooftops, StructureId::WeatherVane),
        (SiteId::SunnyHill, StructureId::SolarArray),
        (SiteId::MeadowsEdge, StructureId::RangerStation),
        (SiteId::TheBrook, StructureId::HushfeltGrotto),
        (SiteId::WhisperingWoods, StructureId::WhisperPost),
        (SiteId::TheFoothills, StructureId::CairnCircle),
        (SiteId::CloudNine, StructureId::SkyDock),
        (SiteId::TheAurora, StructureId::AuroraLantern),
        (SiteId::TheMilkyWay, StructureId::StarNursery),
        (SiteId::TheFarShore, StructureId::FarBeacon),
        (SiteId::TheFurthestStar, StructureId::WishingStar),
    ];
    for (i, (site, outpost)) in outposts.iter().enumerate() {
        assert_eq!(SiteId::ALL[i], *site, "site {i} out of ladder order");
        assert_eq!(
            content::site_def(*site).outpost,
            *outpost,
            "{site:?} hosts the wrong outpost"
        );
        assert_eq!(outpost.ix(), 40 + i, "{outpost:?} appended out of id order");
    }

    // Spot-check four rungs' headline mission fuels (design table).
    let fence = content::site_def(SiteId::FenceTop).mission_fuel;
    assert!(fuel_has(fence, Curiosity, 5000.0) && fuel_has(fence, Skymaps, 50.0));
    let brook = content::site_def(SiteId::TheBrook).mission_fuel;
    assert!(fuel_has(brook, Moonmilk, 1000.0) && fuel_has(brook, Cometsilk, 75.0));
    let cloud = content::site_def(SiteId::CloudNine).mission_fuel;
    assert!(fuel_has(cloud, Cometsilk, 500.0) && fuel_has(cloud, Echoes, 100.0));
    let star = content::site_def(SiteId::TheFurthestStar).mission_fuel;
    assert!(fuel_has(star, JarsOfPurrs, 500.0) && fuel_has(star, Echoes, 16000.0));
}

// =====================================================================
// SIM-SKY-003 — outposts are Site-gated (Locked until reached, never un-reached)
// =====================================================================

// @spec SIM-SKY-003, SIM-ECO-028
#[test]
fn outpost_structures_are_locked_until_their_site_is_reached() {
    let mut s = GameState::new(504);
    // The gate is a Site gate, not an Idea gate.
    assert_eq!(
        content::structure_def(StructureId::LookoutPost).gate,
        StructureGate::Site(SiteId::FenceTop),
        "the outpost carries its site as its gate"
    );

    // Locked before the site is reached, even with the outpost's cost in paw.
    let cost = s.next_cost(StructureId::LookoutPost);
    for (r, amt) in &cost {
        give(&mut s, *r, *amt);
    }
    assert!(!s.structure_unlocked(StructureId::LookoutPost));
    assert_eq!(
        s.apply(Command::Build(StructureId::LookoutPost))
            .unwrap_err(),
        CommandError::Locked
    );

    // Reaching the site opens the outpost.
    s.sites_reached.insert(SiteId::FenceTop);
    assert!(s.structure_unlocked(StructureId::LookoutPost));
    s.apply(Command::Build(StructureId::LookoutPost)).unwrap();
    assert_eq!(s.structures[StructureId::LookoutPost.ix()], 1);

    // A reached site is never un-reached by ordinary play.
    give(&mut s, ResourceId::Catnip, 50.0);
    run_ticks(&mut s, 5);
    assert!(
        s.sites_reached.contains(&SiteId::FenceTop),
        "reached sites are permanent"
    );
}

// =====================================================================
// SIM-SKY-004 / SIM-ENG-021 — the outdoor production channel (reduced transfer)
// =====================================================================

// @spec SIM-SKY-004, SIM-ENG-021
#[test]
fn outdoor_output_takes_the_global_layer_at_a_reduced_transfer() {
    // Isolate the layer-6 (Dynamo, +2%) bonus on an INDOOR Catnip Patch versus
    // an OUTDOOR Treehouse. A structure's global-layer bonus = ratio − 1; the
    // outdoor channel takes only `outdoor_transfer()` of it.
    let base = |seed: u64| {
        let mut s = GameState::new(seed);
        off_boundary(&mut s);
        give(&mut s, ResourceId::Dew, 100.0); // Dynamo fuel
        give(&mut s, ResourceId::Catnip, 0.0);
        s
    };

    let mut patch = base(505);
    grant(&mut patch, StructureId::CatnipPatch, 1);
    let mut patch_boost = patch.clone();
    grant(&mut patch_boost, StructureId::DynamoWheel, 1);

    let mut tree = base(506);
    grant(&mut tree, StructureId::Treehouse, 1);
    let mut tree_boost = tree.clone();
    grant(&mut tree_boost, StructureId::DynamoWheel, 1);

    run_ticks(&mut patch, 1);
    run_ticks(&mut patch_boost, 1);
    run_ticks(&mut tree, 1);
    run_ticks(&mut tree_boost, 1);

    let d_in = patch.stock(ResourceId::Catnip);
    let d_out = tree.stock(ResourceId::Catnip);
    assert!(
        d_in > 0.0 && d_out > 0.0,
        "both channels must produce for the test to bite"
    );

    let indoor_ratio = patch_boost.stock(ResourceId::Catnip) / d_in;
    let outdoor_ratio = tree_boost.stock(ResourceId::Catnip) / d_out;

    let t = tree.outdoor_transfer();
    assert!(
        t > 0.0 && t < 1.0,
        "the transfer fraction dilutes but does not erase"
    );
    assert!(
        (indoor_ratio - 1.02).abs() < 1e-9,
        "indoor takes the full +2%"
    );
    assert!(
        outdoor_ratio < indoor_ratio - 1e-9,
        "outdoor takes strictly less"
    );
    assert!(
        (outdoor_ratio - 1.0 - 0.02 * t).abs() < 1e-9,
        "outdoor bonus is exactly the diluted global layer"
    );

    // Outposts are energy-hungry: a Weather Vane draws from the grid.
    let mut vane = GameState::new(507);
    assert_eq!(vane.energy_consumption(), 0.0);
    grant(&mut vane, StructureId::WeatherVane, 1);
    assert!(
        vane.energy_consumption() > 0.0,
        "an outpost consumes energy"
    );
}

// =====================================================================
// SIM-SKY-005 — skymap & hushfelt production and storage
// =====================================================================

// @spec SIM-SKY-005
#[test]
fn outposts_produce_skymaps_and_hushfelt_and_grant_their_storage() {
    // Skymap production is a flat transient trickle (exempt from layers 6–8):
    // Lookout Post +0.02/t, Weather Vane +0.05/t, Sky Dock +0.2/t. A single
    // off-boundary tick with no Stargazing Ledges means no sky-event noise.
    for (st, rate) in [
        (StructureId::LookoutPost, 0.02),
        (StructureId::WeatherVane, 0.05),
        (StructureId::SkyDock, 0.2),
    ] {
        let mut s = GameState::new(510);
        off_boundary(&mut s);
        grant(&mut s, st, 1);
        give(&mut s, ResourceId::Skymaps, 0.0);
        run_ticks(&mut s, 1);
        assert!(
            (s.stock(ResourceId::Skymaps) - rate).abs() < 1e-9,
            "{st:?} trickles {rate} skymap/t"
        );
    }

    // Skymap storage: Lookout Post +500, Weather Vane +2000, Sky Dock +20000.
    for (st, bonus) in [
        (StructureId::LookoutPost, 500.0),
        (StructureId::WeatherVane, 2000.0),
        (StructureId::SkyDock, 20000.0),
    ] {
        let bare = GameState::new(511).cap(ResourceId::Skymaps);
        let mut s = GameState::new(511);
        grant(&mut s, st, 1);
        assert!(
            (s.cap(ResourceId::Skymaps) - bare - bonus).abs() < 1e-6,
            "{st:?} grants +{bonus} skymap storage"
        );
    }

    // Hushfelt: the Grotto and Whisper Post both produce it and both extend its
    // cap; enough of them make the Memory Chest's 700-hushfelt cost holdable.
    let mut grotto = GameState::new(512);
    off_boundary(&mut grotto);
    grant(&mut grotto, StructureId::HushfeltGrotto, 1);
    let bare_cap = GameState::new(512).cap(ResourceId::Hushfelt);
    assert!(
        grotto.cap(ResourceId::Hushfelt) > bare_cap,
        "the Grotto extends hushfelt storage"
    );
    give(&mut grotto, ResourceId::Hushfelt, 0.0);
    run_ticks(&mut grotto, 1);
    assert!(
        grotto.stock(ResourceId::Hushfelt) > 0.0,
        "the Grotto trickles hushfelt"
    );

    let mut whisper = GameState::new(513);
    off_boundary(&mut whisper);
    grant(&mut whisper, StructureId::WhisperPost, 1);
    assert!(
        whisper.cap(ResourceId::Hushfelt) > bare_cap,
        "the Whisper Post extends storage too"
    );
    give(&mut whisper, ResourceId::Hushfelt, 0.0);
    run_ticks(&mut whisper, 1);
    assert!(
        whisper.stock(ResourceId::Hushfelt) > 0.0,
        "the Whisper Post trickles hushfelt"
    );

    let mut many = GameState::new(514);
    grant(&mut many, StructureId::HushfeltGrotto, 500);
    grant(&mut many, StructureId::WhisperPost, 500);
    assert!(
        many.cap(ResourceId::Hushfelt) >= 700.0,
        "the outposts carry hushfelt storage past the Memory Chest's 700"
    );
}

// =====================================================================
// SIM-SKY-008 — Jars-of-Purrs storage from the outer outposts
// =====================================================================

// @spec SIM-SKY-008
#[test]
fn the_outer_outposts_raise_the_jar_cap_past_two() {
    for (st, bonus) in [
        (StructureId::AuroraLantern, 50.0),
        (StructureId::StarNursery, 100.0),
        (StructureId::FarBeacon, 500.0),
    ] {
        let bare = GameState::new(515).cap(ResourceId::JarsOfPurrs);
        let mut s = GameState::new(515);
        grant(&mut s, st, 1);
        assert!(
            (s.cap(ResourceId::JarsOfPurrs) - bare - bonus).abs() < 1e-6,
            "{st:?} grants +{bonus} Jar storage"
        );
    }

    // With an Aurora Lantern's +50 cap, condensing can now hold far more than
    // the mystic leaf's old +2 ceiling. The Everdream Arch still unlocks it.
    let mut s = GameState::new(516);
    s.arches_bought.insert(ArchId::EverdreamArch);
    grant(&mut s, StructureId::AuroraLantern, 1);
    assert!(
        s.cap(ResourceId::JarsOfPurrs) > 2.0,
        "the Jar cap has climbed past 2"
    );
    give(&mut s, ResourceId::Rainbows, 30000.0);
    for _ in 0..3 {
        s.apply(Command::CondenseJar).unwrap();
    }
    assert!(
        (s.stock(ResourceId::JarsOfPurrs) - 3.0).abs() < 1e-9,
        "three Jars now sit in store"
    );
    assert!(s.stock(ResourceId::JarsOfPurrs) <= s.cap(ResourceId::JarsOfPurrs));
}

// =====================================================================
// SIM-SKY-006 — the Skyward Song: summoning the Star Whale
// =====================================================================

// @spec SIM-SKY-006
#[test]
fn summoning_the_star_whale_needs_the_sky_dock_and_the_song() {
    // Neither prerequisite alone opens the ritual.
    let mut no_dock = GameState::new(520);
    no_dock.ideas.insert(IdeaId::SkywardSong);
    give(&mut no_dock, ResourceId::Curiosity, 1e9);
    give(&mut no_dock, ResourceId::Echoes, 1e9);
    assert_eq!(
        no_dock.apply(Command::SummonStarWhale).unwrap_err(),
        CommandError::Locked,
        "no Sky Dock, no summon"
    );

    let mut no_song = GameState::new(521);
    grant(&mut no_song, StructureId::SkyDock, 1);
    give(&mut no_song, ResourceId::Curiosity, 1e9);
    give(&mut no_song, ResourceId::Echoes, 1e9);
    assert_eq!(
        no_song.apply(Command::SummonStarWhale).unwrap_err(),
        CommandError::Locked,
        "no Skyward Song idea, no summon"
    );

    // Both prerequisites, but nothing to spend: InsufficientResources.
    let mut broke = GameState::new(522);
    grant(&mut broke, StructureId::SkyDock, 1);
    broke.ideas.insert(IdeaId::SkywardSong);
    assert!(matches!(
        broke.apply(Command::SummonStarWhale).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));

    // Both prerequisites, affordable: the whale arrives for 5 years + 1 per
    // contentment point (0 at first arrival → 5), and the cost is spent.
    let mut s = GameState::new(523);
    grant(&mut s, StructureId::SkyDock, 1);
    s.ideas.insert(IdeaId::SkywardSong);
    give(&mut s, ResourceId::Curiosity, 1e9);
    give(&mut s, ResourceId::Echoes, 1e9);
    let events = s.apply(Command::SummonStarWhale).unwrap();
    assert_eq!(
        s.whale_visit_years,
        5 + s.whale_contentment,
        "5 years + 1 per contentment"
    );
    assert_eq!(s.whale_contentment, 0, "a fresh whale is uncontented");
    assert_eq!(s.whale_visit_years, 5);
    assert!(s.stock(ResourceId::Curiosity) < 1e9, "curiosity is spent");
    assert!(s.stock(ResourceId::Echoes) < 1e9, "echoes are spent");
    assert!(events.iter().any(|e| matches!(e, Event::WhaleSummoned)));
}

// =====================================================================
// SIM-SKY-007 — feeding the whale, the hushfelt exchange, the calm departure
// =====================================================================

// @spec SIM-SKY-007
#[test]
fn feeding_lengthens_the_visit_and_the_exchange_is_visit_gated() {
    // Feeding needs a visiting whale and a dreamcorn.
    let mut s = GameState::new(524);
    s.whale_visit_years = 5;
    assert!(
        matches!(
            s.apply(Command::FeedWhale).unwrap_err(),
            CommandError::InsufficientResources { .. }
        ),
        "no dreamcorn, no lullaby"
    );
    give(&mut s, ResourceId::Dreamcorns, 3.0);
    let events = s.apply(Command::FeedWhale).unwrap();
    assert!(
        (s.stock(ResourceId::Dreamcorns) - 2.0).abs() < TOL,
        "one dreamcorn lullaby is spent"
    );
    assert_eq!(s.whale_contentment, 1, "contentment rises");
    assert_eq!(s.whale_visit_years, 6, "and the visit lengthens with it");
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::WhaleFed { contentment } if *contentment == 1)));

    // The hushfelt exchange runs only while the whale visits.
    let mut away = GameState::new(525);
    away.whale_visit_years = 0;
    give(&mut away, ResourceId::Hushfelt, 1e6);
    assert_eq!(
        away.apply(Command::Trade(ColonyId::StarWhales, 1))
            .unwrap_err(),
        CommandError::Locked,
        "no exchange without a visit"
    );

    let mut here = GameState::new(526);
    here.whale_visit_years = 5;
    here.discovered.insert(ColonyId::StarWhales);
    give(&mut here, ResourceId::Hushfelt, 1e6);
    let (sky0, star0, echo0) = (
        here.stock(ResourceId::Skymaps),
        here.stock(ResourceId::Starshards),
        here.stock(ResourceId::Echoes),
    );
    here.apply(Command::Trade(ColonyId::StarWhales, 1)).unwrap();
    assert!(
        here.stock(ResourceId::Hushfelt) < 1e6,
        "hushfelt is given to the whale"
    );
    let gained = here.stock(ResourceId::Skymaps) > sky0
        || here.stock(ResourceId::Starshards) > star0
        || here.stock(ResourceId::Echoes) > echo0;
    assert!(gained, "the whale trades back cosmic goods");

    // A whale fed nothing simply drifts on when its visit ends — never angry.
    let mut depart = GameState::new(527);
    depart.whale_visit_years = 1;
    depart.whale_contentment = 0;
    run_ticks(&mut depart, 2000); // two game-years — well past a 1-year visit
    assert_eq!(
        depart.whale_visit_years, 0,
        "the visit ends and the whale departs"
    );
    assert_eq!(
        depart.whale_contentment, 0,
        "an unfed whale leaves as calm as it came"
    );
}

// =====================================================================
// SIM-DREAM-001 — the Deep Nap: idea-gated, bounded, pure tick() replay
// =====================================================================

// @spec SIM-DREAM-001
#[test]
fn a_deep_nap_fast_forwards_the_meadow_through_the_real_tick() {
    // Locked before the Deep Nap Theory idea, even with starshards in paw.
    let mut cold = GameState::new(530);
    give(&mut cold, ResourceId::Starshards, 1e9);
    assert_eq!(
        cold.apply(Command::DeepNap(1)).unwrap_err(),
        CommandError::Locked
    );

    // A nap longer than max_nap_length is refused (InsufficientResources),
    // however many starshards are in store.
    let mut s = GameState::new(531);
    s.ideas.insert(IdeaId::DeepNapTheory);
    give(&mut s, ResourceId::Starshards, 1e12);
    grant(&mut s, StructureId::CatnipPatch, 1);
    give(&mut s, ResourceId::Catnip, 0.0);
    let over = s.max_nap_length() + 1;
    assert!(matches!(
        s.apply(Command::DeepNap(over)).unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));

    // A valid nap: advance years×1000 ticks, spend starshard_cost(years), and
    // accrue exactly what a pure tick() replay would — no added per-year yields.
    let years = 3u32;
    assert!(years <= s.max_nap_length());
    let cost = s.starshard_cost(years);
    let mut replay = s.clone();
    for _ in 0..(years as u64 * 1000) {
        replay.tick();
    }
    let before_ticks = s.ticks;
    let before_star = s.stock(ResourceId::Starshards);
    let events = s.apply(Command::DeepNap(years)).unwrap();

    assert_eq!(
        s.ticks,
        before_ticks + years as u64 * 1000,
        "years×1000 ticks elapse"
    );
    assert!(
        (before_star - s.stock(ResourceId::Starshards) - cost).abs() < 1e-6,
        "the nap spends exactly starshard_cost(years)"
    );
    assert!(
        (s.stock(ResourceId::Catnip) - replay.stock(ResourceId::Catnip)).abs() < 1e-6,
        "the meadow accrues exactly a tick() replay — nothing double-counted"
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Napped { .. }))
            .count(),
        1,
        "one summarized Napped report"
    );
    // The summary is the folded report — elapsed span and net deltas — carried
    // back by the same route an absence uses, not a bare year count. Its
    // contents are pinned in tests/bulktime.rs (SIM-DREAM-001, SIM-DREAM-008);
    // here it is enough that a report is what comes back at all.
    let napped = events
        .iter()
        .find(|e| matches!(e, Event::Napped { .. }))
        .unwrap();
    let summary = serde_json::to_value(napped).unwrap();
    assert!(
        summary.to_string().contains("resource_deltas"),
        "the nap's summary carries the folded report, not just a year count: {summary}"
    );
    assert!(
        events.len() < 50,
        "a summary, never a per-tick event stream"
    );
}

// =====================================================================
// SIM-DREAM-002 — stirred dreams accrue (heat), raise nap cost, harm nothing
// =====================================================================

// @spec SIM-DREAM-002
#[test]
fn napping_stirs_dreams_that_raise_the_next_nap_cost_but_touch_no_stock() {
    let years = 5u32;
    let mut plain = GameState::new(532);
    plain.ideas.insert(IdeaId::DeepNapTheory);
    give(&mut plain, ResourceId::Starshards, 1e12);
    let mut pillow = plain.clone();
    pillow.dream_looms.insert(DreamLoomId::SoftPillow); // heat efficiency +10%
    assert!(
        pillow.effect(Effect::HeatEfficiency) > 0.0,
        "Soft Pillow raises heat efficiency"
    );

    plain.apply(Command::DeepNap(years)).unwrap();
    pillow.apply(Command::DeepNap(years)).unwrap();
    assert!(plain.stirred_dreams > 0.0, "a nap warms the dream-space");
    assert!(
        pillow.stirred_dreams < plain.stirred_dreams - TOL,
        "the (1 − heat_efficiency) factor means a Soft Pillow stirs less"
    );

    // Stirred dreams raise the starshard cost of the next nap.
    let mut cool = GameState::new(533);
    cool.ideas.insert(IdeaId::DeepNapTheory);
    let c0 = cool.starshard_cost(years);
    cool.stirred_dreams = 500.0;
    let c1 = cool.starshard_cost(years);
    assert!(c1 > c0 + TOL, "a warm dream-space costs more to nap into");

    // Heat is only a cadence brake — it never dims production or drains a stock.
    let mut icy = GameState::new(534);
    grant(&mut icy, StructureId::CatnipPatch, 1);
    off_boundary(&mut icy);
    give(&mut icy, ResourceId::Catnip, 0.0);
    let mut warm = icy.clone();
    warm.stirred_dreams = 1e6;
    run_ticks(&mut icy, 1);
    run_ticks(&mut warm, 1);
    assert!(
        (warm.stock(ResourceId::Catnip) - icy.stock(ResourceId::Catnip)).abs() < TOL,
        "however hot the dream-space, production is untouched"
    );
}

// =====================================================================
// SIM-DREAM-003 — stirred dreams dissipate each tick, floor at zero
// =====================================================================

// @spec SIM-DREAM-003
#[test]
fn stirred_dreams_dissipate_and_cool_faster_with_a_blanket() {
    // A single tick cools the dream-space; it never dips below zero.
    let mut s = GameState::new(535);
    off_boundary(&mut s);
    s.stirred_dreams = 100.0;
    run_ticks(&mut s, 1);
    assert!(
        s.stirred_dreams < 100.0 && s.stirred_dreams >= 0.0,
        "one tick dissipates some heat"
    );

    let mut floored = GameState::new(536);
    floored.stirred_dreams = 0.5;
    run_ticks(&mut floored, 1000);
    assert_eq!(
        floored.stirred_dreams, 0.0,
        "dissipation floors at zero, never negative"
    );

    // The Warm Blanket loom cools faster (dissipation +20%).
    let mut plain = GameState::new(537);
    off_boundary(&mut plain);
    plain.stirred_dreams = 100.0;
    let mut blanket = plain.clone();
    blanket.dream_looms.insert(DreamLoomId::WarmBlanket);
    assert!(
        blanket.effect(Effect::HeatDissipation) > 0.0,
        "Warm Blanket raises dissipation"
    );
    run_ticks(&mut plain, 1);
    run_ticks(&mut blanket, 1);
    assert!(
        blanket.stirred_dreams < plain.stirred_dreams - TOL,
        "a blanket cools the dream-space faster"
    );
}

// =====================================================================
// SIM-DREAM-004 — déjà-vu days: the roll, the pause, the reverie
// =====================================================================

// @spec SIM-DREAM-004
#[test]
fn deja_vu_days_accrue_reverie_over_many_seasons() {
    // Many Memory Chests plus the Dream Catcher loom drive the per-season roll;
    // over many seasons some déjà-vu days land and reverie is gathered.
    let mut s = GameState::new(540);
    grant(&mut s, StructureId::MemoryChest, 200);
    s.dream_looms.insert(DreamLoomId::DreamCatcher);
    assert!(
        s.effect(Effect::DejaVuChance) > 0.0,
        "chests and the loom drive the roll"
    );
    give(&mut s, ResourceId::Reverie, 0.0);
    // 100 seasons = 2500 days = 25000 ticks (day-aligned start).
    run_ticks(&mut s, 25_000);
    assert!(
        s.stock(ResourceId::Reverie) > 0.0,
        "some déjà-vu days landed and gathered reverie"
    );
}

// @spec SIM-DREAM-004
#[test]
fn a_deja_vu_day_pauses_production_and_consumption_so_nothing_is_short() {
    // Consumption pauses: kittens present, no producer. A control day eats into
    // the shelf; a déjà-vu day leaves it exactly full (no shortfall — Tenet 1).
    let mut ctrl = GameState::new(541);
    ctrl.kittens.push(kitten("A"));
    ctrl.kittens.push(kitten("B"));
    ctrl.kittens.push(kitten("C"));
    give(&mut ctrl, ResourceId::Catnip, 100.0);
    let mut dream = ctrl.clone();
    run_ticks(&mut ctrl, 10);
    dream.deja_vu_until = dream.ticks + 100; // force the whole run déjà-vu
    run_ticks(&mut dream, 10);
    assert!(
        ctrl.stock(ResourceId::Catnip) < 100.0,
        "an ordinary day, the clowder eats"
    );
    assert!(
        (dream.stock(ResourceId::Catnip) - 100.0).abs() < TOL,
        "a déjà-vu day pauses consumption — the shelf is untouched"
    );
    assert_eq!(dream.kittens.len(), 3, "no shortfall, so nobody wanders");

    // Production pauses too: patches, no kittens. A control day grows catnip; a
    // déjà-vu day grows none.
    let mut pctrl = GameState::new(542);
    grant(&mut pctrl, StructureId::CatnipPatch, 3);
    give(&mut pctrl, ResourceId::Catnip, 0.0);
    let mut pdream = pctrl.clone();
    run_ticks(&mut pctrl, 10);
    pdream.deja_vu_until = pdream.ticks + 100;
    run_ticks(&mut pdream, 10);
    assert!(
        pctrl.stock(ResourceId::Catnip) > 0.0,
        "an ordinary day, the patches grow"
    );
    assert_eq!(
        pdream.stock(ResourceId::Catnip),
        0.0,
        "a déjà-vu day pauses production"
    );
}

// =====================================================================
// SIM-DREAM-005 — reverie only ever rises (except a Dream Loom purchase)
// =====================================================================

// @spec SIM-DREAM-005
#[test]
fn reverie_is_monotone_except_for_a_loom_purchase() {
    // Across a long run reverie never falls tick-to-tick.
    let mut s = GameState::new(543);
    grant(&mut s, StructureId::MemoryChest, 200);
    s.dream_looms.insert(DreamLoomId::DreamCatcher);
    give(&mut s, ResourceId::Reverie, 50.0);
    let mut prev = s.stock(ResourceId::Reverie);
    for _ in 0..60 {
        run_ticks(&mut s, 10); // a day
        let now = s.stock(ResourceId::Reverie);
        assert!(now + TOL >= prev, "reverie never falls during play");
        prev = now;
    }

    // The only thing that lowers reverie is a player-chosen loom purchase.
    let mut buyer = GameState::new(544);
    give(&mut buyer, ResourceId::Reverie, 5.0);
    buyer
        .apply(Command::BuyDreamLoom(DreamLoomId::SoftPillow))
        .unwrap();
    assert!(
        buyer.stock(ResourceId::Reverie) < 5.0,
        "a loom purchase spends reverie"
    );
}

// =====================================================================
// SIM-DREAM-006 — the nine Dream Loom upgrades
// =====================================================================

// @spec SIM-DREAM-006
#[test]
fn the_dream_loom_ladder_is_nine_with_costs_and_effects() {
    assert_eq!(
        DreamLoomId::ALL.len(),
        9,
        "exactly nine loom upgrades, append-only"
    );

    // Spot-check three rungs' costs (design table).
    assert_eq!(
        content::dream_loom_def(DreamLoomId::SoftPillow).cost,
        &[(ResourceId::Reverie, 5.0)]
    );
    assert_eq!(
        content::dream_loom_def(DreamLoomId::DreamCatcher).cost,
        &[(ResourceId::Reverie, 40.0)]
    );
    assert_eq!(
        content::dream_loom_def(DreamLoomId::LongSlumber).cost,
        &[(ResourceId::Reverie, 1000.0)]
    );
    // The Distant Dawn rung carries a Jar cost too.
    assert_eq!(
        content::dream_loom_def(DreamLoomId::DistantDawn).cost,
        &[
            (ResourceId::Reverie, 10000.0),
            (ResourceId::JarsOfPurrs, 100.0)
        ]
    );

    // Spot-check three rungs' effects flow through the effect-key system.
    let mut pillow = GameState::new(545);
    let mut catcher = GameState::new(545);
    let base_deja = GameState::new(545).effect(Effect::DejaVuChance);
    assert_eq!(
        GameState::new(545).max_nap_length(),
        100,
        "base nap length is 100"
    );
    pillow.dream_looms.insert(DreamLoomId::SoftPillow);
    catcher.dream_looms.insert(DreamLoomId::DreamCatcher);
    let mut slumber = GameState::new(545);
    slumber.dream_looms.insert(DreamLoomId::LongSlumber);
    assert!(
        pillow.effect(Effect::HeatEfficiency) > 0.0,
        "Soft Pillow → heat efficiency"
    );
    assert!(
        catcher.effect(Effect::DejaVuChance) > base_deja,
        "Dream Catcher → déjà-vu chance"
    );
    assert!(slumber.max_nap_length() > 100, "Long Slumber → longer naps");

    // Buyable once each; a second purchase does not spend again.
    let mut s = GameState::new(546);
    give(&mut s, ResourceId::Reverie, 5.0);
    let events = s
        .apply(Command::BuyDreamLoom(DreamLoomId::SoftPillow))
        .unwrap();
    assert!(s.stock(ResourceId::Reverie) < TOL, "reverie is spent");
    assert!(s.dream_looms.contains(&DreamLoomId::SoftPillow));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::DreamLoomBought { id } if *id == DreamLoomId::SoftPillow)));
    give(&mut s, ResourceId::Reverie, 100.0);
    assert!(
        s.apply(Command::BuyDreamLoom(DreamLoomId::SoftPillow))
            .is_err(),
        "no re-buying a loom already woven"
    );
    assert!(
        (s.stock(ResourceId::Reverie) - 100.0).abs() < TOL,
        "no double-spend"
    );

    // Unaffordable rejects with InsufficientResources.
    let mut poor = GameState::new(547);
    give(&mut poor, ResourceId::Reverie, 4.0);
    assert!(matches!(
        poor.apply(Command::BuyDreamLoom(DreamLoomId::SoftPillow))
            .unwrap_err(),
        CommandError::InsufficientResources { .. }
    ));
}

// =====================================================================
// SIM-DREAM-007 — the Distant Dawn era raises the Passed-Down cap to +400%
// =====================================================================

// @spec SIM-DREAM-007, SIM-MOVE-009
#[test]
fn reaching_year_forty_thousand_with_the_loom_lights_the_distant_dawn() {
    // Reaching year 40000 WITH the Distant Dawn loom sets the flag. One bounded
    // nap crosses the boundary from year 39999.
    let mut s = GameState::new(548);
    s.dream_looms.insert(DreamLoomId::DistantDawn);
    s.ideas.insert(IdeaId::DeepNapTheory);
    give(&mut s, ResourceId::Starshards, 1e12);
    s.ticks = 39_999_000; // year 39999 (1000 ticks per game-year)
    assert!(!s.distant_dawn, "not yet dawn");
    s.apply(Command::DeepNap(1)).unwrap(); // → tick 40_000_000 = year 40000
    assert_eq!(s.year(), 40_000);
    assert!(s.distant_dawn, "the Distant Dawn is lit");

    // Without the loom, the same crossing sets nothing.
    let mut no_loom = GameState::new(549);
    no_loom.ideas.insert(IdeaId::DeepNapTheory);
    give(&mut no_loom, ResourceId::Starshards, 1e12);
    no_loom.ticks = 39_999_000;
    no_loom.apply(Command::DeepNap(1)).unwrap();
    assert!(
        !no_loom.distant_dawn,
        "no loom, no dawn — the flag never sets"
    );

    // Once lit, the Passed-Down Wisdom production cap climbs from +100% to +400%.
    let mk = |seed: u64| {
        let mut b = GameState::new(seed);
        grant(&mut b, StructureId::CatnipPatch, 1);
        off_boundary(&mut b);
        give(&mut b, ResourceId::Catnip, 0.0);
        b
    };
    let mut plain = mk(550);
    run_ticks(&mut plain, 1);
    let p = plain.stock(ResourceId::Catnip);

    // Passed-down 200 → limited_dr(2.0, cap): cap 1.0 (no dawn) vs 4.0 (dawn).
    let mut off = mk(550);
    off.passed_down_wisdom = 200.0;
    let mut on = off.clone();
    on.distant_dawn = true;
    run_ticks(&mut off, 1);
    run_ticks(&mut on, 1);
    let bonus_off = limited_dr(2.0, 1.0);
    let bonus_on = limited_dr(2.0, 4.0);
    assert!((off.stock(ResourceId::Catnip) - p * (1.0 + bonus_off)).abs() < 1e-9);
    assert!((on.stock(ResourceId::Catnip) - p * (1.0 + bonus_on)).abs() < 1e-9);
    assert!(
        off.stock(ResourceId::Catnip) < p * 2.0,
        "without dawn, +100% holds"
    );
    assert!(
        on.stock(ResourceId::Catnip) > p * 2.0,
        "the dawn lifts the cap past +100%"
    );
}

// =====================================================================
// SIM-MOVE-004 — cosmic/dreamtime legacies across the Great Meadow Move
// =====================================================================

// @spec SIM-MOVE-004
#[test]
fn a_move_keeps_the_cosmic_dreamtime_legacies_and_resets_the_rest() {
    let mut s = meadow_with(551, 71);
    // Legacies to PRESERVE.
    s.sites_reached.insert(SiteId::FenceTop);
    s.sites_reached.insert(SiteId::OldOak);
    s.dream_looms.insert(DreamLoomId::SoftPillow);
    s.dream_looms.insert(DreamLoomId::DistantDawn);
    give(&mut s, ResourceId::Reverie, 42.0);
    s.distant_dawn = true;
    // Meadow state to RESET.
    s.stirred_dreams = 500.0;
    s.whale_visit_years = 5;
    s.whale_contentment = 3;

    s.apply(Command::GreatMeadowMove).unwrap();

    // Preserved — the permanent frontier knowledge and dreamtime currency.
    assert!(s.sites_reached.contains(&SiteId::FenceTop));
    assert!(
        s.sites_reached.contains(&SiteId::OldOak),
        "reached sites persist a Move"
    );
    assert!(s.dream_looms.contains(&DreamLoomId::SoftPillow));
    assert!(
        s.dream_looms.contains(&DreamLoomId::DistantDawn),
        "woven looms persist a Move"
    );
    assert!(
        (s.stock(ResourceId::Reverie) - 42.0).abs() < TOL,
        "reverie persists a Move"
    );
    assert!(
        s.distant_dawn,
        "the Distant Dawn, once lit, is never un-lit"
    );

    // Reset — the Star Whale visit and the stirred-dreams heat are new-meadow.
    assert_eq!(s.stirred_dreams, 0.0, "stirred dreams reset");
    assert_eq!(s.whale_visit_years, 0, "the whale visit resets");
    assert_eq!(s.whale_contentment, 0, "whale contentment resets");
}

// =====================================================================
// SIM-ECO-028 / SIM-ECO-024 — 53 structures; the outposts are Site-gated & exempt
// =====================================================================

// @spec SIM-ECO-028, SIM-ECO-024
#[test]
fn the_structure_table_is_fifty_three_with_site_gated_outposts() {
    assert_eq!(
        StructureId::ALL.len(),
        53,
        "40 meadow structures + 13 cosmic outposts"
    );
    assert_eq!(
        GameState::new(552).structures,
        [0u32; 53],
        "a new meadow across the widened array"
    );

    // The thirteen outposts (ids 40..=52) each carry a Site gate.
    for i in 40..53 {
        let st = StructureId::ALL[i];
        assert!(
            matches!(content::structure_def(st).gate, StructureGate::Site(_)),
            "{st:?} is Site-gated, not Idea/Always"
        );
    }
    // Non-outpost gates are unchanged (Always / Idea).
    assert_eq!(
        content::structure_def(StructureId::CatnipPatch).gate,
        StructureGate::Always
    );
    assert!(matches!(
        content::structure_def(StructureId::Pantry).gate,
        StructureGate::Idea(_)
    ));
}

// =====================================================================
// Persistence — new fields default; cosmic state round-trips
// =====================================================================

// @spec SAVE-013
#[test]
fn a_fresh_meadow_defaults_every_wave_six_field() {
    let s = GameState::new(553);
    assert!(s.sites_reached.is_empty());
    assert!(s.dream_looms.is_empty());
    assert_eq!(s.stirred_dreams, 0.0);
    assert!(!s.distant_dawn);
    assert_eq!(s.whale_visit_years, 0);
    assert_eq!(s.whale_contentment, 0);
    assert_eq!(s.deja_vu_until, 0);
}

// @spec SAVE-013
#[test]
fn a_v1_save_migrates_with_wave_six_fields_at_defaults() {
    let fixture = include_str!("fixtures/save_v1.json");
    let loaded = load_string(fixture, 1_751_700_000).expect("v1 fixture loads");
    let s = &loaded.state;
    assert!(s.sites_reached.is_empty(), "old saves reach no sites");
    assert!(s.dream_looms.is_empty());
    assert!(!s.distant_dawn, "the Distant Dawn defaults dark");
    assert_eq!(s.whale_visit_years, 0);
    assert_eq!(s.stirred_dreams, 0.0);
}

// @spec SAVE-010, SAVE-011
#[test]
fn a_cosmic_state_round_trips_byte_identically() {
    let mut s = GameState::new(554);
    s.sites_reached.insert(SiteId::FenceTop);
    s.sites_reached.insert(SiteId::TheBrook);
    s.dream_looms.insert(DreamLoomId::SoftPillow);
    s.distant_dawn = true;
    s.stirred_dreams = 7.5;
    s.whale_visit_years = 4;
    s.whale_contentment = 2;
    give(&mut s, ResourceId::Reverie, 12.0);

    let json = save_string(&s, 1000);
    let loaded = load_string(&json, 1000).unwrap();
    let r = &loaded.state;
    assert!(r.sites_reached.contains(&SiteId::FenceTop));
    assert!(
        r.sites_reached.contains(&SiteId::TheBrook),
        "reached sites survive the save"
    );
    assert!(r.dream_looms.contains(&DreamLoomId::SoftPillow));
    assert!(r.distant_dawn, "the Distant Dawn flag persists");
    assert!((r.stirred_dreams - 7.5).abs() < TOL);
    assert_eq!(r.whale_visit_years, 4);
    assert!((r.stock(ResourceId::Reverie) - 12.0).abs() < TOL);

    let again = save_string(&loaded.state, 1000);
    assert_eq!(json, again, "save → load → save is byte-stable");
}
