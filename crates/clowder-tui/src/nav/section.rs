//! Sections and their digits. The map is assigned at design time and never
//! renumbered: a section that unlocks late slots into its reserved digit, so a
//! key the player already knows never changes meaning.
//! @spec TUI-NAV-005, TUI-NAV-006

/// The ten sections, in digit order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    Meadow,
    Clowder,
    Ideas,
    Crafts,
    Sunbeams,
    Wonders,
    Friends,
    Sky,
    Dreams,
    Legacy,
}

impl Section {
    pub const ALL: [Section; 10] = [
        Section::Meadow,
        Section::Clowder,
        Section::Ideas,
        Section::Crafts,
        Section::Sunbeams,
        Section::Wonders,
        Section::Friends,
        Section::Sky,
        Section::Dreams,
        Section::Legacy,
    ];

    /// Position in the fixed digit map, 0-based. `Meadow` is `1`, `Legacy` is `0`.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).expect("ALL is total")
    }

    /// The digit that jumps here. @spec TUI-NAV-005
    pub fn digit(self) -> char {
        todo!("TUI-NAV-005: the static digit map, 1-9 then 0")
    }

    /// The section a digit jumps to, if any. @spec TUI-NAV-005
    pub fn from_digit(_c: char) -> Option<Section> {
        todo!("TUI-NAV-005: inverse of the static digit map")
    }

    /// The tab label. Section names are content, so a tab exists only once one
    /// of its rows is revealed. @spec TUI-NAV-006
    pub fn title(self) -> &'static str {
        todo!("TUI-NAV-006: section titles")
    }
}
