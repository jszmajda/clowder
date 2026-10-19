//! Scrolling the stores sidebar: a window onto the stores, in the stores
//! group's order, moved by `{` and `}` and the wheel and by nothing else.
//!
//! @spec TUI-NAV-140, TUI-NAV-141, TUI-NAV-142, TUI-NAV-143, TUI-NAV-144,
//!       TUI-NAV-145, TUI-NAV-146, TUI-RENDER-022, TUI-RENDER-025,
//!       TUI-RENDER-026, TUI-RENDER-027, TUI-THEME-015
//!
//! Where the window should stand is worked out here from the specs alone
//! (`expected`), and the frame is read back as drawn, so a drawing that agreed
//! only with its own record would still fail.

use std::time::{Duration, Instant};

use clowder_core::{Command, GameState, ResourceId};
use clowder_tui::nav::rows::rows;
use clowder_tui::nav::{
    self, Action, ArmedRow, CountPrompt, Outcome, Overlay, PendingReport, RowKey, Section,
    ViewState,
};
use clowder_tui::render;
use clowder_tui::render::layout::{Body, Layout};
use clowder_tui::render::pointer::PointerRecord;
use clowder_tui::render::widgets::chrome;
use clowder_tui::theme::Role;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;

const SMALL: (u16, u16) = (80, 24);
const WIDE: (u16, u16) = (130, 40);
const SIZES: [(u16, u16); 2] = [SMALL, WIDE];

// ---------------------------------------------------------------------------
// Fixtures and readers
// ---------------------------------------------------------------------------

/// A meadow that has met far more stores than any sidebar has lines for:
/// every resource with room for some held once — capped stores at half their
/// cap, so none wears `full` — and a tick on, so the core records each.
fn crowded() -> GameState {
    let mut s = GameState::new(7);
    for r in ResourceId::ALL {
        let cap = s.cap(r);
        s.resources[r.ix()] = if cap.is_finite() { cap / 2.0 } else { 5.0 };
    }
    s.tick();
    let k = listed(&s).len();
    assert!(
        k > 20,
        "fixture: a crowded meadow lists more stores than fit ({k})"
    );
    s
}

/// A meadow whose few stores all fit in any sidebar.
fn sparse() -> GameState {
    let mut s = GameState::new(7);
    s.resources[ResourceId::Catnip.ix()] = 5.0;
    s.tick();
    s
}

fn listed(state: &GameState) -> Vec<ResourceId> {
    rows(state, Section::Meadow)
        .into_iter()
        .filter_map(|r| match r.key {
            RowKey::Resource(r) => Some(r),
            _ => None,
        })
        .collect()
}

fn fill(state: &mut GameState, r: ResourceId) {
    let cap = state.cap(r);
    assert!(cap.is_finite() && cap > 0.0, "fixture: {r:?} is capped");
    state.resources[r.ix()] = cap;
}

fn capped(state: &GameState, r: ResourceId) -> bool {
    let cap = state.cap(r);
    cap.is_finite() && cap > 0.0
}

fn standing(section: Section) -> ViewState {
    ViewState {
        section: Some(section),
        ..Default::default()
    }
}

/// The view scrolled so its first store is `listed[want]` — the top where
/// `want` is zero.
fn scrolled(state: &GameState, section: Section, want: usize) -> ViewState {
    let mut v = standing(section);
    v.sidebar_top = (want > 0).then(|| listed(state)[want]);
    v
}

struct Shot {
    buf: Buffer,
    layout: Layout,
    record: PointerRecord,
    size: (u16, u16),
}

fn shoot(state: &GameState, view: &ViewState, size: (u16, u16)) -> Shot {
    let mut term = Terminal::new(TestBackend::new(size.0, size.1)).expect("TestBackend");
    let mut out = None;
    term.draw(|f| out = Some(render::frame_recorded(f, state, view, None)))
        .expect("TestBackend is infallible");
    let (layout, record) = out.expect("drawn");
    Shot {
        buf: term.backend().buffer().clone(),
        layout,
        record,
        size,
    }
}

fn sidebar_rect(l: &Layout) -> Rect {
    match l.body {
        Body::Narrow { sidebar, .. } | Body::Wide { sidebar, .. } => sidebar,
        Body::TooSmall(r) => panic!("fixture: {r:?} is below the floor"),
    }
}

fn line(buf: &Buffer, y: u16, x0: u16, x1: u16) -> String {
    (x0..x1)
        .map(|x| {
            buf.cell((x, y))
                .map(|c| c.symbol().to_string())
                .unwrap_or_default()
        })
        .collect()
}

fn lines_in(buf: &Buffer, r: Rect) -> Vec<String> {
    (r.y..r.bottom())
        .map(|y| line(buf, y, r.x, r.right()))
        .collect()
}

/// Each listed store's name as a sidebar `width` columns wide draws it: read
/// from a sidebar tall enough to hold every store, which draws them all in
/// the stores group's order.
fn names_at(state: &GameState, width: u16) -> Vec<String> {
    let k = listed(state).len();
    let h = (2 * k + 2) as u16;
    let mut term = Terminal::new(TestBackend::new(width, h)).expect("TestBackend");
    term.draw(|f| {
        chrome::sidebar(f, Rect::new(0, 0, width, h), state, &ViewState::default());
    })
    .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    let name_col = (width as usize).saturating_sub(14).max(1);
    (0..k)
        .map(|i| {
            line(&buf, (2 * i) as u16, 0, width)
                .chars()
                .take(name_col)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

/// Where TUI-RENDER-022 puts the window: its first store's place, how many
/// stores it draws, and how many stand above and below it.
#[derive(Debug, PartialEq, Clone, Copy)]
struct Window {
    start: usize,
    drawn: usize,
    above: usize,
    below: usize,
    max_start: Option<usize>,
}

fn expected(k: usize, h: usize, want: usize) -> Window {
    if 2 * k <= h {
        return Window {
            start: 0,
            drawn: k,
            above: 0,
            below: 0,
            max_start: None,
        };
    }
    // The earliest store from which every store through the last fits
    // beneath the head note.
    let max_start = k - (h - 1) / 2;
    let start = want.min(max_start);
    let avail = h - usize::from(start > 0);
    let drawn = if 2 * (k - start) <= avail {
        k - start
    } else {
        (avail - 1) / 2
    };
    Window {
        start,
        drawn,
        above: start,
        below: k - start - drawn,
        max_start: Some(max_start),
    }
}

fn note_text(glyph: char, n: usize, full: usize) -> String {
    if full > 0 {
        format!("{glyph} {n} more · {full} full")
    } else {
        format!("{glyph} {n} more")
    }
}

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn wheel(kind: MouseEventKind, x: u16, y: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }
}

fn whole(view: &ViewState) -> String {
    format!("{view:?}")
}

/// The view with the sidebar's scroll set aside.
fn but_the_scroll(view: &ViewState) -> String {
    let mut v = view.clone();
    v.sidebar_top = None;
    format!("{v:?}")
}

// ---------------------------------------------------------------------------
// TUI-RENDER-022, TUI-RENDER-025 — the window, and the record of it
// ---------------------------------------------------------------------------

/// From every place it can be scrolled to, at both sizes, the sidebar draws
/// consecutive stores in the stores group's order, each whole, with `▴ N more`
/// above and `▾ N more` below wherever stores stand out of view — and never
/// runs past the last store, drawing it at the foot instead. The frame's
/// record holds the same window.
/// @spec TUI-RENDER-022, TUI-RENDER-025
#[test]
fn the_sidebar_draws_a_window_of_whole_stores_from_where_it_is_scrolled() {
    let state = crowded();
    let stores = listed(&state);
    let k = stores.len();
    let mut clamped = 0;
    for size in SIZES {
        let probe = shoot(&state, &standing(Section::Meadow), size);
        let side = sidebar_rect(&probe.layout);
        let names = names_at(&state, side.width);
        let h = side.height as usize;
        for want in 0..k {
            let view = scrolled(&state, Section::Meadow, want);
            let shot = shoot(&state, &view, size);
            let lines = lines_in(&shot.buf, side);
            let w = expected(k, h, want);
            if w.start < want {
                clamped += 1;
            }
            let head = usize::from(w.above > 0);
            if w.above > 0 {
                assert_eq!(
                    lines[0].trim(),
                    note_text('▴', w.above, 0),
                    "{size:?} scrolled to {want}: the head note counts the stores above.\n{}",
                    lines.join("\n")
                );
            }
            for i in 0..w.drawn {
                let at = head + 2 * i;
                assert!(
                    lines[at].starts_with(&names[w.start + i]),
                    "{size:?} scrolled to {want}: line {at} begins store {}, {:?}.\n{}",
                    w.start + i,
                    stores[w.start + i],
                    lines.join("\n")
                );
                assert!(
                    lines[at + 1].starts_with(['▓', '░']),
                    "{size:?} scrolled to {want}: {:?} is drawn whole.\n{}",
                    stores[w.start + i],
                    lines.join("\n")
                );
            }
            let after = head + 2 * w.drawn;
            let tail = if w.below > 0 { h - 1 } else { h };
            assert!(
                lines[after..tail].iter().all(|l| l.trim().is_empty()),
                "{size:?} scrolled to {want}: nothing but blank lines between the last store and \
                 the foot.\n{}",
                lines.join("\n")
            );
            if w.below > 0 {
                assert_eq!(
                    lines[h - 1].trim(),
                    note_text('▾', w.below, 0),
                    "{size:?} scrolled to {want}: the foot note counts the stores below.\n{}",
                    lines.join("\n")
                );
            } else {
                assert!(w.start + w.drawn == k, "the last store stands at the foot");
            }
            let rec = shot.record.sidebar().expect("the frame drew a sidebar");
            assert_eq!(rec.region, side, "the record holds the sidebar's region");
            assert_eq!(rec.stores, stores, "the record holds the stores in order");
            assert_eq!(
                (rec.start, rec.drawn, rec.max_start),
                (w.start, w.drawn, w.max_start),
                "{size:?} scrolled to {want}: the record holds the window drawn"
            );
        }
    }
    assert!(clamped > 0, "fixture: some scroll ran past the last store");
}

/// Where every store fits, the sidebar draws them all, from the top, with no
/// note, whatever store the view remembers; and its record says there is
/// nowhere to scroll.
/// @spec TUI-RENDER-022, TUI-RENDER-025
#[test]
fn where_every_store_fits_the_sidebar_draws_them_all_and_has_nothing_to_scroll() {
    let state = sparse();
    let stores = listed(&state);
    for size in SIZES {
        let mut view = standing(Section::Meadow);
        view.sidebar_top = stores.last().copied();
        let shot = shoot(&state, &view, size);
        let side = sidebar_rect(&shot.layout);
        let lines = lines_in(&shot.buf, side);
        assert!(
            !lines.iter().any(|l| l.contains("more")),
            "{size:?}: no note where every store fits.\n{}",
            lines.join("\n")
        );
        let rec = shot.record.sidebar().expect("drawn");
        assert_eq!(
            (rec.start, rec.drawn, rec.max_start),
            (0, stores.len(), None),
            "{size:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-RENDER-027, TUI-THEME-015 — a note's full count
// ---------------------------------------------------------------------------

/// A note counting stores that wear `full` says how many, in `attention`, the
/// rest of the note in `dim`.
/// @spec TUI-RENDER-027, TUI-THEME-015
#[test]
fn a_note_says_how_many_of_the_stores_it_counts_are_full_in_attention() {
    let base = crowded();
    let stores = listed(&base);
    let k = stores.len();
    let capped_at: Vec<usize> = (0..k).filter(|i| capped(&base, stores[*i])).collect();
    let attention = Role::Attention.color().expect("attention carries a colour");
    let dim = Role::Dim.color().expect("dim carries a colour");
    // The capped stores stand together in the core's order, so the sidebar
    // is drawn short, at the frame's width, for a window with capped stores
    // out of view on both sides.
    let width = sidebar_rect(&shoot(&base, &standing(Section::Meadow), SMALL).layout).width;
    let h = 8u16;
    let (want, w) = (1..k)
        .map(|want| (want, expected(k, h as usize, want)))
        .find(|(_, w)| {
            capped_at.iter().any(|i| *i < w.start)
                && capped_at.iter().any(|i| *i >= w.start + w.drawn)
        })
        .expect("fixture: some scroll leaves capped stores out of view both sides");
    let above: Vec<usize> = capped_at
        .iter()
        .copied()
        .filter(|i| *i < w.start)
        .take(2)
        .collect();
    let below: Vec<usize> = capped_at
        .iter()
        .copied()
        .filter(|i| *i >= w.start + w.drawn)
        .take(2)
        .collect();
    let mut state = base.clone();
    for i in above.iter().chain(&below) {
        fill(&mut state, stores[*i]);
    }
    let view = scrolled(&state, Section::Meadow, want);
    let mut term = Terminal::new(TestBackend::new(width, h)).expect("TestBackend");
    term.draw(|f| {
        chrome::sidebar(f, Rect::new(0, 0, width, h), &state, &view);
    })
    .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    for (y, glyph, n, full) in [
        (0, '▴', w.above, above.len()),
        (h - 1, '▾', w.below, below.len()),
    ] {
        let text = line(&buf, y, 0, width);
        assert_eq!(text.trim(), note_text(glyph, n, full));
        let lead = text.find(&format!("{full} full")).expect("the full count");
        let lead_col = text[..lead].chars().count() as u16;
        let full_len = format!("{full} full").chars().count() as u16;
        for x in 0..width {
            let cell = buf.cell((x, y)).expect("in the sidebar");
            if cell.symbol().trim().is_empty() {
                continue;
            }
            let in_full = x >= lead_col && x < lead_col + full_len;
            assert_eq!(
                cell.fg,
                if in_full { attention } else { dim },
                "{:?} at column {x} of {text:?} wears {}",
                cell.symbol(),
                if in_full { "attention" } else { "dim" }
            );
        }
    }
}

/// Where the sidebar is narrower than a whole note, the note's `more` gives
/// way, its triangle, count, and full count standing.
/// @spec TUI-RENDER-027
#[test]
fn a_note_too_wide_for_the_sidebar_gives_up_its_more() {
    let mut state = crowded();
    let stores = listed(&state);
    let k = stores.len();
    let last_capped = (0..k)
        .rev()
        .find(|i| capped(&state, stores[*i]))
        .expect("capped");
    fill(&mut state, stores[last_capped]);
    let h = 20u16;
    let w = expected(k, h as usize, 0);
    let whole_note = note_text('▾', w.below, 1);
    let width = whole_note.chars().count() as u16 - 1;
    let mut term = Terminal::new(TestBackend::new(width, h)).expect("TestBackend");
    term.draw(|f| {
        chrome::sidebar(f, Rect::new(0, 0, width, h), &state, &ViewState::default());
    })
    .expect("TestBackend is infallible");
    let buf = term.backend().buffer().clone();
    let foot = line(&buf, h - 1, 0, width);
    assert_eq!(foot.trim(), format!("▾ {} · 1 full", w.below));
}

// ---------------------------------------------------------------------------
// TUI-RENDER-026 — no sidebar recorded where none is drawn
// ---------------------------------------------------------------------------

/// A full-screen overlay draws over the sidebar, so its frame records none;
/// a plain frame, and one with the narrow-mode swap in, records it.
/// @spec TUI-RENDER-026
#[test]
fn a_frame_under_a_full_screen_overlay_records_no_sidebar() {
    let state = crowded();
    for size in SIZES {
        for overlay in [
            Overlay::Journal { anchor: 0 },
            Overlay::Help,
            Overlay::Palette {
                query: String::new(),
                selected: 0,
            },
        ] {
            let mut view = standing(Section::Meadow);
            view.overlay = Some(overlay.clone());
            assert!(
                shoot(&state, &view, size).record.sidebar().is_none(),
                "{size:?} under {overlay:?}"
            );
        }
        let mut swap = standing(Section::Meadow);
        swap.inspector_wanted = true;
        assert!(
            shoot(&state, &swap, size).record.sidebar().is_some(),
            "{size:?} swap"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-141, TUI-NAV-142, TUI-NAV-143 — the keys
// ---------------------------------------------------------------------------

/// `{` and `}` are the global table's sidebar keys; an open field takes them
/// as text, and a full-screen overlay leaves them unbound.
/// @spec TUI-NAV-141, TUI-NAV-142
#[test]
fn braces_resolve_to_the_sidebar_keys_only_in_the_global_table() {
    let bare = standing(Section::Clowder);
    assert_eq!(nav::keys::resolve(key('}'), &bare), Action::SidebarDown);
    assert_eq!(nav::keys::resolve(key('{'), &bare), Action::SidebarUp);
    let mut prompt = bare.clone();
    prompt.prompt = Some(CountPrompt::default());
    assert_eq!(nav::keys::resolve(key('}'), &prompt), Action::Type('}'));
    let mut journal = bare.clone();
    journal.overlay = Some(Overlay::Journal { anchor: 0 });
    assert_eq!(nav::keys::resolve(key('{'), &journal), Action::Ignored);
}

/// Three stores a press, or one fewer than the window holds where that is
/// fewer, and never less than one.
/// @spec TUI-NAV-143
#[test]
fn the_sidebar_step_is_three_stores_or_one_fewer_than_the_window_holds() {
    for (drawn, step) in [(0, 1), (1, 1), (2, 1), (3, 2), (4, 3), (9, 3), (17, 3)] {
        assert_eq!(nav::sidebar_step(drawn), step, "a window of {drawn} stores");
    }
}

/// Pressed again and again from the top, `}` moves the sidebar down by the
/// step from where each frame drew it, stopping with the last store at the
/// foot; `{` then walks it back up the same way, stopping at the top, where
/// the view holds the top rather than a store. At an end the press changes
/// nothing at all.
/// @spec TUI-NAV-141, TUI-NAV-142, TUI-NAV-143
#[test]
fn braces_walk_the_sidebar_by_the_step_from_where_each_frame_drew_it() {
    let state = crowded();
    let stores = listed(&state);
    let k = stores.len();
    for size in SIZES {
        let mut view = standing(Section::Clowder);
        let mut moves = 0;
        for (action, down) in [(Action::SidebarDown, true), (Action::SidebarUp, false)] {
            let mut ends = 0;
            for _ in 0..(2 * k) {
                let shot = shoot(&state, &view, size);
                let rec = shot.record.sidebar().expect("drawn").clone();
                let step = nav::sidebar_step(rec.drawn);
                let max = rec.max_start.expect("fixture: the sidebar overflows");
                let next = if down {
                    (rec.start + step).min(max)
                } else {
                    rec.start.saturating_sub(step)
                };
                let before = view.clone();
                assert!(nav::sidebar_key(&action, &mut view, Some(&shot.record)));
                if next == rec.start {
                    assert_eq!(whole(&view), whole(&before), "{size:?}: a press at an end");
                    ends += 1;
                } else {
                    let want = (next > 0).then(|| stores[next]);
                    assert_eq!(
                        view.sidebar_top, want,
                        "{size:?}: {action:?} from {}",
                        rec.start
                    );
                    assert_eq!(but_the_scroll(&view), but_the_scroll(&before));
                    moves += 1;
                }
            }
            assert!(ends > 0, "{size:?}: {action:?} reached its end");
        }
        assert_eq!(view.sidebar_top, None, "{size:?}: back at the top");
        assert!(moves > 4, "{size:?}: too few moves ({moves})");
    }
}

/// A scroll moves from where the last frame drew the window, not from a
/// remembered store the drawing passed over: remembering the last store, the
/// window is drawn as far down as it goes, and one `{` moves up from there.
/// @spec TUI-NAV-142
#[test]
fn a_scroll_moves_from_the_window_drawn_not_the_store_remembered() {
    let state = crowded();
    let stores = listed(&state);
    for size in SIZES {
        let mut view = standing(Section::Meadow);
        view.sidebar_top = stores.last().copied();
        let shot = shoot(&state, &view, size);
        let rec = shot.record.sidebar().expect("drawn").clone();
        assert_eq!(
            Some(rec.start),
            rec.max_start,
            "drawn as far down as it goes"
        );
        nav::sidebar_key(&Action::SidebarUp, &mut view, Some(&shot.record));
        let next = rec.start - nav::sidebar_step(rec.drawn);
        assert_eq!(view.sidebar_top, Some(stores[next]), "{size:?}");
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-140 — where the scroll is held
// ---------------------------------------------------------------------------

/// A sitting opens with the sidebar at the top.
/// @spec TUI-NAV-140
#[test]
fn a_sitting_opens_with_the_sidebar_at_the_top() {
    let state = crowded();
    assert_eq!(ViewState::default().sidebar_top, None);
    let mut opened = ViewState::default();
    nav::opening(&mut opened, &state);
    assert_eq!(opened.sidebar_top, None);
    let rec = shoot(&state, &opened, SMALL)
        .record
        .sidebar()
        .expect("drawn")
        .clone();
    assert_eq!(rec.start, 0);
}

/// The view remembers its first store by identity: a store revealed above
/// the window leaves the window's first store where it was, and the head note
/// counts one more. Scrolled all the way up, the view holds the top, so a
/// store revealed at the head of the list joins the window there.
/// @spec TUI-NAV-140
#[test]
fn the_scroll_is_held_by_identity_and_the_top_takes_a_store_revealed_first() {
    let mut state = crowded();
    let before = listed(&state);
    // A resource not yet met, standing before the stores in the core's order.
    let mut fresh = GameState::new(7);
    for r in ResourceId::ALL {
        let cap = fresh.cap(r);
        fresh.resources[r.ix()] = if cap.is_finite() { cap / 2.0 } else { 5.0 };
    }
    let early = before[1];
    fresh.resources[early.ix()] = 0.0;
    fresh.tick();
    state = fresh;
    let without = listed(&state);
    assert!(
        !without.contains(&early),
        "fixture: {early:?} is not yet met"
    );

    let want = 6;
    let top_store = without[want];
    let mut view = standing(Section::Meadow);
    view.sidebar_top = Some(top_store);
    let mut at_top = standing(Section::Meadow);
    at_top.sidebar_top = None;

    state.resources[early.ix()] = 1.0;
    state.tick();
    let with = listed(&state);
    assert!(with.contains(&early) && with.iter().position(|r| *r == early) < Some(want));

    let rec = shoot(&state, &view, SMALL)
        .record
        .sidebar()
        .expect("drawn")
        .clone();
    assert_eq!(
        rec.stores[rec.start], top_store,
        "the window's first store held"
    );
    let top = shoot(&state, &at_top, SMALL)
        .record
        .sidebar()
        .expect("drawn")
        .clone();
    assert_eq!(top.start, 0);
    assert_eq!(top.stores[0], with[0], "the top shows the list's head");
}

/// A change of section, an overlay opened and closed, and a resize leave the
/// scroll as it stands; a store the view names that is no longer listed is
/// taken as the top.
/// @spec TUI-NAV-140
#[test]
fn the_scroll_survives_sections_overlays_and_resizes_and_falls_back_to_the_top() {
    let state = crowded();
    let stores = listed(&state);
    let mut view = scrolled(&state, Section::Meadow, 5);
    let held = view.sidebar_top;
    nav::update(Action::SectionNext, &mut view, &state);
    assert_eq!(view.sidebar_top, held, "a change of section");
    nav::update(Action::OpenJournal, &mut view, &state);
    nav::dismiss(&mut view, false);
    assert_eq!(view.sidebar_top, held, "an overlay opened and closed");
    let small = shoot(&state, &view, SMALL)
        .record
        .sidebar()
        .expect("drawn")
        .clone();
    let wide = shoot(&state, &view, WIDE)
        .record
        .sidebar()
        .expect("drawn")
        .clone();
    assert_eq!(view.sidebar_top, held, "drawn at two sizes");
    assert_eq!(small.stores[small.start], stores[5]);
    assert_eq!(wide.stores[wide.start], stores[5]);

    let unlisted = ResourceId::ALL
        .into_iter()
        .find(|r| !stores.contains(r))
        .expect("fixture: some resource is not listed");
    let mut gone = standing(Section::Meadow);
    gone.sidebar_top = Some(unlisted);
    let rec = shoot(&state, &gone, SMALL)
        .record
        .sidebar()
        .expect("drawn")
        .clone();
    assert_eq!(rec.start, 0, "an unlisted store is the top");
}

// ---------------------------------------------------------------------------
// TUI-NAV-144, TUI-NAV-076 — a scroll is neither an act nor navigation
// ---------------------------------------------------------------------------

/// A scroll by key or wheel moves the sidebar and nothing else: an arming
/// stays armed, the banner stays showing, the cursor, section, and visits
/// stand, and no footer note is set. Where the scroll cannot move, or the
/// last frame drew no sidebar, nothing changes at all.
/// @spec TUI-NAV-144, TUI-NAV-076
#[test]
fn a_sidebar_scroll_touches_nothing_but_the_sidebar() {
    let state = crowded();
    let mut view = standing(Section::Meadow);
    let first = rows(&state, Section::Meadow)
        .into_iter()
        .find(|r| r.selectable())
        .expect("a row");
    view.focus[Section::Meadow.index()] = Some(first.key.clone());
    view.armed = Some(ArmedRow {
        key: first.key.clone(),
        command: Command::GatherCatnip,
        expires_at: Instant::now() + Duration::from_secs(60),
    });
    view.reports = vec![PendingReport {
        entry: 0,
        dismissed: false,
    }];
    // The frame is drawn without the arming and the banner, which the record
    // does not depend on; the scroll is applied to the view that has them.
    let mut drawn = view.clone();
    drawn.armed = None;
    drawn.reports.clear();
    for size in SIZES {
        let shot = shoot(&state, &drawn, size);
        let side = sidebar_rect(&shot.layout);
        let mut by_key = view.clone();
        assert!(nav::sidebar_key(
            &Action::SidebarDown,
            &mut by_key,
            Some(&shot.record)
        ));
        let mut by_wheel = view.clone();
        let out = nav::pointer_event(
            wheel(MouseEventKind::ScrollDown, side.x + 2, side.y + 3),
            &mut by_wheel,
            &state,
            &shot.record,
            shot.size,
        );
        assert_eq!(out, Outcome::Handled);
        for (how, after) in [("key", &by_key), ("wheel", &by_wheel)] {
            assert!(
                after.sidebar_top.is_some(),
                "{size:?} {how}: the sidebar moved"
            );
            assert_eq!(
                but_the_scroll(after),
                but_the_scroll(&view),
                "{size:?} {how}: a scroll changed more than the sidebar"
            );
        }
        // Up from the top: nowhere to go.
        let mut still = view.clone();
        nav::sidebar_key(&Action::SidebarUp, &mut still, Some(&shot.record));
        assert_eq!(whole(&still), whole(&view), "{size:?}: a scroll at the top");
    }
    // No frame drawn: nothing to move from.
    let mut blind = view.clone();
    nav::sidebar_key(&Action::SidebarDown, &mut blind, None);
    assert_eq!(whole(&blind), whole(&view), "no record");
    let mut reduced = view.clone();
    assert_eq!(
        nav::update(Action::SidebarDown, &mut reduced, &state),
        Outcome::Handled
    );
    assert_eq!(
        whole(&reduced),
        whole(&view),
        "update holds no record to move from"
    );
    // A sidebar that fits: nowhere to go.
    let fits = sparse();
    let mut v = standing(Section::Meadow);
    let shot = shoot(&fits, &v, SMALL);
    let before = whole(&v);
    nav::sidebar_key(&Action::SidebarDown, &mut v, Some(&shot.record));
    assert_eq!(whole(&v), before, "a sidebar that fits");
}

// ---------------------------------------------------------------------------
// TUI-NAV-145 — the wheel over the sidebar
// ---------------------------------------------------------------------------

/// Each wheel step over the sidebar moves it one store, from where the frame
/// drew it, down for a step down and up for a step up, stopping at the ends;
/// a press on it does nothing; and under a count prompt the wheel does
/// nothing there either.
/// @spec TUI-NAV-145
#[test]
fn each_wheel_step_over_the_sidebar_moves_it_one_store() {
    let state = crowded();
    let stores = listed(&state);
    let k = stores.len();
    for size in SIZES {
        let mut view = standing(Section::Clowder);
        let mut stepped = 0;
        for (kind, down) in [
            (MouseEventKind::ScrollDown, true),
            (MouseEventKind::ScrollUp, false),
        ] {
            for i in 0..(k + 2) {
                let shot = shoot(&state, &view, size);
                let side = sidebar_rect(&shot.layout);
                let rec = shot.record.sidebar().expect("drawn").clone();
                let max = rec.max_start.expect("overflows");
                let next = if down {
                    (rec.start + 1).min(max)
                } else {
                    rec.start.saturating_sub(1)
                };
                let (x, y) = (
                    side.x + (i as u16 * 3) % side.width,
                    side.y + (i as u16 * 7) % side.height,
                );
                let before = view.clone();
                nav::pointer_event(
                    wheel(kind, x, y),
                    &mut view,
                    &state,
                    &shot.record,
                    shot.size,
                );
                if next == rec.start {
                    assert_eq!(whole(&view), whole(&before), "{size:?}: a step at an end");
                } else {
                    assert_eq!(view.sidebar_top, (next > 0).then(|| stores[next]));
                    stepped += 1;
                }
            }
        }
        assert!(
            stepped >= 2 * 3,
            "{size:?}: the wheel moved the sidebar ({stepped})"
        );

        let shot = shoot(&state, &view, size);
        let side = sidebar_rect(&shot.layout);
        let press = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: side.x + 1,
            row: side.y + 1,
            modifiers: KeyModifiers::NONE,
        };
        let before = whole(&view);
        nav::pointer_event(press, &mut view, &state, &shot.record, shot.size);
        assert_eq!(whole(&view), before, "{size:?}: a press on the sidebar");

        let mut prompted = view.clone();
        prompted.prompt = Some(CountPrompt::default());
        let shot = shoot(&state, &prompted, size);
        let side = sidebar_rect(&shot.layout);
        let before = whole(&prompted);
        nav::pointer_event(
            wheel(MouseEventKind::ScrollDown, side.x + 1, side.y + 1),
            &mut prompted,
            &state,
            &shot.record,
            shot.size,
        );
        assert_eq!(
            whole(&prompted),
            before,
            "{size:?}: the wheel under a count prompt"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-NAV-146 — where the keys are named
// ---------------------------------------------------------------------------

fn hint_line(state: &GameState, view: &ViewState, size: (u16, u16)) -> String {
    let shot = shoot(state, view, size);
    let l = shot.layout.hints;
    line(&shot.buf, l.y, l.x, l.right()).trim().to_string()
}

/// While the sidebar has stores out of view, the hint bar names `{} stores`
/// directly before the help key; it names nothing of it where every store
/// fits, nor while a field holds the bar.
/// @spec TUI-NAV-146
#[test]
fn the_hint_bar_names_the_sidebar_keys_while_stores_are_out_of_view() {
    let crowd = crowded();
    let wide_enough = (240, 40);
    let named = hint_line(&crowd, &standing(Section::Meadow), wide_enough);
    assert!(
        named.ends_with("{} stores · ? help"),
        "the sidebar keys stand directly before the help key: {named:?}"
    );
    let fits = hint_line(&sparse(), &standing(Section::Meadow), wide_enough);
    assert!(!fits.contains("{}"), "nothing to scroll: {fits:?}");
    let mut prompted = standing(Section::Meadow);
    prompted.prompt = Some(CountPrompt::default());
    let field = hint_line(&crowd, &prompted, wide_enough);
    assert!(!field.contains("{}"), "a field holds the bar: {field:?}");
}

/// On the Ideas screen with its done key live, the done key stands just
/// before the sidebar keys, and they just before the help key.
/// @spec TUI-NAV-122, TUI-NAV-146
#[test]
fn the_done_key_stands_before_the_sidebar_keys() {
    let mut state = crowded();
    for b in clowder_core::StructureId::ALL {
        state.structures[b.ix()] = 1;
    }
    for idea in clowder_core::IdeaId::ALL {
        state.revealed.insert(clowder_core::ContentId::Idea(idea));
        state.ideas.insert(idea);
    }
    assert!(
        nav::rows::done_count(&state) > 0,
        "fixture: the done group is listed"
    );
    assert!(
        chrome::stores_out_of_view(&state, 20),
        "fixture: the sidebar overflows"
    );
    let named = hint_line(&state, &standing(Section::Ideas), (240, 40));
    assert!(
        named.ends_with("d show done · {} stores · ? help"),
        "the done key, the sidebar keys, then help: {named:?}"
    );
}

/// When the bar's width runs short, `{} stores` is the first entry given up.
/// @spec TUI-NAV-146
#[test]
fn the_sidebar_keys_are_the_first_hint_given_up() {
    let state = crowded();
    let view = standing(Section::Meadow);
    let full = {
        let mut term = Terminal::new(TestBackend::new(240, 1)).expect("TestBackend");
        term.draw(|f| chrome::hints(f, Rect::new(0, 0, 240, 1), &state, &view, true))
            .expect("drawn");
        line(term.backend().buffer(), 0, 0, 240).trim().to_string()
    };
    assert!(full.contains("{} stores · "), "fixture: {full:?}");
    let width = full.chars().count() as u16 - 1;
    let mut term = Terminal::new(TestBackend::new(width, 1)).expect("TestBackend");
    term.draw(|f| chrome::hints(f, Rect::new(0, 0, width, 1), &state, &view, true))
        .expect("drawn");
    let short = line(term.backend().buffer(), 0, 0, width)
        .trim()
        .to_string();
    assert_eq!(short, full.replace("{} stores · ", ""), "one column short");
}

/// The help overlay lists the sidebar keys among the global keys.
/// @spec TUI-NAV-146
#[test]
fn the_help_overlay_lists_the_sidebar_keys() {
    let state = crowded();
    let mut view = standing(Section::Meadow);
    view.overlay = Some(Overlay::Help);
    let shot = shoot(&state, &view, WIDE);
    let text = lines_in(&shot.buf, Rect::new(0, 0, shot.size.0, shot.size.1)).join("\n");
    let at = text.find("{ / }").expect("the help overlay names { / }");
    let rest = &text[at..];
    let eol = rest.find('\n').unwrap_or(rest.len());
    assert!(
        rest[..eol].contains("Scroll the stores sidebar up / down"),
        "{:?}",
        &rest[..eol]
    );
}
