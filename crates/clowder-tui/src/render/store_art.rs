//! A store drawn small: the fixed picture a stores row's card opens with, as a
//! kitten's card opens with its portrait. A fact about presentation, so the
//! table lives here beside the poses, as the coats live in the theme.

use clowder_core::ResourceId;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::theme::{coat, coat_colours};

/// A blank that wrapping never trims or breaks at, so a row keeps its shape.
const BLANK: char = '\u{a0}';

/// The mark of a painted cell: it wears the store's coat, and every other mark
/// is outline.
const PAINTED: char = '\u{2588}';

/// The store's drawing, one row to an entry: at most five rows of at most
/// sixteen columns, chosen to suit the store and shared with no other.
/// @spec TUI-RENDER-031
pub fn drawing(r: ResourceId) -> &'static [&'static str] {
    use ResourceId::*;
    match r {
        Catnip => &["   ██  ██", "  ███\\/███", "    ██|██", "      |"],
        Twigs => &[" █\\    /█", "   \\__/", "  ██/\\██", "   /  \\"],
        Fluff => &["   .-██-.", "  (██████)", " (████████)", "  '-████-'"],
        Curiosity => &["   .███.", "   '  ██", "     ██", "     ::", "     ██"],
        Pebbles => &["     .--.", "  .-(████)-.", " (███)(█████)", "  '-'  '---'"],
        Buttons => &["  .-----.", " /██o█o██\\", " \\██o█o██/", "  '-----'"],
        Embers => &["    .  '  .", "   '  █  '", "   .█████.", "  (███████)"],
        Shinies => &["     .", "   \\ █ /", "  -█████-", "   / █ \\", "     '"],
        Dew => &["     .", "    /█\\", "   /███\\", "   \\███/", "    '-'"],
        Moonstone => &[
            "    _____",
            "   /█████\\",
            "  |██( ███|",
            "   \\█████/",
            "    '---'",
        ],
        Stardust => &["  █  .   *", "    .  █   .", "  *   █   .", "   .   *  █"],
        Hushfelt => &[
            " .--------.",
            " (████████) z",
            " (████████)  z",
            " '--------'",
        ],
        Pep => &["    __", "   /█/", "  /█/_", " /___█/", "    /█/"],
        Charm => &[
            "  .-. .-.",
            " (███V███)",
            "  \\█████/",
            "   \\███/",
            "    \\█/",
        ],
        Warmth => &[
            "   \\  |  /",
            "    .███.",
            " --(█████)--",
            "    '███'",
            "   /  |  \\",
        ],
        Skymaps => &[
            "  ___________",
            " (_*___.___*_)",
            "  |█ . █ * █|",
            "  |_█__*__█_|",
        ],
        Feathers => &["       ___", "    .-'███/", "  .'████/", " /██.-'", "/'"],
        Bells => &["     _", "    /█\\", "   /███\\", "  /█████\\", "  '--o--'"],
        Mint => &[
            "    .██.",
            "   /████\\",
            "  (██||██)",
            "   \\█||█/",
            "     ||",
        ],
        Unicorns => &["     \\", "   .-\\█.", "  /███ o\\", " /███.--'", " |██/"],
        Rainbows => &[
            "    .-----.",
            "  .'███████'.",
            " /███.---.███\\",
            " |██/     \\██|",
        ],
        Prisms => &[
            "     /\\",
            "    /██\\  ---",
            "   /████\\ ---",
            "  /______\\---",
        ],
        Starshards => &[
            "    /\\",
            "   /██\\   /\\",
            "   \\██/  /█/",
            "    \\/   \\/",
        ],
        Echoes => &["  (( (█) ))", " ((( (█) )))", "  (( (█) ))"],
        Dreamcorns => &["   /|   z", "  /█|  z", " /██|_", "(█████)"],
        JarsOfPurrs => &[
            "   _______",
            "  [_______]",
            "  |███████|",
            "  |██ ~ ██|",
            "  '-------'",
        ],
        Reverie => &["   .--.  .", "  (████).  .", " (███████)", "  '-~--~-'"],
        Wands => &["█\\", " █\\____", "  █    '.", "         :", "        ███"],
        Tiles => &[
            " +---+---+---+",
            " |███|   |███|",
            " +---+---+---+",
            " |   |███|   |",
            " +---+---+---+",
        ],
        Badges => &[
            "    .---.",
            "   (█ * █)",
            "    '---'",
            "    /█ █\\",
            "   /█/ \\█\\",
        ],
        Brass => &[
            "     ______",
            "    /█████/|",
            "   /█████/ |",
            "  /_____/ /",
            "  |█████|/",
        ],
        Cogs => &[
            "   _/|_|\\_",
            "  |██ _ ██|",
            " -|█ (_) █|-",
            "  |██___██|",
            "    \\| |/",
        ],
        Moonsteel => &[
            "    _______",
            "   /█████/|",
            "  /██(██/ /",
            " /_____/ /",
            " '-----'",
        ],
        Lattices => &[" \\█/ \\█/ \\█/", "  X   X   X", " /█\\ /█\\ /█\\"],
        Pavers => &[
            "  .----.----.",
            "  |████|████|",
            "  '-.--'--.-'",
            "    |█████|",
            "    '-----'",
        ],
        Quills => &["        ,██", "      ,███'", "    ,███'", "   /█'", "  /"],
        Stories => &[
            "  ____ ____",
            " /████V████\\",
            " |████|████|",
            " |████|████|",
            " '----'----'",
        ],
        Anthologies => &[
            "  _________",
            " [█████████]",
            " [_________]",
            " [█████████]",
            " [_________]",
        ],
        Schemes => &[
            " .----------.",
            " | o ~> x   |",
            " | █ ███ █  |",
            " '----------'",
        ],
        Cairns => &["     .--.", "    (████)", "   (██████)", "  (████████)"],
        Kites => &[
            "     /\\",
            "    /██\\",
            "   <████>",
            "    \\██/",
            "     \\/~~",
        ],
        Balloons => &[
            "    .--.",
            "   /████\\",
            "   \\████/",
            "    '\\/'",
            "     |",
        ],
        Moonmilk => &["     _", "    |_|", "   /███\\", "   |█(█|", "   '---'"],
        Dreamweave => &[
            " ~█~█~█~█~█~",
            " █~█~█~█~█~█",
            " ~█~█~█~█~█~",
            " █~█~█~█~█~█",
        ],
        Cometsilk => &["            .█", "       .-███'", "  .-████-'", " ███-'"],
        Cleverness => &[
            "   .-~~-.",
            "  /█/██\\█\\  !",
            "  |█|██|█|",
            "  \\█\\██/█/",
            "   '-..-'",
        ],
    }
}

/// The drawing's rows as the inspector draws them: each whole from its left
/// edge, its painted cells in the colour of their column under the store's
/// coat, laid across the drawing's width, every other mark in the inherited
/// foreground. @spec TUI-RENDER-031, TUI-THEME-031
pub(crate) fn drawing_lines(r: ResourceId) -> Vec<Line<'static>> {
    let rows = drawing(r);
    let widest = rows
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    let colours = coat_colours(coat(r), widest);
    rows.iter()
        .map(|row| {
            let spans: Vec<Span<'static>> = row
                .chars()
                .enumerate()
                .map(|(col, ch)| match ch {
                    PAINTED => Span::styled(ch.to_string(), Style::default().fg(colours[col])),
                    ' ' => Span::raw(BLANK.to_string()),
                    _ => Span::raw(ch.to_string()),
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}
