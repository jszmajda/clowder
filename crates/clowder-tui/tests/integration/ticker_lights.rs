//! The ticker's line: the newest event on the left, drawn plainly, and on the
//! right a light for each timed state a glance can use, with its clock.
//!
//! @spec TUI-NAV-019, TUI-NAV-150, TUI-NAV-151, TUI-NAV-152, TUI-NAV-153,
//! TUI-THEME-015, TUI-THEME-025, TUI-THEME-026

use clowder_core::{content, Event, GameState, Season};
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use clowder_tui::nav::ViewState;
use clowder_tui::render;
use clowder_tui::render::widgets::overlays;
use clowder_tui::theme::Role;

const FLOOR_W: u16 = 80;
const FLOOR_H: u16 = 24;
const TPS: u64 = content::TICKS_PER_SECOND;

fn row(w: u16) -> Rect {
    Rect::new(0, 0, w, 1)
}

fn ticker_at(state: &GameState, w: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, 1)).expect("TestBackend is infallible");
    term.draw(|f| overlays::ticker(f, row(w), state))
        .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

fn cells(buf: &Buffer, w: u16) -> Vec<&Cell> {
    (0..w)
        .map(|x| buf.cell((x, 0)).expect("a drawn cell"))
        .collect()
}

fn text_at(state: &GameState, w: u16) -> String {
    cells(&ticker_at(state, w), w)
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// A meadow a little way in, off every boundary, with nothing running.
fn meadow() -> GameState {
    let mut s = GameState::new(41);
    s.ticks = 3 * 1000 + 437;
    s.speed = 1.0;
    s
}

/// A meadow whose journal's newest entry is `event`.
fn with_newest(mut s: GameState, event: Event) -> GameState {
    s.journal.record(s.ticks, &event);
    s
}

fn season() -> Event {
    Event::SeasonChanged {
        season: Season::Winter,
    }
}

/// Seconds of real time at the meadow's speed, as ticks.
fn ticks_for(s: &GameState, seconds: f64) -> u64 {
    (seconds * TPS as f64 * s.speed).round() as u64
}

fn sky(s: &mut GameState, seconds: f64) {
    s.pending_sky_event = Some(s.ticks + ticks_for(s, seconds));
}

fn zoomies(s: &mut GameState, seconds: f64) {
    s.zoomies_until = s.ticks + ticks_for(s, seconds);
}

fn jamboree(s: &mut GameState, seconds: f64) {
    s.jamboree_until = s.ticks + ticks_for(s, seconds);
}

/// All four lights lit.
fn all_four() -> GameState {
    let mut s = meadow();
    sky(&mut s, 180.0);
    s.whale_visit_years = 5;
    jamboree(&mut s, 200.0);
    zoomies(&mut s, 25.0);
    s
}

// ---------------------------------------------------------------------------
// TUI-NAV-151 — which lights
// ---------------------------------------------------------------------------

/// Nothing running: no light.
/// @spec TUI-NAV-151
#[test]
fn a_meadow_with_nothing_on_a_clock_shows_no_light() {
    let text = text_at(&with_newest(meadow(), season()), FLOOR_W);
    assert!(!text.contains('◇') && !text.contains("!sky"), "{text:?}");
}

/// Each timed state lights its own light while the core lists it, and a
/// déjà-vu day lights none.
/// @spec TUI-NAV-151
#[test]
fn each_timed_state_lights_its_own_light_and_a_deja_vu_day_none() {
    let mut s = meadow();
    sky(&mut s, 60.0);
    assert!(text_at(&s, FLOOR_W).contains("!sky"));

    let mut s = meadow();
    s.whale_visit_years = 2;
    assert!(text_at(&s, FLOOR_W).contains("◇ whale"));

    let mut s = meadow();
    jamboree(&mut s, 60.0);
    assert!(text_at(&s, FLOOR_W).contains("◇ jamboree"));

    let mut s = meadow();
    zoomies(&mut s, 60.0);
    assert!(text_at(&s, FLOOR_W).contains("◇ zoomies"));

    let mut s = meadow();
    s.deja_vu_until = s.ticks + 7;
    assert!(!s.timers().is_empty(), "fixture: the core lists the day");
    let text = text_at(&s, FLOOR_W);
    assert!(
        text.trim().is_empty(),
        "a déjà-vu day lights nothing: {text:?}"
    );
}

/// A sky event the core no longer lists — faded — has no light.
/// @spec TUI-NAV-151
#[test]
fn a_faded_sky_event_has_no_light() {
    let mut s = meadow();
    s.pending_sky_event = Some(s.ticks);
    assert!(s.timers().is_empty(), "fixture: the core lists nothing");
    assert!(!text_at(&s, FLOOR_W).contains("!sky"));
}

// ---------------------------------------------------------------------------
// TUI-NAV-152, TUI-NAV-153 — a light's form, and its clock
// ---------------------------------------------------------------------------

/// The sky's light is its chip's glyph and word, its clock, and its key; the
/// others are `◇`, their word, and their clock.
/// @spec TUI-NAV-152
#[test]
fn each_light_has_its_form() {
    let text = text_at(&all_four(), 120);
    for form in ["!sky 3m (o)", "◇ whale ", "◇ jamboree 3m", "◇ zoomies 20s"] {
        assert!(text.contains(form), "no {form:?} in {text:?}");
    }
}

/// The clock in each of its steps, rounded down.
/// @spec TUI-NAV-153
#[test]
fn the_clock_states_real_time_in_coarse_steps_rounded_down() {
    let cases = [
        (2.0, "<10s"),
        (9.75, "<10s"),
        (10.0, "10s"),
        (19.75, "10s"),
        (59.75, "50s"),
        (60.0, "1m"),
        (119.75, "1m"),
        (3599.75, "59m"),
        (3600.0, "1h"),
        (3.0 * 3600.0 + 1799.0, "3h"),
    ];
    for (seconds, clock) in cases {
        let mut s = meadow();
        zoomies(&mut s, seconds);
        let text = text_at(&s, FLOOR_W);
        assert!(
            text.trim_end().ends_with(&format!("◇ zoomies {clock}")),
            "{seconds}s left should read {clock}: {text:?}"
        );
    }
}

/// The clock is real time at the current speed: the same ticks left read
/// half as long at twice the speed.
/// @spec TUI-NAV-153
#[test]
fn the_clock_follows_the_speed() {
    let mut s = meadow();
    s.zoomies_until = s.ticks + 120 * TPS;
    assert!(text_at(&s, FLOOR_W).trim_end().ends_with("◇ zoomies 2m"));
    s.speed = 2.0;
    assert!(text_at(&s, FLOOR_W).trim_end().ends_with("◇ zoomies 1m"));
    s.speed = 8.0;
    assert!(text_at(&s, FLOOR_W).trim_end().ends_with("◇ zoomies 10s"));
}

/// A whale's clock runs to the year close where its last year runs out, as
/// the core reports it.
/// @spec TUI-NAV-153
#[test]
fn a_whales_clock_is_the_cores_ticks_left() {
    let mut s = meadow();
    s.whale_visit_years = 1;
    let left = s.timers()[0].1;
    let minutes = left / TPS / 60;
    let text = text_at(&s, FLOOR_W);
    assert!(
        text.trim_end().ends_with(&format!("◇ whale {minutes}m")),
        "{left} ticks left: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-150 — the line's two parts
// ---------------------------------------------------------------------------

fn lights_of(text: &str) -> Vec<(usize, &'static str)> {
    let mut at: Vec<(usize, &'static str)> = ["!sky", "◇ whale", "◇ jamboree", "◇ zoomies"]
        .into_iter()
        .filter_map(|l| text.find(l).map(|i| (text[..i].chars().count(), l)))
        .collect();
    at.sort();
    at
}

/// The lights stand right-aligned in the fixed order, two blank columns
/// apart, the last ending at the line's right edge.
/// @spec TUI-NAV-150
#[test]
fn the_lights_stand_right_aligned_in_order_two_columns_apart() {
    let text = text_at(&all_four(), FLOOR_W);
    let chars: Vec<char> = text.chars().collect();
    let order: Vec<&str> = lights_of(&text).into_iter().map(|(_, l)| l).collect();
    assert_eq!(
        order,
        ["!sky", "◇ whale", "◇ jamboree", "◇ zoomies"],
        "{text:?}"
    );
    assert_ne!(chars[FLOOR_W as usize - 1], ' ', "right-aligned: {text:?}");
    let starts = lights_of(&text);
    for pair in starts.windows(2) {
        let next = pair[1].0;
        assert_eq!(&chars[next - 2..next], &[' ', ' '], "two blanks: {text:?}");
        assert_ne!(chars[next - 3], ' ', "exactly two blanks: {text:?}");
    }
}

/// A newest event longer than the room the lights leave is shortened with an
/// ellipsis, two blank columns short of the first light, and no light is cut.
/// @spec TUI-NAV-150
#[test]
fn a_long_newest_event_gives_way_to_the_lights() {
    let mut s = all_four();
    s = with_newest(
        s,
        Event::KittenArrived {
            name: "Bartholomew Wigglesworth-Fluffington the Third".into(),
        },
    );
    let text = text_at(&s, FLOOR_W);
    let chars: Vec<char> = text.chars().collect();
    let first = lights_of(&text)[0].0;
    assert_eq!(chars[first - 1], ' ');
    assert_eq!(chars[first - 2], ' ');
    assert_eq!(
        chars[first - 3],
        '…',
        "shortened with an ellipsis: {text:?}"
    );
    assert!(text.starts_with("▪ Bartholomew"), "{text:?}");
    assert_eq!(lights_of(&text).len(), 4, "every light whole: {text:?}");
}

/// A newest event that fits is drawn whole, and the lights stand where they
/// would without it.
/// @spec TUI-NAV-150
#[test]
fn a_newest_event_that_fits_is_whole() {
    let quiet = text_at(&all_four(), FLOOR_W);
    let busy = text_at(&with_newest(all_four(), season()), FLOOR_W);
    assert!(!busy.contains('…'), "{busy:?}");
    assert!(busy.starts_with("▪ "), "{busy:?}");
    assert_eq!(lights_of(&quiet), lights_of(&busy));
}

/// In the whole frame, the ticker's row carries the lights.
/// @spec TUI-NAV-019
#[test]
fn the_frames_ticker_row_carries_the_lights() {
    let s = all_four();
    let mut term =
        Terminal::new(TestBackend::new(FLOOR_W, FLOOR_H)).expect("TestBackend is infallible");
    let mut layout = None;
    term.draw(|f| layout = Some(render::frame(f, &s, &ViewState::default(), None)))
        .expect("TestBackend is infallible");
    let t = layout.expect("a layout").ticker;
    let buf = term.backend().buffer();
    let line: String = (t.x..t.right())
        .map(|x| buf.cell((x, t.y)).expect("a cell").symbol())
        .collect();
    assert_eq!(lights_of(&line).len(), 4, "{line:?}");
}

// ---------------------------------------------------------------------------
// Colour — TUI-THEME-015, TUI-THEME-025, TUI-THEME-026
// ---------------------------------------------------------------------------

/// The columns `form` covers on the line.
fn span_of(text: &str, form: &str) -> std::ops::Range<usize> {
    let i = text
        .find(form)
        .unwrap_or_else(|| panic!("no {form:?} in {text:?}"));
    let start = text[..i].chars().count();
    start..start + form.chars().count()
}

/// The sky's light wears `attention` whole; the blanks around it do not.
/// @spec TUI-THEME-015
#[test]
fn the_sky_light_wears_attention_whole() {
    let s = all_four();
    let buf = ticker_at(&s, FLOOR_W);
    let text = text_at(&s, FLOOR_W);
    let cells = cells(&buf, FLOOR_W);
    let span = span_of(&text, "!sky 3m (o)");
    let attention = Role::Attention.color().expect("attention has a colour");
    for x in span.clone() {
        if cells[x].symbol() != " " {
            assert_eq!(
                cells[x].fg, attention,
                "column {x} of the sky light: {text:?}"
            );
        }
    }
    for x in [span.start - 1, span.end] {
        assert_ne!(
            cells[x].fg, attention,
            "the blank at {x} is not the light's"
        );
    }
}

/// A running light's glyph and word are in the inherited foreground and its
/// clock in `dim`.
/// @spec TUI-THEME-026
#[test]
fn a_running_lights_words_are_plain_and_its_clock_dim() {
    let s = all_four();
    let buf = ticker_at(&s, FLOOR_W);
    let text = text_at(&s, FLOOR_W);
    let cells = cells(&buf, FLOOR_W);
    let dim = Role::Dim.color().expect("dim has a colour");
    for (word, clock) in [("◇ jamboree", "3m"), ("◇ zoomies", "20s")] {
        let w = span_of(&text, word);
        for x in w.clone() {
            if cells[x].symbol() != " " {
                assert_eq!(cells[x].fg, Color::Reset, "{word} at {x}: {text:?}");
                assert!(!cells[x].modifier.contains(Modifier::DIM));
            }
        }
        for (x, cell) in cells.iter().enumerate().skip(w.end + 1).take(clock.len()) {
            assert_eq!(cell.fg, dim, "{word}'s clock at {x}: {text:?}");
        }
    }
}

/// The newest event is plain: a mechanical event in the inherited foreground,
/// a flavor event in `dim`, and no faint attribute on either.
/// @spec TUI-THEME-025
#[test]
fn the_newest_event_is_drawn_plainly_and_flavor_recedes() {
    let dim = Role::Dim.color().expect("dim has a colour");
    let mechanical = ticker_at(&with_newest(meadow(), season()), FLOOR_W);
    let flavor = ticker_at(
        &with_newest(
            meadow(),
            Event::Flavor {
                template: 0,
                kitten: None,
            },
        ),
        FLOOR_W,
    );
    for (what, buf, want) in [
        ("mechanical", &mechanical, Color::Reset),
        ("flavor", &flavor, dim),
    ] {
        let drawn: Vec<&Cell> = cells(buf, FLOOR_W)
            .into_iter()
            .filter(|c| c.symbol() != " ")
            .collect();
        assert!(!drawn.is_empty(), "fixture: {what} drew something");
        for c in drawn {
            assert_eq!(c.fg, want, "{what}: {:?}", c.symbol());
            assert!(
                !c.modifier.contains(Modifier::DIM),
                "{what}: no faint attribute"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-154 — the running lights' glyph in the legend
// ---------------------------------------------------------------------------

/// The help overlay's glyph legend lists `◇` once, as the mark of something
/// running on a clock.
/// @spec TUI-NAV-154
#[test]
fn the_help_overlays_glyph_legend_lists_the_running_lights_mark() {
    let (w, h) = (120u16, 60u16);
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible");
    term.draw(|f| {
        let area = f.area();
        overlays::help(f, area);
    })
    .expect("TestBackend is infallible");
    let buf = term.backend().buffer();
    let lines: Vec<String> = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf.cell((x, y)).expect("a cell").symbol())
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|l| l.starts_with('◇'))
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("clock"), "{lines:?}");
}
