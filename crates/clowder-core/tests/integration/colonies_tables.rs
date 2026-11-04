//! The colonies design's table, read from the design itself, and the figures a
//! caravan's yield is made of, pinned by what many single caravans bring home.
//!
//! Every caravan draws at random, so a figure is held two ways: a yield's bounds,
//! which hold on every draw, and how often an outcome falls, within a band wide
//! enough never to fail by chance at this seed and narrow enough to catch a rate
//! that moved.
//!
//! @spec SIM-TRADE-002, SIM-TRADE-003, SIM-TRADE-004, SIM-TRADE-006

use crate::common;

use clowder_core::*;
use common::*;

const DESIGN: &str = include_str!("../../../../docs/intent/sim/colonies/colonies-design.md");

/// A row's goods, split where no parenthesis is open.
fn goods_of(cell: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0;
    for c in cell.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' | ';' if depth == 0 => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("never empty").push(c);
    }
    out.iter().map(|g| g.trim().to_string()).collect()
}

/// The season a word names.
fn season_named(word: &str) -> Option<Season> {
    Season::ALL
        .into_iter()
        .find(|s| format!("{s:?}").to_lowercase() == word)
}

/// The seven tradeable colonies stand in their rows: name, start standing, what
/// they buy, and each good they sell, with its amount, chance, season and level.
///
/// @spec SIM-TRADE-003
#[test]
fn every_colony_matches_its_row() {
    let rows = table_rows(DESIGN, "## The Eight Colonies", "## What a Neighbour");
    assert_eq!(rows.len(), ColonyId::ALL.len(), "one colony per row");
    let mut goods_read = 0;
    for (n, row) in rows.iter().take(7).enumerate() {
        let colony = ColonyId::ALL[n];
        let def = content::colony_def(colony);
        assert_eq!(def.name, row[1].trim_matches('*'), "colony {n}'s name");

        let start: f64 = row[2]
            .split_whitespace()
            .next()
            .expect("a start standing")
            .replace('−', "-")
            .parse()
            .expect("a start standing");
        assert_eq!(def.start_standing, start, "{}'s start standing", def.name);

        let (buys, amount) = row[3].split_once(" ×").expect("what it buys");
        assert_eq!(def.buy.0, resource_named(buys), "{}'s buy", def.name);
        assert_eq!(
            def.buy.1,
            amount.parse::<f64>().unwrap(),
            "{}'s buy",
            def.name
        );

        let goods: Vec<String> = goods_of(&row[4])
            .into_iter()
            .filter(|g| !g.starts_with("**"))
            .collect();
        assert_eq!(def.sells.len(), goods.len(), "{}'s goods", def.name);
        for (good, sell) in goods.iter().zip(def.sells) {
            goods_read += 1;
            let (head, note) = good.split_once(" (").unwrap_or((good, ""));
            let words: Vec<&str> = head.split_whitespace().collect();
            let [name, amount, "@", chance] = words[..] else {
                panic!(
                    "{}: a good is `name amount @ chance`, not {good:?}",
                    def.name
                );
            };
            assert_eq!(sell.resource, resource_named(name), "{}: {good}", def.name);
            assert_eq!(
                sell.amount,
                amount.parse::<f64>().unwrap(),
                "{}: {good}",
                def.name
            );
            let chance: f64 = chance.trim_end_matches('%').parse().unwrap();
            assert!(
                (sell.chance - chance / 100.0).abs() < 1e-9,
                "{}: {good}",
                def.name
            );

            let note = note.trim_end_matches(')');
            let season = note.split_whitespace().next().and_then(season_named);
            assert_eq!(sell.season, season, "{}: {good}", def.name);
            let bonus = match season {
                Some(_) => {
                    note.split_once('+')
                        .and_then(|(_, b)| b.trim_end_matches('%').parse::<f64>().ok())
                        .expect("a season's bonus")
                        / 100.0
                }
                None => 0.0,
            };
            assert!(
                (sell.season_bonus - bonus).abs() < 1e-9,
                "{}: {good}",
                def.name
            );
            let level = note
                .strip_prefix("needs Pen Pal level ≥ ")
                .map_or(0, |l| l.parse().expect("a level"));
            assert_eq!(sell.min_penpal, level, "{}: {good}", def.name);
        }
    }
    assert!(
        goods_read >= 15,
        "fixture: the table's goods ({goods_read})"
    );
    assert!(
        rows[2][4].contains("**moonstone**"),
        "fixture: the Foxes' row names their kite-scaled moonstone"
    );
    assert_eq!(
        content::colony_goods(ColonyId::Foxes).last(),
        Some(&(ResourceId::Moonstone, 0)),
        "Foxes' moonstone comes after their basket"
    );
}

/// One meadow's caravans, sent one at a time so each can be read.
struct Caravans {
    state: GameState,
    colony: ColonyId,
}

impl Caravans {
    fn to(colony: ColonyId, seed: u64, season: Season) -> Self {
        let mut state = GameState::new(seed);
        state.discovered.insert(colony);
        while state.season() != season {
            state.ticks += content::TICKS_PER_DAY;
        }
        Caravans { state, colony }
    }

    /// One caravan's outcome: whether it napped, and what it brought home.
    fn send(&mut self) -> (bool, Vec<(ResourceId, f64)>) {
        let (buy, amount) = content::colony_def(self.colony).buy;
        // Added, not set: one good can be both a caravan's fee and what it buys.
        for (r, n) in [
            (ResourceId::Pep, content::TRADE_PEP_PER_CARAVAN),
            (ResourceId::Shinies, content::TRADE_SHINIES_PER_CARAVAN),
            (buy, amount),
        ] {
            self.state.resources[r.ix()] += n;
        }
        let events = self.state.apply(Command::Trade(self.colony, 1)).unwrap();
        for event in events {
            if let Event::TradeReturned { napped, goods, .. } = event {
                return (napped == 1, goods);
            }
        }
        panic!("a trade reports what it brought home");
    }

    /// What `n` caravans brought of `good`, those that came home at all, and how
    /// many napped.
    fn run(&mut self, n: usize, good: ResourceId) -> Yields {
        let mut out = Yields::default();
        for _ in 0..n {
            let (napped, goods) = self.send();
            if napped {
                out.napped += 1;
                continue;
            }
            out.home += 1;
            if let Some((_, amount)) = goods.iter().find(|(r, _)| *r == good) {
                out.brought.push(*amount);
            }
        }
        out
    }
}

#[derive(Default)]
struct Yields {
    home: usize,
    napped: usize,
    brought: Vec<f64>,
}

impl Yields {
    fn least(&self) -> f64 {
        self.brought.iter().copied().fold(f64::INFINITY, f64::min)
    }
    fn most(&self) -> f64 {
        self.brought.iter().copied().fold(0.0, f64::max)
    }
    /// The share of caravans that came home bringing the good.
    fn share(&self) -> f64 {
        self.brought.len() as f64 / self.home as f64
    }
    fn share_above(&self, line: f64) -> f64 {
        self.brought.iter().filter(|a| **a > line).count() as f64 / self.home as f64
    }
    /// Every yield in `base` ± the fuzz, and the fuzz's whole width reached.
    fn spans(&self, base: f64) {
        const FUZZ: f64 = 0.15;
        let (least, most) = (self.least(), self.most());
        assert!(
            least >= base * (1.0 - FUZZ) - 1e-9 && most <= base * (1.0 + FUZZ) + 1e-9,
            "yields should stand within ±15% of {base}: {least}..{most}"
        );
        assert!(
            least < base * (1.0 - FUZZ * 0.8) && most > base * (1.0 + FUZZ * 0.8),
            "yields should reach the fuzz's whole width around {base}: {least}..{most}"
        );
    }
}

fn band(what: &str, got: f64, want: f64, by: f64) {
    assert!(
        (got - want).abs() <= by,
        "{what}: {got}, wanted {want} ± {by}"
    );
}

/// A sure good in a neutral colony is its amount ± the fuzz, out of season and,
/// by its bonus, in; and the trade ratio scales it.
///
/// @spec SIM-TRADE-002
#[test]
fn a_yield_is_its_amount_times_the_trade_ratio_and_season_within_the_fuzz() {
    const N: usize = 1_000;
    let pebbles = ResourceId::Pebbles;

    let mut out = Caravans::to(ColonyId::Hedgehogs, 3_501, Season::Summer);
    let y = out.run(N, pebbles);
    assert_eq!(y.napped, 0, "a neutral colony never naps");
    assert_eq!(y.brought.len(), N, "a sure good comes home every time");
    y.spans(1_000.0);

    let mut spring = Caravans::to(ColonyId::Hedgehogs, 3_502, Season::Spring);
    spring.run(N, pebbles).spans(1_000.0 * 1.25);

    let stands = StructureId::ALL
        .into_iter()
        .find(|s| content::structure_def(*s).trade_ratio > 0.0)
        .expect("fixture: a structure raises the trade ratio");
    let mut raised = Caravans::to(ColonyId::Hedgehogs, 3_503, Season::Summer);
    grant(&mut raised.state, stands, 4);
    let ratio = raised.state.effect(Effect::TradeRatio);
    assert!(ratio > 0.0, "fixture: a trade ratio stands");
    raised.run(N, pebbles).spans(1_000.0 * (1.0 + ratio));
}

/// A warm colony brings a bonus outcome ×1.25 as often as its standing says, and
/// a neutral one never does.
///
/// @spec SIM-TRADE-002
#[test]
fn a_warm_colony_brings_a_bonus_as_often_as_its_standing_says() {
    const N: usize = 6_000;
    // How much of a bonus yield stands above every plain one's top.
    let seen = |base: f64| {
        let (least, most) = (base * 0.85 * 1.25, base * 1.15 * 1.25);
        (most - base * 1.15) / (most - least)
    };

    for (colony, seed, base, standing) in [
        (ColonyId::Otters, 3_511, 500.0, 0.25),
        (ColonyId::Rabbits, 3_512, 350.0, 0.15),
    ] {
        let good = content::colony_def(colony).sells[0].resource;
        // A season no sell of theirs favours is not needed: both favour one good,
        // and the one read here is the sure one.
        let season = match colony {
            ColonyId::Otters => Season::Winter,
            _ => Season::Autumn,
        };
        let mut caravans = Caravans::to(colony, seed, season);
        let y = caravans.run(N, good);
        assert_eq!(y.napped, 0, "a warm colony never naps");
        let top = base * 1.15 * 1.25;
        assert!(
            y.most() <= top + 1e-9,
            "{colony:?}: {}, above {top}",
            y.most()
        );
        assert!(y.most() > top * 0.97, "{colony:?}: a bonus reaches ×1.25");
        band(
            &format!("{colony:?}'s bonus share"),
            y.share_above(base * 1.15),
            standing * seen(base),
            0.03,
        );
    }
}

/// A shy colony naps as often as its shyness says, and a napped caravan's cost is
/// spent all the same.
///
/// @spec SIM-TRADE-002
#[test]
fn a_shy_colony_naps_as_often_as_it_is_shy() {
    const N: usize = 4_000;
    for (colony, seed, shy) in [
        (ColonyId::Owls, 3_521, 0.10),
        (ColonyId::Foxes, 3_522, 0.15),
    ] {
        let good = content::colony_def(colony).sells[0].resource;
        let mut caravans = Caravans::to(colony, seed, Season::Summer);
        let y = caravans.run(N, good);
        band(
            &format!("{colony:?}'s naps"),
            y.napped as f64 / N as f64,
            shy,
            0.025,
        );
    }
}

/// A good that is not sure comes home as often as its chance says.
///
/// @spec SIM-TRADE-002
#[test]
fn a_good_comes_home_as_often_as_its_chance_says() {
    let mut caravans = Caravans::to(ColonyId::Raccoons, 3_531, Season::Summer);
    let y = caravans.run(4_000, ResourceId::Stories);
    band("a 15% good", y.share(), 0.15, 0.025);
    y.spans(3.0);
}

/// Foxes' moonstone at each figure of its rule: chance 0.15 + 0.0035 per Kite and
/// amount (1.5 + 0.03 per Kite) × (1 + standing), in any season.
///
/// @spec SIM-TRADE-004
#[test]
fn foxes_moonstone_follows_its_kites_and_standing_in_any_season() {
    const N: usize = 5_000;
    let shy = 1.0 - 0.15;
    for (kites, chance, amount, seed) in [
        (0.0, 0.15, 1.5 * shy, 3_541),
        (100.0, 0.5, 4.5 * shy, 3_542),
        (200.0, 0.85, 7.5 * shy, 3_543),
    ] {
        for (season, shift) in [(Season::Autumn, 0), (Season::Spring, 1)] {
            let mut caravans = Caravans::to(ColonyId::Foxes, seed + shift, season);
            give(&mut caravans.state, ResourceId::Kites, kites);
            let y = caravans.run(N, ResourceId::Moonstone);
            band(
                &format!("the chance at {kites} Kites"),
                y.share(),
                chance,
                0.03,
            );
            y.spans(amount);
        }
    }

    // Warmed to neutral, the Foxes neither nap nor shave it, and the trade ratio
    // scales it as it does any good.
    let mut warm = Caravans::to(ColonyId::Foxes, 3_549, Season::Autumn);
    warm.state.penpal[ColonyId::Foxes.ix()] = 40;
    assert_eq!(
        warm.state.standing(ColonyId::Foxes),
        0.0,
        "fixture: neutral"
    );
    give(&mut warm.state, ResourceId::Kites, 100.0);
    let y = warm.run(N, ResourceId::Moonstone);
    assert_eq!(y.napped, 0, "a neutral colony never naps");
    y.spans(4.5);

    let stands = StructureId::ALL
        .into_iter()
        .find(|s| content::structure_def(*s).trade_ratio > 0.0)
        .expect("fixture: a structure raises the trade ratio");
    let mut raised = Caravans::to(ColonyId::Foxes, 3_550, Season::Autumn);
    raised.state.penpal[ColonyId::Foxes.ix()] = 40;
    grant(&mut raised.state, stands, 4);
    let ratio = raised.state.effect(Effect::TradeRatio);
    give(&mut raised.state, ResourceId::Kites, 100.0);
    raised
        .run(N, ResourceId::Moonstone)
        .spans(4.5 * (1.0 + ratio));
}

/// A good kept for a Pen Pal level stays home below it, and comes home at it, on a
/// real trade.
///
/// @spec SIM-TRADE-006
#[test]
fn a_good_kept_for_a_level_stays_home_until_the_level_is_reached() {
    const N: usize = 2_000;
    let stardust = ResourceId::Stardust;
    let sell = content::colony_def(ColonyId::Deer)
        .sells
        .iter()
        .find(|g| g.resource == stardust)
        .expect("fixture: the Deer keep stardust");
    assert!(sell.min_penpal > 0, "fixture: for a level");

    let mut below = Caravans::to(ColonyId::Deer, 3_551, Season::Summer);
    below.state.penpal[ColonyId::Deer.ix()] = sell.min_penpal - 1;
    assert!(
        below.run(N, stardust).brought.is_empty(),
        "a kept good came home below its level"
    );

    let mut at = Caravans::to(ColonyId::Deer, 3_552, Season::Summer);
    at.state.penpal[ColonyId::Deer.ix()] = sell.min_penpal;
    band(
        "at its level",
        at.run(N, stardust).share(),
        sell.chance,
        0.04,
    );
}
