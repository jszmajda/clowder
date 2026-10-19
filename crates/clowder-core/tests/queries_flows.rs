//! Derived queries: a store's net rate taken apart by what moves it.
//!
//! @spec SIM-ENG-062, SIM-ENG-063, SIM-ENG-064, SIM-ENG-065, SIM-ENG-066,
//!       SIM-ENG-067, SIM-ENG-068
//!
//! The flows are only worth reading if each figure is the share its source
//! really contributes. So beside the bookkeeping checks — the flows add up to
//! the net rate, in a fixed order — the attribution itself is checked by a
//! differential: take one source away and the net rate moves by exactly the
//! figure credited to it. A credit landing on the wrong source passes the sum
//! and fails the differential.

mod common;

use clowder_core::content;
use clowder_core::*;
use common::*;

/// Float residue guard, scaled to the magnitudes being compared.
fn agree(a: f64, b: f64, scale: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + scale.abs())
}

fn per_second() -> f64 {
    content::TICKS_PER_SECOND as f64
}

/// A meadow mid-spring-day with every kind of source live at once: structures
/// that produce, a converter with a refinement-granted extra output, a
/// structure that only boosts another store, workers at three yielding jobs, a
/// tinkerer bound to a recipe, an arch upgrade's drift, and kittens eating and
/// using a luxury. Stocks are ample, so no throttle redistributes an input.
fn busy(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    adopt(&mut s, 4);
    s.zoomies_until = 0;
    s.jamboree_until = 0;
    s.deja_vu_until = 0;
    s.pending_sky_event = None;
    s.grumpy = false;
    s.mess = 0.0;
    grant(&mut s, StructureId::CatnipPatch, 3);
    grant(&mut s, StructureId::Hearth, 2);
    grant(&mut s, StructureId::GravelPit, 1);
    s.refinements.insert(RefinementId::Bellows);
    s.arches_bought.insert(ArchId::SkyArch);
    s.kittens[0].job = Some(JobId::Gardener);
    s.kittens[1].job = Some(JobId::TwigCollector);
    s.kittens[2].job = Some(JobId::Rummager);
    s.kittens[3].job = Some(JobId::Tinkerer);
    s.kittens[3].tinkering = Some(CraftId::Wands);
    for k in &s.kittens {
        assert_eq!(
            k.location,
            Location::Home,
            "fixture wants every kitten home"
        );
    }
    give(&mut s, ResourceId::Catnip, 100.0);
    give(&mut s, ResourceId::Twigs, 500.0);
    give(&mut s, ResourceId::Pebbles, 500.0);
    give(&mut s, ResourceId::Feathers, 10.0);
    s.ticks = 1002; // mid-day 100 — Spring, no boundary on the next tick
    s
}

/// A change to a meadow that takes one source away.
type Removal = Box<dyn Fn(&mut GameState)>;

fn sources(flows: &[(FlowSource, f64)]) -> Vec<FlowSource> {
    flows.iter().map(|(src, _)| *src).collect()
}

fn figure(flows: &[(FlowSource, f64)], src: FlowSource) -> f64 {
    flows
        .iter()
        .find(|(s, _)| *s == src)
        .map(|(_, f)| *f)
        .unwrap_or_else(|| panic!("{src:?} is not among the flows {flows:?}"))
}

// ---------------------------------------------------------------------------
// SIM-ENG-062, SIM-ENG-064 — the flows are the net rate, taken apart
// ---------------------------------------------------------------------------

// @spec SIM-ENG-062, SIM-ENG-064
#[test]
fn every_resources_flows_sum_to_its_net_rate() {
    let s = busy(1);
    let rates = s.net_rates();
    for r in ResourceId::ALL {
        let flows = s.flows(r);
        let sum: f64 = flows.iter().map(|(_, f)| f).sum();
        assert!(
            agree(sum, rates[r.ix()], s.stock(r)),
            "{r:?}: flows {flows:?} sum to {sum}, but the net rate is {}",
            rates[r.ix()]
        );
    }
    // The fixture is alive: every store it exists to exercise is moving.
    for r in [
        ResourceId::Catnip,
        ResourceId::Twigs,
        ResourceId::Pebbles,
        ResourceId::Buttons,
        ResourceId::Embers,
        ResourceId::Wands,
        ResourceId::Feathers,
        ResourceId::Prisms,
    ] {
        assert!(!s.flows(r).is_empty(), "{r:?} has no flows in the fixture");
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-063 — each move is credited to the one source that makes it
// ---------------------------------------------------------------------------

/// Which sources move which stores, and which way.
// @spec SIM-ENG-063
#[test]
fn each_move_is_credited_to_the_source_that_makes_it() {
    use FlowSource::*;
    let s = busy(2);
    let expect: &[(ResourceId, &[(FlowSource, f64)])] = &[
        (
            ResourceId::Catnip,
            &[
                (Structure(StructureId::CatnipPatch), 1.0),
                (Job(JobId::Gardener), 1.0),
                (Kittens, -1.0),
            ],
        ),
        (
            ResourceId::Twigs,
            &[
                (Structure(StructureId::Hearth), -1.0),
                (Job(JobId::TwigCollector), 1.0),
                (Craft(CraftId::Wands), -1.0),
            ],
        ),
        (
            ResourceId::Pebbles,
            &[
                (Structure(StructureId::Hearth), -1.0),
                (Job(JobId::Rummager), 1.0),
            ],
        ),
        (
            ResourceId::Buttons,
            &[(Structure(StructureId::Hearth), 1.0)],
        ),
        (
            ResourceId::Embers,
            &[
                (Structure(StructureId::GravelPit), 1.0),
                (Structure(StructureId::Hearth), 1.0),
            ],
        ),
        (ResourceId::Wands, &[(Craft(CraftId::Wands), 1.0)]),
        (ResourceId::Feathers, &[(Kittens, -1.0)]),
        (ResourceId::Prisms, &[(Arch(ArchId::SkyArch), 1.0)]),
    ];
    for (r, want) in expect {
        let flows = s.flows(*r);
        let mut got = sources(&flows);
        got.sort();
        let mut wanted: Vec<FlowSource> = want.iter().map(|(src, _)| *src).collect();
        wanted.sort();
        assert_eq!(got, wanted, "{r:?}: wrong sources in {flows:?}");
        for (src, sign) in *want {
            let f = figure(&flows, *src);
            assert!(
                f * sign > 0.0,
                "{r:?}: {src:?} should {} the store, but its figure is {f}",
                if *sign > 0.0 { "add to" } else { "draw down" }
            );
        }
    }
}

/// Take a source away and the net rate of each store it is credited with
/// moves by exactly the figure credited to it — so each figure is the
/// source's real share, not merely a number that makes the sum come out.
///
/// The stores compared are the source's own, and none of them is moved by a
/// second source that the removal would also change. Mood couples sources
/// otherwise: the busy meadow's hearth lifts it, and a worker taken off a job
/// shifts it, and either change reaches every worker's yield — that share is
/// inside the workers' flows (SIM-ENG-065). So a kiln, which lifts nothing,
/// stands in as the converter, and no two yielding jobs move one store.
// @spec SIM-ENG-063
#[test]
fn removing_a_source_moves_the_net_rate_by_its_credited_figure() {
    use FlowSource::*;
    let mut base = busy(3);
    base.structures[StructureId::Hearth.ix()] = 0;
    base.structures[StructureId::GravelPit.ix()] = 0;
    base.refinements.remove(&RefinementId::Bellows);
    grant(&mut base, StructureId::Kiln, 2);
    give(&mut base, ResourceId::Dew, 500.0);
    let cases: Vec<(FlowSource, Removal)> = vec![
        (
            Structure(StructureId::CatnipPatch),
            Box::new(|s| s.structures[StructureId::CatnipPatch.ix()] = 0),
        ),
        (
            Structure(StructureId::Kiln),
            Box::new(|s| s.structures[StructureId::Kiln.ix()] = 0),
        ),
        (Job(JobId::Gardener), Box::new(|s| s.kittens[0].job = None)),
        (
            Job(JobId::TwigCollector),
            Box::new(|s| s.kittens[1].job = None),
        ),
        (Job(JobId::Rummager), Box::new(|s| s.kittens[2].job = None)),
        (
            Craft(CraftId::Wands),
            Box::new(|s| s.kittens[3].tinkering = None),
        ),
        (
            Arch(ArchId::SkyArch),
            Box::new(|s| {
                s.arches_bought.remove(&ArchId::SkyArch);
            }),
        ),
    ];
    for (src, remove) in cases {
        let mut without = base.clone();
        remove(&mut without);
        let mut compared = 0;
        for r in ResourceId::ALL {
            let credited = base
                .flows(r)
                .iter()
                .find(|(s, _)| *s == src)
                .map(|(_, f)| *f)
                .unwrap_or(0.0);
            if credited == 0.0 {
                continue;
            }
            let moved = base.net_rate(r) - without.net_rate(r);
            assert!(
                agree(credited, moved, base.stock(r)),
                "{src:?} on {r:?}: credited {credited}, but removing it moved the \
                 net rate by {moved}"
            );
            compared += 1;
        }
        assert!(compared > 0, "{src:?} moved nothing in the fixture");
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-064 — credits are what the stock actually moved
// ---------------------------------------------------------------------------

/// Where a phase floors a stock at zero, the credit is what was there to
/// take, not the demand: the kittens can eat no more catnip than the pantry
/// holds, nor use more feathers than the shelf has.
// @spec SIM-ENG-064
#[test]
fn a_move_cut_short_by_a_floor_is_credited_at_what_it_moved() {
    let mut s = busy(4);
    s.structures[StructureId::CatnipPatch.ix()] = 0;
    s.kittens[0].job = None;
    give(&mut s, ResourceId::Catnip, 0.0001);
    give(&mut s, ResourceId::Feathers, 0.0001);
    for r in [ResourceId::Catnip, ResourceId::Feathers] {
        let flows = s.flows(r);
        assert_eq!(
            sources(&flows),
            vec![FlowSource::Kittens],
            "{r:?}: {flows:?}"
        );
        let f = figure(&flows, FlowSource::Kittens);
        assert!(
            agree(f, -0.0001 * per_second(), 1.0),
            "{r:?}: kittens credited {f}, but only 0.0001 was there to take"
        );
        assert!(
            agree(f, s.net_rate(r), 1.0),
            "{r:?}: credit {f} vs net rate"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-065 — a multiplier is never a source
// ---------------------------------------------------------------------------

/// A structure that raises another source's output, and makes none of that
/// store itself, has no flow there: its boost is inside the flow it raises.
// @spec SIM-ENG-065
#[test]
fn a_boost_counts_inside_the_flow_it_raises() {
    use FlowSource::*;
    let with = busy(5);
    let mut without = with.clone();
    without.structures[StructureId::GravelPit.ix()] = 0;

    let boosted = with.flows(ResourceId::Pebbles);
    let plain = without.flows(ResourceId::Pebbles);
    assert_eq!(
        sources(&boosted),
        sources(&plain),
        "the booster changed which sources move pebbles: {boosted:?} vs {plain:?}"
    );
    assert!(!sources(&boosted).contains(&Structure(StructureId::GravelPit)));
    let raised = figure(&boosted, Job(JobId::Rummager));
    let unraised = figure(&plain, Job(JobId::Rummager));
    assert!(
        raised > unraised + 1e-9,
        "the boost should be inside the rummagers' figure ({raised} vs {unraised})"
    );
    assert!(agree(
        raised - unraised,
        with.net_rate(ResourceId::Pebbles) - without.net_rate(ResourceId::Pebbles),
        1.0
    ));
}

// ---------------------------------------------------------------------------
// SIM-ENG-066 — a fixed order, and nothing that moved nothing
// ---------------------------------------------------------------------------

fn listing_place(src: FlowSource) -> (usize, usize) {
    use FlowSource::*;
    fn pos<T: PartialEq>(all: &[T], x: &T) -> usize {
        all.iter().position(|y| y == x).expect("in its table")
    }
    match src {
        Structure(s) => (0, pos(&StructureId::ALL, &s)),
        Job(j) => (1, pos(&JobId::ALL, &j)),
        Kittens => (2, 0),
        Craft(c) => (3, pos(&CraftId::ALL, &c)),
        Arch(a) => (4, pos(&ArchId::ALL, &a)),
    }
}

// @spec SIM-ENG-066
#[test]
fn flows_stand_in_the_fixed_order_and_none_moved_nothing() {
    let s = busy(6);
    for r in ResourceId::ALL {
        let flows = s.flows(r);
        let places: Vec<(usize, usize)> =
            flows.iter().map(|(src, _)| listing_place(*src)).collect();
        assert!(
            places.windows(2).all(|w| w[0] < w[1]),
            "{r:?}: flows out of the fixed order: {flows:?}"
        );
        for (src, f) in &flows {
            assert!(*f != 0.0, "{r:?}: {src:?} is listed having moved nothing");
        }
    }
}

/// A converter starved of an input runs at nothing, and is then no source.
// @spec SIM-ENG-066
#[test]
fn a_starved_converter_is_not_listed() {
    let mut s = busy(7);
    give(&mut s, ResourceId::Pebbles, 0.0);
    s.kittens[2].job = None; // nobody rummages up more
    let hearth = FlowSource::Structure(StructureId::Hearth);
    for r in [ResourceId::Twigs, ResourceId::Pebbles, ResourceId::Buttons] {
        assert!(
            !sources(&s.flows(r)).contains(&hearth),
            "{r:?}: a starved hearth is listed: {:?}",
            s.flows(r)
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-067 — speed-free, like the net rate
// ---------------------------------------------------------------------------

// @spec SIM-ENG-067
#[test]
fn flows_do_not_read_the_speed() {
    let slow = busy(8);
    let mut fast = slow.clone();
    fast.speed = 8.0;
    for r in ResourceId::ALL {
        assert_eq!(
            slow.flows(r),
            fast.flows(r),
            "{r:?}: speed reached the flows"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-068 — asking changes nothing; a tick keeps no record
// ---------------------------------------------------------------------------

// @spec SIM-ENG-068
#[test]
fn asking_for_flows_changes_nothing_and_a_tick_is_the_same_either_way() {
    let s = busy(9);
    let before = s.clone();
    for r in ResourceId::ALL {
        let _ = s.flows(r);
    }
    assert_eq!(s, before, "asking for flows changed the game state");

    let mut asked = s.clone();
    let mut unasked = s.clone();
    for _ in 0..50 {
        for r in ResourceId::ALL {
            let _ = asked.flows(r);
        }
        asked.tick();
        unasked.tick();
    }
    assert_eq!(
        asked, unasked,
        "asking for flows changed what the ticks did"
    );
}
