---
parent: tui
prefix: TUI-RENDER
---

# Render (layout, widgets, and the frame)

## Context and Design Philosophy

This leaf owns everything between a `GameState` snapshot and the cells on screen: how the frame divides into regions, what each region draws, how the layout responds to size, how numbers and names are formatted to fit, and what a frame is allowed to cost.

Its guiding property is that **a frame is a pure function of (snapshot, view state, terminal size)**. Nothing is cached between frames, no game value is mirrored shell-side, and no drawing decision depends on what was drawn last. This mirrors the determinism the sim core is built on, and it buys the same thing: layout becomes testable without a terminal. A frame rendered to an in-memory buffer at an exact size can be asserted against, so "the inspector appears at 100 columns" and "nothing overflows at 80×24" are tests rather than things someone remembers to check. Given how much of the navigation leaf's intent is stated as size-dependent behaviour, that testability is the difference between those specs being verified and being hoped for.

The leaf inherits its vocabulary rather than inventing it: glyphs and the meaning of every marker come from `navigation`, colour roles from `theme`, and the time available per frame from `runtime`. What is decided here is *arrangement*.

## The Frame

Vertically the frame is fixed furniture around a flexible body:

```
┌──────────────────────────────────────────────┐
│ header                                     1 │
│ tab bar                                    1 │
├──────────────────────────────────────────────┤
│                                              │
│ body                              height − 4 │
│                                              │
├──────────────────────────────────────────────┤
│ ticker                                     T │
│ hint bar                                   1 │
└──────────────────────────────────────────────┘
```

Body height is `height − 3 − T`, where `T` is the ticker's rows — one at the floor, and more only where width allows the navigation leaf's wider log. A formula rather than a table of sizes, so no arithmetic drifts as the chrome changes. At the 80×24 floor `T` is 1 and the body is 20 rows. When the offline banner is present it takes one row from the body rather than adding to the frame, so the body absorbs every variation and the frame always sums to the terminal's height.

The **header rule** is drawn in place rather than given a row, so it never enters the formula. It is the header row's bottom edge: an underline across every cell of the header row, whatever the header draws in them — facts, chips, the blank columns between, the padding — at every width step the header takes (TUI-NAV-086). An underline is a mark beneath a cell's character rather than a character, so the rule changes no glyph or word on the row, and each keeps the colour it would wear without the rule. A terminal that draws no underlines shows no rule, and nothing is substituted for it: the rule carries no information the header's words do not. Its colour belongs to the theme leaf.

Horizontally the body divides by width:

| Width | Body |
|---|---|
| under 80 columns *or* 24 rows | The too-small card; nothing else is drawn, and the sim keeps ticking |
| 80 – 99 | sidebar │ list — the inspector swaps in over the list on demand |
| 100 – 119 | sidebar │ list │ inspector, the inspector taking what remains |
| ≥ 120 | sidebar │ list (capped) │ inspector, extra width going to the inspector |

The sidebar is a fixed width; the list takes what it needs up to a cap; the inspector absorbs the remainder. Capping the list is deliberate — a row stretched across 150 columns puts its name and its cost so far apart that reading one line becomes a saccade, and the width is better spent on the inspector, which has genuinely more to say.

Side-by-side panes are separated by a **pane divider**: one column of `│`, the single-width light vertical box-drawing glyph, running the full height of the panes it separates. Each divider is taken from the list's space, on the edge that faces the neighbouring pane — the sidebar's side at every width, and the inspector's side as well where the inspector is a pane of its own — so 80 – 99 columns draw one divider and 100 columns and wider draw two. Whatever occupies the list's space fits its lines to the width the dividers leave, and the layout's widths are otherwise unchanged. The list gives the column because the sidebar's fixed width and the inspector's minimum are sized to what those panes hold, while the list's rows already shorten with an ellipsis the inspector resolves (Numbers and Names).

The divider belongs to the layout on screen. While the narrow-mode swap puts the inspector in over the list, the divider stays, taken from the same space; within a breakpoint's dead-band it follows whichever layout is drawn; and where a full-screen overlay takes the body there are no side-by-side panes and so no divider, and nothing cuts through the overlay's text. It spans the panes' rows only — never the banner row above them, the ticker, or the hint bar. Its cells hold the glyph and nothing else: a focused row's reverse video, an armed row's expansion, and the list's position indicator all end at the width the dividers leave. Its colour belongs to the theme leaf.

## Breakpoints

Two breakpoints, at 100 and 120 columns. The first is where the inspector stops being a swap and becomes a pane; the second is where the list stops growing and hands surplus width to the inspector.

Both carry a dead-band, so a terminal sitting exactly on a boundary does not flip layouts as the pane wobbles by a column (TUI-NAV-047). The dead-band is asymmetric: growing into a layout happens at the boundary, and falling out of it happens a few columns below, so a resize that overshoots and settles does not thrash.

## The List Viewport

Lists outgrow the body — 59 ideas will not fit 20 rows — so the list scrolls, and the scroll position is derived rather than stored: it is whatever keeps the cursor visible with a small margin above and below. Deriving it preserves the pure-function property and means the viewport can never disagree with the cursor after a reveal inserts rows or a resize changes the body height.

A group header scrolls like any other line. Pinning the current group to the top was considered and dropped: it costs a row of the body permanently, and `[` / `]` already move by group, which is the affordance a player reaches for when they want to know where they are.

When the list is longer than the viewport, the frame says so — a position indicator rather than a scrollbar, since a single column of scrollbar is both harder to read and more expensive in a narrow pane than a short "n more below".

## Numbers and Names

Space is the scarce resource here, and the tenet ordering decides how to spend it: whimsy names the row, but the plain truth must be reachable (tenet 3).

**Numbers are compact where they are glanced at and exact where they are read.** The sidebar and list rows abbreviate to a fixed significant-figure width, so columns stay aligned and a value's magnitude is legible at a glance. The inspector states exact values, unabbreviated — TUI-NAV-018 already requires exact per-component costs there, and that is the contract: every abbreviated number on screen has an exact form one keypress away.

Precision follows magnitude rather than being fixed: a stock of 3 reads as `3`, not `3.00`. Trailing noise makes a number look more precise than it is and costs columns that alignment needs.

**Names truncate, but never into ambiguity.** Where a name will not fit, it is shortened with an explicit ellipsis so the player can see that something was cut. A truncated name is always resolvable — the inspector shows it in full, and the cursor is already on it.

## The Frame Budget

The loop gives render a slice of each frame after input and ticks have taken theirs, and rendering must fit in it. Nothing here should come close: a terminal frame is a few thousand cells, and the expensive-looking parts (46 resources, long lists) are bounded by the viewport, which never exceeds the body's rows.

The two things that could get expensive are worth naming so they are not discovered later. Deriving the sidebar's rates asks the core for a value per revealed resource every frame, and the palette's per-cell work is proportional to what is drawn rather than what exists. Both are bounded, neither is obviously a problem, and both are cheap to measure once there is something to measure — which is the honest state of every frame-cost question at this point.

Whether frames run at a fixed cadence or only when something changed is deferred jointly with `runtime`, whose loop owns the timing. The pure-function property makes the dirty-frame optimisation available later without restructuring: if a frame is a function of its inputs, then "did the inputs change" is a well-formed question.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| Frame derivation | Pure function of snapshot, view state, and size | Incremental/dirty-region drawing; cached widget state | Testable at exact sizes without a terminal, which is what makes the size-dependent navigation specs verifiable; leaves the optimisation available later |
| Chrome height | Body is `height − 4`; chrome is constant | Per-breakpoint row tables | A formula cannot drift out of sync as chrome changes; the banner takes from the body rather than growing the frame |
| Header rule | An underline across the header row's cells, drawn in place | A rule row of its own beneath the header | A row of its own costs the body a row at the 80×24 floor, where the body is already 20 rows; the underline marks the same edge for no space and leaves the frame's algebra untouched |
| Pane divider | One `│` column at each boundary between side-by-side panes, taken from the list's space | Taking the column from the sidebar or the inspector; a blank gutter column | The sidebar's fixed width and the inspector's minimum are sized to their contents, while the list's rows already shorten legibly; a blank column separates the panes less plainly and leaves the chrome's accent nothing to sit on |
| Wide layout | Cap the list, give surplus to the inspector | Let the list grow; add a fourth column | A row stretched across 150 columns separates name from cost by a saccade; the inspector has more to say |
| Scroll position | Derived from the cursor each frame | Stored scroll offset | Stored offset can disagree with the cursor after a reveal or a resize; deriving it cannot |
| Group headers | Scroll with the list | Pin the current group | Pinning costs a permanent body row for what `[` / `]` already provides |
| Overflow indicator | A short "more below" note | A scrollbar column | A column is expensive in a narrow pane and harder to read at a glance |
| Number precision | Compact when glanced, exact in the inspector; precision follows magnitude | Fixed decimal places everywhere; exact everywhere | Fixed places make small values look falsely precise and eat the columns alignment needs; exact everywhere does not fit |
| Truncation | Explicit ellipsis, full text in the inspector | Hard cut; scrolling text; shrinking to fit | A hard cut hides that anything was lost; motion in a glanceable pane is exactly what the design is avoiding |

## Open Questions & Future Decisions

### Deferred

Every question here is one that a real terminal answers better than a document, which is why the layout above is stated as an algebra rather than a set of measurements.

1. The sidebar's exact width and the list's cap. Both are named as roles above; their values want a real pane and real content lengths.
2. The dead-band's width, and whether asymmetric hysteresis is enough to stop thrash during a drag-resize.
3. Frame cadence — fixed rate versus redraw-on-change — decided jointly with `runtime` once frame cost is measurable.
4. Whether the ticker earns more than one row when height allows. The navigation leaf permits up to four lines when wide; whether that is better than giving the rows to the body is a judgement about a real game's event volume.

## References

- Parent: `../tui-design.md`
- Consumes: `navigation/` for glyphs, markers, and every size-dependent behaviour (TUI-NAV-017, 019, 029, 034, 045, 047, 048); `theme/` for colour roles; `runtime/` for the per-frame budget
- Renders from: `clowder_core::GameState` snapshots and the core's derived queries — never from shell-side copies of game data
