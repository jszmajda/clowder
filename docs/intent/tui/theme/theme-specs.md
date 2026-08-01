# Theme Specs (TUI-THEME)

## The Ground

- [x] **TUI-THEME-001**: The system shall paint no background of its own, rendering ordinary text in the terminal's inherited foreground so the game is legible in a light terminal and a dark one without detecting which it is in.
- [x] **TUI-THEME-002**: The system shall keep every palette value within a mid-luminance band that clears the contrast floor against both a near-black and a near-white ground.

## What Colour Means

- [x] **TUI-THEME-003**: The system shall use colour only to convey state, never identity — a resource, structure, or idea shall take its colour from its current state and never from which item it is.
- [x] **TUI-THEME-004**: The system shall define exactly six palette roles — dim, ready, partial, attention, milestone, and the inherited foreground — and shall render no colour outside that set and the seasonal chrome accent.
- [x] **TUI-THEME-005**: The system shall render mood with no colour at all, the mood word being its only channel.
- [ ] **TUI-THEME-006**: The system shall convey focus through the cursor glyph and, where more emphasis is needed, reverse video — never through a palette colour.
- [ ] **TUI-THEME-007**: The system shall render reverse-video emphasis by inverting the terminal's own foreground and background pair rather than a palette colour, so emphasis works in either polarity.

## The Seasonal Accent

- [ ] **TUI-THEME-008**: The system shall tint the inert chrome — the header rule and the pane divider — with an accent chosen by the current season, so the pane's colour tracks the year.
- [x] **TUI-THEME-009**: The system shall apply the seasonal accent to no element that carries state, so no colour is ever ambiguous between a state and a season.

## Colour as the Second Channel

- [x] **TUI-THEME-010**: The system shall pair every meaning colour carries with a glyph, a word, or a fill that carries the same meaning, so that rendering with all colour removed loses no information the player needs.
- [x] **TUI-THEME-011**: The system shall keep the glyphs of the states whose colours are nearest in hue — affordable, partially affordable, and attention — mutually distinct, so a red-green deficiency that compresses those hues never collapses two states into one appearance.

## Terminal Capability

- [x] **TUI-THEME-012**: When the terminal does not advertise truecolor support, the system shall say so once at startup in plain words naming what to set, and shall then render unchanged rather than substituting a reduced palette.
- [x] **TUI-THEME-013**: The system shall emit the truecolor notice before entering the alternate screen, alongside the other startup messages, so it is still on the player's scrollback after the game exits rather than being erased with the alternate screen.
- [ ] **TUI-THEME-014**: The system shall render an owned or completed one-shot in dim, so a finished row recedes rather than competing with the rows that can still be acted on.
- [ ] **TUI-THEME-015**: The system shall use attention for every element that wants notice — a chip, a capped store's tag, an armed row — the role meaning "this wants you" wherever it appears rather than being subdivided per surface.
