//! A kitten's `about` card: its portrait painted in its own fur, the facts
//! the core holds about it, and its favourites.
//!
//! @spec TUI-NAV-164, TUI-NAV-165, TUI-NAV-166, TUI-RENDER-029,
//! TUI-THEME-027, TUI-THEME-028, TUI-THEME-029, TUI-THEME-030

use clowder_core::content;
use clowder_core::{
    fur_words, EyeColour, EyeLook, Fur, FurColour, FurPattern, GameState, JobId, Kitten,
    KittenTrait, Location, Profile, Season,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Terminal;

use clowder_tui::nav::{RowKey, Section, ViewState};
use clowder_tui::render::widgets::inspector;
use clowder_tui::theme::fur::{eye_colour, fur_colour};
use clowder_tui::theme::Role;

const H: u16 = 48;

/// The portrait, as the render design draws it.
const ART: [&str; 4] = [
    " /\u{2588}\\___/\u{2588}\\ ",
    "(\u{2588}\u{2588}o\u{2588}\u{2588}\u{2588}o\u{2588}\u{2588})",
    "=\\\u{2588}\u{2588}\u{2588}^\u{2588}\u{2588}\u{2588}/=",
    "  \\_____/  ",
];

/// The fifteen fur cells, by row and column within the portrait.
const FUR_CELLS: [(usize, usize); 15] = [
    (0, 2),
    (0, 8),
    (1, 1),
    (1, 2),
    (1, 4),
    (1, 5),
    (1, 6),
    (1, 8),
    (1, 9),
    (2, 2),
    (2, 3),
    (2, 4),
    (2, 6),
    (2, 7),
    (2, 8),
];

/// The theme design's masks: the slot a pattern gives a fur cell.
fn mask(p: FurPattern, r: usize, c: usize) -> usize {
    let at = |cells: &[(usize, usize)]| cells.contains(&(r, c));
    match p {
        FurPattern::Solid => 0,
        FurPattern::Tabby => usize::from(c % 2 == 0),
        FurPattern::Tuxedo => usize::from(r == 2 && [3, 4, 6, 7].contains(&c)),
        FurPattern::Bicolour => usize::from((r == 1 && [4, 5, 6].contains(&c)) || r == 2),
        FurPattern::Tortoiseshell => usize::from(at(&[
            (0, 8),
            (1, 1),
            (1, 5),
            (1, 6),
            (1, 9),
            (2, 2),
            (2, 7),
        ])),
        FurPattern::Calico => {
            if at(&[(0, 2), (1, 1), (1, 2), (1, 6)]) {
                1
            } else if at(&[(0, 8), (1, 8), (1, 9), (2, 8)]) {
                2
            } else {
                0
            }
        }
        FurPattern::Colourpoint => usize::from(r == 0 || (r == 2 && [3, 4, 6, 7].contains(&c))),
        FurPattern::Van => usize::from(r == 0 || (r == 1 && [1, 2, 8, 9].contains(&c))),
        FurPattern::Smoke => usize::from(r == 2),
    }
}

/// A fur of `p` whose slots take the first colours they can, distinct.
fn fur_of(p: FurPattern) -> Fur {
    let mut colours: Vec<FurColour> = Vec::new();
    for list in content::fur_pattern_def(p).slots {
        colours.push(list.iter().copied().find(|c| !colours.contains(c)).unwrap());
    }
    Fur {
        pattern: p,
        colours,
    }
}

fn profile(fur: Fur, eyes: EyeLook) -> Profile {
    Profile {
        fur,
        eyes,
        food: "warm toast crusts".into(),
        toy: "a crinkly paper bag".into(),
        nap: "the top of the fridge".into(),
        show: "rain on the window".into(),
        pastime: "sitting in boxes".into(),
        quirk: "sneezes when happy".into(),
    }
}

fn kitten(name: &str, p: Option<Profile>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: None,
        location: Location::Home,
        arrived_day: 12,
        job_seq: None,
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 2,
        trait_id: KittenTrait::Brassy,
        tinkering: None,
        profile: p,
    }
}

fn meadow(k: Kitten) -> GameState {
    let mut s = GameState::new(21);
    s.kittens = vec![k];
    s
}

/// The inspector drawn for the first kitten's row, `w` columns wide.
fn card(s: &GameState, w: u16) -> Buffer {
    let mut view = ViewState {
        section: Some(Section::Clowder),
        ..ViewState::default()
    };
    view.focus[Section::Clowder.index()] = Some(RowKey::Kitten(0, ""));
    let rect = Rect::new(0, 0, w, H);
    let mut term = Terminal::new(TestBackend::new(w, H)).expect("TestBackend is infallible");
    term.draw(|f| inspector::draw(f, rect, s, &view))
        .expect("TestBackend is infallible");
    term.backend().buffer().clone()
}

/// Each row as the player sees it. A no-break space, which the inspector uses
/// where a blank must survive wrapping, draws as a blank like any other.
fn text(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
                .map(|s| if s == "\u{a0}" { " " } else { s })
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn fg(buf: &Buffer, x: usize, y: usize) -> Color {
    buf.cell((x as u16, y as u16)).unwrap().fg
}

fn row_of(lines: &[String], start: &str) -> usize {
    lines
        .iter()
        .position(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("no line starts {start:?}: {lines:#?}"))
}

fn dim() -> Color {
    Role::Dim.color().unwrap()
}

// ---------------------------------------------------------------------------
// TUI-NAV-164 — the card's place and order
// ---------------------------------------------------------------------------

/// Beneath everything else, an `about` divider in dim, then the portrait, the
/// facts, and the favourites, in that order.
/// @spec TUI-NAV-164
#[test]
fn the_card_stands_beneath_the_row_in_its_order() {
    let s = meadow(kitten(
        "Pip",
        Some(profile(fur_of(FurPattern::Tabby), EyeLook::Green)),
    ));
    let buf = card(&s, 80);
    let lines = text(&buf);
    let about = row_of(&lines, "\u{2500}\u{2500} about ");
    for x in 0..80 {
        assert_eq!(fg(&buf, x, about), dim(), "the divider is dim whole");
    }
    assert!(
        lines[..about].iter().any(|l| l.starts_with("Pip")),
        "the name above it"
    );
    let portrait = row_of(&lines, ART[0].trim_end());
    let temperament = row_of(&lines, "\u{25aa} Temperament: ");
    let food = row_of(&lines, "\u{b7} Favourite food: ");
    assert!(about < portrait && portrait < temperament && temperament < food);
    assert!(
        lines[about + 1..].iter().all(|l| !l.starts_with("Pip")),
        "nothing of the row's own stands beneath the card"
    );
}

/// A kitten the core holds no profile for still states its facts.
/// @spec TUI-NAV-164
#[test]
fn a_kitten_without_a_profile_still_has_its_facts() {
    let s = meadow(kitten("Pip", None));
    let lines = text(&card(&s, 80));
    row_of(&lines, "\u{2500}\u{2500} about ");
    row_of(&lines, "\u{25aa} Temperament: ");
    assert!(lines.iter().all(|l| !l.contains("Favourite")), "{lines:#?}");
}

// ---------------------------------------------------------------------------
// TUI-NAV-165 — the facts
// ---------------------------------------------------------------------------

/// Temperament, rank, best at, and since, each worded as the facts query has
/// them; a blurb's resources in their coats.
/// @spec TUI-NAV-165
#[test]
fn the_facts_read_as_the_core_reports_them() {
    let mut k = kitten(
        "Pip",
        Some(profile(fur_of(FurPattern::Solid), EyeLook::Gold)),
    );
    let job = JobId::ALL[1];
    k.skills[job as usize] = 640.0;
    let s = meadow(k);
    let buf = card(&s, 120);
    let lines = text(&buf);
    let def = content::trait_def(KittenTrait::Brassy);
    let temperament = format!(
        "\u{25aa} Temperament: {}. {}",
        def.name,
        content::blurb_text(def.blurb)
    );
    let t = row_of(&lines, "\u{25aa} Temperament: ");
    assert_eq!(lines[t], temperament);
    let job_name = content::job_def(job).name;
    let rank = row_of(&lines, "\u{25aa} Rank ");
    assert_eq!(lines[rank], "\u{25aa} Rank 2");
    assert_eq!(
        lines[rank + 1],
        format!("\u{25aa} Best at: {job_name} (+2.5% from skill)")
    );
    assert_eq!(
        lines[rank + 2],
        format!(
            "\u{25aa} With the clowder since {:?} Day 12 Year 0",
            Season::ALL[0]
        )
    );
    assert_eq!(lines[t + 1], lines[rank], "the facts follow one another");

    // The plain words are the inherited foreground; a resource the blurb
    // names wears a coat.
    let line = &lines[t];
    let col =
        |needle: &str| line.chars().count() - line[line.find(needle).unwrap()..].chars().count();
    assert_eq!(fg(&buf, col("Temperament"), t), Color::Reset);
    let first_resource = content::blurb_parts(def.blurb)
        .into_iter()
        .find_map(|p| match p {
            content::BlurbPart::Resource(_, words) => Some(words),
            content::BlurbPart::Text(_) => None,
        })
        .expect("fixture: the Brassy blurb names a resource");
    assert_ne!(
        fg(&buf, col(first_resource), t),
        Color::Reset,
        "{first_resource} wears its coat"
    );
}

/// A skill below the first tier names the job without a bonus; no skill at
/// all is best at nothing yet; a carried kitten came along.
/// @spec TUI-NAV-165
#[test]
fn the_facts_have_their_plain_cases() {
    let mut k = kitten("Pip", None);
    let job = JobId::ALL[0];
    k.skills[job as usize] = 40.0;
    let lines = text(&card(&meadow(k.clone()), 120));
    assert!(lines.contains(&format!("\u{25aa} Best at: {}", content::job_def(job).name)));

    k.skills = [0.0; 8];
    k.arrived_day = 0;
    k.trait_id = KittenTrait::None;
    let lines = text(&card(&meadow(k), 120));
    assert!(
        lines.contains(&"\u{25aa} Best at: nothing yet".to_string()),
        "{lines:#?}"
    );
    assert!(lines.contains(&"\u{25aa} Came along from the old meadow".to_string()));
    let t = row_of(&lines, "\u{25aa} Temperament: ");
    assert!(lines[t].starts_with("\u{25aa} Temperament: Easygoing. As Elder: "));
}

// ---------------------------------------------------------------------------
// TUI-NAV-166 — the favourites
// ---------------------------------------------------------------------------

/// Six lines, each its label and the entry exactly as held, every cell dim.
/// @spec TUI-NAV-166
#[test]
fn the_favourites_follow_the_facts_in_dim() {
    let p = profile(fur_of(FurPattern::Calico), EyeLook::Blue);
    let s = meadow(kitten("Pip", Some(p.clone())));
    let buf = card(&s, 120);
    let lines = text(&buf);
    let first = row_of(&lines, "\u{b7} Favourite food: ");
    let want = [
        format!("\u{b7} Favourite food: {}", p.food),
        format!("\u{b7} Favourite toy: {}", p.toy),
        format!("\u{b7} Best nap spot: {}", p.nap),
        format!("\u{b7} Loves watching: {}", p.show),
        format!("\u{b7} Pastime: {}", p.pastime),
        format!("\u{b7} Quirk: {}", p.quirk),
    ];
    for (i, w) in want.iter().enumerate() {
        assert_eq!(&lines[first + i], w);
        for x in 0..w.chars().count() {
            assert_eq!(fg(&buf, x, first + i), dim(), "{w:?} column {x}");
        }
    }
    let since = row_of(&lines, "\u{25aa} With the clowder since ");
    assert!(since < first, "facts come before favourites");
}

// ---------------------------------------------------------------------------
// TUI-RENDER-029 — the portrait and its words
// ---------------------------------------------------------------------------

/// The four rows whole from the left edge, the words beside the face where
/// they fit, and the strip beneath.
/// @spec TUI-RENDER-029, TUI-THEME-030
#[test]
fn the_portrait_stands_whole_with_its_words_beside_it() {
    let fur = fur_of(FurPattern::Tabby);
    let s = meadow(kitten("Pip", Some(profile(fur.clone(), EyeLook::Green))));
    let lines = text(&card(&s, 80));
    let top = row_of(&lines, ART[0].trim_end());
    let words = fur_words(&fur);
    let eyes = content::eye_look_def(EyeLook::Green).words;
    assert_eq!(lines[top], ART[0].trim_end());
    assert_eq!(lines[top + 1], format!("{}   {words}", ART[1]));
    assert_eq!(lines[top + 2], format!("{}   {eyes}", ART[2]));
    assert_eq!(lines[top + 3], ART[3].trim_end());
    let strip: Vec<String> = fur
        .colours
        .iter()
        .map(|c| format!("\u{2588}\u{2588} {}", content::fur_colour_name(*c)))
        .collect();
    assert_eq!(lines[top + 4], strip.join("  "));
}

/// Where the words would not fit beside the face whole, they stand beneath
/// it, and a strip too long for one line breaks between items.
/// @spec TUI-RENDER-029, TUI-THEME-030
#[test]
fn a_narrow_card_puts_the_words_beneath_and_breaks_the_strip_between_items() {
    let fur = fur_of(FurPattern::Calico);
    let s = meadow(kitten(
        "Pip",
        Some(profile(fur.clone(), EyeLook::OddBlueCopper)),
    ));
    let lines = text(&card(&s, 22));
    let top = row_of(&lines, ART[0].trim_end());
    for (i, row) in ART.iter().enumerate() {
        assert_eq!(
            lines[top + i],
            row.trim_end(),
            "row {i} whole, nothing beside it"
        );
    }
    let words = fur_words(&fur);
    assert_eq!(
        lines[top + 4],
        words,
        "the fur's words beneath the portrait"
    );
    let strip_at = top
        + 5
        + lines[top + 5..]
            .iter()
            .position(|l| l.starts_with('\u{2588}'))
            .expect("a strip");
    let eyes = content::eye_look_def(EyeLook::OddBlueCopper).words;
    assert_eq!(
        lines[top + 5..strip_at].join(" "),
        eyes,
        "the eyes' words beneath the fur's, wrapped as any line"
    );
    let items: Vec<String> = fur
        .colours
        .iter()
        .map(|c| format!("\u{2588}\u{2588} {}", content::fur_colour_name(*c)))
        .collect();
    let strip: Vec<&String> = lines[strip_at..]
        .iter()
        .take_while(|l| l.starts_with('\u{2588}'))
        .collect();
    let rejoined: Vec<String> = strip
        .iter()
        .flat_map(|l| l.split("  ").map(str::to_string))
        .collect();
    assert_eq!(
        rejoined, items,
        "every swatch stays with its name: {strip:?}"
    );
    assert!(
        strip.len() >= 2,
        "fixture: the strip needs more than one line"
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-027..030 — the painting
// ---------------------------------------------------------------------------

/// The fur and eye colours are the theme design's, true to life.
/// @spec TUI-THEME-027
#[test]
fn the_fur_and_eye_colours_are_the_design_s() {
    let fur = [
        (FurColour::Black, 0x1C1A1A),
        (FurColour::White, 0xF4F1EA),
        (FurColour::Grey, 0x8A8D93),
        (FurColour::Silver, 0xC3C6CC),
        (FurColour::Brown, 0x7A5536),
        (FurColour::Chocolate, 0x5E3A26),
        (FurColour::Ginger, 0xD9822B),
        (FurColour::Cream, 0xEED9B0),
        (FurColour::Cinnamon, 0xA0603A),
        (FurColour::Fawn, 0xD4B79A),
        (FurColour::Seal, 0x4A3328),
        (FurColour::Lilac, 0xB7A5B4),
    ];
    let hex = |v: u32| Color::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    assert_eq!(fur.len(), FurColour::ALL.len());
    for (c, v) in fur {
        assert_eq!(fur_colour(c), hex(v), "{c:?}");
    }
    let eyes = [
        (EyeColour::Green, 0x6DAF4F),
        (EyeColour::Gold, 0xE3C04A),
        (EyeColour::Blue, 0x5B9BD5),
        (EyeColour::Amber, 0xD6962A),
        (EyeColour::Copper, 0xB8642E),
        (EyeColour::Hazel, 0x9C8A3E),
    ];
    assert_eq!(eyes.len(), EyeColour::ALL.len());
    for (c, v) in eyes {
        assert_eq!(eye_colour(c), hex(v), "{c:?}");
    }
}

/// Every pattern paints each fur cell in its mask's slot's colour, the eyes
/// in the look's two colours, and everything else in the portrait — outline,
/// nose, words — in the inherited foreground.
/// @spec TUI-THEME-027, TUI-THEME-028, TUI-THEME-029
#[test]
fn every_pattern_paints_its_mask() {
    for p in FurPattern::ALL {
        let fur = fur_of(p);
        let s = meadow(kitten(
            "Pip",
            Some(profile(fur.clone(), EyeLook::OddBlueCopper)),
        ));
        let buf = card(&s, 80);
        let lines = text(&buf);
        let top = row_of(&lines, ART[0].trim_end());
        for (r, row) in ART.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                let got = fg(&buf, c, top + r);
                let want = if FUR_CELLS.contains(&(r, c)) {
                    fur_colour(fur.colours[mask(p, r, c)])
                } else if (r, c) == (1, 3) {
                    eye_colour(content::eye_look_def(EyeLook::OddBlueCopper).left)
                } else if (r, c) == (1, 7) {
                    eye_colour(content::eye_look_def(EyeLook::OddBlueCopper).right)
                } else {
                    Color::Reset
                };
                assert_eq!(got, want, "{p:?} row {r} column {c} ({ch:?})");
            }
            // The words beside the face wear no fur.
            for x in 11..80 {
                assert_eq!(
                    fg(&buf, x, top + r),
                    Color::Reset,
                    "{p:?} row {r} column {x}"
                );
            }
        }
    }
}

/// A swatch's two blocks wear its fur colour; its name and the blanks do not.
/// @spec TUI-THEME-027, TUI-THEME-030
#[test]
fn a_swatch_wears_its_colour_and_its_name_does_not() {
    let fur = fur_of(FurPattern::Calico);
    let s = meadow(kitten("Pip", Some(profile(fur.clone(), EyeLook::Green))));
    let buf = card(&s, 80);
    let lines = text(&buf);
    let strip = row_of(&lines, "\u{2588}\u{2588} ");
    let mut x = 0;
    for c in &fur.colours {
        let name = content::fur_colour_name(*c);
        assert_eq!(fg(&buf, x, strip), fur_colour(*c));
        assert_eq!(fg(&buf, x + 1, strip), fur_colour(*c));
        for i in 0..=name.chars().count() {
            assert_eq!(fg(&buf, x + 2 + i, strip), Color::Reset, "{name}");
        }
        x += 2 + 1 + name.chars().count() + 2;
    }
}
