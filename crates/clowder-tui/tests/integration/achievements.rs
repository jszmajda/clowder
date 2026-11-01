//! The Achievements overlay, its chip, and the celebration in the ticker.
//!
//! @spec TUI-NAV-020, TUI-NAV-174, TUI-NAV-175, TUI-NAV-176
//!
//! The overlay reads the core's achievement query and holds no rule of its
//! own, so these tests build meadows whose records say what they need and
//! read the screen a real session draws. Names, hints, and words are read
//! from the core's table rather than written here, so the tests say what the
//! overlay must draw without restating the table.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clowder_core::content::{self, achievement_def};
use clowder_core::{
    begin_load, save_string, AchievementId, Earned, Event, GameState, IdeaId, Kitten, KittenTrait,
    Location,
};
use clowder_tui::nav::{self, Overlay, ViewState};
use clowder_tui::render::widgets::overlays;
use clowder_tui::runtime::clock::Reading;
use clowder_tui::runtime::{App, TickAccumulator, Wake};
use clowder_tui::theme::Role;
use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "clowder-achievements-{}-{name}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("save.json")
}

/// A session opened on `state` as a load opens one: the record it holds is
/// what was earned before this sitting.
fn app(state: GameState, name: &str) -> App {
    let mut view = ViewState::default();
    nav::opening(&mut view, &state);
    App {
        state,
        view,
        clock: TickAccumulator::default(),
        save_path: scratch(name),
        last_save: Instant::now(),
        prev_layout: None,
        pointer_record: None,
        quitting: false,
        save_failed: false,
    }
}

fn still() -> Reading {
    Reading {
        monotonic: Duration::ZERO,
        wall: Duration::ZERO,
    }
}

fn press(code: KeyCode) -> TermEvent {
    TermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn press_char(c: char) -> TermEvent {
    press(KeyCode::Char(c))
}

fn term(w: u16, h: u16) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(w, h)).expect("TestBackend is infallible")
}

/// Step once with these keys, no time passing.
fn keys(a: &mut App, t: &mut Terminal<TestBackend>, input: &[TermEvent]) {
    a.step(t, still(), input)
        .expect("a step against a TestBackend does not fail");
}

/// Every row of the screen as text.
fn screen(t: &Terminal<TestBackend>) -> Vec<String> {
    let buf = t.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

/// The screen's rows run together with single spaces, so wrapped words read
/// on as they were written.
fn flowing(t: &Terminal<TestBackend>) -> String {
    screen(t)
        .iter()
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn header(t: &Terminal<TestBackend>) -> String {
    screen(t)[0].clone()
}

fn earned(day: u64) -> Earned {
    Earned { generation: 0, day }
}

fn unearned_secrets(state: &GameState) -> Vec<AchievementId> {
    AchievementId::ALL
        .into_iter()
        .filter(|id| achievement_def(*id).secret && !state.achievements.contains_key(id))
        .collect()
}

/// A meadow with nothing earned that any look would add to — nothing at all
/// in it — so the record holds exactly what a test writes.
fn quiet() -> GameState {
    GameState::new(4242)
}

// ---------------------------------------------------------------------------
// TUI-NAV-174 — opening, closing, seen
// ---------------------------------------------------------------------------

/// `a` opens the scrapbook; `a` or Esc closes it. While the Journal holds the
/// keyboard, `a` leaves it be.
/// @spec TUI-NAV-174
#[test]
fn a_opens_the_scrapbook_and_a_or_esc_closes_it() {
    let mut a = app(quiet(), "open-close");
    let mut t = term(120, 40);
    keys(&mut a, &mut t, &[]);
    keys(&mut a, &mut t, &[press_char('a')]);
    assert!(
        matches!(a.view.overlay, Some(Overlay::Achievements { .. })),
        "{:?}",
        a.view.overlay
    );
    keys(&mut a, &mut t, &[press_char('a')]);
    assert_eq!(a.view.overlay, None, "a closes it");
    keys(&mut a, &mut t, &[press_char('a')]);
    keys(&mut a, &mut t, &[press(KeyCode::Esc)]);
    assert_eq!(a.view.overlay, None, "Esc closes it");

    keys(&mut a, &mut t, &[press_char('e')]);
    keys(&mut a, &mut t, &[press_char('a')]);
    assert!(
        matches!(a.view.overlay, Some(Overlay::Journal { .. })),
        "the Journal holds the keyboard: {:?}",
        a.view.overlay
    );
}

/// The overlay scrolls by the Journal's keys, never past its last line.
/// @spec TUI-NAV-174
#[test]
fn the_scrapbook_scrolls_by_the_journals_keys() {
    let mut a = app(quiet(), "scroll");
    let mut t = term(80, 24);
    keys(&mut a, &mut t, &[press_char('a')]);
    let anchor = |a: &App| match a.view.overlay {
        Some(Overlay::Achievements { anchor }) => anchor,
        ref other => panic!("not open: {other:?}"),
    };
    assert_eq!(anchor(&a), 0);
    keys(&mut a, &mut t, &[press(KeyCode::Down), press_char('j')]);
    assert_eq!(anchor(&a), 2, "a line a press");
    keys(&mut a, &mut t, &[press_char('k')]);
    assert_eq!(anchor(&a), 1);
    keys(&mut a, &mut t, &[press_char(']')]);
    let foot = anchor(&a);
    assert!(foot > 1, "the list outgrows 24 rows");
    keys(&mut a, &mut t, &[press(KeyCode::Down)]);
    assert_eq!(anchor(&a), foot, "never past the last line");
    assert!(
        screen(&t).iter().any(|r| r.contains("still to find")),
        "the foot shows:\n{}",
        screen(&t).join("\n")
    );
    keys(&mut a, &mut t, &[press_char('[')]);
    assert_eq!(anchor(&a), 0, "[ to the top");
}

/// Opening marks what was earned as seen, so the chip clears; one earned
/// while the overlay stands open is seen as it is drawn.
/// @spec TUI-NAV-174, TUI-NAV-020
#[test]
fn opening_marks_what_was_earned_as_seen() {
    let mut a = app(quiet(), "seen");
    let mut t = term(120, 40);
    a.state
        .achievements
        .insert(AchievementId::ABrightIdea, earned(3));
    keys(&mut a, &mut t, &[]);
    assert!(
        header(&t).trim_end().ends_with("✦1 earned"),
        "{}",
        header(&t)
    );
    keys(&mut a, &mut t, &[press_char('a')]);
    assert!(a
        .view
        .achievements_seen
        .contains(&AchievementId::ABrightIdea));
    assert!(!header(&t).contains("earned"), "{}", header(&t));

    a.state
        .achievements
        .insert(AchievementId::WellRead, earned(4));
    keys(&mut a, &mut t, &[]);
    assert!(a.view.achievements_seen.contains(&AchievementId::WellRead));
    keys(&mut a, &mut t, &[press_char('a')]);
    assert!(!header(&t).contains("earned"), "{}", header(&t));
}

/// The seen set begins as the record the save held before its catch-up
/// replayed: what the absence earned counts as new, what came before does not.
/// @spec TUI-NAV-174, TUI-NAV-020
#[test]
fn what_an_absence_earned_is_new_and_what_came_before_is_not() {
    let mut s = quiet();
    s.achievements.insert(AchievementId::MovingDay, earned(0));
    // Holds already; the catch-up's first day close credits it.
    s.ideas.insert(IdeaId::PantryOrganization);
    let json = save_string(&s, 0);
    let mut t = term(120, 40);
    let wake = Wake::begin(begin_load(&json, 60).unwrap(), scratch("absence"), &mut t)
        .expect("a wake against a TestBackend does not fail");
    let mut a = match wake {
        Wake::Awake(app) => app,
        Wake::Waking(_) => panic!("fixture: a minute's catch-up opens at once"),
    };
    assert!(a
        .state
        .achievements
        .contains_key(&AchievementId::ABrightIdea));
    assert_eq!(
        a.view.achievements_seen,
        BTreeSet::from([AchievementId::MovingDay])
    );
    keys(&mut a, &mut t, &[]);
    assert!(
        header(&t).trim_end().ends_with("✦1 earned"),
        "{}",
        header(&t)
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-020 — the chip's place
// ---------------------------------------------------------------------------

/// `✦N earned` stands last among the chips.
/// @spec TUI-NAV-020
#[test]
fn the_earned_chip_stands_last() {
    let mut s = quiet();
    s.kittens = vec![Kitten {
        name: "Biscuit".into(),
        job: None,
        location: Location::Home,
        arrived_day: 0,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }];
    let mut a = app(s, "chip-order");
    a.state
        .achievements
        .insert(AchievementId::AFaceAtTheDoor, earned(1));
    a.state
        .achievements
        .insert(AchievementId::ABrightIdea, earned(2));
    let mut t = term(160, 40);
    keys(&mut a, &mut t, &[]);
    let head = header(&t);
    assert!(head.trim_end().ends_with("◦1 idle  ✦2 earned"), "{head}");
}

// ---------------------------------------------------------------------------
// TUI-NAV-175 — what the overlay draws
// ---------------------------------------------------------------------------

fn scrapbook(state: GameState, seen: &[AchievementId], w: u16, h: u16) -> Terminal<TestBackend> {
    let mut a = app(state, "draw");
    a.view.achievements_seen = seen.iter().copied().collect();
    let mut t = term(w, h);
    keys(&mut a, &mut t, &[press_char('a')]);
    t
}

/// The heading counts every achievement; earned ones read `★ name`, ` new`
/// where unseen, and beneath them their words and when; locked ones that
/// aren't secret read `○ name — hint` in dim; unearned secrets are only
/// counted.
/// @spec TUI-NAV-175
#[test]
fn the_scrapbook_shows_what_was_earned_and_hints_at_the_rest() {
    let mut s = quiet();
    s.achievements.insert(
        AchievementId::AFaceAtTheDoor,
        Earned {
            generation: 2,
            day: 312,
        },
    );
    s.achievements
        .insert(AchievementId::FollowedHome, earned(40));
    let total = AchievementId::ALL.len();
    let t = scrapbook(s.clone(), &[AchievementId::AFaceAtTheDoor], 160, 90);
    let rows = screen(&t);

    let heading = format!("Achievements — 2 of {total} earned");
    let at = rows
        .iter()
        .position(|r| r.contains(&heading))
        .unwrap_or_else(|| panic!("no heading:\n{}", rows.join("\n")));
    assert!(
        rows[at + 1].trim().is_empty(),
        "a blank line beneath the heading"
    );

    // Earned, seen: no ` new`; its words and when, set in by two.
    let face = achievement_def(AchievementId::AFaceAtTheDoor);
    let line = rows
        .iter()
        .position(|r| r.trim_start().starts_with(&format!("★ {}", face.name)))
        .expect("the earned line");
    assert_eq!(rows[line].trim(), format!("★ {}", face.name));
    let indent = rows[line].find('★').unwrap();
    let beneath = &rows[line + 1];
    assert_eq!(
        beneath.trim_end(),
        format!(
            "{}  {} — Gen 2, Spring Day 312 Year 3",
            " ".repeat(indent),
            face.words
        )
    );

    // Earned, unseen: ` new`; its words read as plain words, marks and all.
    let followed = achievement_def(AchievementId::FollowedHome);
    let line = rows
        .iter()
        .position(|r| r.trim_start().starts_with(&format!("★ {}", followed.name)))
        .expect("the earned line");
    assert_eq!(rows[line].trim(), format!("★ {} new", followed.name));
    let plain: String = content::blurb_parts(followed.words)
        .into_iter()
        .map(|p| match p {
            content::BlurbPart::Text(t) => t,
            content::BlurbPart::Resource(_, t) => t,
        })
        .collect();
    assert!(
        rows[line + 1].contains(&format!("{plain} — Gen 0, Summer Day 40 Year 0")),
        "{}",
        rows[line + 1]
    );

    // Locked and not secret: hinted, dim.
    let dim = Role::Dim.color().expect("dim is a colour");
    let buf = t.backend().buffer();
    for id in AchievementId::ALL {
        let def = achievement_def(id);
        if s.achievements.contains_key(&id) {
            continue;
        }
        let shown = rows.iter().position(|r| r.contains(def.name));
        if def.secret {
            assert!(shown.is_none(), "{id:?}, a secret, is named");
            continue;
        }
        let y = shown.unwrap_or_else(|| panic!("{id:?} is not listed"));
        assert_eq!(
            rows[y].trim(),
            format!("○ {} — {}", def.name, def.hint),
            "{id:?}"
        );
        let x = rows[y]
            .find('○')
            .map(|b| rows[y][..b].chars().count())
            .unwrap() as u16;
        assert_eq!(buf[(x, y as u16)].fg, dim, "{id:?} is dim");
    }

    let secrets = unearned_secrets(&s).len();
    assert!(
        rows.iter()
            .any(|r| r.trim() == format!("…and {secrets} secrets still to find.")),
        "{}",
        rows.join("\n")
    );
}

/// Earned ones come first, then the locked, each in table order.
/// @spec TUI-NAV-175
#[test]
fn earned_come_first_then_locked_each_in_table_order() {
    let mut s = quiet();
    for id in [AchievementId::OldMeadow, AchievementId::AProperClowder] {
        s.achievements.insert(id, earned(5));
    }
    let t = scrapbook(s.clone(), &[], 160, 90);
    let rows = screen(&t);
    let place = |id: AchievementId| {
        let name = achievement_def(id).name;
        rows.iter()
            .position(|r| {
                let r = r.trim_start();
                r.starts_with(&format!("★ {name}")) || r.starts_with(&format!("○ {name}"))
            })
            .unwrap_or_else(|| panic!("{id:?} not drawn"))
    };
    let earned_order = [AchievementId::AProperClowder, AchievementId::OldMeadow];
    assert!(place(earned_order[0]) < place(earned_order[1]));
    let first_locked = AchievementId::ALL
        .into_iter()
        .find(|id| !s.achievements.contains_key(id) && !achievement_def(*id).secret)
        .unwrap();
    assert!(place(earned_order[1]) < place(first_locked));
    let locked: Vec<usize> = AchievementId::ALL
        .into_iter()
        .filter(|id| !s.achievements.contains_key(id) && !achievement_def(*id).secret)
        .map(place)
        .collect();
    assert!(locked.windows(2).all(|w| w[0] < w[1]), "table order");
}

/// One secret left reads in the singular; none left, no line at all.
/// @spec TUI-NAV-175
#[test]
fn the_secrets_line_counts_what_is_left() {
    let mut s = quiet();
    let secrets: Vec<AchievementId> = unearned_secrets(&s);
    for id in &secrets[1..] {
        s.achievements.insert(*id, earned(1));
    }
    let t = scrapbook(s.clone(), &[], 160, 90);
    assert!(
        screen(&t)
            .iter()
            .any(|r| r.trim() == "…and 1 secret still to find."),
        "{}",
        screen(&t).join("\n")
    );
    s.achievements.insert(secrets[0], earned(1));
    let t = scrapbook(s, &[], 160, 90);
    assert!(!flowing(&t).contains("still to find"));
}

/// At the narrowest width, every hint and every earned line's words and date
/// read whole: a long line wraps, set in, rather than being cut.
/// @spec TUI-NAV-175
#[test]
fn nothing_is_cut_at_the_narrowest_width() {
    let mut s = quiet();
    for id in [AchievementId::AFaceAtTheDoor, AchievementId::Remembered] {
        s.achievements.insert(id, earned(7));
    }
    let mut a = app(s.clone(), "narrow");
    let mut t = term(80, 24);
    keys(&mut a, &mut t, &[press_char('a')]);
    let mut seen = String::new();
    let mut last = None;
    // Page through the whole overlay a line at a time.
    for _ in 0..400 {
        seen.push(' ');
        seen.push_str(&flowing(&t));
        keys(&mut a, &mut t, &[press(KeyCode::Down)]);
        let now = a.view.overlay.clone();
        if now == last {
            break;
        }
        last = now;
    }
    for id in AchievementId::ALL {
        let def = achievement_def(id);
        if s.achievements.contains_key(&id) {
            let words = format!("{} — Gen 0, Spring Day 7 Year 0", def.words);
            assert!(seen.contains(&words), "{id:?}: {words}");
        } else if !def.secret {
            let hint = format!("○ {} — {}", def.name, def.hint);
            assert!(seen.contains(&hint), "{id:?}: {hint}");
        }
    }
    // No row runs to the overlay's edge with a word cut: wrapped rows are set
    // in by two beyond their line's start.
    let rows = screen(&t);
    assert!(
        rows.iter().all(|r| !r.ends_with('…')),
        "{}",
        rows.join("\n")
    );
}

// ---------------------------------------------------------------------------
// TUI-NAV-176 — the celebration
// ---------------------------------------------------------------------------

/// The ticker words an earned achievement by its name, as a milestone.
/// @spec TUI-NAV-176
#[test]
fn the_ticker_celebrates_by_name() {
    let mut s = quiet();
    let id = AchievementId::TheZoomies;
    s.journal.record(0, &Event::AchievementEarned { id });
    let mut t = term(80, 1);
    t.draw(|f| overlays::ticker(f, f.area(), &s)).unwrap();
    let line = screen(&t)[0].clone();
    assert!(
        line.contains(&format!("★ achievement: {}", achievement_def(id).name)),
        "{line}"
    );
}

/// Colour does the reading: an earned achievement's `★` and name in
/// `milestone` and its ` new` in `attention`, apart from the name; its words in
/// the inherited foreground with each named resource in its coat and when it
/// was earned in `dim`; a locked one's name inherited and its mark and hint in
/// `dim`; and blank lines part the earned, the locked, and the secrets.
/// @spec TUI-NAV-175, TUI-THEME-015
#[test]
fn colour_and_space_make_the_scrapbook_easy_to_read() {
    use ratatui::style::Color;
    let mut s = quiet();
    s.achievements
        .insert(AchievementId::FollowedHome, earned(40));
    let t = scrapbook(s.clone(), &[], 160, 90);
    let rows = screen(&t);
    let buf = t.backend().buffer();
    let fg = |x: usize, y: usize| buf[(x as u16, y as u16)].fg;
    let col = |row: &str, needle: &str| {
        let b = row
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} not in {row:?}"));
        row[..b].chars().count()
    };
    let milestone = Role::Milestone.color().unwrap();
    let dim = Role::Dim.color().unwrap();

    let def = achievement_def(AchievementId::FollowedHome);
    let y = rows
        .iter()
        .position(|r| r.trim_start().starts_with(&format!("★ {}", def.name)))
        .unwrap();
    for needle in ["★", def.name] {
        assert_eq!(fg(col(&rows[y], needle), y), milestone, "{needle:?}");
    }
    let tag = rows[y].rfind(" new").expect("the tag ends the name line");
    let new_at = rows[y][..tag].chars().count() + 1;
    for x in new_at..new_at + 3 {
        assert_eq!(
            fg(x, y),
            Role::Attention.color().unwrap(),
            "` new` wears the chip's orange, apart from the name"
        );
    }
    let words = &rows[y + 1];
    let first = words.trim_start().chars().next().unwrap().to_string();
    assert_eq!(
        fg(col(words, &first), y + 1),
        Color::Reset,
        "the words read plainly"
    );
    let coated = content::blurb_parts(def.words)
        .into_iter()
        .find_map(|p| match p {
            content::BlurbPart::Resource(_, t) => Some(t),
            content::BlurbPart::Text(_) => None,
        })
        .expect("fixture: these words name a resource");
    let at = fg(col(words, coated), y + 1);
    assert!(
        at != Color::Reset && at != dim,
        "{coated:?} wears its coat: {at:?}"
    );
    assert_eq!(fg(col(words, "Gen 0"), y + 1), dim, "when, in dim");
    assert_eq!(fg(col(words, "—"), y + 1), dim, "the dash with it");

    assert!(
        rows[y + 2].trim().is_empty(),
        "a blank line after the earned"
    );
    let locked = AchievementId::ALL
        .into_iter()
        .find(|id| !s.achievements.contains_key(id) && !achievement_def(*id).secret)
        .map(achievement_def)
        .unwrap();
    let ly = y + 3;
    assert!(rows[ly].contains(locked.name), "{}", rows[ly]);
    assert_eq!(fg(col(&rows[ly], "○"), ly), dim);
    assert_eq!(
        fg(col(&rows[ly], locked.name), ly),
        Color::Reset,
        "the name reads plainly"
    );
    assert_eq!(fg(col(&rows[ly], locked.hint), ly), dim, "the hint recedes");

    let sy = rows
        .iter()
        .position(|r| r.contains("still to find"))
        .unwrap();
    assert!(
        rows[sy - 1].trim().is_empty(),
        "a blank line before the secrets"
    );
}
