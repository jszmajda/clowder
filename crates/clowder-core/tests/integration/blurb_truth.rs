//! A blurb is the whole mechanical statement for its kind, so its figures have
//! to be the ones its rule applies. These tests read each figure from the rule
//! itself, a table or a state with the item held, and hold the blurb to it both
//! ways: every figure the rule applies is stated, and nothing else is.

use clowder_core::content::{self, blurb_text};
use clowder_core::*;

/// A figure as a blurb writes it: no trailing zeros, no float noise.
fn plain(v: f64) -> String {
    format!("{}", (v.abs() * 1.0e7).round() / 1.0e7)
}

/// A ratio as a blurb writes it: a share as its percentage.
fn percent(v: f64) -> String {
    plain(v * 100.0)
}

/// The numbers a blurb states, in order.
fn numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || (c == '.' && !cur.is_empty()) {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(cur.trim_end_matches('.').to_string());
            cur.clear();
        }
    }
    out
}

/// Holds one blurb to its figures: each is stated, and no other number is.
fn holds(item: &str, blurb: &str, figures: &[String]) {
    let text = blurb_text(blurb);
    let stated = numbers(&text);
    for want in figures {
        assert!(
            stated.contains(want),
            "{item}: its rule applies {want}, which isn't stated: {text:?}"
        );
    }
    for got in &stated {
        assert!(
            figures.contains(got),
            "{item}: states {got}, which its rule doesn't apply ({figures:?}): {text:?}"
        );
    }
}

/// The figures a tabled effect states, by how its kind of key reads.
fn tabled_figures(key: &Effect, v: f64) -> Vec<String> {
    match key {
        Effect::PounceRatio
        | Effect::MaxCap(_)
        | Effect::HappinessPoints
        | Effect::StructureOutputAdd(..) => vec![plain(v)],
        Effect::SkyEventChanceRatio | Effect::AutoObserveChanceRatio => vec![plain(1.0 + v)],
        Effect::StructureMessDelta(s) => {
            let base = content::structure_def(*s).mess;
            vec![plain(base), plain(base + v)]
        }
        _ => vec![percent(v)],
    }
}

/// Every refinement and house rule states exactly what its table applies.
/// @spec SIM-CUR-016
#[test]
fn a_refinement_or_rule_states_what_its_table_applies() {
    for f in RefinementId::ALL {
        let d = content::refinement_def(f);
        let figures: Vec<String> = d
            .effects
            .iter()
            .flat_map(|(k, v)| tabled_figures(k, *v))
            .collect();
        holds(&format!("{f:?}"), d.blurb, &figures);
    }
    for r in RuleId::ALL {
        let d = content::rule_def(r);
        let figures: Vec<String> = d
            .effects
            .iter()
            .flat_map(|(k, v)| tabled_figures(k, *v))
            .collect();
        holds(&format!("{r:?}"), d.blurb, &figures);
    }
}

/// What holding an item adds to one effect key, read from a state rather than
/// the rule's own table.
fn added(base: &GameState, hold: impl Fn(&mut GameState), key: Effect) -> f64 {
    let mut s = base.clone();
    hold(&mut s);
    s.effect(key) - base.effect(key)
}

/// Every rite states what one copy of it does, at one altar.
/// @spec SIM-CUR-016, SIM-SUN-009
#[test]
fn a_rite_states_what_it_does() {
    use Effect as E;
    use ResourceId as R;
    let mut base = GameState::new(31);
    base.structures[StructureId::SunbeamAltar.ix()] = 1;
    let of = |t: RiteId, key: Effect| {
        added(
            &base,
            |s| {
                s.rites.insert(t);
                s.rite_counts[t.ix()] = 1;
            },
            key,
        )
    };
    let charm_add = E::StructureOutputAdd(StructureId::SunbeamAltar, R::Charm);

    let mut warm = base.clone();
    warm.structures[StructureId::SunRoom.ix()] = 1;
    let plain_cap = warm.cap(R::Warmth);
    assert!(plain_cap > 0.0, "fixture: a warmth cap to multiply");
    let golden_at = |copies: u32| {
        let mut s = warm.clone();
        s.rites.insert(RiteId::GoldenHour);
        s.rite_counts[RiteId::GoldenHour.ix()] = copies;
        s.cap(R::Warmth) / plain_cap
    };
    let golden = golden_at(1);
    let golden_step = golden_at(2) - golden;

    for t in RiteId::ALL {
        let figures: Vec<String> = match t {
            RiteId::MorningStretch => vec![percent(of(t, E::ResourceRatio(R::Warmth)))],
            RiteId::SharedSunpuddle => vec![percent(of(t, E::MaxCapRatio(R::Curiosity)))],
            RiteId::GoldenHour => vec![plain(golden), plain(golden_step)],
            RiteId::WarmWelcome => vec![plain(of(t, E::HappinessPoints))],
            RiteId::StainedSunlight => vec![plain(of(t, charm_add))],
            RiteId::GrandBasking => {
                vec![plain(of(t, E::MaxCap(R::Charm))), plain(of(t, charm_add))]
            }
            RiteId::SunWardensOath => vec![plain(of(t, E::MaxCap(R::Pep)))],
            RiteId::TheLongSun
            | RiteId::TheReverie
            | RiteId::Enlightenment
            | RiteId::EverwarmFrescoes => Vec::new(),
        };
        for f in &figures {
            assert_ne!(f, "0", "fixture: {t:?} applies something");
        }
        holds(&format!("{t:?}"), content::rite_def(t).blurb, &figures);
    }
}

/// Every arch upgrade states what it does.
/// @spec SIM-CUR-016, SIM-UNI-012
#[test]
fn an_arch_upgrade_states_what_it_does() {
    use ArchId as A;
    use Effect as E;
    let base = GameState::new(32);
    let of = |a: ArchId, key: Effect| {
        added(
            &base,
            |s| {
                s.arches_bought.insert(a);
            },
            key,
        )
    };
    for a in ArchId::ALL {
        let figures: Vec<String> = match a {
            A::RibbonArch => vec![percent(of(a, E::UnicornsRatio))],
            A::CrystalArch => vec![
                percent(of(a, E::UnicornsRatio)),
                percent(of(a, E::PrismDescentChance)),
            ],
            A::PrismArch => vec![
                percent(of(a, E::UnicornsRatio)),
                percent(of(a, E::RainbowShowerChance)),
            ],
            A::SkyArch => vec![
                percent(of(a, E::UnicornsRatio)),
                plain(content::SKY_ARCH_PRISM_DRIFT),
            ],
            A::DreamingArch | A::StarlitArch => vec![
                percent(of(a, E::UnicornsRatio)),
                percent(of(a, E::StarshardRefine)),
            ],
            A::MemoryArch => Vec::new(),
            A::EverdreamArch => vec![plain(of(a, E::MaxCap(ResourceId::JarsOfPurrs)))],
        };
        holds(&format!("{a:?}"), content::arch_def(a).blurb, &figures);
    }
    assert_eq!(
        content::PROMISE_SLOTS_PER_EVERDREAM,
        3,
        "the Everdream Arch's blurb words its slots as three"
    );
    assert!(blurb_text(content::arch_def(A::EverdreamArch).blurb).contains("three Promise slots"));
}

/// Every dream loom states what it does.
/// @spec SIM-CUR-016, SIM-DREAM-012
#[test]
fn a_dream_loom_states_what_it_does() {
    use DreamLoomId as L;
    use Effect as E;
    let base = GameState::new(33);
    let of = |l: DreamLoomId, key: Effect| {
        added(
            &base,
            |s| {
                s.dream_looms.insert(l);
            },
            key,
        )
    };
    for l in DreamLoomId::ALL {
        let figures: Vec<String> = match l {
            L::SoftPillow | L::CozyCave => vec![percent(of(l, E::HeatEfficiency))],
            L::WarmBlanket => vec![percent(of(l, E::HeatDissipation))],
            L::DreamCatcher => vec![percent(of(l, E::DejaVuChance))],
            L::DeepCushion => vec![percent(content::DEEP_CUSHION_COST_DISCOUNT)],
            L::LucidNap => vec![plain(content::LUCID_NAP_REVERIE_PER_YEAR)],
            L::LongSlumber => vec![
                plain(f64::from(content::LONG_SLUMBER_MAX_YEARS)),
                percent(content::LONG_SLUMBER_COST_DISCOUNT),
            ],
            L::ReverieNet => vec![percent(of(l, E::ReverieYield))],
            // The cap's ×4 is the rule's own literal, held by SIM-DREAM-007's tests.
            L::DistantDawn => vec![plain(content::DISTANT_DAWN_YEAR as f64), "4".to_string()],
        };
        holds(
            &format!("{l:?}"),
            content::dream_loom_def(l).blurb,
            &figures,
        );
    }
}

/// Every tradition states what it does.
/// @spec SIM-CUR-016, SIM-MOVE-014
#[test]
fn a_tradition_states_what_it_does() {
    use Effect as E;
    use TraditionId as T;
    let base = GameState::new(34);
    let held = |t: TraditionId| {
        let mut s = base.clone();
        s.traditions.insert(t);
        s
    };
    let of = |t: TraditionId, key: Effect| held(t).effect(key) - base.effect(key);
    for t in TraditionId::ALL {
        let figures: Vec<String> = match t {
            T::CarefulPacking
            | T::GoldenRatio
            | T::GrandProportion
            | T::VitruvianCat
            | T::Renaissance => vec![percent(of(t, E::GlobalPriceRatioDelta))],
            T::TravelingBasketsI | T::TravelingBasketsII | T::TravelingBasketsIII => {
                vec![plain(f64::from(held(t).basket_capacity()))]
            }
            T::MemoryChestI | T::MemoryChestII => {
                vec![percent(
                    held(t).memory_chest_ratio() - base.memory_chest_ratio(),
                )]
            }
            T::WarmWelcomeHome => vec![plain(content::WARM_WELCOME_CATNIP)],
            T::ElderStories => vec![
                percent(of(t, E::CraftRatio)),
                percent(of(t, E::CraftRatioFor(CraftId::Quills))),
            ],
            T::GoodNeighbours => vec![percent(of(t, E::StandingRatio))],
            T::WellRested | T::BrightEyed => vec![percent(of(t, E::KittenGrowthRatio))],
            T::FestivalSpirit => vec![plain(
                (content::JAMBOREE_TICKS / content::TICKS_PER_DAY) as f64,
            )],
            T::PrismSight => vec![percent(content::PRISM_SIGHT_RATIO - 1.0)],
            T::TidyPaws => vec![percent(of(t, E::MessAccrualRatio))],
            T::SecondWind | T::Forecaster | T::AdventurousSpirit => Vec::new(),
            _ => vec![percent(
                held(t).nine_lives_ratio() - base.nine_lives_ratio(),
            )],
        };
        holds(&format!("{t:?}"), content::tradition_def(t).blurb, &figures);
    }
}

/// Every trait states what its kitten brings as the Elder.
/// @spec SIM-CUR-016
#[test]
fn a_trait_states_what_it_brings_as_the_elder() {
    use Effect as E;
    use KittenTrait as K;
    let kitten = |t: KittenTrait| Kitten {
        name: "Pip".to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 1,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: t,
        tinkering: None,
        profile: None,
    };
    let of = |t: KittenTrait, key: Effect| {
        let mut s = GameState::new(35);
        s.kittens = vec![kitten(t)];
        s.elder = Some(0);
        s.effect(key)
    };
    for t in KittenTrait::ALL {
        let figures: Vec<String> = match t {
            K::None => Vec::new(),
            K::Crafty => vec![percent(of(t, E::CraftRatio))],
            K::Brassy => vec![percent(of(t, E::CraftRatioFor(CraftId::Brass)))],
            K::Alchemical => vec![percent(of(t, E::CraftRatioFor(CraftId::Pavers)))],
            K::Charming => vec![percent(of(t, E::TradeRatio))],
            K::Bold => vec![plain(of(t, E::PounceRatio))],
            K::Curious => vec![percent(of(t, E::IdeaCostRatio))],
            K::Serene => vec![percent(of(t, E::RiteCostRatio))],
        };
        holds(&format!("{t:?}"), content::trait_def(t).blurb, &figures);
    }
}

/// An idea's unlocks are words, so only the ideas with a number to state state
/// one, and a figure in a joke isn't a claim about the rule.
/// @spec SIM-CUR-016
#[test]
fn an_idea_states_only_the_figures_its_rule_applies() {
    for i in IdeaId::ALL {
        let figures: Vec<String> = match i {
            IdeaId::BiscuitMaking => vec![plain(content::BISCUIT_POINTS)],
            // "the 3 a.m. energy": the joke, not the rule.
            IdeaId::ZoomieChanneling => vec!["3".to_string()],
            _ => Vec::new(),
        };
        holds(&format!("{i:?}"), content::idea_def(i).blurb, &figures);
    }
}

/// The idea that grants a free structure says so, naming what it grants.
/// @spec SIM-UNI-011, SIM-CUR-016
#[test]
fn the_idea_granting_a_free_arch_says_so() {
    let text = blurb_text(content::idea_def(IdeaId::ParadeRoutes).blurb);
    let arch = content::structure_def(StructureId::RainbowArch).name;
    assert!(
        text.contains(arch) && text.contains("free"),
        "the grant goes unstated: {text:?}"
    );
}
