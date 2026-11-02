//! SIM-ENG-018 fixes the order of a day boundary's random draws, and a seed's
//! future depends on it. A golden number would fail on every balance change, so
//! this reads each chance from the state instead and walks a twin of the
//! generator through the schedule as the spec orders it. Any draw reordered,
//! added, or dropped leaves the real generator somewhere the twin isn't, or
//! makes a roll land differently, whatever the balance numbers are.

use clowder_core::content;
use clowder_core::*;

/// Runs of the sweep: enough for every roll to land both ways.
const SEEDS: u64 = 6000;

/// What one day boundary should do, by the spec's order.
#[derive(Debug, Default)]
struct Expected {
    arrival: Option<(String, KittenTrait)>,
    zoomies: bool,
    flavor: Option<(usize, Option<String>)>,
    sky_observed: bool,
    sky_visible: bool,
    prism: bool,
    deja_vu: Option<f64>,
}

fn sum(s: &GameState, per_copy: impl Fn(&content::StructureDef) -> f64) -> f64 {
    StructureId::ALL
        .into_iter()
        .map(|b| per_copy(content::structure_def(b)) * f64::from(s.structures[b.ix()]))
        .sum()
}

/// Walks a twin of the generator through the schedule, in the spec's order,
/// from the state as it stands before the boundary's tick.
fn walk(s: &GameState) -> (Pcg32, Expected) {
    let mut rng = s.rng.clone();
    let mut out = Expected::default();

    // Arrival roll, then name and trait only on success. Pity forces success
    // with no draw, and an ineligible check draws nothing.
    let eligible =
        s.free_slots() >= 1 && s.stock(ResourceId::Catnip) >= content::ARRIVAL_MIN_CATNIP;
    if eligible && (s.pity >= content::PITY_THRESHOLD || rng.chance(s.arrival_chance())) {
        let taken: Vec<String> = s.kittens.iter().map(|k| k.name.to_lowercase()).collect();
        let names: Vec<&str> = content::NAME_POOL
            .iter()
            .copied()
            .filter(|n| !taken.contains(&n.to_lowercase()))
            .collect();
        let name = names[rng.index(names.len())].to_string();
        let t = KittenTrait::ALL[rng.index(KittenTrait::ALL.len())];
        out.arrival = Some((name, t));
    }

    // Zoomies start, only where none run.
    if s.zoomies_until == 0 && rng.chance(content::ZOOMIES_DAILY_CHANCE) {
        out.zoomies = true;
    }

    // Flavor roll, then its template, then its kitten where the template asks.
    if rng.chance(content::FLAVOR_DAILY_CHANCE) {
        let template = rng.index(content::FLAVOR_TEMPLATES.len());
        let kitten = content::FLAVOR_TEMPLATES[template].needs_kitten.then(|| {
            let mut homes: Vec<String> = s
                .kittens
                .iter()
                .filter(|k| k.location == Location::Home)
                .map(|k| k.name.clone())
                .collect();
            homes.extend(out.arrival.iter().map(|(n, _)| n.clone()));
            homes[rng.index(homes.len())].clone()
        });
        out.flavor = Some((template, kitten));
    }

    // Sky roll, then auto-observe on a hit, with nothing drawn while an event
    // waits or where nothing seeds one.
    let seeded = sum(s, |d| d.sky_event_chance);
    if seeded != 0.0
        && s.pending_sky_event.is_none()
        && rng.chance(seeded * (1.0 + s.effect(Effect::SkyEventChanceRatio)))
    {
        let observe =
            sum(s, |d| d.sky_auto_observe) * (1.0 + s.effect(Effect::AutoObserveChanceRatio));
        if rng.chance(observe) {
            out.sky_observed = true;
        } else {
            out.sky_visible = true;
        }
    }

    // Prism descent, then rainbow shower, where a structure seeds one or an arch
    // upgrade stands.
    if sum(s, |d| d.prism_descent) > 0.0 || !s.arches_bought.is_empty() {
        let descent = s.prism_descent_chance();
        if descent > 0.0 && rng.chance(descent) {
            out.prism = true;
        }
        let shower = s.rainbow_shower_chance();
        if shower > 0.0 {
            rng.chance(shower);
        }
    }

    // A season's turn appends the déjà-vu roll, and its amount on a hit.
    let day = (s.ticks + 1) / content::TICKS_PER_DAY;
    if day.is_multiple_of(content::DAYS_PER_SEASON)
        && (sum(s, |d| d.deja_vu_chance) > 0.0 || !s.dream_looms.is_empty())
    {
        let chance = s.effect(Effect::DejaVuChance);
        if chance > 0.0 && rng.chance(chance) {
            let amount = (content::DEJA_VU_REVERIE_BASE
                + rng.uniform() * content::DEJA_VU_REVERIE_SPREAD)
                * (1.0 + s.effect(Effect::ReverieYield));
            out.deja_vu = Some(amount);
        }
    }
    (rng, out)
}

/// A meadow one tick short of a day boundary, varied by seed so every guard in
/// the schedule is met as well as every roll.
fn boundary(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    let seasons = 4 + seed % 8;
    let on_the_turn = !seed.is_multiple_of(3);
    let day = seasons * content::DAYS_PER_SEASON + u64::from(!on_the_turn);
    s.ticks = day * content::TICKS_PER_DAY - 1;
    for r in [ResourceId::Catnip, ResourceId::Twigs] {
        s.resources[r.ix()] = 1000.0;
    }
    s.structures[StructureId::CozyCushion.ix()] = 8;
    s.structures[StructureId::CatnipPatch.ix()] = 50;
    s.structures[StructureId::StargazingLedge.ix()] = 30;
    s.structures[StructureId::RainbowArch.ix()] = 1000;
    s.structures[StructureId::MemoryChest.ix()] = 20;
    s.arches_bought.insert(ArchId::PrismArch);
    s.kittens = (0..3)
        .map(|i| Kitten {
            name: format!("Test Kitten {i}"),
            job: None,
            location: Location::Home,
            arrived_day: 1,
            job_seq: None,
            away_seq: None,
            skills: [0.0; 8],
            exp: 0.0,
            rank: 0,
            trait_id: KittenTrait::None,
            tinkering: None,
            profile: None,
        })
        .collect();
    if seed.is_multiple_of(7) {
        s.pity = content::PITY_THRESHOLD;
    }
    if seed.is_multiple_of(11) {
        s.zoomies_until = s.ticks + 1000;
    }
    if seed.is_multiple_of(13) {
        s.pending_sky_event = Some(s.ticks + 1000);
    }
    if seed.is_multiple_of(17) {
        s.structures[StructureId::CozyCushion.ix()] = 0;
    }
    if seed.is_multiple_of(19) {
        s.structures[StructureId::StargazingLedge.ix()] = 0;
        s.structures[StructureId::RainbowArch.ix()] = 0;
        s.structures[StructureId::MemoryChest.ix()] = 0;
        s.arches_bought.clear();
    }
    s
}

/// One day boundary, on thousands of seeds, draws exactly the spec's schedule:
/// the generator ends where a twin walking that order ends, and each roll lands
/// as the twin's did. Holds every guard and every roll to both of its outcomes.
/// @spec SIM-ENG-018, SIM-UNI-006, SIM-DREAM-004
#[test]
fn a_day_boundary_draws_in_the_order_the_spec_fixes() {
    let mut seen = [0u32; 10];
    for seed in 0..SEEDS {
        let mut s = boundary(seed);
        let (want_rng, want) = walk(&s);
        let reverie = s.stock(ResourceId::Reverie);
        let kittens = s.kittens.len();
        let prisms = s.stock(ResourceId::Prisms);
        if s.free_slots() == 0 {
            seen[0] += 1;
        }
        if s.pity >= content::PITY_THRESHOLD {
            seen[1] += 1;
        }

        let events = s.tick();

        assert_eq!(
            s.rng, want_rng,
            "seed {seed}: the generator moved off the schedule"
        );

        let has = |pred: &dyn Fn(&Event) -> bool| events.iter().any(pred);
        match &want.arrival {
            Some((name, t)) => {
                seen[2] += 1;
                assert_eq!(s.kittens.len(), kittens + 1, "seed {seed}");
                let k = s.kittens.last().unwrap();
                assert_eq!((&k.name, k.trait_id), (name, *t), "seed {seed}");
            }
            None => assert_eq!(s.kittens.len(), kittens, "seed {seed}"),
        }
        assert_eq!(
            has(&|e| matches!(e, Event::ZoomiesStarted { .. })),
            want.zoomies,
            "seed {seed}: zoomies"
        );
        seen[3] += u32::from(want.zoomies);
        let flavor = events.iter().find_map(|e| match e {
            Event::Flavor { template, kitten } => Some((*template, kitten.clone())),
            _ => None,
        });
        assert_eq!(flavor, want.flavor, "seed {seed}: flavor");
        seen[4] += u32::from(want.flavor.is_some());
        assert_eq!(
            has(&|e| matches!(e, Event::SkyObserved { .. })),
            want.sky_observed,
            "seed {seed}: an observation"
        );
        seen[5] += u32::from(want.sky_observed);
        assert_eq!(
            has(&|e| matches!(e, Event::SkyEventVisible)),
            want.sky_visible,
            "seed {seed}: a visible event"
        );
        seen[6] += u32::from(want.sky_visible);
        assert_eq!(
            has(&|e| matches!(e, Event::PrismDescended)),
            want.prism,
            "seed {seed}: a descent"
        );
        seen[7] += u32::from(want.prism);
        if want.prism {
            assert!(s.stock(ResourceId::Prisms) >= prisms + 1.0, "seed {seed}");
        }
        assert_eq!(
            has(&|e| matches!(e, Event::DejaVuDay)),
            want.deja_vu.is_some(),
            "seed {seed}: a déjà-vu day"
        );
        if let Some(amount) = want.deja_vu {
            seen[8] += 1;
            let gained = s.stock(ResourceId::Reverie) - reverie;
            assert!(
                (gained - amount).abs() < 1e-9,
                "seed {seed}: {gained} vs {amount}"
            );
        }
        if s.zoomies_until != 0 && !want.zoomies {
            seen[9] += 1;
        }
    }
    let names = [
        "no room for an arrival",
        "pity",
        "an arrival",
        "zoomies",
        "flavor",
        "an observation",
        "a visible event",
        "a descent",
        "a déjà-vu day",
        "zoomies already running",
    ];
    for (name, n) in names.iter().zip(seen) {
        assert!(n > 0, "fixture: {name} never happened across {SEEDS} seeds");
    }
}
