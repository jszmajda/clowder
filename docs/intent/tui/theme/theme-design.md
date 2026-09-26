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
| `dim` | `#7C7873` | Revealed but not actionable, owned and done, flavour events, chrome rules, spent hints |
| `ready` | `#3E8E5A` | ● affordable now |
| `partial` | `#A8802F` | ◐ partially affordable |
| `attention` | `#C0663A` | Attention chips, the `full` tag on a capped store, an armed row's expansion beneath it |
| `milestone` | `#7E6BA8` | ★ milestones and newly revealed content |

Mood takes no colour at all. The navigation leaf already renders it as a word (TUI-NAV-019), and a word is unambiguous, translatable to a screen reader, and immune to every failure mode a five-step colour ramp has. Where a channel is already carrying meaning well, adding colour is noise.

## The Signature: the Pane Keeps the Season

One deliberate flourish, and it comes from the subject rather than from decoration. The game tracks a season, and the chrome's accent — the header rule and the pane divider, nothing else — drifts with it across the year.

| Season | Accent |
|---|---|
| Spring | `#7FA05A` |
| Summer | `#C9A227` |
| Autumn | `#B5713C` |
| Winter | `#6E8BA8` |

A glance tells you where in the year the clowder is before you have read a word, and a pane left open for weeks slowly changes its light. It is the one thing in this theme that exists because it is lovely rather than because it is necessary.

It is safe precisely because it is confined. **The seasonal accent never touches a colour that carries state.** If it tinted affordability, the player would have to ask whether green meant *affordable* or *spring* — the exact whimsy-over-clarity failure tenet 3 forbids. Inert chrome has no meaning to compete with, so the drift is free.

## Colour Is Always the Second Channel

Every meaning colour carries is carried by something else first — a glyph, a word, or a fill. This is not an accessibility afterthought bolted onto a finished palette; it is why the palette could stay this small.

| Meaning | First channel | Colour |
|---|---|---|
| Affordability | `●` `◐` `○` `✓` | `ready` / `partial` / `dim` |
| Attention | Glyph + count + word | `attention` |
| Capped store | The word `full`, and a filled bar | `attention` |
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
| Luminance range | Mid-band throughout | Full range, tuned per background | The consequence of inheriting the ground; also pushes toward the calm the glance contract wants |
| Focus emphasis | Cursor glyph, and reverse where more is needed | A focus colour | A focus tint would put a second, unrelated colour in a row whose marker is already saying something |
| Where an armed row wears `attention` | On its expansion beneath it; none on its own line, whose marker keeps its availability colour | Tint the whole line, marker included; tint the line around a marker left in its availability colour | An orange `●` or `◐` is the same appearance as a green `●` or an amber `◐` to a red-green deficiency, so a tinted marker would put affordability in doubt; a line tinted around its marker carries two colours saying different things, the arrangement focus is kept off the palette to avoid; the expansion is the arming's own mark and says nothing else |
| Missing truecolor | Detect and say so once; render unchanged | Assume it; ship a 256-colour fallback | A silent approximation reads as a defect in the game; a fallback palette is an explicit HLD non-goal |

## Open Questions & Future Decisions

### Deferred
1. The exact hex values want checking against real terminals — several popular themes use low-contrast backgrounds that a mid-luminance palette could sit too close to. The *roles* and their separation are the design; the six values are a starting point to verify on a real pane, alongside the render leaf's other measurements.
2. Whether `●` and `◐` stay distinct enough. They are the affordable/partial pair, whose colours are the ones a red-green deficiency compresses hardest, so the whole distinction falls to the glyph — and the two differ only by fill. That is a real difference rather than a colour one, but half-filled glyphs render unevenly across fonts, and this wants checking on a real pane before it is trusted. The glyphs belong to the navigation leaf; changing them is a cascade there, not a decision here.
3. Whether `dim` needs to be a true colour at all, or whether the terminal's own dim attribute serves better. The attribute inherits the user's theme and is arguably more native; it is also inconsistently implemented across terminals, which is what makes this worth testing rather than deciding now.

## References

- Parent: `../tui-design.md` (truecolor-only, the thin-shell split)
- Sibling: `navigation/` owns the glyphs and words this palette sits behind — TUI-NAV-009 (markers), TUI-NAV-019 (mood as a word), TUI-NAV-020 (chips), TUI-NAV-021 (single-width glyphs), TUI-NAV-022 (the non-colour channel requirement)
- Consumes: `clowder_core::Season` for the chrome accent
