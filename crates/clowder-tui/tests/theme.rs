//! Colour, and what it is allowed to mean — `theme`.
//!
//! @spec TUI-THEME-001, TUI-THEME-002, TUI-THEME-003, TUI-THEME-004,
//! TUI-THEME-005, TUI-THEME-006, TUI-THEME-007, TUI-THEME-008, TUI-THEME-009,
//! TUI-THEME-010, TUI-THEME-011, TUI-THEME-012, TUI-THEME-013, TUI-THEME-014,
//! TUI-THEME-015, TUI-THEME-016, TUI-THEME-017, TUI-THEME-018, TUI-THEME-019,
//! TUI-THEME-020, TUI-THEME-021
//!
//! **How this test reaches the code.** `clowder-tui` is a binary crate with no
//! library target, so an integration test cannot `use clowder_tui::…`.
//! `theme` depends on nothing else in the crate — only `clowder_core::Season`
//! and `ratatui::style::Color` — so this file compiles the module in by path,
//! as the sibling runtime tests do. If the implementation later reaches for a
//! crate sibling (`crate::…`), this include stops compiling and the crate needs
//! a `[lib]` target instead; that belongs to `Cargo.toml`, which is another
//! chunk's, and is reported as an open question rather than fixed here.
//!
//! **Why so much colour arithmetic lives here.** Three of this leaf's specs are
//! claims about *values*, not about control flow: TUI-THEME-002's contrast
//! floor against both extremes of ground, TUI-THEME-011's red-green
//! compression, and TUI-THEME-014's "recedes rather than competes". None of
//! them can be checked by calling a function and looking at the tag on the
//! answer — they need relative luminance, CIE Lab, and a dichromacy
//! simulation, so those are implemented below and applied to the palette the
//! module returns. Restating the hex table would test nothing; the assertions
//! are about the *separation* the design says those values must have.
//!
//! **The meaning table.** `MEANINGS` transcribes the design's "Colour Is Always
//! the Second Channel" table, with affordability and flavour/mechanical
//! expanded to one row per state, and the armed row added from the palette
//! table. Its glyphs and words belong to the navigation leaf (TUI-NAV-009's
//! markers, TUI-NAV-020's chips, TUI-NAV-015's armed expansion) — this leaf
//! only asserts properties *of* the pairing, so a glyph changing there
//! legitimately fails a test here, which is the cascade signal the design asks
//! for.
//!
//! **What is deliberately not tested here.**
//!
//! - The *confinement* half of TUI-THEME-009 — that nothing but the header rule
//!   and the pane divider ever calls `seasonal_accent` — is a property of call
//!   sites in `render`, which are `todo!()` in a leaf this chunk does not own.
//!   What is checked instead is that no accent is a state colour, and that
//!   nothing about the values themselves keeps the two apart.
//! - TUI-THEME-012's "once" — that the notice is said at startup and never
//!   again — has no seam: `App::step` is `todo!()`, so no run reaches a second
//!   frame to say it twice in.
//! - TUI-THEME-006's cursor glyph and TUI-THEME-007's reversal are rendered by
//!   `render`, and `theme` exposes no style seam for either; what is checked
//!   here is the half that is this leaf's — that no palette colour is spent on
//!   focus, and that a reversal carries no palette value.

use clowder_tui::theme;

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::sync::Mutex;

use clowder_core::{ResourceId, Season};
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use theme::{blend, coat, coat_colours, seasonal_accent, truecolor_advertised, Coat, Dressing, Hue, Role};

// ---------------------------------------------------------------------------
// The roles, and a compile-time guard on there being six of them
// ---------------------------------------------------------------------------

/// Every role the design names, written out rather than derived: a seventh
/// cannot appear without this list and `role_name` below both being revisited.
/// @spec TUI-THEME-004
const ROLES: [Role; 6] = [
    Role::Inherited,
    Role::Dim,
    Role::Ready,
    Role::Partial,
    Role::Attention,
    Role::Milestone,
];

/// The five roles that take a colour of their own. `Inherited` is the sixth,
/// and it is the *absence* of one.
const PALETTE_ROLES: [Role; 5] = [
    Role::Dim,
    Role::Ready,
    Role::Partial,
    Role::Attention,
    Role::Milestone,
];

/// Exhaustive on purpose, with no wildcard arm: a seventh role stops this file
/// compiling, which is what TUI-THEME-004's "no seventh" is worth as a test.
fn role_name(role: Role) -> &'static str {
    match role {
        Role::Inherited => "inherited",
        Role::Dim => "dim",
        Role::Ready => "ready",
        Role::Partial => "partial",
        Role::Attention => "attention",
        Role::Milestone => "milestone",
    }
}

// ---------------------------------------------------------------------------
// Meanings, and the two channels that carry each of them
// ---------------------------------------------------------------------------

/// What reinforces a meaning, once its first channel has already carried it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tint {
    /// A palette role. `Tint::Role(Role::Inherited)` is "no colour at all" —
    /// the terminal's own foreground.
    Role(Role),
    /// The chrome accent, which belongs to a season and never to a state.
    SeasonalAccent,
}

/// One meaning the interface conveys, and the two channels that carry it.
struct Meaning {
    what: &'static str,
    /// The channel that carries it *first* — a glyph, a word, or a fill. If
    /// this is enough on its own, removing all colour loses nothing.
    first_channel: &'static str,
    tint: Tint,
}

/// The design's second-channel table, one row per state.
const MEANINGS: [Meaning; 12] = [
    Meaning { what: "affordable now", first_channel: "●", tint: Tint::Role(Role::Ready) },
    Meaning {
        what: "partially affordable",
        first_channel: "◐",
        tint: Tint::Role(Role::Partial),
    },
    Meaning {
        what: "revealed but not affordable",
        first_channel: "○",
        tint: Tint::Role(Role::Dim),
    },
    Meaning { what: "owned or done", first_channel: "✓", tint: Tint::Role(Role::Dim) },
    Meaning {
        what: "an attention chip",
        first_channel: "the chip's glyph, its count, and its word",
        tint: Tint::Role(Role::Attention),
    },
    Meaning {
        what: "a capped store",
        first_channel: "the word `full`, and a filled bar",
        tint: Tint::Role(Role::Attention),
    },
    Meaning {
        what: "an armed row",
        first_channel: "the row expands in place to a consequence summary",
        tint: Tint::Role(Role::Attention),
    },
    Meaning { what: "a milestone", first_channel: "★", tint: Tint::Role(Role::Milestone) },
    Meaning { what: "a flavour event", first_channel: "·", tint: Tint::Role(Role::Dim) },
    Meaning {
        what: "a mechanical event",
        first_channel: "▪",
        tint: Tint::Role(Role::Inherited),
    },
    Meaning {
        what: "the clowder's mood",
        first_channel: "the mood word itself",
        tint: Tint::Role(Role::Inherited),
    },
    Meaning {
        what: "the season",
        first_channel: "the season's name in the header",
        tint: Tint::SeasonalAccent,
    },
];

fn meaning(what: &str) -> &'static Meaning {
    MEANINGS
        .iter()
        .find(|m| m.what == what)
        .unwrap_or_else(|| panic!("no meaning named {what:?} in the second-channel table"))
}

// ---------------------------------------------------------------------------
// Reading the palette
// ---------------------------------------------------------------------------

/// The 24-bit value behind a colour, or a failure explaining why a colour that
/// is not 24-bit cannot satisfy this leaf.
fn rgb_of(colour: Color, what: &str) -> (u8, u8, u8) {
    match colour {
        Color::Rgb(r, g, b) => (r, g, b),
        other => panic!(
            "{what} is {other:?}. A named or indexed colour is whatever the terminal's own \
             palette happens to hold, so its luminance is unknown — and every claim this leaf \
             makes about contrast against a near-black and a near-white ground is a claim about \
             specific values."
        ),
    }
}

/// The 24-bit value a role paints with. Fails loudly for `Inherited`, which is
/// the one role that must have none.
fn rgb(role: Role) -> (u8, u8, u8) {
    let name = role_name(role);
    match role.color() {
        Some(colour) => rgb_of(colour, &format!("`{name}`")),
        None => panic!(
            "`{name}` returned no colour. Only `inherited` may be absent; every other role is a \
             value the palette owns."
        ),
    }
}

fn accent(season: Season) -> (u8, u8, u8) {
    rgb_of(seasonal_accent(season), &format!("the {season:?} accent"))
}

// ---------------------------------------------------------------------------
// Colour arithmetic
// ---------------------------------------------------------------------------

/// Both extremes of ground the design says every value must hold against.
const NEAR_BLACK: (u8, u8, u8) = (0x00, 0x00, 0x00);
const NEAR_WHITE: (u8, u8, u8) = (0xFF, 0xFF, 0xFF);

/// The contrast floor for a value that has to be *read* — WCAG's ratio for
/// large text and graphical objects. Ordinary body text is the terminal's own
/// foreground and is not clowder's to guarantee; what the palette must clear is
/// the floor for the glyphs and short words it tints.
const CONTRAST_FLOOR: f64 = 3.0;

/// The mid-luminance band: neither a pale tint nor a deep shade. The bounds are
/// generous on purpose — the design's claim is that the whole palette lives
/// between the extremes, not that it hits a particular number.
const MID_BAND: (f64, f64) = (0.05, 0.45);

fn to_linear(component: u8) -> f64 {
    let c = f64::from(component) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(value: f64) -> u8 {
    let v = value.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 { 12.92 * v } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

/// WCAG relative luminance.
fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

/// WCAG contrast ratio between two colours.
fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// CIE L*a*b*, D65.
fn lab((r, g, b): (u8, u8, u8)) -> (f64, f64, f64) {
    let (lr, lg, lb) = (to_linear(r), to_linear(g), to_linear(b));
    let x = (0.4124 * lr + 0.3576 * lg + 0.1805 * lb) / 0.950_47;
    let y = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
    let z = (0.0193 * lr + 0.1192 * lg + 0.9505 * lb) / 1.088_83;
    let f = |t: f64| if t > 0.008_856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

/// CIE76 colour difference. Roughly: under 2 is invisible, around 10 is "the
/// same colour, slightly off", over 25 is unmistakably two colours.
fn delta_e(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let (l1, a1, b1) = lab(a);
    let (l2, a2, b2) = lab(b);
    ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
}

/// Distance from grey. A near-grey recedes; a saturated colour competes.
fn chroma(c: (u8, u8, u8)) -> f64 {
    let (_, a, b) = lab(c);
    a.hypot(b)
}

/// Viénot–Brettel–Mollon dichromacy matrices, applied in linear RGB.
const PROTANOPIA: [[f64; 3]; 3] = [
    [0.11238, 0.88762, 0.0],
    [0.11238, 0.88762, 0.0],
    [0.00401, -0.00401, 1.0],
];
const DEUTERANOPIA: [[f64; 3]; 3] = [
    [0.29275, 0.70725, 0.0],
    [0.29275, 0.70725, 0.0],
    [-0.02234, 0.02234, 1.0],
];

/// What a dichromat sees where a trichromat sees this colour.
fn simulate((r, g, b): (u8, u8, u8), m: [[f64; 3]; 3]) -> (u8, u8, u8) {
    let v = [to_linear(r), to_linear(g), to_linear(b)];
    let out = |row: [f64; 3]| row[0] * v[0] + row[1] * v[1] + row[2] * v[2];
    (from_linear(out(m[0])), from_linear(out(m[1])), from_linear(out(m[2])))
}

/// The smallest difference between any two of a set of colours.
fn closest_pair(colours: &[(&str, (u8, u8, u8))]) -> (String, f64) {
    let mut worst = ("".to_string(), f64::INFINITY);
    for (i, (na, ca)) in colours.iter().enumerate() {
        for (nb, cb) in colours.iter().skip(i + 1) {
            let d = delta_e(*ca, *cb);
            if d < worst.1 {
                worst = (format!("{na}/{nb}"), d);
            }
        }
    }
    worst
}

/// The one correct way to turn a role into a style: a foreground and nothing
/// else. `None` means "leave the cell alone", which is how clowder paints no
/// ground.
fn style_for(role: Role) -> Style {
    match role.color() {
        Some(colour) => Style::default().fg(colour),
        None => Style::default(),
    }
}

/// Render a full-area paragraph at an exact size and hand back the buffer.
fn frame_at(width: u16, height: u16, style: Style) -> ratatui::buffer::Buffer {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("a test backend always builds");
    terminal
        .draw(|f| {
            let area = f.area();
            f.render_widget(Paragraph::new("a line of ordinary text").style(style), area);
        })
        .expect("drawing into an in-memory backend cannot fail");
    terminal.backend().buffer().clone()
}

/// The two sizes the render leaf's breakpoints sit either side of: a frame is a
/// pure function of size, so a claim about the ground has to hold at both.
const SIZES: [(u16, u16); 2] = [(80, 24), (120, 40)];

// ---------------------------------------------------------------------------
// TUI-THEME-001 — the ground belongs to the terminal
// ---------------------------------------------------------------------------

/// Ordinary text has no colour of clowder's, which is the whole reason a light
/// terminal and a dark one both work with nothing detected: the contrast in use
/// is the terminal's own, in either polarity.
///
/// @spec TUI-THEME-001
#[test]
fn ordinary_text_takes_no_colour_so_the_terminal_supplies_its_own() {
    assert_eq!(
        Role::Inherited.color(),
        None,
        "`inherited` returned a colour of its own. It is the role for names, counts, costs and \
         prose — the great majority of the screen — and giving it a value would mean clowder \
         choosing a foreground for a background it deliberately does not detect."
    );

    for role in PALETTE_ROLES {
        assert!(
            role.color().is_some(),
            "`{}` has no colour. Only `inherited` may be absent; the other five are what colour \
             appearing on screen *means*.",
            role_name(role)
        );
    }
}

/// The sentinel has to survive to the buffer: a frame of ordinary text leaves
/// every cell at the terminal's own pair, at any size. If anything along the
/// way substituted a concrete foreground or filled a background, the pane would
/// stop looking native beside the player's other panes — and would be a guess
/// about a background clowder never detected.
///
/// @spec TUI-THEME-001
#[test]
fn a_frame_of_ordinary_text_paints_no_ground_at_any_size() {
    for (width, height) in SIZES {
        let buffer = frame_at(width, height, style_for(Role::Inherited));
        for y in 0..height {
            for x in 0..width {
                let cell = buffer.cell((x, y)).expect("every cell of the area exists");
                assert_eq!(
                    cell.bg,
                    Color::Reset,
                    "cell ({x},{y}) of a {width}x{height} frame had background {:?}. Clowder \
                     paints no background: the ground is the terminal's, which is what makes a \
                     light terminal and a dark one both legible.",
                    cell.bg
                );
                assert_eq!(
                    cell.fg,
                    Color::Reset,
                    "cell ({x},{y}) of a {width}x{height} frame had foreground {:?}. Ordinary \
                     text inherits the terminal's foreground rather than taking a palette value.",
                    cell.fg
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-002 — mid-luminance, against both extremes of ground
// ---------------------------------------------------------------------------

/// The cost of inheriting the ground: every value has to hold against a
/// near-black *and* a near-white one, because clowder never learns which it is
/// on. That rules out pale tints and deep shades in the same breath, which is
/// why the palette is mid-luminance throughout.
///
/// @spec TUI-THEME-002
#[test]
fn every_state_colour_clears_the_contrast_floor_against_both_extremes_of_ground() {
    for role in PALETTE_ROLES {
        let value = rgb(role);
        let name = role_name(role);

        let on_black = contrast(value, NEAR_BLACK);
        assert!(
            on_black >= CONTRAST_FLOOR,
            "`{name}` (#{:02X}{:02X}{:02X}) is {on_black:.2}:1 against a near-black ground, \
             below the {CONTRAST_FLOOR}:1 floor. A player in a dark terminal cannot read it.",
            value.0,
            value.1,
            value.2
        );

        let on_white = contrast(value, NEAR_WHITE);
        assert!(
            on_white >= CONTRAST_FLOOR,
            "`{name}` (#{:02X}{:02X}{:02X}) is {on_white:.2}:1 against a near-white ground, \
             below the {CONTRAST_FLOOR}:1 floor. A light-terminal player was handed a value \
             tuned for the other polarity — the exact failure inheriting the ground exists to \
             avoid.",
            value.0,
            value.1,
            value.2
        );
    }
}

/// The band itself, stated for every value the theme can produce. The seasonal
/// accents are held to the band but not to the reading floor: they tint inert
/// chrome rather than anything the player has to read. (Whether TUI-THEME-002's
/// "every palette value" is meant to reach them is reported as an open
/// question — at the stated values two of the four fall below 3:1 against a
/// near-white ground.)
///
/// @spec TUI-THEME-002
#[test]
fn nothing_in_the_theme_is_a_pale_tint_or_a_deep_shade() {
    let mut values: Vec<(String, (u8, u8, u8))> =
        PALETTE_ROLES.iter().map(|r| (role_name(*r).to_string(), rgb(*r))).collect();
    for season in Season::ALL {
        values.push((format!("the {season:?} accent"), accent(season)));
    }

    for (name, value) in values {
        let l = luminance(value);
        assert!(
            l > MID_BAND.0,
            "{name} (#{:02X}{:02X}{:02X}) has relative luminance {l:.3} — a deep shade, which \
             disappears into a near-black ground.",
            value.0,
            value.1,
            value.2
        );
        assert!(
            l < MID_BAND.1,
            "{name} (#{:02X}{:02X}{:02X}) has relative luminance {l:.3} — a pale tint, which \
             disappears into a near-white ground.",
            value.0,
            value.1,
            value.2
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-003 — colour encodes state, never identity
// ---------------------------------------------------------------------------

/// The load-bearing decision. Identity is carried by the name sitting right
/// there; colour is spent on the handful of states a player learns once and
/// reads everywhere. A vocabulary that grows with the content is a vocabulary
/// nobody learns.
///
/// @spec TUI-THEME-003
#[test]
fn the_colour_vocabulary_is_five_states_and_does_not_grow_with_the_content() {
    // A colour is obtained by naming a *state* — `Role::color` takes nothing
    // else, which is the type-level half of this spec. What is checkable at
    // runtime is that the vocabulary is small, fixed, and fully distinguishing.
    let mut distinct = HashSet::new();
    for role in PALETTE_ROLES {
        let value = rgb(role);
        assert!(
            distinct.insert(value),
            "`{}` shares #{:02X}{:02X}{:02X} with another role. Two states that look identical \
             are one state as far as the player is concerned.",
            role_name(role),
            value.0,
            value.1,
            value.2
        );
    }
    assert_eq!(
        distinct.len(),
        5,
        "the palette should hold exactly five state colours; the sixth role is the absence of one"
    );

    for role in ROLES {
        assert_eq!(
            role.color(),
            role.color(),
            "`{}` answered differently on two calls — a colour must be a function of the state \
             and of nothing else",
            role_name(role)
        );
    }
}

/// The guard against the palette quietly acquiring a hue per item. A resource
/// reaches the theme for its coat and for nothing else; no other identity —
/// a structure, an idea, a job, a kitten — belongs in a colour decision at
/// all, and a coat is a function of which resource and of nothing else.
///
/// @spec TUI-THEME-003
#[test]
fn only_a_resource_reaches_the_theme_and_only_for_its_coat() {
    /// Every identity type the core exports but the one the coats are keyed
    /// by. None of them belongs in a colour decision.
    const IDENTITY_TYPES: [&str; 14] = [
        "StructureId",
        "SiteId",
        "DreamLoomId",
        "JobId",
        "CraftId",
        "IdeaId",
        "RefinementId",
        "RuleId",
        "ColonyId",
        "RiteId",
        "ArchId",
        "TraditionId",
        "AdventureId",
        "ContentId",
    ];

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/theme/mod.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));

    for identity in IDENTITY_TYPES {
        assert!(
            !source.contains(identity),
            "{} mentions `{identity}`. Only a resource wears a coat: the moment a colour can be \
             reached from which structure, idea, job, or kitten this is, the palette has to grow \
             with the content and stops meaning anything.",
            path.display()
        );
    }

    // A coat is a function of which resource, and of nothing else — the same
    // answer on two calls, with no stock, cap, rate, or season in sight.
    for r in ResourceId::ALL {
        assert_eq!(
            coat(r),
            coat(r),
            "{r:?} answered differently on two calls — a coat must be a function of the resource \
             and of nothing else"
        );
    }

    // The season is the one thing outside the state vocabulary that reaches the
    // theme for a state colour, and it reaches only the chrome.
    assert_eq!(
        PALETTE_ROLES.iter().map(|r| rgb(*r)).collect::<HashSet<_>>().len(),
        5,
        "five state colours, however many items exist"
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-004 — six roles, and nothing outside them but the accent
// ---------------------------------------------------------------------------

/// Everything the theme can put on screen, counted. Five states, four seasons,
/// seven hues of the ring, and the terminal's own foreground; beyond those,
/// only the colours a blended coat passes through between two of its stops.
///
/// @spec TUI-THEME-004
#[test]
fn the_theme_can_produce_sixteen_colours_and_the_paths_between_stops_and_no_others() {
    // The compile-time half: `role_name` matches on `Role` with no wildcard, so
    // a seventh role stops this file building.
    for role in ROLES {
        assert!(!role_name(role).is_empty());
    }

    let mut all = HashSet::new();
    for role in PALETTE_ROLES {
        all.insert(rgb(role));
    }
    for season in Season::ALL {
        all.insert(accent(season));
    }
    for hue in Hue::RING {
        all.insert(hue_rgb(hue));
    }

    assert_eq!(
        all.len(),
        16,
        "the theme should reach exactly sixteen values — five states, four seasonal accents, and \
         seven ring hues — and it reached {}. Each of the sixteen is a thing the player has to learn.",
        all.len()
    );

    assert_eq!(
        Role::Inherited.color(),
        None,
        "the sixth role is the absence of a colour, not a seventeenth value"
    );

    // Every colour a solid or patched coat puts on a character is a stop, and
    // every colour a blended coat puts on one lies on the path between two
    // consecutive stops — nothing a coat draws is a colour of its own.
    let ring: HashSet<(u8, u8, u8)> = Hue::RING.iter().map(|h| hue_rgb(*h)).collect();
    for r in ResourceId::ALL {
        let c = coat(r);
        for n in 1..=16 {
            let colours = coat_colours(c, n);
            assert_eq!(colours.len(), n, "{r:?}: one colour per character");
            match c.dressing {
                Dressing::Solid | Dressing::Patched => {
                    for (i, colour) in colours.iter().enumerate() {
                        let value = rgb_of(*colour, &format!("{r:?}'s coat, character {i}"));
                        assert!(
                            ring.contains(&value),
                            "{r:?}'s {:?} coat puts #{:02X}{:02X}{:02X} on character {i} of {n}, which is \
                             not a hue of the ring",
                            c.dressing,
                            value.0,
                            value.1,
                            value.2
                        );
                    }
                }
                Dressing::Blended => {
                    for (i, colour) in colours.iter().enumerate() {
                        let expected = expected_blend_colour(c.stops, i, n);
                        assert_eq!(
                            rgb_of(*colour, "a blended coat"),
                            rgb_of(expected, "the path"),
                            "{r:?}'s blended coat, character {i} of {n}, is off the path between its stops"
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-005 — mood takes no colour
// ---------------------------------------------------------------------------

/// The word already says it, unambiguously, in a form a screen reader can speak
/// and no deficiency can compress. Adding a ramp on top would spend a channel
/// to say nothing new to most players and something unreliable to the rest.
///
/// @spec TUI-THEME-005
#[test]
fn mood_is_a_word_and_takes_no_colour_at_all() {
    let mood = meaning("the clowder's mood");

    assert_eq!(
        mood.tint,
        Tint::Role(Role::Inherited),
        "mood took {:?}. Mood is rendered as a word in the terminal's own foreground; a palette \
         value on it would be a second channel saying the same thing less reliably.",
        mood.tint
    );
    assert_eq!(
        Role::Inherited.color(),
        None,
        "the role mood renders in must carry no colour of its own"
    );

    let palette: Vec<(u8, u8, u8)> = PALETTE_ROLES.iter().map(|r| rgb(*r)).collect();
    assert_eq!(palette.len(), 5, "and none of the five is spent on a mood ramp");

    assert!(
        !mood.first_channel.is_empty(),
        "mood's only channel is the word itself, so it had better have one"
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-006 — focus takes no colour
// ---------------------------------------------------------------------------

/// Every palette colour is claimed by a state, so there is none spare to spend
/// on focus. That is the real protection: a focused row's state is whatever its
/// own marker says, and a focus tint would put two colours in one row saying
/// different things.
///
/// @spec TUI-THEME-006
#[test]
fn no_palette_colour_is_left_over_to_be_spent_on_focus() {
    for role in PALETTE_ROLES {
        let claimants: Vec<&str> = MEANINGS
            .iter()
            .filter(|m| m.tint == Tint::Role(role))
            .map(|m| m.what)
            .collect();
        assert!(
            !claimants.is_empty(),
            "`{}` is a palette colour no state claims. A spare role is exactly how a focus tint \
             gets in — and focus is not a state.",
            role_name(role)
        );
        // And it really is a colour that state is spending.
        let _ = rgb(role);
    }

    // The cursor is a glyph, and it collides with no state's channel: a focused
    // row still shows its own marker beside the cursor.
    const CURSOR: &str = "▸";
    for m in &MEANINGS {
        assert_ne!(
            m.first_channel, CURSOR,
            "the cursor glyph is also {:?}'s first channel, so a focused row and that state \
             would look the same",
            m.what
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-007 — reverse video inverts the terminal's own pair
// ---------------------------------------------------------------------------

/// An emphasis that names no colour is the one emphasis that works on a ground
/// clowder does not own. The buffer keeps the reversal *unresolved* — foreground
/// and background both left at the terminal's own — so the inversion happens in
/// whichever polarity the player is actually in.
///
/// @spec TUI-THEME-007
#[test]
fn reverse_video_leaves_the_pair_for_the_terminal_to_invert() {
    let emphasis = Style::default().add_modifier(Modifier::REVERSED);

    for (width, height) in SIZES {
        let buffer = frame_at(width, height, emphasis);
        for y in 0..height {
            for x in 0..width {
                let cell = buffer.cell((x, y)).expect("every cell of the area exists");
                assert!(
                    cell.modifier.contains(Modifier::REVERSED),
                    "cell ({x},{y}) of a {width}x{height} frame lost the reversal"
                );
                assert_eq!(
                    (cell.fg, cell.bg),
                    (Color::Reset, Color::Reset),
                    "cell ({x},{y}) of a {width}x{height} frame resolved the reversal to \
                     ({:?}, {:?}) shell-side. Reverse video has to reach the terminal as a \
                     swap of *its* pair, or it only works in one polarity.",
                    cell.fg,
                    cell.bg
                );
            }
        }
    }

    // Why the palette cannot supply an emphasis: none of its values is the
    // terminal's own colour, so a palette-tinted row would be a fixed pair
    // rather than an inversion of whatever the player is using.
    for role in PALETTE_ROLES {
        assert_ne!(
            role.color(),
            Some(Color::Reset),
            "`{}` claims to be the terminal's own colour; it is a fixed value",
            role_name(role)
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-008 — the pane keeps the season
// ---------------------------------------------------------------------------

/// A glance tells you where in the year the clowder is, so the four have to be
/// four — a season that shared an accent with its neighbour would make half the
/// year unreadable.
///
/// @spec TUI-THEME-008
#[test]
fn each_season_gives_the_chrome_an_accent_of_its_own() {
    const EXPECTED: [(Season, (u8, u8, u8)); 4] = [
        (Season::Spring, (0x7F, 0xA0, 0x5A)),
        (Season::Summer, (0xC9, 0xA2, 0x27)),
        (Season::Autumn, (0xB5, 0x71, 0x3C)),
        (Season::Winter, (0x6E, 0x8B, 0xA8)),
    ];

    for (season, want) in EXPECTED {
        let got = accent(season);
        assert_eq!(
            got, want,
            "the {season:?} accent is #{:02X}{:02X}{:02X}, expected #{:02X}{:02X}{:02X}",
            got.0, got.1, got.2, want.0, want.1, want.2
        );
        assert_eq!(
            seasonal_accent(season),
            seasonal_accent(season),
            "the accent must be a function of the season and nothing else"
        );
    }

    let distinct: HashSet<(u8, u8, u8)> = Season::ALL.iter().map(|s| accent(*s)).collect();
    assert_eq!(
        distinct.len(),
        4,
        "two seasons share an accent, so a pane left open across the year stops changing its \
         light where they meet"
    );

    // The accent is the season's second channel; the header still names it.
    assert_eq!(meaning("the season").tint, Tint::SeasonalAccent);
}

/// Every accent a season can give, so a cell can be checked for wearing any of
/// them — the snapshot's own season's or another's.
fn all_accents() -> [Color; 4] {
    Season::ALL.map(seasonal_accent)
}

/// A meadow on a chosen day, with a list long enough to scroll and a store
/// sidebar with something in it, so every region of a frame draws something
/// that could, wrongly, wear the accent.
fn meadow_on_day(day: u64) -> clowder_core::GameState {
    use clowder_core::{content, ContentId, GameState, ResourceId, StructureId};
    let mut s = GameState::new(11);
    s.ticks = day * content::TICKS_PER_DAY;
    s.revealed = StructureId::ALL.iter().map(|b| ContentId::Structure(*b)).collect();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1_234.0;
    }
    for b in StructureId::ALL.iter().take(6) {
        s.structures[b.ix()] = 2;
    }
    s
}

/// The views a chrome check is made over: the list with its cursor near the
/// top and deep in its scroll, the inspector asked for (a swap below 100
/// columns, a pane above), and a full-screen overlay over the body.
fn chrome_views() -> Vec<(&'static str, clowder_tui::nav::ViewState)> {
    use clowder_core::{ContentId, StructureId};
    use clowder_tui::nav::{Overlay, RowKey, Section, ViewState};
    let at = |i: usize| {
        let mut v = ViewState { section: Some(Section::Meadow), ..Default::default() };
        v.focus[Section::Meadow.index()] =
            Some(RowKey::Content(ContentId::Structure(StructureId::ALL[i]), ""));
        v
    };
    vec![
        ("the list", at(4)),
        ("the list scrolled", at(44)),
        ("the inspector asked for", ViewState { inspector_wanted: true, ..at(4) }),
        ("help open", ViewState { overlay: Some(Overlay::Help), ..at(4) }),
    ]
}

/// Sizes either side of both layout breakpoints: one divider below 100
/// columns, two at 100 and wider.
const CHROME_SIZES: [(u16, u16); 8] =
    [(80, 24), (90, 24), (99, 24), (100, 30), (110, 30), (119, 40), (120, 40), (150, 50)];

/// Draw one frame into `term` after `prev`, returning its cells and layout.
fn draw_frame(
    term: &mut Terminal<TestBackend>,
    state: &clowder_core::GameState,
    view: &clowder_tui::nav::ViewState,
    prev: Option<&clowder_tui::render::layout::Layout>,
) -> (ratatui::buffer::Buffer, clowder_tui::render::layout::Layout) {
    let mut layout = None;
    term.draw(|f| layout = Some(clowder_tui::render::frame(f, state, view, prev)))
        .expect("drawing into an in-memory backend cannot fail");
    (term.backend().buffer().clone(), layout.expect("frame returns the layout it used"))
}

/// The cells a frame's pane dividers occupy (TUI-RENDER-016): each body row
/// of the list's space's edge column facing the sidebar, and, where the
/// inspector is a pane of its own, of its edge column facing the inspector —
/// and none while a full-screen overlay takes the body.
fn divider_cells(
    layout: &clowder_tui::render::layout::Layout,
    view: &clowder_tui::nav::ViewState,
) -> HashSet<(u16, u16)> {
    use clowder_tui::render::layout::Body;
    if view.overlay.is_some() {
        return HashSet::new();
    }
    let (space, columns) = match layout.body {
        Body::Narrow { main, .. } => (main, vec![main.x]),
        Body::Wide { list, .. } => (list, vec![list.x, list.right() - 1]),
        Body::TooSmall(_) => return HashSet::new(),
    };
    columns.into_iter().flat_map(|x| (space.y..space.bottom()).map(move |y| (x, y))).collect()
}

/// The claim at one frame: the header rule's underline, on every header cell,
/// wears `accent` while each character above it keeps the colour the header
/// widget gives it alone; every divider cell wears `accent`; and no other
/// cell's foreground, background or underline wears any season's accent.
#[track_caller]
fn assert_only_the_chrome_wears(
    buf: &ratatui::buffer::Buffer,
    layout: &clowder_tui::render::layout::Layout,
    state: &clowder_core::GameState,
    view: &clowder_tui::nav::ViewState,
    accent: Color,
    what: &str,
) {
    let (w, h) = (buf.area.width, buf.area.height);
    let accents = all_accents();
    let dividers = divider_cells(layout, view);

    let mut alone = Terminal::new(TestBackend::new(w, 1)).expect("a test backend always builds");
    alone
        .draw(|f| {
            let area = f.area();
            clowder_tui::render::widgets::chrome::header(f, area, state, view);
        })
        .expect("drawing into an in-memory backend cannot fail");
    let alone = alone.backend().buffer().clone();

    for y in 0..h {
        for x in 0..w {
            let cell = buf.cell((x, y)).expect("every cell of the area exists");
            if y == layout.header.y {
                let own = alone.cell((x, 0)).expect("header cell drawn alone");
                assert!(
                    cell.modifier.contains(Modifier::UNDERLINED) && cell.underline_color == accent,
                    "{w}x{h} with {what}: header column {x} (`{}`) has underline {} in {:?}; the \
                     header rule runs beneath every header cell in the snapshot's season's \
                     accent {accent:?}",
                    cell.symbol(),
                    cell.modifier.contains(Modifier::UNDERLINED),
                    cell.underline_color
                );
                assert_eq!(
                    (cell.fg, cell.bg),
                    (own.fg, own.bg),
                    "{w}x{h} with {what}: header column {x} (`{}`) is drawn in {:?} on {:?}, \
                     but the header gives it {:?} on {:?}. The accent colours the underline \
                     alone; every character above it keeps its own colour.",
                    cell.symbol(),
                    cell.fg,
                    cell.bg,
                    own.fg,
                    own.bg
                );
                continue;
            }
            if dividers.contains(&(x, y)) {
                assert_eq!(
                    cell.fg, accent,
                    "{w}x{h} with {what}: the divider cell ({x},{y}) (`{}`) is drawn in {:?}, \
                     not the snapshot's season's accent {accent:?}",
                    cell.symbol(),
                    cell.fg
                );
                continue;
            }
            for (part, colour) in
                [("foreground", cell.fg), ("background", cell.bg), ("underline", cell.underline_color)]
            {
                assert!(
                    !accents.contains(&colour),
                    "{w}x{h} with {what}: `{}` at ({x},{y}) wears a seasonal accent ({colour:?}) \
                     as its {part}. The accent is the header rule's and the pane dividers' and \
                     nothing else's.",
                    cell.symbol()
                );
            }
        }
    }
}

/// The header rule and every pane divider wear the accent of the season the
/// drawn snapshot holds — for each of the four seasons, and for a snapshot
/// the core holds in a season its calendar day is not in, so the accent is
/// the snapshot's season and not one worked out from the date. On the rule
/// the accent is the underline's colour alone, every character above it
/// keeping the colour the header gives it; and no other cell of the frame —
/// foreground, background or underline — wears any season's accent. At every
/// size either side of both breakpoints, with the list, the inspector asked
/// for, and a full-screen overlay (which leaves no divider to wear it).
///
/// @spec TUI-THEME-008
#[test]
fn the_rule_and_every_divider_wear_the_snapshots_season_and_nothing_else_does() {
    use clowder_core::{content, AdventureId};

    let mut snapshots: Vec<(String, clowder_core::GameState, Season)> = Season::ALL
        .iter()
        .enumerate()
        .map(|(i, season)| {
            let s = meadow_on_day(i as u64 * content::DAYS_PER_SEASON + 3);
            (format!("{season:?}"), s, *season)
        })
        .collect();
    let held = {
        let mut s = meadow_on_day(3);
        s.active_adventure = Some(AdventureId::LongWinter);
        s
    };
    snapshots.push(("a season held by the core".to_string(), held, Season::Winter));

    for (name, state, season) in &snapshots {
        assert_eq!(state.season(), *season, "fixture: the {name} snapshot holds {season:?}");
        let accent = seasonal_accent(*season);
        for (w, h) in CHROME_SIZES {
            for (what, view) in chrome_views() {
                let mut term =
                    Terminal::new(TestBackend::new(w, h)).expect("a test backend always builds");
                let (buf, layout) = draw_frame(&mut term, state, &view, None);
                assert_only_the_chrome_wears(
                    &buf,
                    &layout,
                    state,
                    &view,
                    accent,
                    &format!("{what} in the {name} snapshot"),
                );
            }
        }
    }
    // The held snapshot's calendar day is in a different season from the one
    // it holds, so an accent read off the date would fail above.
    assert_ne!(
        meadow_on_day(3).season(),
        Season::Winter,
        "fixture: the held snapshot's calendar season differs from the season it holds"
    );
}

/// At each of the four turns of the year, the frame that draws the first
/// snapshot of the new season draws the new season's accent on every chrome
/// cell — on the same frame as the header's season word changes, with no cell
/// still wearing the old accent or anything between the two — and that frame,
/// drawn straight after the old season's, is cell for cell the frame the new
/// snapshot draws cold: no blend, because nothing about the frame before
/// reaches it. The frame after that is the same again.
///
/// @spec TUI-THEME-008
#[test]
fn the_accent_turns_on_the_frame_the_season_word_does_with_no_blend() {
    use clowder_core::content;

    let word = |season: Season| -> String { format!("{season:?}").chars().take(3).collect() };
    let view = chrome_views().remove(0).1;

    for turn in 1..=4u64 {
        let boundary = turn * content::DAYS_PER_SEASON;
        let mut before = meadow_on_day(boundary);
        before.ticks -= 1;
        let after = meadow_on_day(boundary);
        let (old, new) = (before.season(), after.season());
        assert_ne!(old, new, "fixture: turn {turn} crosses a season boundary");
        let (old_accent, new_accent) = (seasonal_accent(old), seasonal_accent(new));

        for (w, h) in [(80, 24), (130, 40)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test backend always builds");
            let (buf_old, layout_old) = draw_frame(&mut term, &before, &view, None);
            let header_old: String = (0..w).map(|x| buf_old.cell((x, 0)).expect("cell").symbol()).collect();
            assert!(header_old.contains(&word(old)), "fixture: the header names {old:?}: {header_old:?}");
            assert_only_the_chrome_wears(&buf_old, &layout_old, &before, &view, old_accent, "the last frame of a season");

            let (buf_new, layout_new) = draw_frame(&mut term, &after, &view, Some(&layout_old));
            let header_new: String = (0..w).map(|x| buf_new.cell((x, 0)).expect("cell").symbol()).collect();
            assert!(
                header_new.contains(&word(new)) && !header_new.contains(&word(old)),
                "fixture: the first frame of the new season names {new:?} and not {old:?}: {header_new:?}"
            );
            assert_only_the_chrome_wears(&buf_new, &layout_new, &after, &view, new_accent, "the first frame of a season");
            for y in 0..h {
                for x in 0..w {
                    let c = buf_new.cell((x, y)).expect("cell");
                    assert!(
                        c.fg != old_accent && c.underline_color != old_accent,
                        "{w}x{h}: at turn {turn}, ({x},{y}) still wears the old season's accent \
                         on the frame the header names the new one"
                    );
                }
            }

            let mut fresh = Terminal::new(TestBackend::new(w, h)).expect("a test backend always builds");
            let (cold, _) = draw_frame(&mut fresh, &after, &view, None);
            assert_eq!(
                buf_new, cold,
                "{w}x{h}: at turn {turn}, the first frame of the new season drawn after the old \
                 one differs from the frame the new snapshot draws cold — the accent carried \
                 something over from the frame before"
            );
            let (again, _) = draw_frame(&mut term, &after, &view, Some(&layout_new));
            assert_eq!(
                again, cold,
                "{w}x{h}: at turn {turn}, the frame after the turn changed again with the \
                 snapshot unchanged — the accent is still moving"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-009 — the accent never touches anything carrying state
// ---------------------------------------------------------------------------

/// If the accent could land on something stateful, the player would have to ask
/// whether a green meant *affordable* or *spring* — whimsy bought with clarity,
/// which tenet 3 forbids. Only one meaning in the whole interface wears the
/// accent, and it is the season itself.
///
/// @spec TUI-THEME-009
#[test]
fn only_the_season_wears_the_seasonal_accent() {
    let wearers: Vec<&str> =
        MEANINGS.iter().filter(|m| m.tint == Tint::SeasonalAccent).map(|m| m.what).collect();
    assert_eq!(
        wearers,
        vec!["the season"],
        "the seasonal accent reached {wearers:?}. It may tint the header rule and the pane \
         divider and nothing else; anything carrying state that wore it would have two possible \
         readings."
    );

    for season in Season::ALL {
        let a = accent(season);
        for role in PALETTE_ROLES {
            let s = rgb(role);
            assert_ne!(
                a, s,
                "the {season:?} accent is exactly `{}` (#{:02X}{:02X}{:02X}). One value cannot \
                 mean both a season and a state.",
                role_name(role),
                s.0,
                s.1,
                s.2
            );
        }
    }
}

/// And the reason confinement has to do all the work: the accents were chosen
/// from the same mid-luminance band as the states, so several of them sit close
/// enough to a state colour to be mistaken for it. Nothing about the *values*
/// keeps a seasonal rule from reading as a state marker — only never putting it
/// on one does.
///
/// @spec TUI-THEME-009
#[test]
fn the_accent_is_kept_unambiguous_by_confinement_and_not_by_colour_distance() {
    let mut nearest = ("".to_string(), f64::INFINITY);
    for season in Season::ALL {
        for role in PALETTE_ROLES {
            let d = delta_e(accent(season), rgb(role));
            if d < nearest.1 {
                nearest = (format!("the {season:?} accent and `{}`", role_name(role)), d);
            }
        }
    }

    assert!(
        nearest.1 < 25.0,
        "{} differ by ΔE {:.1}, so the accents happen to be separable from the states by eye. \
         The design does not rely on that and callers must not either: the guarantee is that the \
         accent only ever touches inert chrome. If the palette has been re-tuned so the two sets \
         are genuinely far apart, update the design's reasoning before relaxing this.",
        nearest.0,
        nearest.1
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-010 — colour is always the second channel
// ---------------------------------------------------------------------------

/// The actual test of "turn every colour off and the game is still completely
/// playable": strip the colour column and every meaning must still be told
/// apart by what is left. Restating the table would prove nothing — what is
/// asserted is that the non-colour channels are *injective* (no two meanings
/// look alike without colour) while the colours are not (several meanings share
/// one), which is precisely why the glyph has to be there first.
///
/// @spec TUI-THEME-010
#[test]
fn removing_every_colour_loses_no_meaning() {
    // Every meaning has something other than colour carrying it.
    for m in &MEANINGS {
        assert!(
            !m.first_channel.trim().is_empty(),
            "{:?} has no glyph, word or fill — colour would be its only channel",
            m.what
        );
    }

    // With colour gone, the channels that remain still tell every meaning apart.
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for m in &MEANINGS {
        if let Some((other, "")) = seen.iter().find(|(_, c)| *c == m.first_channel) {
            panic!(
                "{:?} and {other:?} are both carried by {:?}, so with colour removed they are \
                 the same thing on screen. Colour may only ever reinforce a distinction \
                 something else already makes.",
                m.what, m.first_channel
            );
        }
        seen.push((m.what, m.first_channel));
    }

    // And colour on its own could not have done it: several meanings share one.
    let distinct_tints: HashSet<&'static str> = MEANINGS
        .iter()
        .map(|m| match m.tint {
            Tint::Role(r) => role_name(r),
            Tint::SeasonalAccent => "seasonal accent",
        })
        .collect();
    assert!(
        distinct_tints.len() < MEANINGS.len(),
        "every meaning has a colour to itself, which is the fifty-hue palette this design \
         rejects — and it would make the non-colour channel look optional"
    );

    // The colours the sharing meanings reinforce are real values, so the
    // sharing is deliberate rather than an unassigned state.
    for role in PALETTE_ROLES {
        let _ = rgb(role);
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-011 — the glyphs of the hues a deficiency compresses
// ---------------------------------------------------------------------------

/// The palette's riskiest axis. Ready, partial and attention are the
/// green-amber-orange run, and a red-green deficiency compresses exactly that
/// run — so those three states fall entirely to their glyphs. This asserts both
/// halves: that the glyphs are distinct, and that the colours really do
/// collapse, which is what makes the glyphs load-bearing rather than a courtesy.
///
/// @spec TUI-THEME-011
#[test]
fn the_three_states_a_red_green_deficiency_compresses_are_told_apart_by_glyph() {
    // The glyphs (TUI-NAV-009's markers, TUI-NAV-020's chips).
    const AFFORDABLE: &str = "●";
    const PARTIAL: &str = "◐";
    const ATTENTION_CHIPS: [&str; 4] = ["◦", "!", "★", "~"];

    assert_eq!(AFFORDABLE, meaning("affordable now").first_channel);
    assert_eq!(PARTIAL, meaning("partially affordable").first_channel);

    assert_ne!(
        AFFORDABLE, PARTIAL,
        "affordable and partially affordable share a glyph, so the only thing separating them \
         is a green from an amber"
    );
    for chip in ATTENTION_CHIPS {
        assert_ne!(
            chip, AFFORDABLE,
            "an attention chip uses the affordable marker; with the hues compressed the two are \
             one appearance"
        );
        assert_ne!(
            chip, PARTIAL,
            "an attention chip uses the partial marker; with the hues compressed the two are one \
             appearance"
        );
        assert_eq!(
            chip.chars().count(),
            1,
            "{chip:?} is not a single glyph, so it cannot be the thing that carries the state"
        );
    }
    assert_eq!(AFFORDABLE.chars().count(), 1);
    assert_eq!(PARTIAL.chars().count(), 1);

    // Now the colours, to show why the above matters.
    let three = [
        ("ready", rgb(Role::Ready)),
        ("partial", rgb(Role::Partial)),
        ("attention", rgb(Role::Attention)),
    ];

    let (normal_pair, normal) = closest_pair(&three);
    assert!(
        normal > 20.0,
        "with ordinary colour vision {normal_pair} already differ by only ΔE {normal:.1}; these \
         three states are meant to be plainly distinct to a player who can see them"
    );

    for (deficiency, matrix) in [("protanopia", PROTANOPIA), ("deuteranopia", DEUTERANOPIA)] {
        let simulated: Vec<(&str, (u8, u8, u8))> =
            three.iter().map(|(n, c)| (*n, simulate(*c, matrix))).collect();
        let (pair, distance) = closest_pair(&simulated);
        assert!(
            distance < normal,
            "under {deficiency} the closest pair ({pair}, ΔE {distance:.1}) is no closer than \
             under ordinary vision (ΔE {normal:.1}). If this palette no longer compresses, say \
             so in the design — but the glyph requirement stands either way."
        );
        assert!(
            distance < 20.0,
            "under {deficiency} the closest pair is {pair} at ΔE {distance:.1}. The point of \
             this assertion is that these hues *do* collapse, which is why the glyph carries the \
             state and the colour only reinforces it."
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-012 — truecolor is checked, and nothing is substituted
// ---------------------------------------------------------------------------

/// Serialises the tests that reach into the process environment. Poisoning is
/// ignored deliberately: these tests are expected to fail until the palette
/// exists, and one panicking must not turn the rest into a different failure.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Sets an environment variable for as long as it is alive.
struct EnvVar {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVar {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let previous = std::env::var_os(key);
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        EnvVar { key, previous }
    }
}

impl Drop for EnvVar {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

/// A terminal that has not been told to advertise RGB — the tmux default —
/// silently approximates every value, and the player sees a muddy game they
/// will reasonably assume just looks like that. Detecting it is the whole point
/// of saying anything at all.
///
/// @spec TUI-THEME-012
#[test]
fn truecolor_is_read_from_what_the_terminal_advertises() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _term = EnvVar::set("TERM", Some("xterm-256color"));

    {
        let _c = EnvVar::set("COLORTERM", Some("truecolor"));
        assert!(
            truecolor_advertised(),
            "a terminal advertising COLORTERM=truecolor was not believed, so a correctly \
             configured player is told about a problem they do not have"
        );
    }
    {
        let _c = EnvVar::set("COLORTERM", Some("24bit"));
        assert!(
            truecolor_advertised(),
            "COLORTERM=24bit is the other spelling of the same advertisement"
        );
    }
    {
        let _c = EnvVar::set("COLORTERM", None);
        assert!(
            !truecolor_advertised(),
            "a terminal that advertises nothing was treated as advertising RGB — which is the \
             case that silently approximates the whole palette"
        );
    }
    {
        // rxvt set COLORTERM long before 24-bit colour existed; it means
        // "colour", not "RGB".
        let _c = EnvVar::set("COLORTERM", Some("rxvt"));
        assert!(
            !truecolor_advertised(),
            "COLORTERM set to something other than truecolor/24bit says only that the terminal \
             has colour, not that it has 24-bit colour"
        );
    }
}

/// No fallback palette, ever: a reduced scheme is an explicit non-goal, and the
/// notice exists precisely so the game can go on rendering exactly as designed.
/// The values must therefore not depend on what was detected.
///
/// @spec TUI-THEME-012
#[test]
fn the_palette_is_identical_whether_or_not_truecolor_is_advertised() {
    let _serial = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _term = EnvVar::set("TERM", Some("xterm-256color"));

    let snapshot = || {
        let roles: Vec<Option<Color>> = ROLES.iter().map(|r| r.color()).collect();
        let seasons: Vec<Color> = Season::ALL.iter().map(|s| seasonal_accent(*s)).collect();
        (roles, seasons)
    };

    let advertised = {
        let _c = EnvVar::set("COLORTERM", Some("truecolor"));
        snapshot()
    };
    let not_advertised = {
        let _c = EnvVar::set("COLORTERM", None);
        snapshot()
    };

    assert_eq!(
        advertised, not_advertised,
        "the theme rendered differently once truecolor was missing. Clowder says so once and \
         renders unchanged; a substituted palette turns a known limitation back into an \
         apparent defect, and a low-colour fallback is an explicit non-goal."
    );
}

/// Every value is a 24-bit one. An indexed or named colour would be the
/// terminal's own palette rather than the design's, which is the reduced scheme
/// the previous test forbids by another route.
///
/// @spec TUI-THEME-012
#[test]
fn nothing_the_theme_produces_is_a_reduced_palette_value() {
    for role in PALETTE_ROLES {
        let colour = role.color().expect("a palette role has a colour");
        assert!(
            matches!(colour, Color::Rgb(..)),
            "`{}` is {colour:?} rather than a 24-bit value",
            role_name(role)
        );
    }
    for season in Season::ALL {
        let colour = seasonal_accent(season);
        assert!(
            matches!(colour, Color::Rgb(..)),
            "the {season:?} accent is {colour:?} rather than a 24-bit value"
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-013 — the notice lands on the player's own terminal
// ---------------------------------------------------------------------------
//
// These run the real binary, as `tests/loop_order.rs` does, because the claim
// is about *ordering* between something printed and the terminal being taken
// over, and that is only observable from outside the process.
//
// Two things make the observation safe. The child's stdout and stderr are the
// same file, so their relative order is real rather than an artefact of two
// pipes. And the child is put in its own session before exec, so it has no
// controlling terminal at all: `/dev/tty` cannot be opened, the handover fails,
// and the developer's own terminal is never put into raw mode. That also
// sharpens the test — a notice emitted after the handover would never appear.

#[cfg(unix)]
const NOTICE_WORDS: [&str; 5] = ["truecolor", "true color", "true colour", "24-bit", "24 bit"];

/// Something the player can act on. Whichever of these the notice names, it is
/// telling them what to set rather than only what is wrong.
#[cfg(unix)]
const SETTING_WORDS: [&str; 4] = ["colorterm", "terminal-overrides", "rgb", ":tc"];

/// Every spelling of "switch to the alternate screen". Anything printed after
/// one of these is erased when the game exits.
#[cfg(unix)]
const ALTERNATE_SCREEN: [(&str, &str); 4] = [
    ("\x1b[?1049h", "enter alternate screen"),
    ("\x1b[?1049l", "leave alternate screen"),
    ("\x1b[?1047h", "enter alternate screen (legacy)"),
    ("\x1b[?47h", "enter alternate screen (legacy)"),
];

#[cfg(unix)]
extern "C" {
    /// POSIX `setsid(2)`. Rust links libc on every unix target, so reaching it
    /// needs no extra dependency — the same route `runtime::term` takes to
    /// `signal(2)`.
    fn setsid() -> i32;
}

#[cfg(unix)]
struct TempDir {
    path: std::path::PathBuf,
}

#[cfg(unix)]
impl TempDir {
    fn new(tag: &str) -> TempDir {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("clowder-theme-{}-{}-{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory should be creatable");
        TempDir { path }
    }
}

#[cfg(unix)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Start the real binary against a scratch home, with stdout and stderr sharing
/// one file so their order is observable, and return everything it wrote.
#[cfg(unix)]
fn run_clowder(tag: &str, colorterm: Option<&str>) -> String {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let home = TempDir::new(tag);
    let output_path = home.path.join("clowder-output.txt");
    let file = std::fs::File::create(&output_path).expect("creating the child's output file");
    let stderr = file.try_clone().expect("duplicating the output file for stderr");

    let mut command = Command::new(env!("CARGO_BIN_EXE_clowder"));
    command
        .env_clear()
        .env("HOME", &home.path)
        .env("XDG_DATA_HOME", home.path.join(".local/share"))
        .env("PATH", "/usr/bin:/bin")
        .env("TERM", "xterm-256color")
        .current_dir(&home.path)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::from(stderr));
    if let Some(value) = colorterm {
        command.env("COLORTERM", value);
    }

    // SAFETY: `setsid` is async-signal-safe, and the closure does nothing else.
    // Failing here rather than continuing matters: without a new session the
    // child would open the developer's own terminal and put it into raw mode.
    unsafe {
        command.pre_exec(|| {
            if setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = command.spawn().expect("spawning the clowder binary");

    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait().expect("polling the child") {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                break;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let _ = child.wait();

    std::fs::read_to_string(&output_path)
        .map(|s| s.to_string())
        .unwrap_or_else(|e| panic!("reading the child's output: {e}"))
}

#[cfg(unix)]
fn first_notice(output: &str) -> Option<usize> {
    let lower = output.to_lowercase();
    NOTICE_WORDS.iter().filter_map(|w| lower.find(w)).min()
}

/// The difference between a known limitation and an apparent defect is entirely
/// whether anyone said anything — and saying it inside the alternate screen is
/// the same as not saying it, because the screen is torn down on the way out
/// and takes the message with it.
///
/// @spec TUI-THEME-012, TUI-THEME-013
#[cfg(unix)]
#[test]
fn a_terminal_without_truecolor_is_told_so_before_the_alternate_screen() {
    let output = run_clowder("no-truecolor", None);

    let at = first_notice(&output).unwrap_or_else(|| {
        panic!(
            "the shell said nothing about truecolor to a terminal that advertises none, so every \
             value will be silently approximated and the player will assume the game just looks \
             like that.\n--- child output ---\n{}",
            output.escape_debug()
        )
    });

    let lower = output.to_lowercase();
    assert!(
        SETTING_WORDS.iter().any(|w| lower.contains(w)),
        "the shell mentioned truecolor but named nothing to set, so the player is told they have \
         a problem and not how to fix it.\n--- child output ---\n{}",
        output.escape_debug()
    );

    for (sequence, what) in ALTERNATE_SCREEN {
        if let Some(screen_at) = output.find(sequence) {
            assert!(
                at < screen_at,
                "the notice was printed after the shell asked the terminal to {what} ({}), so it \
                 lands on a screen that disappears when the game exits.\n--- child output ---\n{}",
                sequence.escape_debug(),
                output.escape_debug()
            );
        }
    }
}

/// "When the terminal does not advertise truecolor" is a condition, and both
/// sides of it are the requirement. A player whose terminal is configured
/// correctly hears nothing: a notice printed unconditionally is noise on every
/// launch, and noise on every launch is how a pane that lives beside real work
/// becomes irritating on day three.
///
/// Both directions are asserted together, because either alone is satisfiable
/// by a shell that simply never speaks.
///
/// @spec TUI-THEME-012
#[cfg(unix)]
#[test]
fn the_notice_is_said_when_truecolor_is_missing_and_not_otherwise() {
    let missing = run_clowder("conditional-missing", None);
    let advertised = run_clowder("conditional-advertised", Some("truecolor"));

    assert!(
        first_notice(&missing).is_some(),
        "a terminal advertising no truecolor was told nothing.\n--- child output ---\n{}",
        missing.escape_debug()
    );
    assert_eq!(
        first_notice(&advertised),
        None,
        "a terminal that advertises truecolor was told about truecolor anyway, so the notice is \
         unconditional and becomes noise on every launch.\n--- child output ---\n{}",
        advertised.escape_debug()
    );
}

// ---------------------------------------------------------------------------
// TUI-THEME-014 — a finished row recedes
// ---------------------------------------------------------------------------

/// Dim is what "done" looks like, and done must not compete with the rows that
/// can still be acted on. What makes it recede is that it is very nearly grey —
/// chroma is what pulls the eye, and dim has almost none while every other role
/// is strongly coloured. It still clears the reading floor, though: a finished
/// row is revealed, not hidden.
///
/// @spec TUI-THEME-014
#[test]
fn an_owned_or_completed_row_recedes_without_becoming_unreadable() {
    assert_eq!(
        meaning("owned or done").tint,
        Tint::Role(Role::Dim),
        "a finished one-shot renders in dim"
    );

    let dim = rgb(Role::Dim);
    let dim_chroma = chroma(dim);

    for role in [Role::Ready, Role::Partial, Role::Attention, Role::Milestone] {
        let other = chroma(rgb(role));
        assert!(
            other > dim_chroma * 4.0,
            "`dim` has chroma {dim_chroma:.1} and `{}` has {other:.1}. Dim has to be plainly the \
             quieter of the two, or a finished row goes on competing for the eye with the rows \
             that can still be acted on.",
            role_name(role)
        );
    }

    for ground in [NEAR_BLACK, NEAR_WHITE] {
        let ratio = contrast(dim, ground);
        assert!(
            ratio >= CONTRAST_FLOOR,
            "`dim` is {ratio:.2}:1 against #{:02X}{:02X}{:02X}, below the {CONTRAST_FLOOR}:1 \
             floor. Dim means revealed-but-not-actionable, and a row the player cannot read is \
             not revealed.",
            ground.0,
            ground.1,
            ground.2
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-015 — one colour for everything that wants notice
// ---------------------------------------------------------------------------

/// The role means "this wants you", wherever it appears. Subdividing it per
/// surface — one orange for chips, another for capped stores — would make the
/// player learn three colours for one idea, and would spend the palette on
/// information the surface already gives them.
///
/// @spec TUI-THEME-015
#[test]
fn everything_that_wants_notice_wears_the_same_one_colour() {
    let wants_notice = ["an attention chip", "a capped store", "an armed row"];

    for what in wants_notice {
        assert_eq!(
            meaning(what).tint,
            Tint::Role(Role::Attention),
            "{what:?} does not use the attention role, so \"this wants you\" has more than one \
             appearance"
        );
    }

    let value = rgb(Role::Attention);
    let distinct: HashSet<(u8, u8, u8)> = wants_notice
        .iter()
        .map(|what| match meaning(what).tint {
            Tint::Role(role) => rgb(role),
            Tint::SeasonalAccent => panic!("{what:?} wears the seasonal accent"),
        })
        .collect();
    assert_eq!(
        distinct,
        HashSet::from([value]),
        "a chip, a capped store's tag and an armed row resolved to {} different values",
        distinct.len()
    );

    // And there is no second notice-role waiting to be used for the next
    // surface: `role_name`'s exhaustive match is the guard, and only one of the
    // six is attention.
    let notice_roles: Vec<&str> = PALETTE_ROLES
        .iter()
        .filter(|r| {
            MEANINGS
                .iter()
                .filter(|m| m.tint == Tint::Role(**r))
                .any(|m| wants_notice.contains(&m.what))
        })
        .map(|r| role_name(*r))
        .collect();
    assert_eq!(
        notice_roles,
        vec!["attention"],
        "more than one role is carrying \"this wants you\""
    );
}

// ---------------------------------------------------------------------------
// Coats — reading the ring, and the colour arithmetic a blend is held to
// ---------------------------------------------------------------------------

/// The 24-bit value behind a hue of the ring.
fn hue_rgb(hue: Hue) -> (u8, u8, u8) {
    rgb_of(hue.color(), &format!("the {} hue", hue.name()))
}

/// OKLab, the space a blend's path is straight in. Björn Ottosson's
/// published matrices, applied to linear RGB.
fn oklab((r, g, b): (u8, u8, u8)) -> (f64, f64, f64) {
    let (lr, lg, lb) = (to_linear(r), to_linear(g), to_linear(b));
    let l = (0.412_221_470_8 * lr + 0.536_332_536_3 * lg + 0.051_445_992_9 * lb).cbrt();
    let m = (0.211_903_498_2 * lr + 0.680_699_545_1 * lg + 0.107_396_956_6 * lb).cbrt();
    let s = (0.088_302_461_9 * lr + 0.281_718_837_6 * lg + 0.629_978_700_5 * lb).cbrt();
    (
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    )
}

fn from_oklab((l, a, b): (f64, f64, f64)) -> (u8, u8, u8) {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let (l3, m3, s3) = (l_.powi(3), m_.powi(3), s_.powi(3));
    let r = 4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3;
    let g = -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3;
    let bb = -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701_0 * s3;
    (from_linear(r.clamp(0.0, 1.0)), from_linear(g.clamp(0.0, 1.0)), from_linear(bb.clamp(0.0, 1.0)))
}

/// The colour at character `i` of `n` on the piecewise-linear OKLab path
/// through `stops`, each segment an equal share of the span — the design's
/// own statement of a blended coat, computed here independently of the
/// module's `coat_colours` so the two can disagree.
fn expected_blend_colour(stops: &[Hue], i: usize, n: usize) -> Color {
    if n == 1 || stops.len() == 1 {
        return stops[0].color();
    }
    let t = i as f64 / (n - 1) as f64;
    let segments = (stops.len() - 1) as f64;
    let along = t * segments;
    let k = (along.floor() as usize).min(stops.len() - 2);
    let u = along - k as f64;
    blend(stops[k].color(), stops[k + 1].color(), u)
}

/// The clearance floor: the CIE76 difference every colour a coat can put on
/// a character keeps from every state colour, `dim` included. Twenty is
/// unmistakably two colours; eighteen leaves the ring room to be lively
/// without a name ever reading as a marker's colour.
const CLEARANCE_FLOOR: f64 = 18.0;

// ---------------------------------------------------------------------------
// TUI-THEME-002 — the coats hold the band and the floor too
// ---------------------------------------------------------------------------

/// Every colour a coat can put on a character — each hue of the ring and
/// every colour along a blended coat's path — reads on a near-black and a
/// near-white ground alike, and sits in the same mid band as the states.
///
/// @spec TUI-THEME-002
#[test]
fn every_coat_colour_clears_the_contrast_floor_against_both_grounds_and_keeps_the_band() {
    let mut values: Vec<(String, (u8, u8, u8))> =
        Hue::RING.iter().map(|h| (format!("the {} hue", h.name()), hue_rgb(*h))).collect();
    for r in ResourceId::ALL {
        let c = coat(r);
        if c.dressing != Dressing::Blended {
            continue;
        }
        for n in [2usize, 3, 5, 8, 13, 32] {
            for (i, colour) in coat_colours(c, n).into_iter().enumerate() {
                values.push((
                    format!("{r:?}'s blended coat, character {i} of {n}"),
                    rgb_of(colour, "a blended coat"),
                ));
            }
        }
    }

    for (name, value) in values {
        let on_black = contrast(value, NEAR_BLACK);
        assert!(
            on_black >= CONTRAST_FLOOR,
            "{name} (#{:02X}{:02X}{:02X}) is {on_black:.2}:1 against a near-black ground, below \
             the {CONTRAST_FLOOR}:1 floor.",
            value.0,
            value.1,
            value.2
        );
        let on_white = contrast(value, NEAR_WHITE);
        assert!(
            on_white >= CONTRAST_FLOOR,
            "{name} (#{:02X}{:02X}{:02X}) is {on_white:.2}:1 against a near-white ground, below \
             the {CONTRAST_FLOOR}:1 floor — a coat tuned for the dark ground alone.",
            value.0,
            value.1,
            value.2
        );
        let l = luminance(value);
        assert!(
            l > MID_BAND.0 && l < MID_BAND.1,
            "{name} has luminance {l:.3}, outside the mid band ({}, {})",
            MID_BAND.0,
            MID_BAND.1
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-009 — no coat is a season
// ---------------------------------------------------------------------------

/// The accent belongs to the chrome and a coat to a store; neither may be the
/// other, or a store would look like the year.
///
/// @spec TUI-THEME-009
#[test]
fn no_coat_colour_is_a_seasonal_accent() {
    for hue in Hue::RING {
        for season in Season::ALL {
            assert_ne!(
                hue_rgb(hue),
                accent(season),
                "the {} hue is the {season:?} accent; a store would wear the season",
                hue.name()
            );
        }
    }
    for r in ResourceId::ALL {
        let c = coat(r);
        for (i, colour) in coat_colours(c, 32).into_iter().enumerate() {
            let value = rgb_of(colour, "a coat");
            for season in Season::ALL {
                assert_ne!(
                    value,
                    accent(season),
                    "{r:?}'s coat, character {i} of 32, is the {season:?} accent"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-016 — every resource wears a coat of its own
// ---------------------------------------------------------------------------

/// Every resource has a coat; a solid one has one stop and a patched or
/// blended one two to four; no two consecutive stops match; no two resources
/// share a coat; and the clowder is diverse — each dressing is worn somewhere.
///
/// @spec TUI-THEME-016
#[test]
fn every_resource_wears_a_coat_of_its_own() {
    let mut worn: HashMap<Coat, ResourceId> = HashMap::new();
    let mut dressings = HashSet::new();
    for r in ResourceId::ALL {
        let c = coat(r);
        let n = c.stops.len();
        match c.dressing {
            Dressing::Solid => assert_eq!(
                n,
                1,
                "{r:?} wears a solid coat of {n} stops; a solid coat has exactly one"
            ),
            Dressing::Patched | Dressing::Blended => assert!(
                (2..=4).contains(&n),
                "{r:?} wears a {:?} coat of {n} stops; a patched or blended coat has two to four",
                c.dressing
            ),
        }
        for pair in c.stops.windows(2) {
            assert_ne!(
                pair[0],
                pair[1],
                "{r:?}'s coat repeats {} on consecutive stops — a solid coat in disguise",
                pair[0].name()
            );
        }
        if let Some(other) = worn.insert(c, r) {
            panic!("{r:?} and {other:?} wear the same coat, {c:?}; one coat is one resource");
        }
        dressings.insert(c.dressing);
    }
    for d in [Dressing::Solid, Dressing::Patched, Dressing::Blended] {
        assert!(dressings.contains(&d), "no resource wears a {d:?} coat; the clowder is diverse");
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-017 — how a coat lies across the letters
// ---------------------------------------------------------------------------

/// A solid coat is its one stop on every character.
///
/// @spec TUI-THEME-017
#[test]
fn a_solid_coat_is_its_one_stop_on_every_character() {
    for hue in Hue::RING {
        let stops: &'static [Hue] = Box::leak(Box::new([hue]));
        let c = Coat { stops, dressing: Dressing::Solid };
        for n in 1..=13 {
            for (i, colour) in coat_colours(c, n).into_iter().enumerate() {
                assert_eq!(
                    rgb_of(colour, "a solid coat"),
                    hue_rgb(hue),
                    "a solid {} coat on {n} characters wears something else on character {i}",
                    hue.name()
                );
            }
        }
    }
}

/// A patched coat divides the name into one run per stop, in order, every run
/// the quotient long and the first `remainder` runs one longer; a name shorter
/// than its stops wears the first stops, one character each.
///
/// @spec TUI-THEME-017
#[test]
fn a_patched_coat_lies_in_near_equal_runs_the_first_ones_a_character_longer() {
    const STOPS: [&[Hue]; 4] = [
        &[Hue::Poppy, Hue::Sky],
        &[Hue::Pond, Hue::Blush, Hue::Clover],
        &[Hue::Sky, Hue::Bluebell, Hue::Orchid, Hue::Blush],
        &[Hue::Pond, Hue::Blush, Hue::Pond, Hue::Blush],
    ];
    for stops in STOPS {
        let c = Coat { stops, dressing: Dressing::Patched };
        let k = stops.len();
        for n in 1..=20 {
            let drawn: Vec<(u8, u8, u8)> =
                coat_colours(c, n).into_iter().map(|col| rgb_of(col, "a patched coat")).collect();
            let expected: Vec<(u8, u8, u8)> = if n < k {
                stops[..n].iter().map(|h| hue_rgb(*h)).collect()
            } else {
                let (q, rem) = (n / k, n % k);
                stops
                    .iter()
                    .enumerate()
                    .flat_map(|(i, h)| std::iter::repeat_n(hue_rgb(*h), q + usize::from(i < rem)))
                    .collect()
            };
            assert_eq!(
                drawn,
                expected,
                "a {k}-stop patched coat on {n} characters: runs should be near-equal, the first \
                 {} one longer, in the coat's order",
                n % k
            );
        }
    }
}

/// A blend is a straight path in OKLab: its ends are the two stops, and its
/// midpoint is the OKLab midpoint, computed here independently.
///
/// @spec TUI-THEME-017
#[test]
fn a_blend_is_a_straight_path_in_oklab_between_its_two_colours() {
    for (i, a) in Hue::RING.iter().enumerate() {
        for b in Hue::RING.iter().skip(i + 1) {
            let (ca, cb) = (a.color(), b.color());
            assert_eq!(rgb_of(blend(ca, cb, 0.0), "a blend"), hue_rgb(*a), "a blend at 0 is its first colour");
            assert_eq!(rgb_of(blend(ca, cb, 1.0), "a blend"), hue_rgb(*b), "a blend at 1 is its second colour");
            let (la, lb) = (oklab(hue_rgb(*a)), oklab(hue_rgb(*b)));
            let mid = from_oklab(((la.0 + lb.0) / 2.0, (la.1 + lb.1) / 2.0, (la.2 + lb.2) / 2.0));
            let got = rgb_of(blend(ca, cb, 0.5), "a blend");
            let off = |x: u8, y: u8| (i16::from(x) - i16::from(y)).unsigned_abs();
            assert!(
                off(got.0, mid.0) <= 1 && off(got.1, mid.1) <= 1 && off(got.2, mid.2) <= 1,
                "the midpoint of {} and {} is #{:02X}{:02X}{:02X}, not the OKLab midpoint #{:02X}{:02X}{:02X}",
                a.name(),
                b.name(),
                got.0,
                got.1,
                got.2,
                mid.0,
                mid.1,
                mid.2
            );
        }
    }
}

/// A blended coat runs from its first stop on the first character to its last
/// on the last, positions evenly spaced, each segment an equal share — so a
/// five-character name on three stops lands its middle character exactly on
/// the middle stop, and a two-character name never wears it at all.
///
/// @spec TUI-THEME-017
#[test]
fn a_blended_coat_runs_from_first_stop_to_last_with_each_segment_an_equal_share() {
    const THREE: &[Hue] = &[Hue::Sky, Hue::Bluebell, Hue::Orchid];
    const TWO: &[Hue] = &[Hue::Poppy, Hue::Sky];
    let three = Coat { stops: THREE, dressing: Dressing::Blended };
    let two = Coat { stops: TWO, dressing: Dressing::Blended };
    let value = |c: Color| rgb_of(c, "a blended coat");

    let one = coat_colours(three, 1);
    assert_eq!(value(one[0]), hue_rgb(Hue::Sky), "a one-character name wears the first stop");

    let pair = coat_colours(three, 2);
    assert_eq!(value(pair[0]), hue_rgb(Hue::Sky));
    assert_eq!(value(pair[1]), hue_rgb(Hue::Orchid), "a two-character name ends on the last stop");

    let five = coat_colours(three, 5);
    assert_eq!(value(five[0]), hue_rgb(Hue::Sky));
    assert_eq!(value(five[2]), hue_rgb(Hue::Bluebell), "the middle character lands on the middle stop");
    assert_eq!(value(five[4]), hue_rgb(Hue::Orchid));
    assert_eq!(value(five[1]), value(blend(Hue::Sky.color(), Hue::Bluebell.color(), 0.5)));
    assert_eq!(value(five[3]), value(blend(Hue::Bluebell.color(), Hue::Orchid.color(), 0.5)));

    let nine = coat_colours(two, 9);
    for (i, colour) in nine.iter().enumerate() {
        assert_eq!(
            value(*colour),
            value(blend(Hue::Poppy.color(), Hue::Sky.color(), i as f64 / 8.0)),
            "character {i} of 9 on a two-stop blend sits at {}/8 of the path",
            i
        );
    }
}

// ---------------------------------------------------------------------------
// TUI-THEME-018 — the ring keeps its distance from the states
// ---------------------------------------------------------------------------

/// Every hue of the ring, and every colour along every blended coat's path
/// that a name of up to thirty-two characters can land on, keeps the
/// clearance floor from each of the five state colours.
///
/// @spec TUI-THEME-018
#[test]
fn every_coat_colour_keeps_the_clearance_floor_from_every_state_colour() {
    let states: Vec<(&str, (u8, u8, u8))> =
        PALETTE_ROLES.iter().map(|r| (role_name(*r), rgb(*r))).collect();
    let check = |what: &str, value: (u8, u8, u8)| {
        for (name, state) in &states {
            let d = delta_e(value, *state);
            assert!(
                d >= CLEARANCE_FLOOR,
                "{what} (#{:02X}{:02X}{:02X}) is ΔE {d:.1} from `{name}`, under the \
                 {CLEARANCE_FLOOR} clearance floor: a name could read as that state's colour",
                value.0,
                value.1,
                value.2
            );
        }
    };
    for hue in Hue::RING {
        check(&format!("the {} hue", hue.name()), hue_rgb(hue));
    }
    for r in ResourceId::ALL {
        let c = coat(r);
        if c.dressing != Dressing::Blended {
            continue;
        }
        for n in 2..=32 {
            for (i, colour) in coat_colours(c, n).into_iter().enumerate() {
                check(&format!("{r:?}'s blended coat, character {i} of {n}"), rgb_of(colour, "a blend"));
            }
        }
    }
}

/// Which pairs of stops a blend may run between, derived by measuring every
/// pair's path, and every blended coat in the table held to the pairs that
/// pass — the test that names the blendable pairs.
///
/// @spec TUI-THEME-018
#[test]
fn a_blend_runs_only_between_stops_whose_path_keeps_the_clearance() {
    let mut allowed: Vec<(Hue, Hue)> = Vec::new();
    let mut forbidden: Vec<(Hue, Hue)> = Vec::new();
    for (i, a) in Hue::RING.iter().enumerate() {
        for b in Hue::RING.iter().skip(i + 1) {
            let clear = (0..=64).all(|k| {
                let colour = rgb_of(blend(a.color(), b.color(), f64::from(k) / 64.0), "a blend");
                PALETTE_ROLES.iter().all(|role| delta_e(colour, rgb(*role)) >= CLEARANCE_FLOOR)
            });
            if clear {
                allowed.push((*a, *b));
            } else {
                forbidden.push((*a, *b));
            }
        }
    }
    assert!(
        !allowed.is_empty() && !forbidden.is_empty(),
        "fixture: the ring should have both blendable and unblendable pairs; allowed {allowed:?}, \
         forbidden {forbidden:?}"
    );
    let names = |pairs: &[(Hue, Hue)]| -> Vec<String> {
        pairs.iter().map(|(a, b)| format!("{}–{}", a.name(), b.name())).collect()
    };
    for r in ResourceId::ALL {
        let c = coat(r);
        if c.dressing != Dressing::Blended {
            continue;
        }
        for pair in c.stops.windows(2) {
            let ok = allowed
                .iter()
                .any(|(a, b)| (*a, *b) == (pair[0], pair[1]) || (*a, *b) == (pair[1], pair[0]));
            assert!(
                ok,
                "{r:?} blends {} to {}, whose path passes within the clearance floor of a state \
                 colour. The blendable pairs are {:?}",
                pair[0].name(),
                pair[1].name(),
                names(&allowed)
            );
        }
    }
}
