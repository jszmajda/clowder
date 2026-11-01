//! What an act brought home: a pounce's or a trade's event line names what
//! came back, and the inspector states the last time beneath those rows'
//! prices.
//!
//! @spec TUI-NAV-147, TUI-NAV-148, TUI-NAV-149, TUI-THEME-019, TUI-THEME-024
//!
//! The shell's own words are not asserted beyond the inspector's lines, whose
//! wording the specs pin; on the ticker and in the Journal the tests look for
//! the stores and figures an event carries, and the order they stand in.

use clowder_core::{content, ColonyId, Event, GameState, IdeaId, ResourceId};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

use clowder_tui::nav::{Overlay, RowKey, Section, ViewState};
use clowder_tui::render::fmt;
use clowder_tui::render::widgets::{inspector, overlays};
use clowder_tui::theme::{coat, coat_colours};

const W: u16 = 200;
const H: u16 = 30;

fn drawn(render: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(W, H)).expect("TestBackend is infallible");
    term.draw(|f| render(f)).expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

fn lines(buf: &Buffer, rect: Rect) -> Vec<String> {
    (rect.y..rect.bottom())
        .map(|y| {
            (rect.x..rect.right())
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn ticker_rect() -> Rect {
    Rect::new(0, 0, W, 1)
}

fn ticker(state: &GameState) -> Buffer {
    drawn(|f| overlays::ticker(f, ticker_rect(), state))
}

fn ticker_text(state: &GameState) -> String {
    lines(&ticker(state), ticker_rect()).join("\n")
}

fn journal_text(state: &GameState) -> String {
    let view = ViewState {
        overlay: Some(Overlay::Journal { anchor: 0 }),
        ..ViewState::default()
    };
    let rect = Rect::new(0, 0, W, H);
    lines(&drawn(|f| overlays::journal(f, rect, state, &view)), rect).join("\n")
}

fn inspector_rect() -> Rect {
    Rect::new(0, 0, 60, H)
}

fn inspector_buf(state: &GameState, section: Section, key: RowKey) -> Buffer {
    let mut view = ViewState {
        section: Some(section),
        ..ViewState::default()
    };
    view.focus[section.index()] = Some(key);
    drawn(|f| inspector::draw(f, inspector_rect(), state, &view))
}

fn inspector_lines(state: &GameState, section: Section, key: RowKey) -> Vec<String> {
    lines(&inspector_buf(state, section, key), inspector_rect())
}

/// A resource named as its stores row names it.
fn label(r: ResourceId) -> String {
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

fn journaled(events: &[Event]) -> GameState {
    let mut s = GameState::new(21);
    s.ideas.insert(IdeaId::PouncingPractice);
    s.discovered.insert(ColonyId::Otters);
    s.discovered.insert(ColonyId::Foxes);
    for (i, e) in events.iter().enumerate() {
        s.journal.record(100 + i as u64, e);
    }
    s
}

fn pounce(squads: u32, feathers: f64, bells: f64, shinies: f64, unicorns: u32) -> Event {
    Event::PounceReturned {
        squads,
        feathers,
        bells,
        shinies,
        unicorns,
    }
}

fn trade(colony: ColonyId, caravans: u32, napped: u32, goods: &[(ResourceId, f64)]) -> Event {
    Event::TradeReturned {
        colony,
        caravans,
        napped,
        goods: goods.to_vec(),
    }
}

/// Where each needle stands in `text`, asserting every one is there.
fn positions(what: &str, text: &str, needles: &[String]) -> Vec<usize> {
    needles
        .iter()
        .map(|n| {
            text.find(n.as_str())
                .unwrap_or_else(|| panic!("{what} does not say {n:?}:\n{text}"))
        })
        .collect()
}

fn ascending(v: &[usize]) -> bool {
    v.windows(2).all(|w| w[0] < w[1])
}

// ---------------------------------------------------------------------------
// TUI-NAV-147 — an event's line says what came home
// ---------------------------------------------------------------------------

/// A pounce's line names each store its haul raised, with the amount, in the
/// core's resource order — shinies before feathers, though the event lists
/// them after — and names no store the haul left at nothing.
/// @spec TUI-NAV-147
#[test]
fn a_pounces_line_names_each_store_it_raised_in_resource_order() {
    let state = journaled(&[pounce(3, 12.5, 0.0, 2.0, 1)]);
    let want: Vec<String> = [
        (ResourceId::Shinies, 2.0),
        (ResourceId::Feathers, 12.5),
        (ResourceId::Unicorns, 1.0),
    ]
    .iter()
    .map(|(r, a)| format!("+{} {}", fmt::compact(*a), label(*r)))
    .collect();
    for (what, text) in [
        ("the ticker", ticker_text(&state)),
        ("the Journal", journal_text(&state)),
    ] {
        let at = positions(what, &text, &want);
        assert!(
            ascending(&at),
            "{what}: the stores stand in resource order:\n{text}"
        );
        assert!(
            !text.contains(&label(ResourceId::Bells)),
            "{what} names a store the haul left at nothing:\n{text}"
        );
    }
}

/// A trade's line names the colony, each store the event lists with its
/// amount in the event's own order, and how many caravans napped — and says
/// nothing of napping where none did.
/// @spec TUI-NAV-147
#[test]
fn a_trades_line_names_the_colony_its_goods_and_the_caravans_that_napped() {
    let goods = [(ResourceId::Twigs, 120.0), (ResourceId::Catnip, 30.0)];
    let napping = journaled(&[trade(ColonyId::Otters, 12, 7, &goods)]);
    let mut want = vec![content::colony_def(ColonyId::Otters).name.to_string()];
    want.extend(
        goods
            .iter()
            .map(|(r, a)| format!("+{} {}", fmt::compact(*a), label(*r))),
    );
    for (what, text) in [
        ("the ticker", ticker_text(&napping)),
        ("the Journal", journal_text(&napping)),
    ] {
        let at = positions(what, &text, &want);
        assert!(
            ascending(&at[1..]),
            "{what}: the goods stand in the event's order:\n{text}"
        );
        assert!(
            text.contains('7'),
            "{what} says how many caravans napped:\n{text}"
        );
    }

    let none_napped = journaled(&[trade(ColonyId::Otters, 12, 0, &goods)]);
    let text = ticker_text(&none_napped);
    assert!(
        !text.contains("napped"),
        "no caravan napped, yet the line says so:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-024 — the stores an act brought home wear their coats
// ---------------------------------------------------------------------------

/// Every character of each store's name on a line wears that store's coat.
fn assert_coated(what: &str, buf: &Buffer, rect: Rect, stores: &[ResourceId]) {
    let mut found = std::collections::BTreeSet::new();
    for (y, row) in lines(buf, rect).iter().enumerate() {
        let chars: Vec<char> = row.chars().collect();
        for r in stores {
            let name: Vec<char> = label(*r).chars().collect();
            let Some(at) = (0..chars.len().saturating_sub(name.len() - 1))
                .find(|&i| chars[i..i + name.len()] == name[..])
            else {
                continue;
            };
            let want = coat_colours(coat(*r), name.len());
            for (k, colour) in want.iter().enumerate() {
                let cell = buf
                    .cell((rect.x + (at + k) as u16, rect.y + y as u16))
                    .expect("a drawn cell");
                assert_eq!(
                    &cell.fg, colour,
                    "{what}: {r:?}'s name, character {k}: {row:?}"
                );
            }
            found.insert(*r);
        }
    }
    for r in stores {
        assert!(found.contains(r), "fixture: {what} names {r:?}");
    }
}

/// @spec TUI-THEME-024
#[test]
fn the_stores_an_act_brought_home_wear_their_coats_on_the_ticker() {
    let state = journaled(&[trade(
        ColonyId::Otters,
        4,
        0,
        &[(ResourceId::Twigs, 10.0), (ResourceId::Catnip, 3.0)],
    )]);
    let buf = ticker(&state);
    assert_coated(
        "the ticker",
        &buf,
        ticker_rect(),
        &[ResourceId::Twigs, ResourceId::Catnip],
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-148, TUI-NAV-149 — the last time, in the inspector
// ---------------------------------------------------------------------------

fn pounce_row() -> RowKey {
    RowKey::Fixed("Pounce")
}

fn trade_row(c: ColonyId) -> RowKey {
    RowKey::Colony(c, "trade")
}

/// Where the divider stands among the inspector's lines, if it is drawn.
fn divider_at(lines: &[String]) -> Option<usize> {
    lines.iter().position(|l| l.starts_with("── last time"))
}

/// The lines beneath the divider, down to the first blank.
fn last_time(lines: &[String]) -> Vec<String> {
    match divider_at(lines) {
        Some(at) => lines[at + 1..]
            .iter()
            .take_while(|l| !l.is_empty())
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}

/// A meadow whose journal holds each event at its own tick.
fn journaled_at(events: &[(u64, Event)]) -> GameState {
    let mut s = journaled(&[]);
    for (tick, e) in events {
        s.journal.record(*tick, e);
    }
    s
}

/// The Pounce row states the newest pounce's haul — never an older one's —
/// each store on a line of its own, name then amount in the exact form, in
/// the core's resource order.
/// @spec TUI-NAV-148
#[test]
fn the_pounce_row_states_what_the_newest_pounce_brought_home() {
    let state = journaled(&[pounce(9, 50.0, 50.0, 50.0, 5), pounce(3, 12.5, 4.0, 0.0, 1)]);
    let got = last_time(&inspector_lines(&state, Section::Clowder, pounce_row()));
    let want = vec![
        "3 squads came home with:".to_string(),
        format!("{} +{}", label(ResourceId::Feathers), fmt::exact(12.5)),
        format!("{} +{}", label(ResourceId::Bells), fmt::exact(4.0)),
        format!("{} +{}", label(ResourceId::Unicorns), fmt::exact(1.0)),
    ];
    assert_eq!(got, want);
}

/// One press of `X` makes one call per squad, every one at the same tick: the
/// last time adds up all of them, and none from an earlier press.
/// @spec TUI-NAV-148
#[test]
fn the_last_time_adds_up_every_return_of_the_newest_press() {
    let state = journaled_at(&[
        (100, pounce(1, 9.0, 9.0, 9.0, 9)),
        (200, pounce(1, 3.0, 0.0, 0.5, 0)),
        (
            200,
            trade(ColonyId::Otters, 1, 0, &[(ResourceId::Twigs, 5.0)]),
        ),
        (200, pounce(1, 4.0, 2.0, 0.0, 1)),
    ]);
    let got = last_time(&inspector_lines(&state, Section::Clowder, pounce_row()));
    let want = vec![
        "2 squads came home with:".to_string(),
        format!("{} +{}", label(ResourceId::Shinies), fmt::exact(0.5)),
        format!("{} +{}", label(ResourceId::Feathers), fmt::exact(7.0)),
        format!("{} +{}", label(ResourceId::Bells), fmt::exact(2.0)),
        format!("{} +{}", label(ResourceId::Unicorns), fmt::exact(1.0)),
    ];
    assert_eq!(got, want);

    let state = journaled_at(&[
        (
            100,
            trade(ColonyId::Otters, 1, 0, &[(ResourceId::Catnip, 99.0)]),
        ),
        (
            200,
            trade(ColonyId::Otters, 1, 0, &[(ResourceId::Twigs, 5.0)]),
        ),
        (
            200,
            trade(ColonyId::Foxes, 1, 0, &[(ResourceId::Pebbles, 8.0)]),
        ),
        (200, trade(ColonyId::Otters, 1, 1, &[])),
        (
            200,
            trade(
                ColonyId::Otters,
                1,
                0,
                &[(ResourceId::Catnip, 2.0), (ResourceId::Twigs, 1.5)],
            ),
        ),
    ]);
    let got = last_time(&inspector_lines(
        &state,
        Section::Friends,
        trade_row(ColonyId::Otters),
    ));
    let want = vec![
        "3 caravans came home with:".to_string(),
        format!("{} +{}", label(ResourceId::Catnip), fmt::exact(2.0)),
        format!("{} +{}", label(ResourceId::Twigs), fmt::exact(6.5)),
        "1 of them napped.".to_string(),
    ];
    assert_eq!(got, want);
}

/// One squad, one caravan: the noun is singular.
/// @spec TUI-NAV-148
#[test]
fn one_squad_or_caravan_is_counted_in_the_singular() {
    let state = journaled(&[
        pounce(1, 2.0, 0.0, 0.0, 0),
        trade(ColonyId::Otters, 1, 0, &[(ResourceId::Twigs, 5.0)]),
    ]);
    let p = last_time(&inspector_lines(&state, Section::Clowder, pounce_row()));
    assert_eq!(
        p.first().map(String::as_str),
        Some("1 squad came home with:")
    );
    let t = last_time(&inspector_lines(
        &state,
        Section::Friends,
        trade_row(ColonyId::Otters),
    ));
    assert_eq!(
        t.first().map(String::as_str),
        Some("1 caravan came home with:")
    );
}

/// A Trade row states its own colony's newest trade, not a later trade with
/// another colony, with the napped caravans beneath the goods.
/// @spec TUI-NAV-148
#[test]
fn a_trade_row_states_its_own_colonys_newest_trade() {
    let state = journaled(&[
        trade(ColonyId::Otters, 9, 0, &[(ResourceId::Twigs, 999.0)]),
        trade(
            ColonyId::Otters,
            5,
            2,
            &[(ResourceId::Catnip, 30.0), (ResourceId::Twigs, 120.0)],
        ),
        trade(ColonyId::Foxes, 4, 0, &[(ResourceId::Pebbles, 8.0)]),
    ]);
    let got = last_time(&inspector_lines(
        &state,
        Section::Friends,
        trade_row(ColonyId::Otters),
    ));
    let want = vec![
        "5 caravans came home with:".to_string(),
        format!("{} +{}", label(ResourceId::Catnip), fmt::exact(30.0)),
        format!("{} +{}", label(ResourceId::Twigs), fmt::exact(120.0)),
        "2 of them napped.".to_string(),
    ];
    assert_eq!(got, want);
}

/// An act that raised nothing says the squads or caravans came home, and
/// lists nothing beneath but the napped caravans.
/// @spec TUI-NAV-148
#[test]
fn an_act_that_raised_nothing_says_only_that_it_came_home() {
    let state = journaled(&[
        trade(ColonyId::Otters, 5, 5, &[]),
        pounce(2, 0.0, 0.0, 0.0, 0),
    ]);
    let t = last_time(&inspector_lines(
        &state,
        Section::Friends,
        trade_row(ColonyId::Otters),
    ));
    assert_eq!(
        t,
        vec![
            "5 caravans came home.".to_string(),
            "5 of them napped.".to_string()
        ]
    );
    let p = last_time(&inspector_lines(&state, Section::Clowder, pounce_row()));
    assert_eq!(p, vec!["2 squads came home.".to_string()]);
}

/// The last time stands beneath the row's price, set apart by its divider:
/// below every other line the inspector draws for the row.
/// @spec TUI-NAV-148
#[test]
fn the_last_time_stands_beneath_the_price_under_its_divider() {
    let state = journaled(&[pounce(3, 12.5, 4.0, 0.0, 1)]);
    let all = inspector_lines(&state, Section::Clowder, pounce_row());
    let at = divider_at(&all).expect("a divider stands above the last time");
    let price = all
        .iter()
        .position(|l| l.contains(&label(ResourceId::Pep)))
        .expect("fixture: the one-unit price names its pep");
    assert!(
        price < at,
        "the last time stands beneath the price:\n{}",
        all.join("\n")
    );
    let block = last_time(&all).len();
    assert!(block > 0, "lines stand beneath the divider");
    assert!(
        all[at + 1 + block..].iter().all(|l| l.is_empty()),
        "nothing is drawn beneath the last time:\n{}",
        all.join("\n")
    );
}

/// The divider is a group header's line across the inspector's width, with
/// the words `last time`, every cell in `dim`.
/// @spec TUI-NAV-148
#[test]
fn the_divider_is_a_dim_group_header_across_the_inspector() {
    let state = journaled(&[pounce(3, 12.5, 4.0, 0.0, 1)]);
    let rect = inspector_rect();
    let buf = inspector_buf(&state, Section::Clowder, pounce_row());
    let all = lines(&buf, rect);
    let at = divider_at(&all).expect("a divider");
    assert_eq!(
        all[at],
        fmt::group_header_line("last time", rect.width as usize)
    );
    let dim = clowder_tui::theme::Role::Dim
        .color()
        .expect("dim has a colour");
    for x in rect.x..rect.right() {
        let cell = buf.cell((x, rect.y + at as u16)).expect("a cell");
        assert_eq!(cell.fg, dim, "column {x} of the divider");
    }
}

/// Where the journal holds no such act, the row draws no divider and states
/// no last time — not another act's, and not another colony's.
/// @spec TUI-NAV-149
#[test]
fn a_row_whose_act_the_journal_does_not_hold_states_no_last_time() {
    let state = journaled(&[trade(ColonyId::Foxes, 4, 0, &[(ResourceId::Pebbles, 8.0)])]);
    for (section, key) in [
        (Section::Clowder, pounce_row()),
        (Section::Friends, trade_row(ColonyId::Otters)),
    ] {
        let all = inspector_lines(&state, section, key.clone());
        assert!(
            divider_at(&all).is_none() && !all.iter().any(|l| l.contains("came home")),
            "{key:?} states a last time the journal does not hold:\n{}",
            all.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-147, TUI-NAV-155 — watching the sky is answered on the ticker
// ---------------------------------------------------------------------------

/// A sky observation's line names each store the sky raised, with its amount,
/// in resource order, and says differently whether the clowder looked up or
/// a kitten happened to.
/// @spec TUI-NAV-147
#[test]
fn a_sky_observations_line_names_what_the_sky_gave() {
    let watched = journaled(&[Event::SkyObserved {
        curiosity: 87.5,
        skymaps: 1.0,
        by_chance: false,
    }]);
    let lucky = journaled(&[Event::SkyObserved {
        curiosity: 87.5,
        skymaps: 1.0,
        by_chance: true,
    }]);
    let want: Vec<String> = [(ResourceId::Curiosity, 87.5), (ResourceId::Skymaps, 1.0)]
        .iter()
        .map(|(r, a)| format!("+{} {}", fmt::compact(*a), label(*r)))
        .collect();
    for (what, text) in [
        ("the ticker", ticker_text(&watched)),
        ("the Journal", journal_text(&watched)),
        ("the ticker by chance", ticker_text(&lucky)),
    ] {
        let at = positions(what, &text, &want);
        assert!(ascending(&at), "{what}: resource order:\n{text}");
    }
    assert_ne!(ticker_text(&watched), ticker_text(&lucky));

    let capped = journaled(&[Event::SkyObserved {
        curiosity: 87.5,
        skymaps: 0.0,
        by_chance: false,
    }]);
    let text = ticker_text(&capped);
    assert!(
        !text.contains(&label(ResourceId::Skymaps)),
        "a store the sky did not raise is not named:\n{text}"
    );
}

/// An accepted `o` leaves no footer note: the observation's line answers it.
/// @spec TUI-NAV-155
#[test]
fn an_accepted_observation_leaves_no_footer_note() {
    let mut state = GameState::new(5);
    state.pending_sky_event = Some(state.ticks + 1000);
    let mut view = ViewState::default();
    let outcome = clowder_tui::nav::update(clowder_tui::nav::Action::ObserveSky, &mut view, &state);
    assert!(
        matches!(
            outcome,
            clowder_tui::nav::Outcome::Apply(clowder_core::Command::ObserveSky)
        ),
        "{outcome:?}"
    );
    assert!(view.footer.is_none(), "{:?}", view.footer);
    let events = state
        .apply(clowder_core::Command::ObserveSky)
        .expect("a waiting sky is observed");
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::SkyObserved {
                by_chance: false,
                ..
            }
        )),
        "the core answers with the observation: {events:?}"
    );
}

/// The names in the last time wear their coats.
/// @spec TUI-THEME-019
#[test]
fn the_stores_in_the_last_time_wear_their_coats() {
    let state = journaled(&[trade(
        ColonyId::Otters,
        5,
        0,
        &[(ResourceId::Catnip, 30.0), (ResourceId::Twigs, 120.0)],
    )]);
    let buf = inspector_buf(&state, Section::Friends, trade_row(ColonyId::Otters));
    assert_coated(
        "the inspector",
        &buf,
        inspector_rect(),
        &[ResourceId::Catnip, ResourceId::Twigs],
    );
}

/// End to end against the core: five calls of one squad each, as one `X`
/// press makes them, read back as five squads and everything they raised.
/// @spec TUI-NAV-148
#[test]
fn five_calls_of_one_press_read_back_as_five_squads() {
    let mut state = journaled(&[]);
    let pep = 5.0 * content::POUNCE_PEP_PER_SQUAD;
    state.resources[ResourceId::Pep.ix()] = pep;
    let before = state.resources;
    for _ in 0..5 {
        state
            .apply(clowder_core::Command::Pounce(1))
            .expect("five squads are paid for");
    }
    let got = last_time(&inspector_lines(&state, Section::Clowder, pounce_row()));
    assert_eq!(
        got.first().map(String::as_str),
        Some("5 squads came home with:")
    );
    for line in &got[1..] {
        let r = ResourceId::ALL
            .into_iter()
            .find(|r| line.starts_with(&format!("{} +", label(*r))))
            .unwrap_or_else(|| panic!("a store's line: {line:?}"));
        let rose = state.resources[r.ix()] - before[r.ix()];
        assert_eq!(line, &format!("{} +{}", label(r), fmt::exact(rose)));
    }
}
