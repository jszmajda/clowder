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
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .expect("ALL is total")
    }

    /// The digit that jumps here. @spec TUI-NAV-005
    pub fn digit(self) -> char {
        let ix = self.index();
        if ix == Self::ALL.len() - 1 {
            '0'
        } else {
            (b'1' + ix as u8) as char
        }
    }

    /// The section a digit jumps to, if any. @spec TUI-NAV-005
    pub fn from_digit(c: char) -> Option<Section> {
        Self::ALL.iter().copied().find(|s| s.digit() == c)
    }

    /// The tab label. Section names are content, so a tab exists only while
    /// the section lists a selectable row, or while the player is standing in
    /// it. @spec TUI-NAV-006, TUI-NAV-085
    pub fn title(self) -> &'static str {
        match self {
            Section::Meadow => "Meadow",
            Section::Clowder => "Clowder",
            Section::Ideas => "Ideas",
            Section::Crafts => "Crafts",
            Section::Sunbeams => "Sunbeams",
            Section::Wonders => "Wonders",
            Section::Friends => "Friends",
            Section::Sky => "Sky",
            Section::Dreams => "Dreams",
            Section::Legacy => "Legacy",
        }
    }
}
