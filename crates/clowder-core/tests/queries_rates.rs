//! Derived queries: net production rates, and availability over every content
//! kind.
//!
//! @spec SIM-ENG-030, SIM-ENG-032, SIM-ENG-034, SIM-ENG-035
//!
//! The load-bearing property of the rate query is *where the number comes
//! from*: SIM-ENG-030 requires it be derived from the same production and
//! consumption path `tick()` runs, never a parallel formula. So almost every
//! rate test here is a differential against a real tick — a reimplementation
//! that agrees on a plain meadow and forgets one multiplier fails the moment
//! that multiplier is switched on.

mod common;

use clowder_core::content;
use clowder_core::*;
use common::*;

const RESOURCES: usize = 46;

/// Ticks per game second — the unit the rate is quoted in.
fn per_second() -> f64 {
    content::TICKS_PER_SECOND as f64
}

/// Float residue guard, scaled to the magnitudes being differenced. A rate
/// derived by differencing stocks carries ~1 ulp of the stock; a real
/// disagreement (a missing layer) is orders of magnitude larger.
fn agree(a: f64, b: f64, scale: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + scale.abs())
}

/// The core differential: what the query reports must be what the tick
/// pipeline actually moves, per game second, for *every* resource.
///
/// Resources the clamp phase trimmed are excluded — the rate is deliberately
/// pre-cap (SIM-ENG-035), so a capped stock is not a fair comparison; those
/// get their own test. `must_cover` names the resources the fixture exists to
/// exercise, and the helper asserts each of them really was compared.
fn agrees_with_tick(s: &GameState, must_cover: &[ResourceId]) -> [f64; RESOURCES] {
    assert_ne!(
        (s.ticks + 1) % content::TICKS_PER_DAY,
        0,
        "fixture must sit mid-day: a day-boundary tick folds daily processing \
         into the delta, which is not production or consumption"
    );
    let rates = s.net_rates();

    let mut after = s.clone();
    after.tick();

    let mut covered: Vec<ResourceId> = Vec::new();
    for r in ResourceId::ALL {
        let before = s.stock(r);
        let now = after.stock(r);
        let cap = after.cap(r);
        if now >= cap - 1e-9 {
            continue; // the clamp may have trimmed this one
        }
        covered.push(r);
        let moved = (now - before) * per_second();
        assert!(
            agree(rates[r.ix()], moved, before.abs() + now.abs()),
            "{r:?}: query reports {} per second but the tick moved {} per second \
             ({before} -> {now}); the rate is not coming from the tick's own \
             production/consumption path",
            rates[r.ix()],
            moved
        );
    }
    for r in must_cover {
        assert!(
            covered.contains(r),
            "{r:?} was clamped away, so this fixture never actually compared it"
        );
    }
    rates
}

/// A meadow mid-spring-day with structures, workers, and a pantry: several
/// stack layers live at once.
fn meadow(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    adopt(&mut s, 2);
    // Settle everything time-boxed so the fixture is a plain steady state.
    s.zoomies_until = 0;
    s.jamboree_until = 0;
    s.deja_vu_until = 0;
    s.pending_sky_event = None;
    s.grumpy = false;
    s.mess = 0.0;
    grant(&mut s, StructureId::CatnipPatch, 3);
    s.kittens[0].job = Some(JobId::Gardener);
    s.kittens[1].job = Some(JobId::TwigCollector);
    for k in &s.kittens {
        assert_eq!(k.location, Location::Home, "fixture wants both kittens home");
    }
    give(&mut s, ResourceId::Catnip, 100.0);
    give(&mut s, ResourceId::Twigs, 20.0);
    s.ticks = 1002; // mid-day 100 — Spring, no boundary on the next tick
    assert_eq!(s.season(), Season::Spring);
    s
}

// ---------------------------------------------------------------------------
// SIM-ENG-030 — the rate is the tick pipeline's own arithmetic
// ---------------------------------------------------------------------------

// @spec SIM-ENG-030
#[test]
fn net_rate_is_what_the_tick_actually_moves() {
    let s = meadow(1);
    let rates = agrees_with_tick(&s, &[ResourceId::Catnip, ResourceId::Twigs]);
    // The fixture is alive: growth and gathering both register.
    assert!(rates[ResourceId::Catnip.ix()] > 0.0, "patches and a gardener grow catnip");
    assert!(rates[ResourceId::Twigs.ix()] > 0.0, "a twig collector gathers twigs");
}

/// Every multiplier the pipeline applies has to reach the rate. Each variant
/// below both (a) must still agree with a real tick and (b) must move the
/// reported catnip rate off the baseline — so a parallel formula that skipped
/// that layer would report the baseline number and fail (a).
// @spec SIM-ENG-030
#[test]
fn net_rate_tracks_every_layer_the_pipeline_applies() {
    let base = meadow(2);
    let baseline = agrees_with_tick(&base, &[ResourceId::Catnip])[ResourceId::Catnip.ix()];

    // Mood: a grumpy clowder's paws slow down.
    let mut grumpy = base.clone();
    grumpy.grumpy = true;
    let r = agrees_with_tick(&grumpy, &[ResourceId::Catnip])[ResourceId::Catnip.ix()];
    assert!(r < baseline - 1e-6, "mood must reach the rate (grumpy {r} vs {baseline})");

    // Zoomies: a live multiplier on job output.
    let mut zoomy = base.clone();
    zoomy.zoomies_until = zoomy.ticks + 500;
    assert!(zoomy.zoomies_active());
    let r = agrees_with_tick(&zoomy, &[ResourceId::Catnip])[ResourceId::Catnip.ix()];
    assert!(r > baseline + 1e-6, "zoomies must reach the rate ({r} vs {baseline})");

    // Season: the patch's seasonal modifier.
    let mut winter = base.clone();
    winter.ticks = 802; // mid-day 80
    assert_eq!(winter.season(), Season::Winter);
    let r = agrees_with_tick(&winter, &[ResourceId::Catnip])[ResourceId::Catnip.ix()];
    assert!(r < baseline - 1e-6, "season must reach the rate ({r} vs {baseline})");

    // A per-job ratio bought as a refinement.
    let mut refined = base.clone();
    refined.refinements.insert(RefinementId::PebbleCombs);
    let r = agrees_with_tick(&refined, &[ResourceId::Catnip])[ResourceId::Catnip.ix()];
    assert!(r > baseline + 1e-6, "a job-ratio effect must reach the rate ({r} vs {baseline})");

    // A global production layer, itself fuel-scaled.
    let mut global = base.clone();
    grant(&mut global, StructureId::DynamoWheel, 2);
    give(&mut global, ResourceId::Dew, 5.0);
    let r = agrees_with_tick(&global, &[ResourceId::Catnip])[ResourceId::Catnip.ix()];
    assert!(r > baseline + 1e-6, "the global layer must reach the rate ({r} vs {baseline})");
}

/// "Production less consumption" — the consumption half is not optional, and
/// the answer is allowed to be negative.
// @spec SIM-ENG-030
#[test]
fn net_rate_is_production_less_consumption() {
    let mut s = meadow(3);
    // Nobody works and nothing grows: what is left is the eating.
    s.structures[StructureId::CatnipPatch.ix()] = 0;
    for k in &mut s.kittens {
        k.job = None;
    }
    give(&mut s, ResourceId::Feathers, 10.0);

    let rates = agrees_with_tick(&s, &[ResourceId::Catnip, ResourceId::Feathers]);
    assert!(
        rates[ResourceId::Catnip.ix()] < 0.0,
        "a fed clowder with no production has a negative catnip rate, not zero"
    );
    assert!(
        rates[ResourceId::Feathers.ix()] < 0.0,
        "the luxury drain is consumption too"
    );
}

// @spec SIM-ENG-030
#[test]
fn net_rate_is_per_game_second_not_per_tick() {
    let mut s = GameState::new(4);
    grant(&mut s, StructureId::CatnipPatch, 4);
    s.ticks = 1002;
    give(&mut s, ResourceId::Catnip, 100.0);

    let rate = s.net_rate(ResourceId::Catnip);
    let mut after = s.clone();
    after.tick();
    let per_tick = after.stock(ResourceId::Catnip) - s.stock(ResourceId::Catnip);

    assert!(per_tick > 0.0, "fixture must actually grow");
    assert!(
        agree(rate, per_tick * per_second(), rate),
        "a game second is {} ticks: rate {rate} vs per-tick {per_tick}",
        content::TICKS_PER_SECOND
    );
    assert!(
        (rate - per_tick).abs() > 1e-9,
        "the rate is quoted per game second, not per tick"
    );
}

/// `net_rate` is sugar over `net_rates`, so the two can never disagree — the
/// sidebar and the inspector must not be able to show different numbers.
// @spec SIM-ENG-030
#[test]
fn the_single_resource_rate_is_sugar_over_the_whole_array() {
    let s = meadow(5);
    let all = s.net_rates();
    for r in ResourceId::ALL {
        assert_eq!(
            s.net_rate(r),
            all[r.ix()],
            "{r:?}: net_rate and net_rates must be the same number"
        );
    }
}

/// Asking is free: reading the rate must not advance the sim, spend RNG, or
/// vary between identical calls.
// @spec SIM-ENG-030
#[test]
fn asking_for_a_rate_changes_nothing_and_repeats() {
    let s = meadow(6);
    let snapshot = s.clone();
    let first = s.net_rates();
    for r in ResourceId::ALL {
        let _ = s.net_rate(r);
    }
    let second = s.net_rates();
    assert_eq!(first, second, "the same state must yield the same rates");
    assert_eq!(s, snapshot, "reading a rate must leave the game state untouched");
}

/// The rate describes production and consumption *at current state* — not
/// whatever the next tick happens to bring with it. Two states identical but
/// for where they sit inside the same day report the same rate, even though
/// one of them is about to cross a day boundary and do daily processing.
// @spec SIM-ENG-030
#[test]
fn net_rate_reads_current_state_not_the_next_ticks_daily_processing() {
    let mut base = meadow(7);
    grant(&mut base, StructureId::CozyCushion, 1); // room for one more
    base.pity = 4; // the next day close would adopt, no RNG needed
    give(&mut base, ResourceId::Catnip, 100.0);

    let mut mid_day = base.clone();
    mid_day.ticks = 1002; // next tick stays inside day 100
    let mut day_end = base.clone();
    day_end.ticks = 1009; // next tick crosses into day 101
    assert_eq!(mid_day.day(), day_end.day(), "same day, same season, same production");

    // The fixture is discriminating: the two next ticks really do differ.
    let mut a = mid_day.clone();
    a.tick();
    let mut b = day_end.clone();
    b.tick();
    assert_eq!(a.kittens.len(), mid_day.kittens.len());
    assert_eq!(b.kittens.len(), day_end.kittens.len() + 1, "the day close adopts");
    let moved_a = a.stock(ResourceId::Catnip) - mid_day.stock(ResourceId::Catnip);
    let moved_b = b.stock(ResourceId::Catnip) - day_end.stock(ResourceId::Catnip);
    assert!((moved_a - moved_b).abs() > 1e-9, "the ticks differ; the rates must not");

    assert_eq!(
        mid_day.net_rates(),
        day_end.net_rates(),
        "the rate is production less consumption at current state"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-034 — speed-free
// ---------------------------------------------------------------------------

/// No tick-internal rule reads speed (SIM-ENG-024), and the rate is no
/// exception: it is quoted in game seconds. A caller wanting real time
/// multiplies by speed itself, at the boundary that already turns real
/// seconds into ticks.
// @spec SIM-ENG-034
#[test]
fn net_rate_is_free_of_the_speed_multiplier() {
    let base = meadow(8);
    let at_default = base.net_rates();

    for speed in [0.25, 0.5, 2.0, 8.0] {
        let mut s = base.clone();
        s.apply(Command::SetSpeed(speed)).expect("SetSpeed is always legal");
        assert_eq!(s.speed, speed);
        assert_eq!(
            s.net_rates(),
            at_default,
            "speed {speed} must not scale the reported rate by so much as a bit"
        );
    }
}

/// And the speed-free rate stays the tick's own arithmetic at any speed —
/// speed changes how many ticks the boundary drives, never what one does.
// @spec SIM-ENG-034
#[test]
fn a_sped_up_meadow_still_reports_the_ticks_own_arithmetic() {
    let mut s = meadow(9);
    s.apply(Command::SetSpeed(8.0)).expect("SetSpeed is always legal");
    agrees_with_tick(&s, &[ResourceId::Catnip, ResourceId::Twigs]);
}

// ---------------------------------------------------------------------------
// SIM-ENG-035 — advisory: throttling in, guarantees out
// ---------------------------------------------------------------------------

/// The rate promises nothing about a cap. A stock sitting at its ceiling still
/// reports the production behind it, even though the clamp phase will trim
/// every drop away — the number describes the flow, not the outcome.
// @spec SIM-ENG-035
#[test]
fn a_capped_stock_still_reports_the_flow_behind_it() {
    let mut s = GameState::new(10);
    grant(&mut s, StructureId::CatnipPatch, 4);
    s.ticks = 1002;
    let cap = s.cap(ResourceId::Catnip);
    give(&mut s, ResourceId::Catnip, cap);

    let rate = s.net_rate(ResourceId::Catnip);
    assert!(
        rate > 0.0,
        "at cap {cap} the rate must still report the growth behind it ({rate}): \
         it carries no guarantee against a stock reaching its ceiling"
    );

    let mut after = s.clone();
    after.tick();
    assert!(
        (after.stock(ResourceId::Catnip) - cap).abs() < 1e-9,
        "meanwhile the clamp really did eat the whole tick's growth"
    );
}

/// Throttling *is* reflected: a converter starved of one input scales all of
/// its inputs and outputs by the same satisfaction fraction, and the rate
/// says so rather than quoting the unstarved figure.
// @spec SIM-ENG-035
#[test]
fn net_rate_reflects_converter_throttling() {
    let stocked = |pebbles: f64| {
        let mut s = GameState::new(11);
        grant(&mut s, StructureId::Hearth, 2);
        s.ticks = 1002;
        give(&mut s, ResourceId::Twigs, 40.0);
        give(&mut s, ResourceId::Pebbles, pebbles);
        give(&mut s, ResourceId::Buttons, 10.0);
        give(&mut s, ResourceId::Embers, 5.0);
        s
    };

    let flowing = stocked(50.0); // both inputs abundant: fraction 1
    let full = agrees_with_tick(
        &flowing,
        &[ResourceId::Twigs, ResourceId::Pebbles, ResourceId::Buttons, ResourceId::Embers],
    );
    assert!(full[ResourceId::Buttons.ix()] > 0.0);
    assert!(full[ResourceId::Twigs.ix()] < 0.0, "inputs are drawn down");

    // Pebbles cover exactly half of one tick's demand across the two hearths.
    let demand = 0.10 * 2.0;
    let starved = stocked(demand / 2.0);
    let half = agrees_with_tick(
        &starved,
        &[ResourceId::Twigs, ResourceId::Pebbles, ResourceId::Buttons, ResourceId::Embers],
    );
    for r in [ResourceId::Twigs, ResourceId::Pebbles, ResourceId::Buttons, ResourceId::Embers] {
        assert!(
            agree(half[r.ix()], full[r.ix()] * 0.5, full[r.ix()]),
            "{r:?}: throttling scales every flow by the same fraction — expected {} , got {}",
            full[r.ix()] * 0.5,
            half[r.ix()]
        );
    }

    // Fully starved: the converter idles, drawing nothing and yielding nothing.
    let idle = stocked(0.0);
    let none = agrees_with_tick(
        &idle,
        &[ResourceId::Twigs, ResourceId::Buttons, ResourceId::Embers],
    );
    for r in [ResourceId::Twigs, ResourceId::Buttons, ResourceId::Embers] {
        assert!(
            none[r.ix()].abs() < 1e-9,
            "{r:?}: a starved converter contributes nothing, in or out (got {})",
            none[r.ix()]
        );
    }
}

/// Advisory in the other direction too: the rate is a snapshot of now, and a
/// supply that runs out before the rate is realised simply changes it. The
/// query never promised the flow would last.
// @spec SIM-ENG-035
#[test]
fn a_reported_rate_is_a_snapshot_a_changing_supply_can_overtake() {
    let mut s = GameState::new(12);
    grant(&mut s, StructureId::Hearth, 1);
    s.ticks = 1002;
    give(&mut s, ResourceId::Twigs, 40.0);
    give(&mut s, ResourceId::Pebbles, 0.5); // five ticks of fuel, no more
    give(&mut s, ResourceId::Buttons, 10.0);

    let now = s.net_rate(ResourceId::Buttons);
    assert!(now > 0.0, "buttons are flowing right now");

    run_ticks(&mut s, 5);
    assert!(s.stock(ResourceId::Pebbles) < 1e-9, "the fuel ran out");
    assert!(
        s.net_rate(ResourceId::Buttons).abs() < 1e-9,
        "with the supply gone the rate is simply a different number now"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-032 — availability answers for every content kind
// ---------------------------------------------------------------------------

/// One id of every kind, so a caller holding any `ContentId` gets an answer.
fn one_of_each() -> Vec<ContentId> {
    vec![
        ContentId::Structure(StructureId::CatnipPatch),
        ContentId::Idea(IdeaId::PantryOrganization),
        ContentId::Craft(CraftId::Wands),
        ContentId::Job(JobId::Gardener),
        ContentId::Rite(RiteId::MorningStretch),
        ContentId::Arch(ArchId::RibbonArch),
        ContentId::Refinement(RefinementId::PebbleCombs),
        ContentId::Rule(RuleId::EarlyBirds),
        ContentId::Tradition(TraditionId::CarefulPacking),
        ContentId::DreamLoom(DreamLoomId::SoftPillow),
    ]
}

/// Totality is the point: the query answers for every revealable kind, so the
/// shell never has to branch on which kind it is holding (TUI-NAV-053). Every
/// id of every kind, on a fresh meadow and on a played-in one.
// @spec SIM-ENG-032
#[test]
fn availability_answers_for_every_id_of_every_kind() {
    let fresh = GameState::new(20);
    let mut played = meadow(21);
    played.ideas.insert(IdeaId::Tinkering);
    played.refinements.insert(RefinementId::PebbleCombs);
    played.rules.insert(RuleId::EarlyBirds);
    played.peak_devotion = 400.0;

    for s in [&fresh, &played] {
        let mut ids: Vec<ContentId> = Vec::new();
        ids.extend(StructureId::ALL.iter().map(|x| ContentId::Structure(*x)));
        ids.extend(IdeaId::ALL.iter().map(|x| ContentId::Idea(*x)));
        ids.extend(CraftId::ALL.iter().map(|x| ContentId::Craft(*x)));
        ids.extend(JobId::ALL.iter().map(|x| ContentId::Job(*x)));
        ids.extend(RiteId::ALL.iter().map(|x| ContentId::Rite(*x)));
        ids.extend(ArchId::ALL.iter().map(|x| ContentId::Arch(*x)));
        ids.extend(RefinementId::ALL.iter().map(|x| ContentId::Refinement(*x)));
        ids.extend(RuleId::ALL.iter().map(|x| ContentId::Rule(*x)));
        ids.extend(TraditionId::ALL.iter().map(|x| ContentId::Tradition(*x)));
        ids.extend(DreamLoomId::ALL.iter().map(|x| ContentId::DreamLoom(*x)));

        for id in ids {
            let a = s.availability(id);
            assert!(
                matches!(a, Availability::Hidden | Availability::Teased | Availability::Available),
                "{id:?} must answer with an availability"
            );
        }
    }
}

/// Kinds with no reveal ladder have no Teased tier to report — they are
/// either there to act on or not there at all. A caller may rely on that:
/// dimming a Teased row is a thing it only ever has to do for the two
/// reveal-tracked kinds.
// @spec SIM-ENG-032
#[test]
fn kinds_without_a_reveal_ladder_are_never_teased() {
    let fresh = GameState::new(22);

    // A played-in meadow where the ladderless kinds are open but the stock to
    // act on them is missing — the very shape that would read Teased if these
    // kinds had a middle state.
    let mut open_but_broke = GameState::new(23);
    open_but_broke.ideas.insert(IdeaId::Tinkering);
    open_but_broke.ideas.insert(IdeaId::CouchSpelunking);
    open_but_broke.ideas.insert(IdeaId::ParadeRoutes);
    open_but_broke.ideas.insert(IdeaId::GroomedToPurrfection);
    open_but_broke.peak_devotion = 1000.0;
    assert_eq!(open_but_broke.resources, [0.0; RESOURCES], "nothing in the pantry");

    for s in [&fresh, &open_but_broke] {
        for id in one_of_each() {
            if id.is_reveal_tracked() {
                continue;
            }
            assert_ne!(
                s.availability(id),
                Availability::Teased,
                "{id:?} carries no reveal ladder, so it has no Teased state to report"
            );
        }
    }

    // The contrast, in one state: totality must not flatten the kinds that do
    // carry a ladder. A revealed-but-unaffordable structure still reads Teased
    // while the ladderless kinds beside it never can.
    let mut laddered = GameState::new(26);
    laddered.ideas.insert(IdeaId::Tinkering);
    give(&mut laddered, ResourceId::Catnip, 4.0); // past the reveal fraction, short of the price
    run_ticks(&mut laddered, 1);
    assert_eq!(
        laddered.availability(ContentId::Structure(StructureId::CatnipPatch)),
        Availability::Teased,
        "a reveal-tracked kind keeps all three states"
    );
    assert_ne!(
        laddered.availability(ContentId::Craft(CraftId::Wands)),
        Availability::Teased,
        "a ladderless kind in the same state still has only two"
    );
}

/// The two states, on kinds that have exactly two: shut gate reads Hidden,
/// open gate with the wherewithal reads Available.
// @spec SIM-ENG-032
#[test]
fn a_ladderless_kind_opens_from_hidden_to_available_with_its_gate() {
    let mut s = GameState::new(24);
    adopt(&mut s, 1); // one idle home kitten, for the job
    s.kittens[0].job = None;

    let cases: Vec<ContentId> = vec![
        ContentId::Job(JobId::Groomer),
        ContentId::Craft(CraftId::Wands),
        ContentId::Refinement(RefinementId::PebbleCombs),
        ContentId::Rite(RiteId::MorningStretch),
        ContentId::Arch(ArchId::RibbonArch),
    ];
    for id in &cases {
        assert_eq!(
            s.availability(*id),
            Availability::Hidden,
            "{id:?} is not there at all until its gate opens"
        );
    }

    // Open every gate, and stock enough that the item is actionable outright.
    s.ideas.insert(IdeaId::GroomedToPurrfection);
    s.ideas.insert(IdeaId::Tinkering);
    s.ideas.insert(IdeaId::CouchSpelunking);
    s.ideas.insert(IdeaId::ParadeRoutes);
    s.peak_devotion = 1000.0;
    give(&mut s, ResourceId::Twigs, 500.0);
    give(&mut s, ResourceId::Pebbles, 500.0);
    give(&mut s, ResourceId::Curiosity, 500.0);
    give(&mut s, ResourceId::Warmth, 500.0);
    give(&mut s, ResourceId::Rainbows, 1000.0);

    for id in &cases {
        assert_eq!(
            s.availability(*id),
            Availability::Available,
            "{id:?} is open and actionable, so it is simply there"
        );
    }
}

/// SIM-ENG-030's real teeth: the pipeline has a stock-moving phase between
/// production and consumption, and every other fixture here leaves it dormant.
///
/// This is a regression guard with a story. The first implementation derived
/// the rate by calling the production and consumption phases by name — which
/// *looks* like derivation but is a second, hand-curated list of the pipeline's
/// structure, and it silently omitted the phase in between. Every test above
/// passed, because none of them enabled it. The spec's stated fear ("it would
/// drift the moment any leaf adds a multiplier") had already come true before
/// the first player ever ran the game.
///
/// If someone adds a stock-moving phase and wires it only into the tick, this
/// test fails and the one above it does not.
// @spec SIM-ENG-030
#[test]
fn the_rate_covers_every_stock_moving_phase_not_just_the_obvious_two() {
    let mut s = GameState::new(4242);
    give(&mut s, ResourceId::Catnip, 500.0);
    grant(&mut s, StructureId::CatnipPatch, 3);
    adopt(&mut s, 2);

    // Wake the between-phase, which is dormant while no arch is owned.
    s.arches_bought.insert(ArchId::SkyArch);

    let mut moved_by_the_between_phase = false;
    let mut probe = s.clone();
    let before = probe.stock(ResourceId::Prisms);
    probe.tick();
    if (probe.stock(ResourceId::Prisms) - before).abs() > 1e-12 {
        moved_by_the_between_phase = true;
    }
    assert!(
        moved_by_the_between_phase,
        "fixture must actually drive the between-phase, or this guard proves nothing"
    );

    agrees_with_tick(&s, &[ResourceId::Prisms]);
}
