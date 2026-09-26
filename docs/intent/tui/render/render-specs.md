# Render Specs (TUI-RENDER)

## The Frame

- [x] **TUI-RENDER-001**: The system shall derive every frame as a pure function of the game-state snapshot, the shell's view state, and the terminal size, caching nothing between frames and mirroring no value the core owns.
- [x] **TUI-RENDER-002**: The system shall lay the frame out as a header row, a tab-bar row, a body, a ticker of one or more rows, and a hint-bar row, with the body taking whatever height the chrome leaves so the regions always sum to the terminal's height.
- [x] **TUI-RENDER-003**: When an offline banner is present, the system shall take its row from the body rather than adding a row to the frame.
- [x] **TUI-RENDER-004**: Where the terminal is narrower than 80 columns or shorter than 24 rows, the system shall draw only the too-small card, while the simulation continues to advance.
- [x] **TUI-RENDER-016**: The system shall draw the header rule as an underline across every cell of the header row at every width step the header takes, changing no character's colour; and shall draw a pane divider — one column of the single-width `│`, running the full height of the panes it separates and no other row — at each boundary between side-by-side panes, taken from the list's space on its edge facing the sidebar and, where the inspector is a pane of its own, on its edge facing the inspector, whatever occupies the list's space, with that space fitting its lines to the width the dividers leave and nothing else drawn in a divider's cells; both in place, adding no row to the frame and changing no region's size, and no divider where a full-screen overlay takes the body.

## Horizontal Layout

- [x] **TUI-RENDER-005**: Between 80 and 99 columns the system shall draw a sidebar and a list, the inspector appearing only in place of the list when the player calls for it.
- [x] **TUI-RENDER-006**: At 100 columns and wider the system shall draw the sidebar, the list, and the inspector side by side, the inspector taking the remaining width.
- [x] **TUI-RENDER-007**: At 120 columns and wider the system shall cap the list's width and give the surplus to the inspector, so a row's name and cost never separate further than they can be read together.
- [x] **TUI-RENDER-008**: The system shall apply an asymmetric dead-band at each breakpoint — entering a layout at the boundary and leaving it some columns below — so a resize that overshoots and settles does not flip the layout repeatedly.

## The List Viewport

- [x] **TUI-RENDER-009**: The system shall derive the list's scroll position each frame from the cursor's position, keeping the cursor visible with a margin above and below, rather than storing a scroll offset.
- [x] **TUI-RENDER-010**: The system shall scroll group headers with the list rather than pinning them.
- [x] **TUI-RENDER-011**: When a list is longer than its viewport, the system shall indicate that more rows exist beyond the visible range without spending a column on a scrollbar.

## Numbers and Names

- [ ] **TUI-RENDER-012**: The system shall render numbers in the sidebar and list rows compactly to a fixed significant-figure width so columns stay aligned, and shall render them exactly and unabbreviated in the inspector.
- [x] **TUI-RENDER-013**: The system shall scale a number's displayed precision to its magnitude, showing no trailing decimals that imply precision the value does not carry.
- [x] **TUI-RENDER-014**: When a name does not fit its column, the system shall shorten it with an explicit ellipsis so the player can see that text was cut, the full name remaining available in the inspector.

## The Frame Budget

- [x] **TUI-RENDER-015**: The system shall bound each frame's drawing work by what is visible rather than by what exists, so a frame's cost does not grow with the size of the game's content tables.
