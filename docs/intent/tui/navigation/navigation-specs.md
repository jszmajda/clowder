# Navigation & Input Specs (TUI-NAV)

## Key Model

- [ ] **TUI-NAV-001**: The system shall map keys only from the tmux-safe set — plain printable characters, Tab, BackTab, Enter, Esc, Space, Backspace, and the four arrow keys — plus Ctrl-C as emergency quit; no other Ctrl chord, Alt/Meta, or function key shall be bound.
- [ ] **TUI-NAV-002**: While a screen is focused with no prompt or overlay open, the system shall move the list cursor on ↑/↓ (and j/k), and perform the focused row's primary action once on Enter.
- [ ] **TUI-NAV-003**: When Space (or i) is pressed on a focused row, the system shall toggle the inspector for that row.
- [ ] **TUI-NAV-004**: The system shall treat uppercase as the modifier tier — x opens a typed-count action and X performs the focused row's action to the maximum affordable.
- [ ] **TUI-NAV-037**: While a count prompt or the palette field is open, the system shall route printable characters and digits into that field and suspend every global single-key binding, so a digit types instead of jumping to a section; Enter shall accept the field and Esc shall cancel it with no effect.
- [ ] **TUI-NAV-052**: While a full-screen overlay (Journal or help) is open, the system shall give the overlay the cursor, section, and action keys for its own navigation and suspend their global meanings, while keeping the speed keys, q, Ctrl-C, and Esc live — so the player can always retime, leave, or close from anywhere.

## Navigation

- [ ] **TUI-NAV-005**: The system shall assign sections to the digits 1–0 by a fixed design-time map that never renumbers; pressing a section's digit jumps to it, and pressing the digit of an unrevealed section is a no-op with a footer note.
- [ ] **TUI-NAV-006**: The system shall reveal a section's tab only once at least one of its rows is revealed, place it in its fixed digit slot when it appears, and badge a newly appeared tab until it is first visited.
- [ ] **TUI-NAV-007**: Tab/BackTab, l/h, and →/← shall cycle only revealed sections; ↓/↑ and j/k shall move within the focused list; [ and ] shall jump between the list's group headers.
- [ ] **TUI-NAV-008**: The system shall remember each section's focused item by content identity (not list index), so returning to a section restores the same item even after intervening reveals inserted rows.
- [ ] **TUI-NAV-038**: When / is pressed, the system shall open a find-anything palette over all revealed content, narrow its results by substring match as characters are typed, and on Enter move to the matched item's section and focus that item; Esc shall close the palette with no effect.

## Action Model

- [ ] **TUI-NAV-009**: The system shall render each actionable item as a row carrying a glyph-first availability marker (● affordable, ◐ partial, ○ revealed-unaffordable, ✓ owned/done), the flavor name, owned count, and next cost; Teased rows render dimmed with their marker and Hidden content is not rendered.
- [ ] **TUI-NAV-053**: The system shall take every availability and affordability answer behind a row's marker from a core query rather than computing it shell-side, and shall render a content kind for which the core reports no availability as two-state — present and actionable, or absent — never inventing a Teased or partial state the core did not report.
- [ ] **TUI-NAV-010**: While Enter is held on a row that is not a heavy action, the system shall repeat the focused row's action at no more than ten times per second regardless of the terminal's key-repeat rate, and the cursor shall not move on action.
- [ ] **TUI-NAV-051**: The system shall commit a heavy action only on a distinct, deliberate second Enter — a held or auto-repeating Enter shall arm the row and go no further, and X shall never commit a heavy action — so no heavy action can be reached by holding a key.
- [ ] **TUI-NAV-011**: When x is pressed on a focused row that is not a heavy action, the system shall open a count prompt that captures digit input and previews total cost, and on Enter apply the row's command that many times (or pass the count to a count-taking command); Esc cancels with no effect.
- [ ] **TUI-NAV-012**: When X is pressed on a focused row that is not a heavy action, the system shall apply the row's command repeatedly until the core rejects it or a shell-side iteration cap is reached.
- [ ] **TUI-NAV-013**: On a count row (jobs, tinkerers), the system shall assign one worker on + or Enter and unassign one on - or Backspace.
- [ ] **TUI-NAV-014**: When a command is rejected by the core, the system shall surface the CommandError as a plain-language footer message for a few seconds and shall not open any blocking dialog.
- [ ] **TUI-NAV-015**: For the heavy actions (GreatMeadowMove, PassItDown, AdoptRule, DeepNap), the first Enter shall arm the row — expanding it in place to a consequence summary computed from current state — and only a second Enter shall commit; arming auto-expires after about eight seconds or on any navigation, and the simulation keeps advancing throughout.
- [ ] **TUI-NAV-016**: When q is pressed, the system shall save and quit immediately with no confirmation; Ctrl-C shall also save and quit.
- [ ] **TUI-NAV-039**: The system shall make every player-facing Command variant that belongs to a section reachable as a list row in that section — including the stand-down choice on a declare list and the unassign action on a count row — so that no sectioned command is reachable only through a mnemonic, the palette, or an accelerator key; the two commands belonging to no section (SetSpeed, ObserveSky) are reachable by their global keys instead.
- [ ] **TUI-NAV-040**: The system shall bind ObserveSky to o from every screen; while no sky-event window is open the key shall be a no-op with a footer note.
- [ ] **TUI-NAV-041**: On a row whose one-shot action is already owned, the system shall treat Enter, x, and X as no-ops with a footer note; on a revealed row whose command is neither count-taking nor usefully repeatable, x and X shall likewise be no-ops with a footer note.
- [ ] **TUI-NAV-042**: The system shall ignore a printable key with no binding in the current context and show no footer message, and shall show a footer note only for a key that is bound but not meaningful in the current state.
- [ ] **TUI-NAV-043**: Arming a heavy row shall reserve no resources and shall disarm any previously armed row; the committing Enter shall re-run the real command so the core re-validates it against current state, and a commit that is no longer valid shall fail to the footer like any other rejection.

## The Inspector

- [ ] **TUI-NAV-017**: Where the terminal is at least about 100 columns wide, the system shall show the inspector as a persistent pane that live-updates for the focused row; on an 80-column terminal, Space shall swap the inspector in place of the list (and Space or Esc returns), never as a popup that stops the render or sim.
- [ ] **TUI-NAV-018**: The inspector shall show the focused item's flavor name, plain mechanical description, exact next cost with have/need per component, time-to-afford at the current net rate, owned count and effects, and — for a locked or Teased item — the plain reason it is locked.

## Glanceability

- [ ] **TUI-NAV-019**: The system shall keep persistent chrome on screen at every supported size: a header (clowder name/generation, date, kitten count, mood as a word, current speed, and the attention chips), the tab bar, a resource sidebar, an event ticker, and the hint bar.
- [ ] **TUI-NAV-020**: The system shall render the attention chips as glyph+count+word (never color alone) and show a chip only when its opportunity is live: ◦N idle (unassigned kittens), !sky (an open sky-event window), ★N new (content revealed since last viewed), and ~away (an unread offline report); when no opportunity is live the chip cluster is empty.
- [ ] **TUI-NAV-054**: The system shall count in the idle chip only kittens the player can assign right now, excluding any kitten who is away from the meadow, so that no chip asks for an action the player cannot take.
- [ ] **TUI-NAV-021**: The system shall render every UI symbol as a single-width character and shall use no emoji or ambiguous-width glyphs.
- [ ] **TUI-NAV-022**: The system shall convey affordability, mood, capped-storage, and attention through at least one non-color channel (glyph, word, or fill) so the game is fully playable without color perception.
- [ ] **TUI-NAV-044**: The system shall clear each attention chip on its own signal — the new-content chip when its badged section is visited, and the away chip when its banner is dismissed or its pinned Journal entry is opened.
- [ ] **TUI-NAV-049**: The system shall list every revealed resource as an inspectable row in the home section's stores group, keeping the resource sidebar a curated display-only glance rather than the complete table.

## Discoverability

- [ ] **TUI-NAV-023**: The system shall display a persistent hint bar whose keys adapt to the focused context, ordered most-relevant-first and always ending with the help key, and shall surface the count key the first time a count-taking row is focused.
- [ ] **TUI-NAV-024**: On a new game the only revealed section shall be the Meadow with its gather action focused, so the first learnable actions teach the cursor/Enter/Space/Tab backbone.
- [ ] **TUI-NAV-025**: When ? is pressed, the system shall show a help overlay listing the full key map and the glyph legend.
- [ ] **TUI-NAV-045**: When the hint bar's context keys exceed the available width, the system shall drop the least-relevant keys first and always retain the help key last, the complete key map remaining available through the help overlay.
- [ ] **TUI-NAV-046**: The system shall assign a single-letter mnemonic only to an action already reachable by cursor and Enter, shall never assign one from the reserved set (the cursor, section, action, overlay, and speed keys), and shall show the focused screen's live mnemonics in the hint bar.

## Events & Offline Report

- [ ] **TUI-NAV-026**: The system shall render typed events into a dim ticker and, on e, a scrollable Journal overlay over the retained journal — the recent-event window plus every milestone and report the core has kept — which survives closing and reopening the game; no event shall flash, block, or expire unseen while it is retained.
- [ ] **TUI-NAV-057**: The system shall rank an event's prominence from the classification the core supplies — flavor quieter than mechanical, milestones marked — and shall not decide from the event's identity which tier it belongs to.
- [ ] **TUI-NAV-027**: On load, the system shall present the OfflineReport as a non-blocking one-line banner (dismissed on Esc or first navigation) and as a pinned Journal entry with the full folded detail, so dismissing the banner loses nothing; a Napped report reuses the same path.
- [ ] **TUI-NAV-050**: When more than one bulk-time report is unread, the system shall queue their banners newest-first while pinning every one as its own Journal entry, so no report is displaced unseen.
- [ ] **TUI-NAV-055**: The system shall fill the offline banner's one line with the elapsed span, the largest resource gains, and any kittens who arrived, and shall leave departures and absences to the pinned Journal entry — so the glance the player returns to leads with what grew, never with what left.
- [ ] **TUI-NAV-056**: The system shall report kittens who wandered while away as a present absence that reverses on their return — naming where they went and that they come back — and shall never render a departure as a loss or a count decrease.

## Speed & Layout

- [ ] **TUI-NAV-028**: The system shall change game speed one notch along 0.25→0.5→1→2→4→8 on < and >, applying SetSpeed and always showing the current speed in the header; from a loaded speed that is not on the ladder, the first press shall move to the nearest notch in the pressed direction, and at either end of the ladder a further press shall be a no-op.
- [ ] **TUI-NAV-029**: The system shall compute the layout as a pure function of terminal size with breakpoints, preserving the cursor, per-section item memory, armed state, and open overlays across a resize; below 80×24 it shall show a "terminal too small" card while the simulation keeps advancing.
- [ ] **TUI-NAV-030**: Esc shall only ever close the topmost transient (prompt, palette, overlay, armed row, or banner) and shall be a no-op on a bare screen; the system shall bind no Esc-prefixed key sequence, so a late Esc under tmux escape-time can never act, quit, or lose state.
- [ ] **TUI-NAV-047**: The system shall apply a dead-band to the inspector's layout breakpoint so a single-column resize wobble does not flip the layout between the persistent-pane and swap modes.
- [ ] **TUI-NAV-048**: Where the terminal is about 120 columns or wider, the system shall lay out three columns — sidebar, list capped for scannability, and inspector taking the remainder — giving extra width to the inspector and the ticker.

## Resolved Edge Cases

- [ ] **TUI-NAV-031**: The system shall tag each actionable row's command as count-taking (the count is a command argument — Pounce, Trade, DeepNap) or repeat-N (the shell calls the command N times — Build, Craft, buys); x passes N to a count-taking command and repeat-calls a repeat-N one, an empty or zero count cancels, and repeat-N calls run until one is rejected (partial success intended).
- [ ] **TUI-NAV-032**: When entering a section with no remembered item (or whose remembered item is no longer revealed), the system shall place the cursor on the first Available row, else the first row.
- [ ] **TUI-NAV-033**: Section cycling (Tab/BackTab, l/h, →/←) shall wrap around the revealed-section ring.
- [ ] **TUI-NAV-034**: The system shall render the tab bar responsively — full [digit name] labels where they fit, else a compact digit strip that labels the focused and newly-badged tabs — never truncating a name mid-word and never scrolling a tab out of reach; the digit is the stable section identity.
- [ ] **TUI-NAV-035**: While the inspector occupies the list's space on a narrow terminal, ↑/↓ and j/k shall still move the underlying cursor with the inspector following it, and Space or Esc shall return to the list view.
- [ ] **TUI-NAV-036**: For the DeepNap row, the count prompt's Enter shall set the year count and arm the row (showing the nap's cost and wake-year from current state), and a second Enter shall commit; the commit re-runs the real command, which re-validates against current state.
