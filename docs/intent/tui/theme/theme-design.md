---
parent: tui
prefix: TUI-THEME
---

# Theme (colour, contrast, and the channel discipline)

## Context and Design Philosophy

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the game fully playable by someone who cannot distinguish any of it.

The brief is unusual, and it should drive the design rather than be decorated over. Clowder is not a page someone visits; it is **a pane that sits beside real work for weeks**. The player looks at it for two seconds between compiles. It shares a screen with their editor, their logs, and their shell. A theme that announces itself is a theme that becomes irritating on day three — and irritating-on-day-three is a failure of tenet 1 as surely as anything grim would be.

So the governing principle is not "what palette is beautiful" but **how little colour can carry the hierarchy**. Colour spent on decoration is colour that no longer means anything when something actually needs the player. The two-second glance contract in the navigation leaf — empty chip cluster means nothing needs you — only works if the eye has learned that colour appearing is *information*.

## Colour Encodes State, Never Identity

The single load-bearing decision. There are 46 resources, 40 structures, and 59 ideas; giving each a colour would need a palette nobody can learn, and would spend the entire colour budget on information the player already has for free from the name sitting right there.

**Identity is carried by the name and its position. Colour is reserved for state.** Whether a thing is affordable, whether a store is full, whether something wants attention — these are what colour says, and they are the same handful of states across every screen. A player learns six meanings once and reads them everywhere, instead of learning fifty and reading none reliably.

This is also what keeps the palette small enough to be disciplined about contrast and colour-vision safety, which a fifty-hue scheme could never be.

## The Ground Belongs to the Terminal

Clowder paints no background. Ordinary text is the terminal's own foreground on the terminal's own background, and only meaningful things take a colour from the palette below.

This is the choice that makes light and dark work without detecting anything. Terminal background detection is unreliable in general and worse under tmux, and a wrong guess is more damaging than no guess — a light-theme user handed a dark palette gets an unreadable pane. Inheriting sidesteps the question: the game is legible in a light terminal and a dark one because it is using that terminal's own contrast in both.

It also makes clowder look *native* beside the player's other panes rather than like a window pasted over them, which matters for something that lives in a tmux split for weeks. The cost is real and accepted: every palette colour must hold against both a near-black and a near-white ground, which rules out both pale tints and deep shades. The palette is therefore mid-luminance throughout — a constraint that pushes toward calm, which is what the glance contract wanted anyway.

## The Palette

Six roles. Every value sits in a mid-luminance band so it clears the contrast floor against both extremes of ground.

| Role | Value | Carries |
|---|---|---|
| *(inherited)* | terminal foreground | All ordinary text — names, counts, costs, prose |
| `dim` | `#7C7873` | Revealed but not actionable, owned and done, flavour events, a list's group-header rules, spent hints |
| `ready` | `#3E8E5A` | ● affordable now |
| `partial` | `#A8802F` | ◐ partially affordable |
| `attention` | `#C0663A` | Each attention chip whole, the `full` tag on a capped store, an armed row's expansion beneath it — and nothing else |
| `milestone` | `#7E6BA8` | ★ milestones and newly revealed content |

Mood takes no colour at all. The navigation leaf already renders it as a word (TUI-NAV-019), and a word is unambiguous, translatable to a screen reader, and immune to every failure mode a five-step colour ramp has. Where a channel is already carrying meaning well, adding colour is noise.

### What wears `attention`

`attention` means one thing — *this wants you* — and the glance contract depends on it meaning nothing else. An empty chip cluster and no `full` tag is how the player reads, in two seconds, that nothing needs them, and the eye can only learn that if orange appearing is always that signal. So the role is not subdivided by surface, and it is not lent to anything that is merely information.

Three elements wear it, each on exactly the mark that says it wants notice:

- **An attention chip, whole.** Its glyph, its count where it has one, and its word all wear `attention`; the blank columns between chips, and the header's facts beside them, do not. This holds at every width step the header takes (TUI-NAV-086) — a chip that has shed its count is still a chip — and for every chip alike, the `★N new` chip included. That chip counts tabs wearing the `milestone` ★, but the chip itself is asking for the player, and a cluster in two colours would leave the eye to work out which colour means "come look".
- **The `full` tag on a capped store, and only the tag.** The store's name, figures, fill bar, and rate are drawn as any store's are. The tag is present exactly while the store is capped (TUI-NAV-022), whereas the bar's cells can read full a little before the cap is reached, so the word is the state's exact mark and the colour sits on it.
- **An armed row's expansion.** The summary lines beneath the row, and their copy in the inspector while the narrow-mode swap holds the list's space — never the row's own line (The compressed run). Where the layout leaves part of the expansion out of view, the colour stays on what is drawn of it and does not move onto the row's line or the hint bar in its place. The hint bar's armed keys are drawn as any hint is: they are the arming's word channel, not a second coloured mark.

All three wear the same `attention` value, whichever surface draws them, so there is one orange to learn.

Nothing else wears `attention`. A footer note — the reply to a key just pressed (TUI-NAV-014) — is ordinary prose in the inherited foreground. It is drawn where the player is already looking and lasts seconds, so it needs no colour to be seen; in orange it would be one more thing the glance has to rule out before concluding nothing needs the player, and a refused keypress drawn in the colour of an opportunity would read as a scolding.

## The Signature: the Pane Keeps the Season

One deliberate flourish, and it comes from the subject rather than from decoration. The game tracks a season, and the chrome's accent — the header rule and the pane dividers (render leaf: The Frame, Horizontal Layout), nothing else — drifts with it across the year.

| Season | Accent |
|---|---|
| Spring | `#7FA05A` |
| Summer | `#C9A227` |
| Autumn | `#B5713C` |
| Winter | `#6E8BA8` |

A glance tells you where in the year the clowder is before you have read a word, and a pane left open for weeks slowly changes its light. It is the one thing in this theme that exists because it is lovely rather than because it is necessary.

It is safe precisely because it is confined. **The seasonal accent never touches a colour that carries state.** If it tinted affordability, the player would have to ask whether green meant *affordable* or *spring* — the exact whimsy-over-clarity failure tenet 3 forbids. Inert chrome has no meaning to compete with, so the drift is free.

The accent is the season of the snapshot the frame draws, so it turns on the same frame as the header's season word, with no blend across the turn. A blend would need a frame to remember what it drew before, which no frame does, and a rule still wearing the old season beside the new season's name would be the one place the pane said two things about the year.

On the header rule the accent colours the underline alone. The facts and chips above it keep their own colours — a chip's glyph and word stay `attention` — so the accent sits beneath state without touching it. A terminal that does not colour underlines draws the rule in each cell's own colour, or draws none, and nothing is substituted: the season's name in the header already says everything the accent says.

Autumn's accent sits near `attention` in hue, and it is safe for the same reason the confinement is. The accent is only ever a line — an underline or a column of `│` — always present, always in the same places. `attention` is only ever worn by glyphs and words, and it appears only when something wants the player. The glance reads orange *appearing*; a line that never comes or goes gives it nothing to rule out.

## Colour Is Always the Second Channel

Every meaning colour carries is carried by something else first — a glyph, a word, or a fill. This is not an accessibility afterthought bolted onto a finished palette; it is why the palette could stay this small.

| Meaning | First channel | Colour |
|---|---|---|
| Affordability | `●` `◐` `○` `✓` | `ready` / `partial` / `dim` |
| Attention | Glyph + count + word | `attention`, on the whole chip |
| Capped store | The word `full`, and a filled bar | `attention`, on the word |
| Armed row | Its expansion beneath itself, and its keys in the hint bar | `attention`, on the expansion |
| Milestone | `★` | `milestone` |
| Flavour vs mechanical | `·` vs `▪` | `dim` vs inherited |
| Mood | The word itself | *(none)* |
| Season | The season's name in the header | Chrome accent |

Turn every colour off and the game is still completely playable — which is the actual test, and the one TUI-NAV-022 states.

### The compressed run

The palette's riskiest axis for colour vision is `ready` against `partial` against `attention`, the green–amber–orange run that red-green deficiencies compress. For a player with such a deficiency these three colours are close to one colour, so between these three states the glyph is not a courtesy but the whole distinction. Two rules keep it whole.

**Each of the three has a non-colour channel the other two never use.** Affordable is `●`. Partially affordable is `◐`. Attention is carried by each chip's own glyph (TUI-NAV-020), by a word — the `full` tag — or, on an armed row, by the row's expansion beneath itself, whose summary is words (TUI-NAV-077); none of these is ever `●` or `◐`. Every glyph among them is a different single-width character (TUI-NAV-021); none differs from another only by its colour.

**Among the three colours, each glyph wears only its own.** `●` is never drawn in `partial` or `attention`, `◐` never in `ready` or `attention`, and a chip's glyph never in `ready` or `partial`. If any glyph could appear in a neighbour's colour, the two pairings would be one appearance to exactly the player this section is for — an amber `◐` and an orange `◐` are the same mark once amber and orange are the same colour. Dim lies outside the run and still recedes a marker as TUI-NAV-009 asks: a Teased row's `◐` is drawn dim.

The second rule matters most on an armed row, which wants notice while its marker is still saying something. The row wears `attention` on its expansion — the summary lines the arming opens beneath it, in the list or, while the narrow-mode swap holds the list's space, in the inspector — and nowhere on its own line. Its own line takes no `attention`: its marker keeps its availability colour, whatever that marker is, and the rest of the line is drawn as any row's is. The marker is the row's affordability channel, and colour on it says affordability only. The arming needs nothing from that line, because it has a non-colour channel of its own — the expansion, and its keys in the hint bar (TUI-NAV-077) — and the expansion is exactly where its colour sits. So the row's line says one thing in colour, as focus is kept off the palette to ensure (Three Things the Palette Does Not Cover), and nothing about affordability is ever put in doubt by the arming.

Both rules hold whatever values the palette settles on (Deferred 1). Retuning the hexes may widen the three hues' separation for ordinary vision; it cannot be relied on to survive every deficiency, and the glyphs carry the state either way.

## Three Things the Palette Does Not Cover

**Focus takes no colour.** The cursor is the `▸` glyph, and where more emphasis is wanted the row reverses. Giving focus a palette colour would break the rule that colour means state — the focused row's state is whatever its marker says, and tinting it would put two colours on the row's line saying different things.

**Reverse video inverts the terminal's own pair.** The offline banner and any emphasised row swap the terminal's foreground and background rather than reversing a palette colour. This works on a ground we do not own and in either polarity, and it is the one emphasis that needs no colour at all.

**Truecolor is checked, not assumed.** The HLD rules out designing degraded palettes, and that stands — but a tmux that has not been configured to advertise RGB will silently approximate every value to the nearest 256-colour cell, and the player sees a muddy version of a game they will reasonably assume just looks like that. Startup checks whether truecolor is advertised and, when it is not, says so once in plain words with what to set. One line, once, and the game plays on exactly as before: the difference between a known limitation and an apparent defect is entirely whether anyone said anything.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| What colour means | State only; identity comes from names | A hue per resource tier or category | Fifty hues is a vocabulary nobody learns; six states are read everywhere |
| Background | Inherit the terminal's | Paint a designed ground; detect and switch | Inheriting is legible in light and dark with no detection, and looks native beside other panes; detection is unreliable under tmux and a wrong guess is worse than none |
| Palette size | Six roles | A fuller set with per-tier ramps | Every added colour dilutes the signal that colour *is* meaning |
| Mood rendering | A word, no colour | A five-step colour ramp | The word already says it unambiguously; a ramp adds a channel that fails for some players and says nothing new to the rest |
| Seasonal accent | Chrome only, never state colours | Tint the whole palette seasonally; no seasonal element | Confined, it is free delight; on state colours it would make the player ask whether a colour meant affordability or season |
| Accent at a season's turn | Changes on the frame the season word changes, with no blend | A gradual blend between the two seasons' accents | A blend needs the frame to remember what it drew, and while it ran the rule would disagree with the season's name beside it |
| Luminance range | Mid-band throughout | Full range, tuned per background | The consequence of inheriting the ground; also pushes toward the calm the glance contract wants |
| Focus emphasis | Cursor glyph, and reverse where more is needed | A focus colour | A focus tint would put a second, unrelated colour in a row whose marker is already saying something |
| Where an armed row wears `attention` | On its expansion beneath it; none on its own line, whose marker keeps its availability colour | Tint the whole line, marker included; tint the line around a marker left in its availability colour | An orange `●` or `◐` is the same appearance as a green `●` or an amber `◐` to a red-green deficiency, so a tinted marker would put affordability in doubt; a line tinted around its marker carries two colours saying different things, the arrangement focus is kept off the palette to avoid; the expansion is the arming's own mark and says nothing else |
| What wears `attention` | Each chip whole, the `full` tag alone, an armed row's expansion; one value everywhere, and nothing else | A distinct hue per surface; lending the role to other notices such as footer notes | One colour for one meaning is what lets an empty cluster and no `full` tag read as calm; a hue per surface is three colours to learn for one idea, and a notice that is not an opportunity dilutes the signal |
| The `★N new` chip's colour | `attention`, as every chip | `milestone`, matching the ★ on the tabs it counts | The cluster is read as one signal; a chip in another colour would make the player ask which colour means "come look", and the chip's own glyph and word already say what it concerns |
| Where a capped store wears `attention` | On the `full` tag alone | Tint the whole sidebar line; tint the fill bar with the tag | A tinted line colours the store's name and figures, which carry identity and quantity rather than the state; the bar's cells can read full before the cap is reached, so only the word marks the state exactly |
| Footer notes | Inherited foreground | `attention` | A note answers the key just pressed, where the player is already looking, and lasts seconds; in `attention` it would be one more orange the glance must rule out, and a refusal in an opportunity's colour reads as a scolding |
| Missing truecolor | Detect and say so once; render unchanged | Assume it; ship a 256-colour fallback | A silent approximation reads as a defect in the game; a fallback palette is an explicit HLD non-goal |

## Open Questions & Future Decisions

### Deferred
1. The exact hex values want checking against real terminals — several popular themes use low-contrast backgrounds that a mid-luminance palette could sit too close to. The *roles* and their separation are the design; the six values and the four seasonal accents are a starting point to verify on a real pane, alongside the render leaf's other measurements.
2. Whether `●` and `◐` stay distinct enough. They are the affordable/partial pair, whose colours are the ones a red-green deficiency compresses hardest, so the whole distinction falls to the glyph — and the two differ only by fill. That is a real difference rather than a colour one, but half-filled glyphs render unevenly across fonts, and this wants checking on a real pane before it is trusted. The glyphs belong to the navigation leaf; changing them is a cascade there, not a decision here.
3. Whether `dim` needs to be a true colour at all, or whether the terminal's own dim attribute serves better. The attribute inherits the user's theme and is arguably more native; it is also inconsistently implemented across terminals, which is what makes this worth testing rather than deciding now.

## References

- Parent: `../tui-design.md` (truecolor-only, the thin-shell split)
- Sibling: `navigation/` owns the glyphs and words this palette sits behind — TUI-NAV-009 (markers), TUI-NAV-019 (mood as a word), TUI-NAV-020 (chips), TUI-NAV-021 (single-width glyphs), TUI-NAV-022 (the non-colour channel requirement)
- Consumes: `clowder_core::Season` for the chrome accent
