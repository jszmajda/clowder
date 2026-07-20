# Navigation & Input Specs (TUI-NAV)

## Key Model

- [ ] **TUI-NAV-001**: The system shall map keys only from the tmux-safe set — plain printable characters, Tab, BackTab, Enter, Esc, Space, Backspace, and the four arrow keys — plus Ctrl-C as emergency quit; no other Ctrl chord, Alt/Meta, or function key shall be bound.
- [ ] **TUI-NAV-002**: While a screen is focused with no prompt or overlay open, the system shall move the list cursor on ↑/↓ (and j/k), and perform the focused row's primary action once on Enter.
- [ ] **TUI-NAV-003**: When Space (or i) is pressed on a focused row, the system shall toggle the inspector for that row.
- [ ] **TUI-NAV-004**: The system shall treat uppercase as the modifier tier — x opens a typed-count action and X performs the focused row's action to the maximum affordable.

## Navigation

- [ ] **TUI-NAV-005**: The system shall assign sections to the digits 1–0 by a fixed design-time map that never renumbers; pressing a section's digit jumps to it, and pressing the digit of an unrevealed section is a no-op with a footer note.
- [ ] **TUI-NAV-006**: The system shall reveal a section's tab only once at least one of its rows is revealed, place it in its fixed digit slot when it appears, and badge a newly appeared tab until it is first visited.
- [ ] **TUI-NAV-007**: Tab/BackTab, l/h, and →/← shall cycle only revealed sections; ↓/↑ and j/k shall move within the focused list; [ and ] shall jump between the list's group headers.
- [ ] **TUI-NAV-008**: The system shall remember each section's focused item by content identity (not list index), so returning to a section restores the same item even after intervening reveals inserted rows.

## Action Model

- [ ] **TUI-NAV-009**: The system shall render each actionable item as a row carrying a glyph-first availability marker (● affordable, ◐ partial, ○ revealed-unaffordable, ✓ owned/done), the flavor name, owned count, and next cost; Teased rows render dimmed with their marker and Hidden content is not rendered.
- [ ] **TUI-NAV-010**: While Enter is held, the system shall repeat the focused row's action at no more than ten times per second regardless of the terminal's key-repeat rate, and the cursor shall not move on action.
- [ ] **TUI-NAV-011**: When x is pressed on a focused row, the system shall open a count prompt that captures digit input and previews total cost, and on Enter apply the row's command that many times (or pass the count to a count-taking command); Esc cancels with no effect.
- [ ] **TUI-NAV-012**: When X is pressed on a focused row, the system shall apply the row's command repeatedly until the core rejects it or a shell-side iteration cap is reached.
- [ ] **TUI-NAV-013**: On a count row (jobs, tinkerers), the system shall assign one worker on + or Enter and unassign one on - or Backspace.
- [ ] **TUI-NAV-014**: When a command is rejected by the core, the system shall surface the CommandError as a plain-language footer message for a few seconds and shall not open any blocking dialog.
- [ ] **TUI-NAV-015**: For the heavy actions (GreatMeadowMove, PassItDown, AdoptRule, DeepNap), the first Enter shall arm the row — expanding it in place to a consequence summary computed from current state — and only a second Enter shall commit; arming auto-expires after about eight seconds or on any navigation, and the simulation keeps advancing throughout.
- [ ] **TUI-NAV-016**: When q is pressed, the system shall save and quit immediately with no confirmation; Ctrl-C shall also save and quit.

## The Inspector

- [ ] **TUI-NAV-017**: Where the terminal is at least about 100 columns wide, the system shall show the inspector as a persistent pane that live-updates for the focused row; on an 80-column terminal, Space shall swap the inspector in place of the list (and Space or Esc returns), never as a popup that stops the render or sim.
- [ ] **TUI-NAV-018**: The inspector shall show the focused item's flavor name, plain mechanical description, exact next cost with have/need per component, time-to-afford at the current net rate, owned count and effects, and — for a locked or Teased item — the plain reason it is locked.

## Glanceability

- [ ] **TUI-NAV-019**: The system shall keep persistent chrome on screen at every supported size: a header (clowder name/generation, date, kitten count, mood as a word, current speed, and the attention chips), the tab bar, a resource sidebar, an event ticker, and the hint bar.
- [ ] **TUI-NAV-020**: The system shall render the attention chips as glyph+count+word (never color alone) and show a chip only when its opportunity is live: ◦N idle (unassigned kittens), !sky (an open sky-event window), ★N new (content revealed since last viewed), and ~away (an unread offline report); when no opportunity is live the chip cluster is empty.
- [ ] **TUI-NAV-021**: The system shall render every UI symbol as a single-width character and shall use no emoji or ambiguous-width glyphs.
- [ ] **TUI-NAV-022**: The system shall convey affordability, mood, capped-storage, and attention through at least one non-color channel (glyph, word, or fill) so the game is fully playable without color perception.

## Discoverability

- [ ] **TUI-NAV-023**: The system shall display a persistent hint bar whose keys adapt to the focused context, ordered most-relevant-first and always ending with the help key, and shall surface the count key the first time a count-taking row is focused.
- [ ] **TUI-NAV-024**: On a new game the only revealed section shall be the Meadow with its gather action focused, so the first learnable actions teach the cursor/Enter/Space/Tab backbone.
- [ ] **TUI-NAV-025**: When ? is pressed, the system shall show a help overlay listing the full key map and the glyph legend.

## Events & Offline Report

- [ ] **TUI-NAV-026**: The system shall render typed events into a dim ticker (flavor events quieter than mechanical ones, milestones marked) and, on e, a scrollable Journal overlay retaining full scrollback; no event shall flash, block, or expire unseen.
- [ ] **TUI-NAV-027**: On load, the system shall present the OfflineReport as a non-blocking one-line banner (dismissed on Esc or first navigation) and as a pinned Journal entry with the full folded detail, so dismissing the banner loses nothing; a Napped report reuses the same path.

## Resolved Edge Cases

- [ ] **TUI-NAV-031**: The system shall tag each actionable row's command as count-taking (the count is a command argument — Pounce, Trade, DeepNap) or repeat-N (the shell calls the command N times — Build, Craft, buys); x passes N to a count-taking command and repeat-calls a repeat-N one, an empty or zero count cancels, and repeat-N calls run until one is rejected (partial success intended).
- [ ] **TUI-NAV-032**: When entering a section with no remembered item (or whose remembered item is no longer revealed), the system shall place the cursor on the first Available row, else the first row.
- [ ] **TUI-NAV-033**: Section cycling (Tab/BackTab, l/h, →/←) shall wrap around the revealed-section ring.
- [ ] **TUI-NAV-034**: The system shall render the tab bar responsively — full [digit name] labels where they fit, else a compact digit strip that labels the focused and newly-badged tabs — never truncating a name mid-word and never scrolling a tab out of reach; the digit is the stable section identity.
- [ ] **TUI-NAV-035**: While the inspector occupies the list's space on a narrow terminal, ↑/↓ and j/k shall still move the underlying cursor with the inspector following it, and Space or Esc shall return to the list view.
- [ ] **TUI-NAV-036**: For the DeepNap row, the count prompt's Enter shall set the year count and arm the row (showing the nap's cost and wake-year from current state), and a second Enter shall commit; the commit re-runs the real command, which re-validates against current state.

## Speed & Edge Cases

- [ ] **TUI-NAV-028**: The system shall change game speed one notch along 0.25→0.5→1→2→4→8 on < and >, applying SetSpeed and always showing the current speed in the header.
- [ ] **TUI-NAV-029**: The system shall compute the layout as a pure function of terminal size with breakpoints, preserving the cursor, per-section item memory, armed state, and open overlays across a resize; below 80×24 it shall show a "terminal too small" card while the simulation keeps advancing.
- [ ] **TUI-NAV-030**: Esc shall only ever close the topmost transient (prompt, palette, overlay, armed row, or banner) and shall be a no-op on a bare screen; the system shall bind no Esc-prefixed key sequence, so a late Esc under tmux escape-time can never act, quit, or lose state.
