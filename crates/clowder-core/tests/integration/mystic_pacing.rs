//! Mystic pacing: the Rainbow Arch ladder timed against the mystic design's
//! reference clowder (SIM-UNI-018), the supply it leaves for refining
//! (SIM-UNI-019), and the ladder's stardust against storage (SIM-UNI-020).
//!
//! Climbing the ladder takes weeks of play, which is millions of ticks and far
//! past the core suite's budget, so the climb is timed with a model that steps
//! from one Parade to the next. Every rate it uses is the sim's own: unicorns
//! and the steady prism drift from the net-rate and flows queries, the
//! day-boundary chances from their queries (taken at their expectation), the
//! dream-drift from one real tick of a probe, and every Parade and purchase
//! through the command API. A real tick closes each step, so the dreaming rule
//! runs as written. The calibration test holds the model to the real tick over
//! a day of play.

use crate::common;

use clowder_core::content::{self, arch_def, structure_def};
use clowder_core::*;
use common::*;

/// Ticks in one hour of 1× play.
const HOUR: u64 = content::TICKS_PER_SECOND * 3600;
/// Ticks in one day of 1× play.
const DAY: u64 = 24 * HOUR;

/// The mystic design's reference clowder: at the Parade Routes idea with its
/// free Rainbow Arch, nineteen Unicorn Meadows, the Dream Tending idea, and
/// 15000 cairns.
fn reference_clowder() -> GameState {
    let mut s = GameState::new(53);
    for idea in [
        IdeaId::UnicornLore,
        IdeaId::ParadeRoutes,
        IdeaId::DreamTending,
    ] {
        s.ideas.insert(idea);
    }
    grant(&mut s, StructureId::UnicornMeadow, 19);
    grant(&mut s, StructureId::RainbowArch, 1);
    give(&mut s, ResourceId::Cairns, 15000.0);
    s
}

/// Add to a stock (`give` sets one).
fn add(s: &mut GameState, r: ResourceId, amount: f64) {
    let held = s.stock(r);
    give(s, r, held + amount);
}

/// What an Arch upgrade prices in one resource.
fn price_in(arch: ArchId, r: ResourceId) -> f64 {
    arch_def(arch)
        .cost
        .iter()
        .filter(|(x, _)| *x == r)
        .map(|(_, n)| *n)
        .sum()
}

/// The reference clowder holds the ladder's shinies and stardust in hand. They
/// are capped stores a real tick trims, so they are put back before each turn.
fn hold_outside_materials(s: &mut GameState) {
    for r in [ResourceId::Shinies, ResourceId::Stardust] {
        let need: f64 = ArchId::ALL
            .into_iter()
            .filter(|a| !s.arches_bought.contains(a))
            .map(|a| price_in(a, r))
            .sum();
        if s.stock(r) < need {
            give(s, r, need);
        }
    }
}

/// One turn of the reference player: hold every Parade it has the unicorns
/// for, buy the next Arch upgrade if it can, then build Rainbow Arches while it
/// can, keeping back the cairns the Everdream Arch asks.
fn take_turn(s: &mut GameState) -> Option<ArchId> {
    hold_outside_materials(s);
    while s.stock(ResourceId::Unicorns) >= content::PARADE_UNICORN_COST {
        s.apply(Command::HoldParade).unwrap();
    }
    let next = ArchId::ALL
        .into_iter()
        .find(|a| !s.arches_bought.contains(a));
    let bought = next.filter(|a| s.apply(Command::BuyArch(*a)).is_ok());
    let keep = if s.arches_bought.contains(&ArchId::EverdreamArch) {
        0.0
    } else {
        price_in(ArchId::EverdreamArch, ResourceId::Cairns)
    };
    loop {
        let mut built = s.clone();
        if built
            .apply(Command::Build(StructureId::RainbowArch))
            .is_err()
            || built.stock(ResourceId::Cairns) < keep
        {
            break;
        }
        *s = built;
    }
    bought
}

/// Advance the model by `n` ticks (at least one): every rate read from the sim
/// at the current state, credited for `n − 1` ticks, then one real tick.
fn advance_by(s: &mut GameState, n: u64) {
    let per_second = content::TICKS_PER_SECOND as f64;
    let credited = (n - 1) as f64;
    let seconds = credited / per_second;
    let days = credited / content::TICKS_PER_DAY as f64;
    let unicorns = s.net_rate(ResourceId::Unicorns) * seconds;
    let steady_prisms: f64 = s
        .flows(ResourceId::Prisms)
        .iter()
        .map(|(_, rate)| rate.max(0.0))
        .sum::<f64>()
        * seconds;
    let descents = s.prism_descent_chance() * days;
    let showers = s.rainbow_shower_chance() * days * content::RAINBOW_SHOWER_AMOUNT;
    let mut probe = s.clone();
    probe.dream_drift = 0.0;
    probe.tick();
    let drift = probe.dream_drift * credited;

    add(s, ResourceId::Unicorns, unicorns);
    add(s, ResourceId::Prisms, steady_prisms + descents);
    add(s, ResourceId::Rainbows, showers);
    s.dream_drift += drift;
    s.ticks += n - 1;
    s.tick();
}

/// Advance to the tick the next Parade can be held, or an hour, whichever
/// comes first. Returns the ticks advanced.
fn advance(s: &mut GameState) -> u64 {
    let per_tick = s.net_rate(ResourceId::Unicorns) / content::TICKS_PER_SECOND as f64;
    let short = content::PARADE_UNICORN_COST - s.stock(ResourceId::Unicorns);
    let to_parade = if per_tick > 0.0 {
        (short / per_tick).ceil().max(1.0) as u64
    } else {
        HOUR
    };
    let n = to_parade.min(HOUR);
    advance_by(s, n);
    n
}

/// Play the reference clowder up the ladder for at most `limit` ticks,
/// returning each upgrade with the tick (from the start) it was bought at.
fn climb(s: &mut GameState, limit: u64) -> Vec<(ArchId, u64)> {
    let mut bought = vec![];
    let mut elapsed = 0;
    while bought.len() < ArchId::ALL.len() && elapsed <= limit {
        if let Some(a) = take_turn(s) {
            bought.push((a, elapsed));
        }
        elapsed += advance(s);
    }
    bought
}

fn in_days(ticks: u64) -> String {
    format!("{:.1} d", ticks as f64 / DAY as f64)
}

/// The reference clowder buys each Arch upgrade within ten days of the one
/// before, and the whole ladder in three to seven weeks.
// @spec SIM-UNI-018
#[test]
fn the_reference_clowder_climbs_the_ladder_a_few_days_a_step() {
    let mut s = reference_clowder();
    let steps = climb(&mut s, 10 * 7 * DAY);
    let shown: Vec<String> = steps
        .iter()
        .map(|(a, at)| format!("{a:?} at {}", in_days(*at)))
        .collect();
    assert_eq!(
        steps.len(),
        ArchId::ALL.len(),
        "the whole ladder within ten weeks: {shown:?}"
    );
    let mut before = 0;
    for (arch, at) in &steps {
        assert!(
            at - before <= 10 * DAY,
            "{arch:?} came {} after the step before: {shown:?}",
            in_days(at - before)
        );
        before = *at;
    }
    let (_, top) = steps[steps.len() - 1];
    assert!(
        (3 * 7 * DAY..=7 * 7 * DAY).contains(&top),
        "the top of the ladder at {}: {shown:?}",
        in_days(top)
    );
}

/// At the top of the ladder a day of play brings prisms enough to refine four
/// echoes.
// @spec SIM-UNI-019
#[test]
fn the_top_of_the_ladder_brings_prisms_for_four_echoes_a_day() {
    let mut s = reference_clowder();
    let steps = climb(&mut s, 10 * 7 * DAY);
    assert_eq!(steps.len(), ArchId::ALL.len(), "the ladder climbed");
    for r in [
        ResourceId::Prisms,
        ResourceId::Starshards,
        ResourceId::Echoes,
    ] {
        give(&mut s, r, 0.0);
    }
    let mut elapsed = 0;
    while elapsed < DAY {
        take_turn(&mut s);
        elapsed += advance(&mut s);
    }
    while s.apply(Command::RefineStarshard).is_ok() {}
    while s.apply(Command::RefineEcho).is_ok() {}
    assert!(
        s.stock(ResourceId::Echoes) >= 4.0,
        "a day's prisms refined into {} echoes",
        s.stock(ResourceId::Echoes)
    );
}

/// The model, stepping an hour at a time as the climb does at most, matches a
/// day of real ticks: unicorns and dreaming exactly as the tick runs them, and
/// the day-boundary chances at their expectation.
// @spec SIM-UNI-018
#[test]
fn the_pacing_model_keeps_to_the_real_tick() {
    let mut s = reference_clowder();
    grant(&mut s, StructureId::RainbowArch, 29);
    for a in [
        ArchId::RibbonArch,
        ArchId::CrystalArch,
        ArchId::PrismArch,
        ArchId::SkyArch,
        ArchId::MemoryArch,
    ] {
        s.arches_bought.insert(a);
    }
    give(&mut s, ResourceId::Prisms, 1000.0);
    // A dreamcorn already dreams, so the drift runs at its throttled rate
    // throughout and the two meet the same whole dreamcorns.
    give(&mut s, ResourceId::Dreamcorns, 1.0);
    let mut real = s.clone();
    let mut model = s.clone();
    run_ticks(&mut real, DAY);
    for _ in 0..24 {
        advance_by(&mut model, HOUR);
    }

    assert_eq!(real.ticks, model.ticks);
    let close = |r: ResourceId, share: f64| {
        let (a, b) = (real.stock(r) - s.stock(r), model.stock(r) - s.stock(r));
        assert!(
            (a - b).abs() <= share * a.abs().max(1.0),
            "{r:?} gained: real {a}, model {b}"
        );
    };
    close(ResourceId::Unicorns, 1e-9);
    // Dreaming: the same whole dreamcorns, the drift within a tick's worth.
    assert_eq!(
        real.stock(ResourceId::Dreamcorns),
        model.stock(ResourceId::Dreamcorns)
    );
    assert!((real.dream_drift - model.dream_drift).abs() < 1e-4);
    // Descents and showers are drawn by chance; the model takes their
    // expectation, which a day of real draws meets within its spread.
    close(ResourceId::Prisms, 0.1);
    close(ResourceId::Rainbows, 0.35);
}

/// No Arch upgrade asks more stardust than four Star Basins store.
// @spec SIM-UNI-020
#[test]
fn no_arch_upgrade_prices_more_stardust_than_four_star_basins_store() {
    let per_basin: f64 = structure_def(StructureId::StarBasin)
        .cap_bonuses
        .iter()
        .filter(|(r, _)| *r == ResourceId::Stardust)
        .map(|(_, n)| *n)
        .sum();
    assert!(per_basin > 0.0, "a Star Basin stores stardust");
    for a in ArchId::ALL {
        assert!(
            price_in(a, ResourceId::Stardust) <= 4.0 * per_basin,
            "{a:?} asks {} stardust; four Star Basins store {}",
            price_in(a, ResourceId::Stardust),
            4.0 * per_basin
        );
    }
}
