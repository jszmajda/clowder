//! Where a short store comes from: the inspector's `Comes from:` line beneath
//! a short component nothing in the meadow is making.
//!
//! @spec TUI-NAV-163

use std::collections::BTreeSet;

use clowder_core::content::{
    arch_def, colony_def, craft_def, dream_loom_def, job_def, refinement_def, rite_def,
    structure_def,
};
use clowder_core::{Act, ColonyId, ContentId, GameState, IdeaId, ResourceId, Source, StructureId};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::rows::rows;
use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::widgets::inspector;

const W: u16 = 220;
const H: u16 = 60;

fn inspector_lines(state: &GameState, section: Section, key: RowKey) -> Vec<String> {
    let mut view = ViewState {
        section: Some(section),
        ..ViewState::default()
    };
    view.focus[section.index()] = Some(key);
    let rect = Rect::new(0, 0, W, H);
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, &view))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    (0..H)
        .map(|y| {
            (0..W)
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
                .trim()
                .to_string()
        })
        .collect()
}

fn spaced(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The line the spec words for `r`, from the core's sources and nothing else.
fn expected(state: &GameState, r: ResourceId) -> Option<String> {
    let sources = state.sources_of(r);
    if sources.is_empty() {
        return None;
    }
    let mut items: Vec<String> = Vec::new();
    for (src, known) in &sources {
        if !known {
            continue;
        }
        items.push(match src {
            Source::Structure(s) => structure_def(*s).name.to_string(),
            Source::Job(j) => format!("{} (job)", job_def(*j).name),
            Source::Craft(c) => format!("{} (recipe)", craft_def(*c).name),
            Source::Colony(c) => format!("trade with {}", colony_def(*c).name),
            Source::Act(a) => spaced(&format!("{a:?}")),
            Source::Arch(a) => arch_def(*a).name.to_string(),
            Source::Refinement(f) => refinement_def(*f).name.to_string(),
            Source::Rite(t) => rite_def(*t).name.to_string(),
            Source::Loom(l) => dream_loom_def(*l).name.to_string(),
        });
    }
    let mut kinds: BTreeSet<usize> = BTreeSet::new();
    for (src, known) in &sources {
        if *known {
            continue;
        }
        kinds.insert(match src {
            Source::Structure(_) => 0,
            Source::Job(_) => 1,
            Source::Craft(_) => 2,
            Source::Colony(_) => 3,
            Source::Act(_) => 4,
            Source::Arch(_) => 5,
            Source::Refinement(_) => 6,
            Source::Rite(_) => 7,
            Source::Loom(_) => 8,
        });
    }
    let phrases = [
        "a building you haven't found",
        "work no kitten knows yet",
        "a recipe you haven't learned",
        "trade with a neighbour",
        "something you can't do yet",
        "an arch upgrade you haven't found",
        "a refinement you haven't found",
        "a rite you haven't learned",
        "a loom you haven't found",
    ];
    items.extend(kinds.into_iter().map(|k| phrases[k].to_string()));
    let joined = match items.len() {
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        n => format!("{}, and {}", items[..n - 1].join(", "), items[n - 1]),
    };
    Some(format!("Comes from: {joined}."))
}

fn workshop_meadow() -> GameState {
    let mut s = GameState::new(41);
    s.ideas.insert(IdeaId::Toymaking);
    s.revealed
        .insert(ContentId::Structure(StructureId::ToyWorkshop));
    // Room for the moonstone, so its line is the not-growing one rather than
    // the stores-cannot-hold one.
    s.structures[StructureId::Shed.ix()] = 2;
    assert!(
        s.cap(ResourceId::Moonstone) >= 18.0,
        "fixture: room to hold it"
    );
    s
}

fn workshop_row() -> RowKey {
    RowKey::Content(ContentId::Structure(StructureId::ToyWorkshop), "")
}

/// The line beneath a component's line, if a `Comes from:` line stands there.
fn beneath(lines: &[String], r: ResourceId) -> Option<String> {
    let head = format!("{}: need", spaced(&format!("{r:?}")));
    let at = lines.iter().position(|l| l.starts_with(&head))?;
    lines
        .get(at + 1)
        .filter(|l| l.starts_with("Comes from:"))
        .cloned()
}

/// Where nothing makes the short store, the line says where it can come
/// from: a source not met is only gestured at; once met it is named.
/// @spec TUI-NAV-163
#[test]
fn a_short_store_nothing_makes_says_where_it_can_come_from() {
    let mut s = workshop_meadow();
    let lines = inspector_lines(&s, Section::Meadow, workshop_row());
    assert_eq!(
        beneath(&lines, ResourceId::Moonstone).as_deref(),
        Some("Comes from: a building you haven't found and trade with a neighbour."),
        "{}",
        lines.join("\n")
    );

    s.ideas.insert(IdeaId::CeramicArts);
    s.revealed.insert(ContentId::Structure(StructureId::Kiln));
    let kiln = structure_def(StructureId::Kiln).name;
    let lines = inspector_lines(&s, Section::Meadow, workshop_row());
    assert_eq!(
        beneath(&lines, ResourceId::Moonstone),
        Some(format!("Comes from: {kiln} and trade with a neighbour.")),
    );

    // Met, but not yet seen to trade it: still only a neighbour.
    s.discovered.insert(ColonyId::Deer);
    s.learned_goods.insert(ColonyId::Deer, BTreeSet::new());
    let lines = inspector_lines(&s, Section::Meadow, workshop_row());
    assert_eq!(
        beneath(&lines, ResourceId::Moonstone),
        Some(format!("Comes from: {kiln} and trade with a neighbour.")),
    );

    // Once a caravan has brought it home, the neighbour is named.
    s.learned_goods.insert(
        ColonyId::Deer,
        [ResourceId::Moonstone].into_iter().collect(),
    );
    let deer = colony_def(ColonyId::Deer).name;
    let lines = inspector_lines(&s, Section::Meadow, workshop_row());
    assert_eq!(
        beneath(&lines, ResourceId::Moonstone),
        Some(format!(
            "Comes from: {kiln}, trade with {deer}, and trade with a neighbour."
        )),
    );
}

/// Across every structure row of meadows in several states, a `Comes from:`
/// line stands beneath a component exactly where the component is short with
/// the not-growing note, nothing raises its store, and the core lists a
/// source — and reads as the spec words it.
/// @spec TUI-NAV-163
#[test]
fn the_line_stands_exactly_where_nothing_makes_a_short_store() {
    let mut states = vec![workshop_meadow()];
    let mut all = GameState::new(42);
    all.ideas = IdeaId::ALL.into_iter().collect();
    all.revealed = StructureId::ALL
        .into_iter()
        .map(ContentId::Structure)
        .collect();
    states.push(all.clone());
    let mut half = all.clone();
    half.ideas = IdeaId::ALL.into_iter().step_by(2).collect();
    states.push(half);
    let mut running = all;
    running.structures[StructureId::CatnipPatch.ix()] = 5;
    running.structures[StructureId::CardboardBox.ix()] = 5;
    states.push(running);

    let (mut stated, mut withheld) = (0, 0);
    for s in &states {
        for row in rows(s, Section::Meadow) {
            let RowKey::Content(ContentId::Structure(b), _) = row.key else {
                continue;
            };
            let lines = inspector_lines(s, Section::Meadow, row.key.clone());
            for (r, _) in s.cost_of_next(ContentId::Structure(b), 1) {
                let head = format!("{}: need", spaced(&format!("{r:?}")));
                let Some(at) = lines.iter().position(|l| l.starts_with(&head)) else {
                    continue;
                };
                let still = lines[at].contains("not growing");
                let made = s.flows(r).iter().any(|(_, rate)| *rate > 0.0);
                let want = if still && !made { expected(s, r) } else { None };
                let got = beneath(&lines, r);
                assert_eq!(got, want, "{b:?}, {r:?}:\n{}", lines.join("\n"));
                if got.is_some() {
                    stated += 1;
                } else {
                    withheld += 1;
                }
            }
        }
    }
    assert!(stated > 10, "fixture: lines stated ({stated})");
    assert!(
        withheld > 0,
        "fixture: some components carry none ({withheld})"
    );
}

/// A component with a wait — its store growing — carries no line.
/// @spec TUI-NAV-163
#[test]
fn a_growing_store_carries_no_line() {
    let mut s = GameState::new(43);
    s.structures[StructureId::CatnipPatch.ix()] = 3;
    s.revealed
        .insert(ContentId::Structure(StructureId::CatnipPatch));
    let key = RowKey::Content(ContentId::Structure(StructureId::CatnipPatch), "");
    let lines = inspector_lines(&s, Section::Meadow, key);
    let at = lines
        .iter()
        .position(|l| l.starts_with("Catnip: need"))
        .unwrap_or_else(|| panic!("{}", lines.join("\n")));
    assert!(lines[at].contains("wait"), "fixture: catnip is growing");
    assert_eq!(beneath(&lines, ResourceId::Catnip), None);
    let _ = Act::GatherCatnip;
}

/// The cells of the inspector line beginning `Comes from:`, each with its
/// colour.
fn comes_from_cells(state: &GameState, key: RowKey) -> Vec<(char, ratatui::style::Color)> {
    let mut view = ViewState {
        section: Some(Section::Meadow),
        ..ViewState::default()
    };
    view.focus[Section::Meadow.index()] = Some(key);
    let rect = Rect::new(0, 0, W, H);
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, state, &view))
        .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    for y in 0..H {
        let cells: Vec<(char, ratatui::style::Color)> = (0..W)
            .map(|x| {
                let c = buf.cell((x, y)).expect("in the buffer");
                (c.symbol().chars().next().unwrap_or(' '), c.fg)
            })
            .collect();
        let text: String = cells.iter().map(|(c, _)| *c).collect();
        if text.trim_start().starts_with("Comes from:") {
            return cells;
        }
    }
    panic!("no Comes from line");
}

/// A recipe is named for the store it makes and wears that store's coat; a
/// structure's or an act's name that merely holds a store's word does not.
/// @spec TUI-NAV-163, TUI-THEME-019
#[test]
fn a_recipe_wears_its_stores_coat_and_other_names_do_not() {
    use clowder_tui::theme::{coat, coat_colours};
    use ratatui::style::Color;

    let mut s = GameState::new(44);
    s.ideas.insert(IdeaId::Tinkering);
    s.ideas.insert(IdeaId::PackRattery);
    s.revealed.insert(ContentId::Structure(StructureId::Shed));
    let shed = RowKey::Content(ContentId::Structure(StructureId::Shed), "");
    let cells = comes_from_cells(&s, shed);
    let text: String = cells.iter().map(|(c, _)| *c).collect();
    let at = text
        .find("Wands (recipe)")
        .unwrap_or_else(|| panic!("{text}"));
    let at = text[..at].chars().count();
    let want = coat_colours(coat(ResourceId::Wands), 5);
    for (k, w) in want.iter().enumerate() {
        assert_eq!(
            &cells[at + k].1,
            w,
            "the recipe's name wears its coat: {text}"
        );
    }
    for k in 5..14 {
        assert_eq!(
            cells[at + k].1,
            Color::Reset,
            "` (recipe)` is plain: {text}"
        );
    }

    let mut s = GameState::new(45);
    s.revealed
        .insert(ContentId::Structure(StructureId::CozyCushion));
    let cushion = RowKey::Content(ContentId::Structure(StructureId::CozyCushion), "");
    let cells = comes_from_cells(&s, cushion);
    let text: String = cells.iter().map(|(c, _)| *c).collect();
    let at = text
        .find("Gather Catnip")
        .unwrap_or_else(|| panic!("{text}"));
    let at = text[..at].chars().count();
    for k in 0.."Gather Catnip".len() {
        assert_eq!(
            cells[at + k].1,
            Color::Reset,
            "an act's name is plain: {text}"
        );
    }
}
