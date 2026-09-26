# Theme Specs (TUI-THEME)

## The Ground

- [x] **TUI-THEME-001**: The system shall paint no background of its own, rendering ordinary text in the terminal's inherited foreground so the game is legible in a light terminal and a dark one without detecting which it is in.
- [x] **TUI-THEME-002**: The system shall keep every palette value within a mid-luminance band that clears the contrast floor against both a near-black and a near-white ground.

## What Colour Means

- [x] **TUI-THEME-003**: The system shall use colour only to convey state, never identity — a resource, structure, or idea shall take its colour from its current state and never from which item it is.
- [x] **TUI-THEME-004**: The system shall define exactly six palette roles — dim, ready, partial, attention, milestone, and the inherited foreground — and shall render no colour outside that set and the seasonal chrome accent.
- [x] **TUI-THEME-005**: The system shall render mood with no colour at all, the mood word being its only channel.
- [x] **TUI-THEME-006**: The system shall convey focus through the cursor glyph and, where more emphasis is needed, reverse video — never through a palette colour.
- [x] **TUI-THEME-007**: The system shall render reverse-video emphasis by inverting the terminal's own foreground and background pair rather than a palette colour, so emphasis works in either polarity.

## The Seasonal Accent

- [x] **TUI-THEME-008**: The system shall draw the inert chrome — the header rule and every pane divider (TUI-RENDER-016), and nothing else — in the accent of the season held by the snapshot the frame draws, changing on the same frame as the header's season word with no blend across the turn, and on the header rule colouring the underline alone so every character above it keeps its own colour, so the pane's chrome tracks the year.
- [x] **TUI-THEME-009**: The system shall apply the seasonal accent to no element that carries state, so no colour is ever ambiguous between a state and a season.

## Colour as the Second Channel

- [x] **TUI-THEME-010**: The system shall pair every meaning colour carries with a glyph, a word, or a fill that carries the same meaning, so that rendering with all colour removed loses no information the player needs.
- [x] **TUI-THEME-011**: The system shall give each of the three states whose colours are nearest in hue — affordable, partially affordable, and attention — a non-colour channel neither of the other two uses: affordable the marker `●`, partially affordable the marker `◐`, and attention each chip's own glyph (TUI-NAV-020), a word — the `full` tag — or an armed row's expansion beneath itself (TUI-NAV-077), never `●` or `◐`; and, among `ready`, `partial`, and `attention`, shall draw each of those glyphs only in its own state's colour — `●` never in `partial` or `attention`, `◐` never in `ready` or `attention`, a chip's glyph never in `ready` or `partial` — an armed row wearing `attention` on its expansion and nowhere on its own line, whose marker keeps its availability colour; both whatever values the palette holds, so a red-green deficiency that compresses those hues never collapses two states into one appearance.

## Terminal Capability

- [x] **TUI-THEME-012**: When the terminal does not advertise truecolor support, the system shall say so once at startup in plain words naming what to set, and shall then render unchanged rather than substituting a reduced palette.
- [x] **TUI-THEME-013**: The system shall emit the truecolor notice before entering the alternate screen, alongside the other startup messages, so it is still on the player's scrollback after the game exits rather than being erased with the alternate screen.
- [x] **TUI-THEME-014**: The system shall render an owned or completed one-shot in dim, so a finished row recedes rather than competing with the rows that can still be acted on.
- [x] **TUI-THEME-015**: The system shall draw every element that wants notice in the one `attention` value, and nothing else in it — each attention chip whole, its glyph, any count, and its word, at every width step the header takes and the `★N new` chip included, but not the blank columns between chips or the header's facts; the `full` tag on a capped store, but not the store's name, figures, fill bar, or rate; and an armed row's expansion wherever it is drawn, but not the row's own line or the hint bar — so the role means "this wants you" wherever it appears, is never subdivided per surface, and is lent to nothing that does not want notice, a footer note being drawn in the inherited foreground.
