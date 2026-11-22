//! A row that spends states what it takes: every action row on a meadow where each
//! can be taken, read against what applying its command does.
//!
//! @spec TUI-NAV-207, TUI-RENDER-020

use clowder_core::{ArchId, ColonyId, Command, GameState, IdeaId, Kitten, KittenTrait, Location};
use clowder_core::{ResourceId, SiteId, StructureId};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::{rows, Row, RowKind};
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render;
use clowder_tui::render::fmt;
use clowder_tui::render::layout::{compute, Body};
use clowder_tui::render::widgets::inspector;

const WIDE: u16 = 200;
const TALL: u16 = 70;

fn rich() -> GameState {
    let mut s = GameState::new(46);
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.arches_bought = ArchId::ALL.into_iter().collect();
    s.sites_reached = SiteId::ALL.into_iter().take(3).collect();
    s.discovered = ColonyId::ALL.into_iter().collect();
    s.whale_visit_years = 3;
    for st in StructureId::ALL {
        s.structures[st.ix()] = 8;
    }
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e7;
    }
    s.resources[ResourceId::Twigs.ix()] = 0.0;
    s.resources[ResourceId::JarsOfPurrs.ix()] = 0.0;
    s.kittens = vec![Kitten {
        name: "Pip".to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 1.0e9,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }];
    s
}

fn looking_at(section: Section, key: &RowKey) -> ViewState {
    let mut v = ViewState {
        section: Some(section),
        ..Default::default()
    };
    v.focus[section.index()] = Some(key.clone());
    v
}

fn pane(s: &GameState, v: &ViewState) -> String {
    let rect = match compute(Rect::new(0, 0, WIDE, TALL), None, false).body {
        Body::Wide { inspector, .. } => inspector,
        other => panic!("fixture: three columns, not {other:?}"),
    };
    let mut term = Terminal::new(TestBackend::new(WIDE, TALL)).unwrap();
    term.draw(|f| inspector::draw(f, rect, s, v)).unwrap();
    let buf = term.backend().buffer().clone();
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A store's stores-row name, spaced at each capital.
fn store(r: ResourceId) -> String {
    let tag = format!("{r:?}");
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn action_rows(s: &GameState) -> Vec<(Section, Row, Command)> {
    Section::ALL
        .into_iter()
        .flat_map(|section| {
            rows(s, section).into_iter().filter_map(move |row| {
                let RowKind::Act { command, .. } = row.kind else {
                    return None;
                };
                Some((section, row, command))
            })
        })
        .collect()
}

/// Every action row that takes something from the stores says so in the inspector,
/// with need and have, read against what applying it does, a kitten's row among
/// them. A nap's price is previewed when its length is typed (TUI-NAV-067).
///
/// @spec TUI-NAV-207, TUI-NAV-208
#[test]
fn every_row_that_takes_from_the_stores_states_it_in_the_inspector() {
    let s = rich();
    let mut stated = 0;
    for (section, row, command) in action_rows(&s) {
        if matches!(command, Command::DeepNap(_)) {
            continue;
        }
        if matches!(row.key, RowKey::Content(..)) {
            continue; // Priced by a content item, as TUI-NAV-078 holds.
        }
        let mut after = s.clone();
        if after.apply(command).is_err() {
            continue;
        }
        let fell: Vec<ResourceId> = ResourceId::ALL
            .into_iter()
            .filter(|r| s.stock(*r) - after.stock(*r) > 1e-6)
            .collect();
        let text = pane(&s, &looking_at(section, &row.key));
        if fell.is_empty() {
            assert!(
                !text.contains(": need"),
                "{:?} takes nothing and states a price:\n{text}",
                row.key
            );
        }
        for r in fell {
            stated += 1;
            let head = format!("{}: need", store(r));
            assert!(
                text.contains(&head),
                "{:?} takes {r:?} and does not say so:\n{text}",
                row.key
            );
            assert!(
                text.contains(&format!("have {}", fmt::exact(s.stock(r)))),
                "{:?} does not say what is held of {r:?}:\n{text}",
                row.key
            );
        }
    }
    assert!(
        stated >= 12,
        "fixture: rows that spend were read ({stated})"
    );
}

/// The list draws the same price in the row's cost field, the Sky list among them.
///
/// @spec TUI-NAV-207, TUI-RENDER-020
#[test]
fn every_priced_row_draws_its_price_in_the_cost_field() {
    let s = rich();
    let mut drawn = 0;
    let mut sections = std::collections::BTreeSet::new();
    for (section, row, command) in action_rows(&s) {
        let Some(price) = s.act_price(&command) else {
            continue;
        };
        drawn += 1;
        sections.insert(format!("{section:?}"));
        let v = looking_at(section, &row.key);
        let mut term = Terminal::new(TestBackend::new(WIDE, TALL)).unwrap();
        term.draw(|f| {
            render::frame(f, &s, &v, None);
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        let line = (0..TALL)
            .map(|y| {
                (0..WIDE)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .find(|l| l.contains('▸'))
            .expect("the cursor's row is drawn");
        let (r, amount) = price[0];
        assert!(
            line.contains(&format!("{} {}", fmt::compact(amount), store(r))),
            "{:?} does not draw its first component in its cost field: {line}",
            row.key
        );
    }
    assert!(drawn >= 10, "fixture: priced rows were listed ({drawn})");
    assert!(sections.contains("Sky"), "fixture: the Sky list is read");
}

/// A row that takes nothing leaves its cost field blank, and an empty store leaves
/// the act that spends all of it with nothing to state.
///
/// @spec TUI-NAV-207
#[test]
fn a_row_with_nothing_to_state_states_nothing() {
    let mut s = rich();
    s.resources[ResourceId::Warmth.ix()] = 0.0;
    let (section, row, _) = action_rows(&s)
        .into_iter()
        .find(|(_, _, c)| matches!(c, Command::Bask))
        .expect("fixture: the row is listed");
    let text = pane(&s, &looking_at(section, &row.key));
    assert!(!text.contains(": need"), "nothing to state:\n{text}");
}

/// A kitten's row states its next rank's price beneath the kitten's name: the
/// shinies with need and have, the exp with need and has, before the card.
///
/// @spec TUI-NAV-208
#[test]
fn a_kittens_row_states_its_next_rank() {
    let mut s = rich();
    s.kittens[0].exp = 120.0;
    s.resources[ResourceId::Shinies.ix()] = 7.0;
    let price = s.promotion_price(0).expect("a kitten stands at 0");
    let (section, row, _) = action_rows(&s)
        .into_iter()
        .find(|(_, r, _)| matches!(r.key, RowKey::Kitten(0, _)))
        .expect("fixture: the kitten's row is listed");
    let text = pane(&s, &looking_at(section, &row.key));
    let shinies = format!(
        "Shinies: need {}, have {}",
        fmt::exact(price.shinies),
        fmt::exact(7.0)
    );
    let exp = format!(
        "Experience: need {}, has {}",
        fmt::exact(price.exp),
        fmt::exact(120.0)
    );
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle:?} is not stated:\n{text}"))
    };
    let card = text.find("about").expect("fixture: the card is drawn");
    assert!(
        at(&shinies) < card && at(&exp) < card,
        "before the card:\n{text}"
    );
}

/// A refusal for short exp names it in plain words, with how much more.
///
/// @spec TUI-NAV-208, TUI-NAV-014
#[test]
fn a_refusal_for_short_exp_names_it() {
    let words = clowder_tui::nav::plain_error(&clowder_core::CommandError::NotEnoughExperience {
        short: 380.0,
    });
    assert!(
        words.contains("experience") && words.contains("380"),
        "{words}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-218 — an action row's next cost is marked and waited for
// ---------------------------------------------------------------------------

/// The price an action row's inspector states, as the core reports it: a
/// letter's, a promotion's shinies, a count-taking row's one unit, or a fixed
/// act's.
fn stated_price(s: &GameState, row: &Row, command: &Command) -> Option<Vec<(ResourceId, f64)>> {
    match (&row.key, command) {
        (RowKey::Colony(c, "penpal"), _) => Some(vec![(ResourceId::Charm, s.pen_pal(*c)?.price)]),
        (_, Command::Promote(ix)) => {
            Some(vec![(ResourceId::Shinies, s.promotion_price(*ix)?.shinies)])
        }
        _ => s.unit_price(command).or_else(|| s.act_price(command)),
    }
}

/// The inspector's line for one component of a next cost.
fn component_line(text: &str, r: ResourceId) -> &str {
    let head = format!("{}: need", store(r));
    text.lines()
        .find(|l| l.contains(&head))
        .unwrap_or_else(|| panic!("no next-cost line for {r:?}:\n{text}"))
}

/// What a wait states, in seconds, and the length of the unit it is stated in.
fn stated_wait(line: &str) -> Option<(f64, f64)> {
    let rest = &line[line.find("— wait ")? + "— wait ".len()..];
    let mut words = rest.split_whitespace();
    let n: f64 = words.next()?.parse().ok()?;
    let unit = match words.next()?.trim_end_matches('.') {
        "seconds" | "second" => 1.0,
        "minutes" | "minute" => 60.0,
        "hours" | "hour" => 3_600.0,
        "days" | "day" => 86_400.0,
        _ => return None,
    };
    Some((n * unit, unit))
}

/// Every priced action row marks each component of its next cost as the
/// core's shortfall reports it: with one component in turn a quarter of its
/// need and the rest in plenty, that one is short by what it lacks and carries
/// the note its store's cap and rate call for, a where-it-comes-from line
/// beneath it where nothing makes it, and every other component is covered.
///
/// @spec TUI-NAV-218, TUI-NAV-163
#[test]
fn every_priced_action_row_marks_its_next_cost_and_waits_as_a_content_row_does() {
    let base = rich();
    let (mut waited, mut stalled, mut capped) = (0, 0, 0);
    for (section, row, command) in action_rows(&base) {
        if matches!(row.key, RowKey::Content(..)) {
            continue;
        }
        let Some(price) = stated_price(&base, &row, &command) else {
            continue;
        };
        if matches!(command, Command::Bask) {
            continue; // Priced at the whole store it spends: always covered.
        }
        for (r, need) in &price {
            let mut s = base.clone();
            for (other, n) in &price {
                s.resources[other.ix()] = n * 2.0;
            }
            s.resources[r.ix()] = need * 0.25;
            let shown = pane(&s, &looking_at(section, &row.key));
            let line = component_line(&shown, *r);
            let short = need - s.stock(*r);
            assert!(
                line.contains(&format!("short by {}", fmt::exact(short))),
                "{:?}: {r:?} is short by {}:\n{shown}",
                row.key,
                fmt::exact(short)
            );
            if *need > s.cap(*r) {
                capped += 1;
                assert!(
                    line.contains("the stores cannot hold that much yet"),
                    "{line}"
                );
                assert!(stated_wait(line).is_none(), "no wait above the cap: {line}");
            } else if s.net_rate(*r) <= 0.0 {
                stalled += 1;
                assert!(line.contains("the stock is not growing"), "{line}");
                assert!(
                    stated_wait(line).is_none(),
                    "no wait while not growing: {line}"
                );
                let made = s.flows(*r).iter().any(|(_, rate)| *rate > 0.0);
                if !made && !s.sources_of(*r).is_empty() {
                    assert!(
                        shown.contains("Comes from: "),
                        "{:?}: nothing makes {r:?}, so the inspector says where it comes from:\n{shown}",
                        row.key
                    );
                }
            } else {
                waited += 1;
                assert!(
                    stated_wait(line).is_some(),
                    "{:?}: {r:?} grows, so it waits: {line}",
                    row.key
                );
            }
            for (other, _) in price.iter().filter(|(o, _)| o != r) {
                assert!(
                    component_line(&shown, *other).contains(", covered"),
                    "{:?}: {other:?} is in plenty:\n{shown}",
                    row.key
                );
            }
        }
    }
    assert!(
        waited > 3 && stalled + capped > 0,
        "fixture: action rows whose short stores grow, and some that do not ({waited}, {stalled}, {capped})"
    );
}

/// An action row's wait is the shortfall over the core's net rate and the
/// current speed, in real time: never stated sooner than the rate gives and
/// within one unit of its rounding, at the slowest speed and the fastest.
///
/// @spec TUI-NAV-218
#[test]
fn an_action_rows_wait_is_its_shortfall_over_the_rate_and_the_speed() {
    let base = rich();
    let mut seen = 0;
    for (section, row, command) in action_rows(&base) {
        let Some(price) = stated_price(&base, &row, &command) else {
            continue;
        };
        if matches!(row.key, RowKey::Content(..)) || matches!(command, Command::Bask) {
            continue;
        }
        let (r, need) = price[0];
        let mut s = base.clone();
        for (other, n) in &price {
            s.resources[other.ix()] = n * 2.0;
        }
        s.resources[r.ix()] = need * 0.25;
        let rate = s.net_rate(r);
        if need > s.cap(r) || rate <= 0.0 {
            continue;
        }
        seen += 1;
        let short = need - s.stock(r);
        for speed in [0.25, 8.0] {
            s.speed = speed;
            let shown = pane(&s, &looking_at(section, &row.key));
            let line = component_line(&shown, r).to_string();
            let (seconds, unit) = stated_wait(&line)
                .unwrap_or_else(|| panic!("{:?}: a wait in plain units: {line}", row.key));
            let expected = short / (rate * speed);
            assert!(
                seconds >= expected * (1.0 - 1e-9),
                "never sooner: {expected}s vs {line}"
            );
            assert!(
                seconds < expected + unit,
                "within one unit of rounding: {expected}s vs {line}"
            );
        }
    }
    assert!(
        seen > 3,
        "fixture: rows whose first component grows ({seen})"
    );
}

/// A kitten's next rank marks its shinies as any next cost, and its exp stands
/// with need and has, unmarked and with no wait.
///
/// @spec TUI-NAV-208, TUI-NAV-218
#[test]
fn a_promotions_shinies_are_marked_and_its_exp_is_not() {
    let mut s = rich();
    s.kittens[0].exp = 120.0;
    let price = s.promotion_price(0).expect("a kitten stands at 0");
    s.resources[ResourceId::Shinies.ix()] = price.shinies * 0.25;
    let (section, row, _) = action_rows(&s)
        .into_iter()
        .find(|(_, r, _)| matches!(r.key, RowKey::Kitten(0, _)))
        .expect("fixture: the kitten's row is listed");
    let text = pane(&s, &looking_at(section, &row.key));
    let shinies = component_line(&text, ResourceId::Shinies);
    assert!(
        shinies.contains("short by"),
        "the shinies are marked: {shinies}"
    );
    let exp = text
        .lines()
        .find(|l| l.contains("Experience: need"))
        .expect("the exp line is stated");
    assert_eq!(
        exp.trim(),
        format!(
            "Experience: need {}, has {}",
            fmt::exact(price.exp),
            fmt::exact(120.0)
        ),
        "the exp stands unmarked"
    );
}
