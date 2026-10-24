//! Colour, and what it is allowed to mean.
//!
//! Colour has two jobs here and they never share a place. *State* is worn by
//! marks — glyphs, tags, chips, whole lines — in five colours a player learns
//! once and reads everywhere. *Identity* is worn by a resource's name as its
//! coat, fixed for the life of the game and chosen to suit the name; a coat
//! says nothing the name does not. Every meaning colour carries is carried
//! first by a glyph, a word, or a fill — turn all colour off and the game is
//! still completely playable. One fenced picture is neither: a kitten's
//! portrait is painted in its own fur, which depicts and means nothing
//! (`fur`). @spec TUI-THEME-003, TUI-THEME-010

pub mod fur;

use clowder_core::{ResourceId, Season};
use ratatui::style::Color;

/// The six roles. There is no seventh: every added colour dilutes the signal
/// that a state colour *is* meaning. @spec TUI-THEME-004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The terminal's own foreground. Ordinary text takes no colour at all.
    Inherited,
    /// Revealed but not actionable, owned and done, flavour, chrome rules.
    /// @spec TUI-THEME-014
    Dim,
    /// Affordable now.
    Ready,
    /// Partially affordable.
    Partial,
    /// Wants notice — a chip, a capped store, an armed row. @spec TUI-THEME-015
    Attention,
    /// A moment worth remembering, and newly revealed content.
    Milestone,
}

impl Role {
    /// `None` means "use the terminal's own foreground" — clowder paints no
    /// ground, so light and dark both work with nothing detected.
    /// @spec TUI-THEME-001, TUI-THEME-002
    pub fn color(self) -> Option<Color> {
        match self {
            Role::Inherited => None,
            Role::Dim => Some(Color::Rgb(0x7C, 0x78, 0x73)),
            Role::Ready => Some(Color::Rgb(0x3E, 0x8E, 0x5A)),
            Role::Partial => Some(Color::Rgb(0xA8, 0x80, 0x2F)),
            Role::Attention => Some(Color::Rgb(0xC0, 0x66, 0x3A)),
            Role::Milestone => Some(Color::Rgb(0x7E, 0x6B, 0xA8)),
        }
    }
}

/// The chrome accent for a season. Confined to inert chrome — the header rule
/// and the pane divider — so no colour is ever ambiguous between a state and a
/// season, or between a season and a coat. @spec TUI-THEME-008, TUI-THEME-009
pub fn seasonal_accent(season: Season) -> Color {
    match season {
        Season::Spring => Color::Rgb(0x7F, 0xA0, 0x5A),
        Season::Summer => Color::Rgb(0xC9, 0xA2, 0x27),
        Season::Autumn => Color::Rgb(0xB5, 0x71, 0x3C),
        Season::Winter => Color::Rgb(0x6E, 0x8B, 0xA8),
    }
}

/// Whether the terminal advertises truecolor. A tmux that has not been
/// configured for RGB approximates every value silently, and the player sees a
/// muddy game they will reasonably assume just looks like that.
/// @spec TUI-THEME-012
pub fn truecolor_advertised() -> bool {
    match std::env::var("COLORTERM") {
        Ok(value) => {
            let value = value.to_ascii_lowercase();
            value == "truecolor" || value == "24bit"
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Coats
// ---------------------------------------------------------------------------

/// A hue of the ring the coats are cut from. Seven, every one in the same
/// mid-luminance band as the state colours with the chroma turned up, and no
/// yellow: in the band a yellow is the amber of `partial`. The ring keeps a
/// measured distance from every state colour. @spec TUI-THEME-004, TUI-THEME-018
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hue {
    Poppy,
    Clover,
    Pond,
    Sky,
    Bluebell,
    Orchid,
    Blush,
}

impl Hue {
    /// The ring, in hue order.
    pub const RING: [Hue; 7] = [
        Hue::Poppy,
        Hue::Clover,
        Hue::Pond,
        Hue::Sky,
        Hue::Bluebell,
        Hue::Orchid,
        Hue::Blush,
    ];

    /// The hue's one value. @spec TUI-THEME-002, TUI-THEME-016
    pub fn color(self) -> Color {
        match self {
            Hue::Poppy => Color::Rgb(0xD9, 0x53, 0x4F),
            Hue::Clover => Color::Rgb(0x6E, 0x9E, 0x2E),
            Hue::Pond => Color::Rgb(0x2E, 0x9C, 0x8A),
            Hue::Sky => Color::Rgb(0x3F, 0x96, 0xD6),
            Hue::Bluebell => Color::Rgb(0x6A, 0x7F, 0xE0),
            Hue::Orchid => Color::Rgb(0xB4, 0x5F, 0xC8),
            Hue::Blush => Color::Rgb(0xD4, 0x58, 0x8E),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Hue::Poppy => "poppy",
            Hue::Clover => "clover",
            Hue::Pond => "pond",
            Hue::Sky => "sky",
            Hue::Bluebell => "bluebell",
            Hue::Orchid => "orchid",
            Hue::Blush => "blush",
        }
    }
}

/// How a coat's stops lie across the characters of a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dressing {
    /// One stop, worn by every character.
    Solid,
    /// One run of characters per stop, in order, near-equal in length.
    Patched,
    /// Each character the colour at its position along the path through the
    /// stops.
    Blended,
}

/// A resource's coat: its stops, in order, and its dressing. A solid coat has
/// one stop; a patched or blended coat has two to four, no two consecutive
/// the same hue. One coat is one resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coat {
    pub stops: &'static [Hue],
    pub dressing: Dressing,
}

const fn solid(stops: &'static [Hue]) -> Coat {
    Coat {
        stops,
        dressing: Dressing::Solid,
    }
}

const fn patched(stops: &'static [Hue]) -> Coat {
    Coat {
        stops,
        dressing: Dressing::Patched,
    }
}

const fn blended(stops: &'static [Hue]) -> Coat {
    Coat {
        stops,
        dressing: Dressing::Blended,
    }
}

/// The coat a resource wears, chosen to suit its name rather than hashed from
/// it, and fixed for the life of the game: a function of which resource and
/// of nothing else. A blended coat runs only between stops whose path keeps
/// clear of every state colour, which the theme's tests measure.
/// @spec TUI-THEME-003, TUI-THEME-016, TUI-THEME-018
pub fn coat(r: ResourceId) -> Coat {
    use Hue::*;
    match r {
        ResourceId::Catnip => solid(&[Clover]),
        ResourceId::Twigs => patched(&[Clover, Poppy]),
        ResourceId::Fluff => solid(&[Blush]),
        ResourceId::Curiosity => blended(&[Sky, Bluebell]),
        ResourceId::Pebbles => patched(&[Pond, Sky]),
        ResourceId::Buttons => patched(&[Poppy, Bluebell]),
        ResourceId::Embers => blended(&[Poppy, Blush]),
        ResourceId::Shinies => patched(&[Sky, Blush, Orchid]),
        ResourceId::Dew => solid(&[Pond]),
        ResourceId::Moonstone => blended(&[Bluebell, Orchid]),
        ResourceId::Stardust => patched(&[Sky, Blush]),
        ResourceId::Hushfelt => solid(&[Bluebell]),
        ResourceId::Pep => solid(&[Poppy]),
        ResourceId::Charm => blended(&[Orchid, Blush]),
        ResourceId::Warmth => patched(&[Blush, Poppy]),
        ResourceId::Skymaps => blended(&[Pond, Sky, Bluebell]),
        ResourceId::Feathers => patched(&[Sky, Pond]),
        ResourceId::Bells => patched(&[Bluebell, Sky]),
        ResourceId::Mint => patched(&[Pond, Clover]),
        ResourceId::Unicorns => blended(&[Bluebell, Orchid, Blush]),
        ResourceId::Rainbows => patched(&[Poppy, Clover, Sky, Orchid]),
        ResourceId::Prisms => blended(&[Sky, Bluebell, Orchid]),
        ResourceId::Starshards => patched(&[Sky, Bluebell]),
        ResourceId::Echoes => blended(&[Pond, Bluebell]),
        ResourceId::Dreamcorns => patched(&[Orchid, Blush, Bluebell]),
        ResourceId::JarsOfPurrs => patched(&[Bluebell, Blush]),
        ResourceId::Reverie => blended(&[Sky, Bluebell, Orchid, Blush]),
        ResourceId::Wands => patched(&[Bluebell, Orchid]),
        ResourceId::Tiles => patched(&[Pond, Blush, Pond, Blush]),
        ResourceId::Badges => patched(&[Poppy, Sky]),
        ResourceId::Brass => patched(&[Poppy, Blush]),
        ResourceId::Cogs => patched(&[Bluebell, Poppy]),
        ResourceId::Moonsteel => patched(&[Bluebell, Sky, Bluebell]),
        ResourceId::Lattices => patched(&[Clover, Sky]),
        ResourceId::Pavers => solid(&[Sky]),
        ResourceId::Quills => patched(&[Sky, Orchid]),
        ResourceId::Stories => blended(&[Poppy, Sky]),
        ResourceId::Anthologies => patched(&[Orchid, Poppy, Sky]),
        ResourceId::Schemes => patched(&[Orchid, Clover]),
        ResourceId::Cairns => patched(&[Pond, Bluebell]),
        ResourceId::Kites => blended(&[Blush, Poppy, Sky]),
        ResourceId::Balloons => patched(&[Poppy, Sky, Blush]),
        ResourceId::Moonmilk => patched(&[Blush, Bluebell]),
        ResourceId::Dreamweave => blended(&[Orchid, Blush, Poppy, Sky]),
        ResourceId::Cometsilk => blended(&[Pond, Sky]),
        ResourceId::Cleverness => solid(&[Orchid]),
    }
}

/// The colour each of a name's `chars` characters wears under `coat`, by its
/// dressing: a solid coat's one stop on every character; a patched coat's
/// stops as consecutive runs, every run the quotient long and the first
/// `remainder` runs one longer, a name shorter than its stops wearing the
/// first stops one character each; a blended coat's colour at each evenly
/// spaced position along the piecewise-linear OKLab path through its stops,
/// each segment an equal share, a one-character name wearing the first stop.
/// A space counts as a character. @spec TUI-THEME-017
pub fn coat_colours(coat: Coat, chars: usize) -> Vec<Color> {
    let stops = coat.stops;
    if stops.is_empty() || chars == 0 {
        return Vec::new();
    }
    match coat.dressing {
        Dressing::Solid => vec![stops[0].color(); chars],
        Dressing::Patched => {
            let k = stops.len();
            let (quotient, remainder) = (chars / k, chars % k);
            let mut out = Vec::with_capacity(chars);
            for (i, stop) in stops.iter().enumerate() {
                let run = quotient + usize::from(i < remainder);
                out.extend(std::iter::repeat_n(stop.color(), run));
            }
            out
        }
        Dressing::Blended => {
            if chars == 1 || stops.len() == 1 {
                return vec![stops[0].color(); chars];
            }
            let segments = (stops.len() - 1) as f64;
            (0..chars)
                .map(|i| {
                    let t = i as f64 / (chars - 1) as f64;
                    let along = t * segments;
                    let k = (along.floor() as usize).min(stops.len() - 2);
                    let u = along - k as f64;
                    blend(stops[k].color(), stops[k + 1].color(), u)
                })
                .collect()
        }
    }
}

/// The colour `t` of the way from `a` to `b` along a straight line in OKLab,
/// so a blend between two hues is short and even and never sags through
/// grey. At the ends it is exactly `a` and exactly `b`. @spec TUI-THEME-017
pub fn blend(a: Color, b: Color, t: f64) -> Color {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let (Some(a), Some(b)) = (rgb(a), rgb(b)) else {
        // Only 24-bit values blend; anything else has no known position.
        return if t < 0.5 { a } else { b };
    };
    let (la, lb) = (oklab(a), oklab(b));
    from_oklab((
        la.0 + (lb.0 - la.0) * t,
        la.1 + (lb.1 - la.1) * t,
        la.2 + (lb.2 - la.2) * t,
    ))
}

fn rgb(colour: Color) -> Option<(u8, u8, u8)> {
    match colour {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        _ => None,
    }
}

fn to_linear(component: u8) -> f64 {
    let c = f64::from(component) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(value: f64) -> u8 {
    let v = value.clamp(0.0, 1.0);
    let c = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Björn Ottosson's OKLab, from linear RGB.
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

fn from_oklab((l, a, b): (f64, f64, f64)) -> Color {
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let (l3, m3, s3) = (l_.powi(3), m_.powi(3), s_.powi(3));
    let r = 4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3;
    let g = -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3;
    let bb = -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701_0 * s3;
    Color::Rgb(from_linear(r), from_linear(g), from_linear(bb))
}
