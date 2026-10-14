---
parent: tui
prefix: TUI-THEME
---

# Theme (colour, contrast, and the channel discipline)

## Context and Design Philosophy

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the coats the stores wear, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the game fully playable by someone who cannot distinguish any of it.

The brief is unusual, and it should drive the design rather than be decorated over. Clowder is not a page someone visits; it is **a pane that sits beside real work for weeks**. The player looks at it for two seconds between compiles. It shares a screen with their editor, their logs, and their shell. A theme that announces itself is a theme that becomes irritating on day three — and irritating-on-day-three is a failure of tenet 1 as surely as anything grim would be.

So colour has exactly two jobs here, and the design keeps them apart. **State** is the job the two-second glance depends on: whether a thing is affordable, whether a store is full, whether something wants the player. **Identity** is the job that makes the pane a clowder rather than a ledger: every resource wears a coat of its own, so the stores have the variety a litter has, and a name reads as the same thing wherever it stands. The glance contract in the navigation leaf — empty chip cluster means nothing needs you — only works if the eye has learned that a state colour *appearing* is information, which is why the two jobs never share a place: state colours sit on marks, coats sit on names, and nothing wears both.

## Two Jobs for Colour

The load-bearing decision. There are 46 resources, 40 structures, and 59 ideas. Of these only the resources are nouns every screen shares: a store stands in the sidebar, in the stores group, in every price on every list, and in the inspector's need-and-have lines, and the eye is forever matching a price's name to the store it draws on. So **resources wear coats, and nothing else does.** A structure, an idea, a job, or a kitten appears once, in its own list, with its name beside it; a colour of its own would be a vocabulary nobody learns, spent on information the name already gives for free.

**Marks wear state.** Whether a row is affordable, whether a store is full, whether something wants notice — these are what the five state colours say, and they are the same handful of states on every screen. A player learns five meanings once and reads them everywhere. The state colours are worn by glyphs, tags, chips, and whole lines; they are never worn by a name as a name.

**Names wear coats.** A coat is a resource's own colours, fixed for the life of the game, chosen for the name rather than for anything the store is doing. A coat says nothing the name does not say; it is texture, learned by familiarity, never asked to be read. Because a coat is always present, like the chrome's accent, the eye files it with the furniture, and a state colour appearing is still the thing that stands out.

This is also what keeps the state palette small enough to be disciplined about contrast and colour-vision safety.

## The Ground Belongs to the Terminal

Clowder paints no background. Ordinary text is the terminal's own foreground on the terminal's own background, and only meaningful things take a colour from the palette below.

This is the choice that makes light and dark work without detecting anything. Terminal background detection is unreliable in general and worse under tmux, and a wrong guess is more damaging than no guess — a light-theme user handed a dark palette gets an unreadable pane. Inheriting sidesteps the question: the game is legible in a light terminal and a dark one because it is using that terminal's own contrast in both.

It also makes clowder look *native* beside the player's other panes rather than like a window pasted over them, which matters for something that lives in a tmux split for weeks. The cost is real and accepted: every palette colour — the state colours and the coats alike — must hold against both a near-black and a near-white ground, a WCAG contrast ratio of at least 3:1 against pure black and against pure white, which rules out both pale tints and deep shades. The palette is therefore mid-luminance throughout, a constraint that pushes toward calm, which is what the glance contract wanted anyway. The coats take their liveliness from chroma, not from brightness.

## The Palette

Six roles. Every value sits in a mid-luminance band so it clears the contrast floor against both extremes of ground.

| Role | Value | Carries |
|---|---|---|
| *(inherited)* | terminal foreground | All ordinary text — counts, costs, prose, and every name that wears no coat |
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
- **The `full` tag on a capped store, and only the tag.** The store's name wears its coat, and its figures, fill bar, and rate are drawn as any store's are. The tag is present exactly while the store is capped (TUI-NAV-022), whereas the bar's cells can read full a little before the cap is reached, so the word is the state's exact mark and the colour sits on it.
- **An armed row's expansion.** The summary lines beneath the row, and their copy in the inspector while the narrow-mode swap holds the list's space — never the row's own line (The compressed run). Where the layout leaves part of the expansion out of view, the colour stays on what is drawn of it and does not move onto the row's line or the hint bar in its place. The hint bar's armed keys are drawn as any hint is: they are the arming's word channel, not a second coloured mark.

All three wear the same `attention` value, whichever surface draws them, so there is one orange to learn.

Nothing else wears `attention`. A footer note — the reply to a key just pressed (TUI-NAV-014) — is ordinary prose in the inherited foreground. It is drawn where the player is already looking and lasts seconds, so it needs no colour to be seen; in orange it would be one more thing the glance has to rule out before concluding nothing needs the player, and a refused keypress drawn in the colour of an opportunity would read as a scolding.

## Coats

Every resource has a coat: one to four **stops**, each a hue from the ring below, and a **dressing** that says how the stops lie across the characters of its name. A solid coat has exactly one stop; a patched or blended coat has two to four. A hue may recur among a coat's stops — a checked coat alternates two — but no two consecutive stops are the same hue, so no coat is a solid coat in disguise and no two coats look alike while counting as different. A character is one Unicode scalar value, every one in the resource vocabulary being single-width; a space counts as a character under every dressing and wears nothing visible.

- **Solid.** One stop, worn by every character.
- **Patched.** The name's characters are divided into as many consecutive runs as the coat has stops, in the coat's order, near-equal in length: every run has the quotient of characters by stops, and the first runs, as many as the remainder, have one more — a six-letter name with two stops wears three and three, a seven-letter name four and three, an eight-letter name with three stops three, three, and two. A name with fewer characters than stops wears the first as many stops as it has characters.
- **Blended.** Each character takes the colour at its position along the path through the stops, the first character at the first stop and the last at the last, the positions evenly spaced between. The path is piecewise linear in a perceptual space (OKLab), each segment between consecutive stops taking an equal share of the span, so a blend of two hues is short and even and never dips through grey. A one-character name wears its first stop, and a stop no character's position lands on is not worn — a two-character name with three stops wears its first and its last.

The clowder is diverse, and so is its vocabulary: the three dressings mix across the 46 resources, with no one dressing for all. Each coat is **chosen to suit its name** — a name that evokes a colour wears it, a name that evokes several wears several — rather than hashed from the name or dealt from the table's order. Hashing is quick and arbitrary, and arbitrary is exactly the wrong feel for the one place the theme is allowed to be fond. No two resources wear the same coat: the same stops in the same order with the same dressing is one coat, and one coat is one resource.

A coat is a fact about presentation, so the table of coats lives in this leaf's module beside the palette, not in the core's content tables. The core never knows a colour.

### The ring

Seven hues, every one in the same mid-luminance band as the state colours, with the chroma turned up so they read as lively on a dark ground without being bright.

| Hue | Value |
|---|---|
| poppy | `#D9534F` |
| clover | `#6E9E2E` |
| pond | `#2E9C8A` |
| sky | `#3F96D6` |
| bluebell | `#6A7FE0` |
| orchid | `#B45FC8` |
| blush | `#D4588E` |

There is no yellow. Within the band a yellow is the amber of `partial`, and a name wearing it beside a ◐ would be two things in one colour. The ring as a whole keeps clear of the five state colours — the green of `ready`, the amber of `partial`, the orange of `attention`, the violet of `milestone`, and the grey of `dim`, which a blend that sags toward grey would land on — and the clearance is **measured, not assumed**: every colour a coat can put on a character stands at a CIE76 colour difference of at least 18, the clearance floor, from each of the five. That holds for every stop, and for every position along a blended coat's path that a name of up to thirty-two characters can put a character at, which is finer than any name in the vocabulary. That is what bounds the blended dressing: a blend between two stops whose path passes closer than the floor to a state colour — clover to pond crosses the green of `ready`, poppy to clover crosses the amber of `partial`, poppy to pond sags into the grey of `dim` — is not an allowed coat, and the test names which pairs of stops may be blended. A patched or solid coat may use any stops, since it never puts a colour on a character that is not a stop.

### Where a coat is worn

A coat is worn where a resource's name **stands as a field** — a thing the eye scans and matches — and nowhere a name is set inside a sentence, which the eye reads. The fields:

- the store's name in the sidebar (render leaf: The sidebar);
- the store's name on its row in the stores group;
- the name of each component in a list row's cost field (TUI-RENDER-020), and in an open count prompt's preview, on the row and in the narrow swap's inspector alike (TUI-RENDER-021);
- in the inspector: the name line of a stores row, and the name at the head of each need-and-have cost line, unit-price line, and preview component (TUI-NAV-018, TUI-NAV-078, TUI-NAV-067).

Those are the fields, and nowhere else wears a coat. The inspector's cost lines are drawn in the inherited foreground whatever a component's shortfall — the shortfall is said in words beside the figures (TUI-NAV-078) — so a coat is worn there whether or not the store is short, which is exactly where the matching matters.

A name set in a sentence wears the sentence's colour, whatever the sentence says: the header's facts, the ticker, the report banner, the Journal, the away report, a footer note, the hint bar, an armed row's consequence summary, the inspector's plain statement of what a thing does, and the find palette's result lines. A coat there would be a highlight the sentence did not ask for, and a sentence read with one word in colour is read as being *about* that word.

Only the name wears the coat. The figures beside it, the `/` between a stock and its cap, a cost's separators, the `:` after a cost line's name, and every blank column wear the line's own colour, so a column of figures reads as one column.

### Which lines a coat may sit on

A coat is worn where the line is drawn in the inherited foreground, in `ready`, or in `partial`. On a ready or partial row the marker and the row's own name say the row's state and keep the row's colour, and the names in its cost field wear their coats: a state that says *you can act* loses nothing to a coat beside it, and the marker, which is the state's exact mark, never wears one.

A coat is not worn on three kinds of line, each of which is already saying one thing with its whole colour:

- **A line in `dim`.** A teased row, an owned or completed one-shot, an unaffordable row: a finished or out-of-reach row recedes whole (TUI-THEME-014, TUI-NAV-009), and a coat on its cost would make it the liveliest row on the list.
- **A line in `attention` or `milestone`.** An armed row's expansion is one signal, and a coat inside it would be a second colour inside the one that means "this wants you"; a line drawn whole in `milestone` is the same case.
- **A reversed line.** Focus is the terminal's own pair inverted (Three Things the Palette Does Not Cover), on the whole line; a coat under a reversal is a block of colour the design never chose.

### A shortened name

Where a name is shortened with an ellipsis (TUI-RENDER-014), each character kept wears the colour it has in the whole name, and the ellipsis — one cell — wears the colour of the character at its position in the whole name. A store's first letters therefore wear the same colours at every width the sidebar takes, so a store looks like itself whether or not it was cut.

## The Signature: the Pane Keeps the Season

One flourish on the chrome, and it comes from the subject rather than from decoration. The game tracks a season, and the chrome's accent — the header rule and the pane dividers (render leaf: The Frame, Horizontal Layout), nothing else — drifts with it across the year.

| Season | Accent |
|---|---|
| Spring | `#7FA05A` |
| Summer | `#C9A227` |
| Autumn | `#B5713C` |
| Winter | `#6E8BA8` |

A glance tells you where in the year the clowder is before you have read a word, and a pane left open for weeks slowly changes its light.

It is safe precisely because it is confined. **The seasonal accent never touches a colour that carries state, and never touches a coat.** If it tinted affordability, the player would have to ask whether green meant *affordable* or *spring* — the exact whimsy-over-clarity failure tenet 3 forbids. Inert chrome has no meaning to compete with, so the drift is free.

The accent is the season of the snapshot the frame draws, so it turns on the same frame as the header's season word, with no blend across the turn. A blend would need a frame to remember what it drew before, which no frame does, and a rule still wearing the old season beside the new season's name would be the one place the pane said two things about the year.

On the header rule the accent colours the underline alone. The facts and chips above it keep their own colours — a chip's glyph and word stay `attention` — so the accent sits beneath state without touching it. A terminal that does not colour underlines draws the rule in each cell's own colour, or draws none, and nothing is substituted: the season's name in the header already says everything the accent says.

Autumn's accent sits near `attention` in hue, and it is safe for the same reason the confinement is. The accent is only ever a line — an underline or a column of `│` — always present, always in the same places. `attention` is only ever worn by glyphs and words, and it appears only when something wants the player. The glance reads orange *appearing*; a line that never comes or goes gives it nothing to rule out. The coats are on the same side of that line as the accent: always present, never appearing.

## Colour Is Always the Second Channel

Every meaning colour carries is carried by something else first — a glyph, a word, or a fill. This is not an accessibility afterthought bolted onto a finished palette; it is why the state palette could stay this small.

| Meaning | First channel | Colour |
|---|---|---|
| Affordability | `●` `◐` `○` `✓` | `ready` / `partial` / `dim` |
| Attention | Glyph + count + word | `attention`, on the whole chip |
| Capped store | The word `full`, and a filled bar | `attention`, on the word |
| Armed row | Its expansion beneath itself, and its keys in the hint bar | `attention`, on the expansion |
| Milestone | `★` | `milestone` |
| Flavour vs mechanical | `·` vs `▪` | `dim` vs inherited |
| Which resource | The name itself | Its coat |
| Mood | The word itself | *(none)* |
| Season | The season's name in the header | Chrome accent |

Turn every colour off and the game is still completely playable — which is the actual test, and the one TUI-NAV-022 states. A coat passes it trivially: it carries nothing the name does not.

### The compressed run

The palette's riskiest axis for colour vision is `ready` against `partial` against `attention`, the green–amber–orange run that red-green deficiencies compress. For a player with such a deficiency these three colours are close to one colour, so between these three states the glyph is not a courtesy but the whole distinction. Two rules keep it whole.

**Each of the three has a non-colour channel the other two never use.** Affordable is `●`. Partially affordable is `◐`. Attention is carried by each chip's own glyph (TUI-NAV-020), by a word — the `full` tag — or, on an armed row, by the row's expansion beneath itself, whose summary is words (TUI-NAV-077); none of these is ever `●` or `◐`. Every glyph among them is a different single-width character (TUI-NAV-021); none differs from another only by its colour.

**Among the three colours, each glyph wears only its own.** `●` is never drawn in `partial` or `attention`, `◐` never in `ready` or `attention`, and a chip's glyph never in `ready` or `partial`. If any glyph could appear in a neighbour's colour, the two pairings would be one appearance to exactly the player this section is for — an amber `◐` and an orange `◐` are the same mark once amber and orange are the same colour. Dim lies outside the run and still recedes a marker as TUI-NAV-009 asks: a Teased row's `◐` is drawn dim. A coat lies outside the run too, and no glyph ever wears one: coats sit on letters, and a marker is never a letter.

The second rule matters most on an armed row, which wants notice while its marker is still saying something. The row wears `attention` on its expansion — the summary lines the arming opens beneath it, in the list or, while the narrow-mode swap holds the list's space, in the inspector — and nowhere on its own line. Its own line takes no `attention`: its marker keeps its availability colour, whatever that marker is, and the rest of the line is drawn as any row's is. The marker is the row's affordability channel, and colour on it says affordability only. The arming needs nothing from that line, because it has a non-colour channel of its own — the expansion, and its keys in the hint bar (TUI-NAV-077) — and the expansion is exactly where its colour sits. So the row's line says one thing in state colour, as focus is kept off the palette to ensure (Three Things the Palette Does Not Cover), and nothing about affordability is ever put in doubt by the arming.

Both rules hold whatever values the palette settles on (Deferred 1). Retuning the hexes may widen the three hues' separation for ordinary vision; it cannot be relied on to survive every deficiency, and the glyphs carry the state either way. The ring's clearance from the state hues is likewise a courtesy to ordinary vision — it keeps the vocabulary legible — and never a channel: to a player who cannot tell the ring from the run, a coat is still only ever on a name, and a state colour only ever on a mark.

## Three Things the Palette Does Not Cover

**Focus takes no colour.** The cursor is the `▸` glyph, and where more emphasis is wanted the row reverses. Giving focus a palette colour would break the rule that colour means state — the focused row's state is whatever its marker says, and tinting it would put two colours on the row's line saying different things. A reversed row wears no coat either (Which lines a coat may sit on). The focused tab on the tab bar reverses the same way, its label a block of the terminal's own pair inverted (TUI-NAV-092); a section in focus has no cursor to carry the glyph, so the reversal alone marks it.

**Reverse video inverts the terminal's own pair.** The offline banner and any emphasised row swap the terminal's foreground and background rather than reversing a palette colour. This works on a ground we do not own and in either polarity, and it is the one emphasis that needs no colour at all.

**Truecolor is checked, not assumed.** The HLD rules out designing degraded palettes, and that stands — but a tmux that has not been configured to advertise RGB will silently approximate every value to the nearest 256-colour cell, and the player sees a muddy version of a game they will reasonably assume just looks like that. Startup checks whether truecolor is advertised and, when it is not, says so once in plain words with what to set. One line, once, and the game plays on exactly as before: the difference between a known limitation and an apparent defect is entirely whether anyone said anything.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|---|---|---|---|
| What colour means | State on marks; identity, as a coat, on resource names | State only, names in the inherited foreground; a hue per resource tier or category | Resources are the nouns every screen shares, so a coat lets the eye match a price to its store and gives the stores a litter's variety; marks keep the state vocabulary at five, read everywhere. A tier hue says something mechanical the name does not, which is a sixth meaning to learn |
| Which identities wear a coat | Resources only | Every content kind; kittens too | A structure, idea, job, or kitten appears once, in its own list, with its name beside it; a coat there is a vocabulary with nothing to match against. Kittens also come and go, and a coat should be learned once |
| How coats are assigned | Chosen per name, in a table | Hashed from the name; dealt from the table's order | A hash is arbitrary, and arbitrary is the wrong feel for the one place the theme is fond; a chosen coat can suit its name |
| Where the coat table lives | This leaf's module, beside the palette | The core's content tables, beside each resource's definition | A coat is presentation; the core never knows a colour, and the sim's tables stay about rules |
| Dressings | Solid, patched, and blended, mixed across the vocabulary | One dressing for every coat | The clowder is diverse; one dressing for all reads as a scheme, three as a litter |
| Blend space | OKLab along the stops, with the stops of a blended coat restricted to pairs whose path keeps clear of the state colours, checked by measurement | RGB or HSL; blends between any two stops | A straight RGB blend dips through grey and an HSL blend sweeps through every hue between; an OKLab path is short and even, and measuring every intermediate is the only way to know a blend never lands on a state colour |
| The ring | Seven hues in the mid band, no yellow | A dark-tuned ring with a yellow, brighter than the state colours | In the band a yellow is `partial`'s amber. A brighter ring fails the light ground and out-shines the chip and the marker, so the signals the glance depends on go quiet beside the names |
| Ring clearance from the state colours | Measured in CIE76 against all five, `dim` included, at a floor of 18, for every colour a coat can put on a character | Assumed from the hue table; measured against the four hues only; a perceptual metric such as ΔE2000 | A hue table says nothing about a blend's intermediates; a blend can sag toward grey, which is `dim`; CIE76 is the difference the leaf's other separations are measured in, and one metric keeps every floor comparable |
| Stops within a coat | A solid has one; patched and blended have two to four; a hue may recur but never consecutively | Any count; distinct hues only | Two consecutive equal stops are a solid coat in disguise, so two resources could look alike while counting as distinct; a recurring hue apart from itself is what a checked coat is |
| Patched remainder | The first runs, as many as the remainder, take one extra character each | The first run takes the whole remainder | Near-equal runs is the look; a first run two longer than the rest is a lopsided coat |
| A blend's path | Piecewise linear in OKLab, equal share per segment, evenly spaced positions, a space counting as a position | A smooth curve; arc-length spacing; spaces skipped | A straight segment is the shortest path between two stops, so its intermediates stay nearest the stops; equal shares and counted spaces keep the rule the same under every dressing |
| Which lines carry a coat | Inherited, `ready`, and `partial` lines; never `dim`, `attention`, `milestone`, or a reversal | Every line; inherited lines only | A dim row recedes whole, an expansion or a milestone line is one signal, and a reversal is the terminal's own emphasis. Inherited lines only would leave every priced row's cost uncoloured, since a priced row always wears its marker's colour |
| Where a coat is worn | Where a name stands as a field; never in a sentence | Everywhere a resource is named; the sidebar alone | A field is scanned and matched, a sentence is read, and a sentence with one coloured word reads as being about that word. The sidebar alone leaves the prices, where the matching happens, uncoloured |
| What beside the name wears the coat | Nothing: figures, separators, and blanks wear the line's colour | The whole component, figure and name | A column of figures in mixed colours reads as several columns |
| A shortened name's colours | Each kept character keeps its colour; the ellipsis wears the replaced character's | Recomputed over the drawn text; the ellipsis in `dim` | A store should look like itself at every width, and a dim ellipsis would be a second meaning on the one glyph that already says "cut" |
| Background | Inherit the terminal's | Paint a designed ground; detect and switch | Inheriting is legible in light and dark with no detection, and looks native beside other panes; detection is unreliable under tmux and a wrong guess is worse than none |
| Palette size | Six roles | A fuller set with per-tier ramps | Every added state colour dilutes the signal that a state colour *is* meaning |
| Mood rendering | A word, no colour | A five-step colour ramp | The word already says it unambiguously; a ramp adds a channel that fails for some players and says nothing new to the rest |
| Seasonal accent | Chrome only, never state colours or coats | Tint the whole palette seasonally; no seasonal element | Confined, it is free delight; on state colours it would make the player ask whether a colour meant affordability or season, and on coats a store would change its look four times a year |
| Accent at a season's turn | Changes on the frame the season word changes, with no blend | A gradual blend between the two seasons' accents | A blend needs the frame to remember what it drew, and while it ran the rule would disagree with the season's name beside it |
| Luminance range | Mid-band throughout, coats included | Full range, tuned per background | The consequence of inheriting the ground; also pushes toward the calm the glance contract wants |
| Focus emphasis | Cursor glyph, and reverse where more is needed | A focus colour | A focus tint would put a second, unrelated colour in a row whose marker is already saying something |
| Where an armed row wears `attention` | On its expansion beneath it; none on its own line, whose marker keeps its availability colour | Tint the whole line, marker included; tint the line around a marker left in its availability colour | An orange `●` or `◐` is the same appearance as a green `●` or an amber `◐` to a red-green deficiency, so a tinted marker would put affordability in doubt; a line tinted around its marker carries two state colours saying different things, the arrangement focus is kept off the palette to avoid; the expansion is the arming's own mark and says nothing else |
| What wears `attention` | Each chip whole, the `full` tag alone, an armed row's expansion; one value everywhere, and nothing else | A distinct hue per surface; lending the role to other notices such as footer notes | One colour for one meaning is what lets an empty cluster and no `full` tag read as calm; a hue per surface is three colours to learn for one idea, and a notice that is not an opportunity dilutes the signal |
| The `★N new` chip's colour | `attention`, as every chip | `milestone`, matching the ★ on the tabs it counts | The cluster is read as one signal; a chip in another colour would make the player ask which colour means "come look", and the chip's own glyph and word already say what it concerns |
| Where a capped store wears `attention` | On the `full` tag alone | Tint the whole sidebar line; tint the fill bar with the tag | A tinted line colours the store's figures, which carry quantity rather than the state, and would cover its coat; the bar's cells can read full before the cap is reached, so only the word marks the state exactly |
| Footer notes | Inherited foreground | `attention` | A note answers the key just pressed, where the player is already looking, and lasts seconds; in `attention` it would be one more orange the glance must rule out, and a refusal in an opportunity's colour reads as a scolding |
| Missing truecolor | Detect and say so once; render unchanged | Assume it; ship a 256-colour fallback | A silent approximation reads as a defect in the game; a fallback palette is an explicit HLD non-goal |

## Open Questions & Future Decisions

### Deferred
1. The exact hex values want checking against real terminals — several popular themes use low-contrast backgrounds that a mid-luminance palette could sit too close to. The *roles* and their separation, and the ring and its clearance, are the design; the five state values, the seven ring values, and the four seasonal accents are a starting point to verify on a real pane, alongside the render leaf's other measurements.
2. Whether `●` and `◐` stay distinct enough. They are the affordable/partial pair, whose colours are the ones a red-green deficiency compresses hardest, so the whole distinction falls to the glyph — and the two differ only by fill. That is a real difference rather than a colour one, but half-filled glyphs render unevenly across fonts, and this wants checking on a real pane before it is trusted. The glyphs belong to the navigation leaf; changing them is a cascade there, not a decision here.
3. Whether `dim` needs to be a true colour at all, or whether the terminal's own dim attribute serves better. The attribute inherits the user's theme and is arguably more native; it is also inconsistently implemented across terminals, which is what makes this worth testing rather than deciding now.
4. Whether the find palette's result lines should wear coats. A result line names a row, and a stores row's result is a name standing nearly as a field; the palette is left uncoloured until the list of results has been looked at on a real pane with coats everywhere else.

## References

- Parent: `../tui-design.md` (truecolor-only, the thin-shell split)
- Sibling: `navigation/` owns the glyphs and words this palette sits behind — TUI-NAV-009 (markers), TUI-NAV-019 (mood as a word), TUI-NAV-020 (chips), TUI-NAV-021 (single-width glyphs), TUI-NAV-022 (the non-colour channel requirement)
- Sibling: `render/` draws the fields a coat is worn in — the sidebar, a list row's cost field, the count prompt's preview, and the inspector's cost lines (Numbers and Names) — and shortens names with the ellipsis a coat follows (TUI-RENDER-014)
- Consumes: `clowder_core::Season` for the chrome accent; `clowder_core::ResourceId` for the coat table
