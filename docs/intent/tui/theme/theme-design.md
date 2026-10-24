---
parent: tui
prefix: TUI-THEME
---

# Theme (colour, contrast, and the channel discipline)

## Context and Design Philosophy

This leaf owns every colour decision in clowder: the palette, what each colour is allowed to mean, the coats the stores wear, the fur a kitten's portrait is painted in, the contrast floor, the posture toward light and dark terminals, and the discipline that keeps the game fully playable by someone who cannot distinguish any of it.

The brief is unusual, and it should drive the design rather than be decorated over. Clowder is not a page someone visits; it is **a pane that sits beside real work for weeks**. The player looks at it for two seconds between compiles. It shares a screen with their editor, their logs, and their shell. A theme that announces itself is a theme that becomes irritating on day three — and irritating-on-day-three is a failure of tenet 1 as surely as anything grim would be.

So colour has exactly two jobs here, and the design keeps them apart. (One fenced picture uses colour for neither: a kitten's portrait is painted in its fur, which depicts the kitten and asks nothing of the reader — Fur, below.) **State** is the job the two-second glance depends on: whether a thing is affordable, whether a store is full, whether something wants the player. **Identity** is the job that makes the pane a clowder rather than a ledger: every resource wears a coat of its own, so the stores have the variety a litter has, and a name reads as the same thing wherever it stands. The glance contract in the navigation leaf — empty chip cluster means nothing needs you — only works if the eye has learned that a state colour *appearing* is information, which is why the two jobs never share a place: state colours sit on marks, coats sit on names, and nothing wears both.

## Two Jobs for Colour

The load-bearing decision. There are 46 resources, 40 structures, and 59 ideas. Of these only the resources are nouns every screen shares: a store stands in the sidebar, in the stores group, in every price on every list, and in the inspector's need-and-have lines, and the eye is forever matching a price's name to the store it draws on. So **resources wear coats, and nothing else does.** A structure, an idea, a job, or a kitten appears once, in its own list, with its name beside it; a colour of its own would be a vocabulary nobody learns, spent on information the name already gives for free.

**Marks wear state.** Whether a row is affordable, whether a store is full, whether something wants notice — these are what the five state colours say, and they are the same handful of states on every screen. A player learns five meanings once and reads them everywhere. The state colours are worn by glyphs, tags, chips, and whole lines; they are never worn by a name as a name.

**Names wear coats.** A coat is a resource's own colours, fixed for the life of the game, chosen for the name rather than for anything the store is doing, and worn wherever the name is drawn. A coat says nothing the name does not say; it is texture, learned by familiarity, never asked to be read. Because a coat is on every resource name the player sees, the eye files it with the names, and a state colour appearing is still the thing that stands out — the ring is measured clear of every state colour (The ring), so no coat is ever read as one.

This is also what keeps the state palette small enough to be disciplined about contrast and colour-vision safety.

One picture is the exception, and it is fenced: a kitten's **portrait**, in the inspector, is painted in the kitten's own fur (Fur, below). That colour is neither state nor identity. It depicts: it is what the kitten looks like, and it sits on block cells inside the portrait and its swatches, never on a name, a mark, or a word. A kitten's name still wears no colour of its own.

## The Ground Belongs to the Terminal

Clowder paints no background. Ordinary text is the terminal's own foreground on the terminal's own background, and only meaningful things take a colour from the palette below.

This is the choice that makes light and dark work without detecting anything. Terminal background detection is unreliable in general and worse under tmux, and a wrong guess is more damaging than no guess — a light-theme user handed a dark palette gets an unreadable pane. Inheriting sidesteps the question: the game is legible in a light terminal and a dark one because it is using that terminal's own contrast in both.

It also makes clowder look *native* beside the player's other panes rather than like a window pasted over them, which matters for something that lives in a tmux split for weeks. The cost is real and accepted: every palette colour — the state colours and the coats alike — must hold against both a near-black and a near-white ground, a WCAG contrast ratio of at least 3:1 against pure black and against pure white, which rules out both pale tints and deep shades. The palette is therefore mid-luminance throughout, a constraint that pushes toward calm, which is what the glance contract wanted anyway. The coats take their liveliness from chroma, not from brightness.

## The Palette

Six roles. Every value sits in a mid-luminance band so it clears the contrast floor against both extremes of ground.

| Role | Value | Carries |
|---|---|---|
| *(inherited)* | terminal foreground | All ordinary text — counts, costs, prose, and every name that wears no coat |
| `dim` | `#7C7873` | Revealed but not actionable, owned and done, flavour events, a list's group-header rules and the inspector's `last time` and `about` dividers, a kitten's favourites, spent hints |
| `ready` | `#3E8E5A` | ● affordable now |
| `partial` | `#A8802F` | ◐ partially affordable |
| `attention` | `#C0663A` | Each attention chip whole, the `full` tag on a capped store, a sidebar note's count of capped stores out of view, an armed row's expansion beneath it — and nothing else |
| `milestone` | `#7E6BA8` | ★ milestones and newly revealed content |

Mood takes no colour at all. The navigation leaf already renders it as a word (TUI-NAV-019), and a word is unambiguous, translatable to a screen reader, and immune to every failure mode a five-step colour ramp has. Where a channel is already carrying meaning well, adding colour is noise.

### What wears `attention`

`attention` means one thing — *this wants you* — and the glance contract depends on it meaning nothing else. An empty chip cluster and no `full` tag or full count is how the player reads, in two seconds, that nothing needs them, and the eye can only learn that if orange appearing is always that signal. So the role is not subdivided by surface, and it is not lent to anything that is merely information.

Five elements wear it, each on exactly the mark that says it wants notice:

- **An attention chip, whole.** Its glyph, its count where it has one, and its word all wear `attention`; the blank columns between chips, and the header's facts beside them, do not. This holds at every width step the header takes (TUI-NAV-086) — a chip that has shed its count is still a chip — and for every chip alike, the `★N new` chip included. That chip counts tabs wearing the `milestone` ★, but the chip itself is asking for the player, and a cluster in two colours would leave the eye to work out which colour means "come look".
- **The `full` tag on a capped store, and only the tag.** The store's name wears its coat, and its figures, fill bar, and rate are drawn as any store's are. The tag is present exactly while the store is capped (TUI-NAV-022), whereas the bar's cells can read full a little before the cap is reached, so the word is the state's exact mark and the colour sits on it.
- **A sidebar note's full count, and only that.** Where the sidebar is scrolled and stores stand out of view, a note at its head or foot counts them, and where any of them is capped it goes on to say how many, `· 2 full` (render leaf: Which stores it draws). The `2 full` wears `attention`, as the `full` tag it stands in for does; the note's triangle, its count of stores, its `more`, and the `·` before the full count stay `dim`, since they are information about the scroll and want nothing.
- **An armed row's expansion.** The summary lines beneath the row, and their copy in the inspector while the narrow-mode swap holds the list's space — never the row's own line (The compressed run). Where the layout leaves part of the expansion out of view, the colour stays on what is drawn of it and does not move onto the row's line or the hint bar in its place. The hint bar's armed keys are drawn as any hint is: they are the arming's word channel, not a second coloured mark.
- **The ticker's sky light, whole.** Its glyph, its word, its clock, and its key all wear `attention`, as the chip it gives a clock to wears it (navigation leaf: The ticker's line); the blank columns around it do not. The light and the chip are one opportunity, and one opportunity in two colours would be two things to learn. The ticker's other lights want nothing and wear none of it (The ticker, below).

All five wear the same `attention` value, whichever surface draws them, so there is one orange to learn.

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

A resource is one thing, and it looks like one thing wherever it is named: **every resource name the game draws wears its coat** — where the name stands as a field the eye scans and matches, and where a sentence names it — on every line whose colour leaves it room (Which lines a coat may sit on). A name plain in one place and coated in another would read as two things, and the player would have to learn which surfaces colour and which do not.

The fields:

- the store's name in the sidebar (render leaf: The sidebar);
- the store's name on its row in the stores group;
- the name of each component in a list row's cost field (TUI-RENDER-020), and in an open count prompt's preview, on the row and in the narrow swap's inspector alike (TUI-RENDER-021);
- in the inspector: the name line of a stores row, and the name at the head of each need-and-have cost line, unit-price line, and preview component (TUI-NAV-018, TUI-NAV-078, TUI-NAV-067);
- a store's result line in the find palette (TUI-NAV-063);
- in the inspector, the name on each line of what an act brought home last time (TUI-NAV-148).

The inspector's cost lines are drawn in the inherited foreground whatever a component's shortfall — the shortfall is said in words beside the figures (TUI-NAV-078) — so a coat is worn there whether or not the store is short, which is exactly where the matching matters.

The sentences:

- the inspector's description of a row — each resource named in the mechanical statement (TUI-NAV-018), whether the statement is stated from the definition's fields or is a blurb, whose marks say which of its words name a resource (curiosity leaf: Blurbs), and each resource named in the plain reason an item is held back (TUI-NAV-079);
- the ticker, where its line names a resource — the gains of a time away, or what an act brought home;
- the Journal, where its lines name one — each changed store in a pinned or kept away report, and each store an act brought home;
- a footer note, where it names one — the store a refused command is short of (TUI-NAV-014);
- and any line added later that names a resource: the rule is the name, not the surface.

A surface whose text names no resource — the header's facts, the hint bar, the help overlay — has nothing to colour. An armed row's consequence summary and the report banner do name resources, and keep their line's colour whole (Which lines a coat may sit on).

A sentence spells a resource as a sentence does: a blurb may name one in the singular or in lower case, as the craft that makes it, or at the start of a sentence, and a footer note names it in lower case. The coat is laid across the characters as the sentence spells them, by its dressing, as across a name of that many characters, so a blended coat still runs end to end and a patched coat still divides the word it is on. A coat is worn at its full strength wherever it is worn: a line drawn quiet, as the ticker is, keeps its quiet on its own words, and the names in it wear their coats as they do everywhere else, with no faint attribute — the same coat in every place is the point.

Only the name wears the coat. The figures beside it, the `/` between a stock and its cap, a cost's separators, the `:` after a cost line's name, the words and punctuation of a sentence around it, and every blank column wear the line's own colour, so a column of figures reads as one column.

### Which lines a coat may sit on

A coat is worn on a line drawn in the inherited foreground, in `ready`, or in `partial`, and on the ticker's newest event, whose `dim`, where a flavor event wears it, is its quiet rather than a state. On a ready or partial row the marker and the row's own name say the row's state and keep the row's colour, and the names in its cost field wear their coats: a state that says *you can act* loses nothing to a coat beside it, and the marker, which is the state's exact mark, never wears one.

A coat is not worn on three kinds of line, each of which is already saying one thing with its whole colour:

- **A row in `dim`.** A teased row, an owned or completed one-shot, an unaffordable row: a finished or out-of-reach row recedes whole (TUI-THEME-014, TUI-NAV-009), and a coat on its cost would make it the liveliest row on the list.
- **A line in `attention` or `milestone`.** An armed row's expansion is one signal, and a coat inside it would be a second colour inside the one that means "this wants you"; a line drawn whole in `milestone` is the same case.
- **A reversed line.** Focus is the terminal's own pair inverted (Three Things the Palette Does Not Cover), on the whole line, and the report banner is reversed the same way; a coat under a reversal is a block of colour the design never chose, since a reversal turns a text colour into a ground.

### A shortened name

Where a name is shortened with an ellipsis (TUI-RENDER-014), each character kept wears the colour it has in the whole name, and the ellipsis — one cell — wears the colour of the character at its position in the whole name. A store's first letters therefore wear the same colours at every width the sidebar takes, so a store looks like itself whether or not it was cut.

## The Ticker

The ticker's line is read at a glance, so it is drawn plainly. Its newest event is in the inherited foreground, and only a flavor event, which changes nothing, recedes into `dim`, as flavor does in the Journal. No faint attribute is laid over either: `dim` already sits at the contrast floor, and a faint attribute over it sinks below the floor on many terminals, leaving a line the player has to lean in to read. The names in it wear their coats (Where a coat is worn).

The lights at the line's right end say what runs on a clock (navigation leaf: The ticker's line). The sky's light wears `attention` whole, being its chip's clock (What wears `attention`). Every other light is a running effect that wants nothing of the player: its glyph and word are in the inherited foreground, and its clock in `dim`, the words saying what runs and the clock being the detail. A running light never takes a colour of its own: that would be a sixth state colour to learn for something its glyph and word already say, and the glance contract needs every colour that appears in the chrome to be either a coat, the season's accent, or one of the five states.

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

Autumn's accent sits near `attention` in hue, and it is safe for the same reason the confinement is. The accent is only ever a line — an underline or a column of `│` — always present, always in the same places. `attention` is only ever worn by glyphs and words, and it appears only when something wants the player. The glance reads orange *appearing*; a line that never comes or goes gives it nothing to rule out. The coats keep clear of it another way: they sit only on names, never on a glyph or a tag, and every colour a coat can put on a character is measured clear of `attention` (The ring), so a name in its coat — even one that has just arrived in the ticker — is never read as something wanting the player.

## Fur

Each kitten's profile carries its fur — a pattern and a colour for each of the pattern's slots — and its eyes (kittens leaf: Profiles). The inspector draws the kitten as a small portrait (render leaf: The portrait) and lists the fur's colours as a strip of swatches. This section says what those colours are and where they may go.

**Fur colours are true to life.** A black cat is black and a white cat is white. The palette's mid-luminance band exists so that every colour that carries meaning holds against both grounds; a fur colour carries none — the fur's words say everything it says (Colour Is Always the Second Channel, below) — so the band does not bind it, and a fur held to the band would draw every black cat charcoal and every white one grey. The cost is that a fur close to the terminal's ground nearly disappears against it: a black cat on a dark terminal, a white one on a light terminal. The portrait answers that with its outline, which is drawn in the inherited foreground, so the cat's shape stands against any ground whatever its fur. A swatch has no outline. A swatch the ground swallows still stands directly before its colour's name, so nothing is lost but the block.

| Fur colour | Value | | Fur colour | Value |
|---|---|---|---|---|
| black | `#1C1A1A` | | ginger | `#D9822B` |
| white | `#F4F1EA` | | cream | `#EED9B0` |
| grey | `#8A8D93` | | cinnamon | `#A0603A` |
| silver | `#C3C6CC` | | fawn | `#D4B79A` |
| brown | `#7A5536` | | seal | `#4A3328` |
| chocolate | `#5E3A26` | | lilac | `#B7A5B4` |

Surprising colours, which about one kitten in forty wears (kittens leaf: Once in a blue moon), are true to their names too:

| Surprising colour | Value | | Surprising colour | Value |
|---|---|---|---|---|
| lavender | `#B497D6` | | marigold | `#F2A93B` |
| periwinkle | `#8E9BEF` | | plum | `#8E4585` |
| seafoam | `#71D6B0` | | teal | `#2A9D8F` |
| rose | `#E58FA9` | | cornflower | `#6495ED` |

| Eye colour | Value | | Eye colour | Value |
|---|---|---|---|---|
| green | `#6DAF4F` | | amber | `#D6962A` |
| gold | `#E3C04A` | | copper | `#B8642E` |
| blue | `#5B9BD5` | | hazel | `#9C8A3E` |

**Where fur may go.** Fur colours and eye colours are worn only by the cells of a portrait and by swatches: a portrait's fur cells, its two eye cells, and each swatch's block. Every other character in and around them — the portrait's outline, its nose, the fur's and eyes' words, each swatch's colour name — is drawn in the inherited foreground. No fur or eye colour is ever worn by a name, a mark, a figure, or any word, and no state colour or coat is ever worn by a portrait's cell or a swatch, so the depiction never reads as a state and no state is ever mistaken for a cat.

**How a pattern lays its colours.** Each pattern assigns every fur cell of the portrait one of its slots, and the cell is painted in the colour that slot took. A pattern's rule is stated by where a cell sits on the cat — its part (top, back, chest, under, paw, tail), its patch (A, B, or neither), and its column — as the render leaf labels every fur cell of every pose (render leaf: The portrait), so one rule paints a pattern in any pose. A cell the rule does not name takes slot 0.

| Pattern | Slots | Rule |
|---|---|---|
| solid | 0 colour | every fur cell slot 0 |
| tabby | 0 base, 1 stripe | slot 1 on every fur cell in an even column — the stripes run across the body |
| tuxedo | 0 colour, 1 white | slot 1 on chest and paw cells — the white bib and socks |
| bicolour | 0 colour, 1 white | slot 1 on chest, under, and paw cells — the white front and belly |
| tortoiseshell | 0 dark, 1 warm | slot 1 on patch A — the warm patches |
| calico | 0 white, 1 warm, 2 dark | slot 1 on patch A, slot 2 on patch B — patches on white |
| colourpoint | 0 body, 1 point | slot 1 on paw and tail cells — the dark points |
| van | 0 white, 1 colour | slot 1 on top and tail cells — colour near the head and on the tail |
| smoke | 0 top, 1 undercoat | slot 1 on under cells — the pale undercoat showing beneath |

A pattern's rule follows its slots as the kittens leaf orders them, so a slot index means the same thing in both leaves; a test holds every pattern in the core's table to a rule here.

**Eyes.** Where the pose shows its kitten's eyes, the left eye cell is painted in the eye look's left colour and the right eye cell in its right colour, so odd eyes show as two colours. A napping kitten's closed eyes are outline, and its eyes are told in words alone.

**A swatch** is two cells of the full block `█`, both in one fur colour. The strip lists the fur's colours in slot order, each swatch followed by one blank column and the colour's name.

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
| A kitten's fur and eyes | The fur's words, the eyes' words, and each swatch's colour name | The portrait's cells and the swatches |
| Mood | The word itself | *(none)* |
| Season | The season's name in the header | Chrome accent |

Turn every colour off and the game is still completely playable — which is the actual test, and the one TUI-NAV-022 states. A coat passes it trivially: it carries nothing the name does not.

### The compressed run

The palette's riskiest axis for colour vision is `ready` against `partial` against `attention`, the green–amber–orange run that red-green deficiencies compress. For a player with such a deficiency these three colours are close to one colour, so between these three states the glyph is not a courtesy but the whole distinction. Two rules keep it whole.

**Each of the three has a non-colour channel the other two never use.** Affordable is `●`. Partially affordable is `◐`. Attention is carried by each chip's own glyph (TUI-NAV-020), and by the same glyph and word on the ticker's sky light (TUI-NAV-152), by a word — the `full` tag, or the `full` of a sidebar note's count — or, on an armed row, by the row's expansion beneath itself, whose summary is words (TUI-NAV-077); none of these is ever `●` or `◐`. Every glyph among them is a different single-width character (TUI-NAV-021); none differs from another only by its colour.

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
| Which lines carry a coat | Inherited, `ready`, and `partial` lines and the ticker; never a `dim` row, an `attention` or `milestone` line, or a reversal | Every line; inherited lines only | A dim row recedes whole, an expansion or a milestone line is one signal, and a reversal turns a text colour into a ground. The ticker's dim is its quiet, not a state, so its names wear their coats as everywhere. Inherited lines only would leave every priced row's cost uncoloured, since a priced row always wears its marker's colour |
| Where a coat is worn | On every resource name drawn, in fields and in sentences alike, except on a line whose whole colour says one thing | Fields only; fields and the inspector's description only; the sidebar alone | A resource is one thing and should look like one thing wherever it is named; a name coated on one surface and plain on another reads as two things, and leaves the player to learn which surfaces colour. A coloured word in a report draws the eye to it, which is accepted: the resource is usually what the line is about. The sidebar alone leaves the prices, where the matching happens, uncoloured |
| The colour of the ticker's lights | The sky's in `attention`, as its chip; every other light's glyph and word in the inherited foreground and its clock in `dim` | A colour of its own for running lights; every light in `attention` | A running effect wants nothing, so `attention` on it would light the corner whenever a boost ran and break the glance contract; a colour of its own is one more state to learn for what the glyph and word already say |
| The ticker's newest event | Inherited foreground, a flavor event in `dim`; no faint attribute | `dim` with the terminal's faint attribute over every line | `dim` alone sits at the contrast floor, and the faint attribute over it falls below the floor on many terminals: a line the player must lean in to read is not glanceable |
| A coat on a quiet line | Full strength, the line's quiet kept on its own words | The coat dimmed or faint with the line | The same coat in every place is the point of a coat; a faint coat in the ticker is a second look for the same store |
| A name a sentence spells otherwise | The coat laid across the characters as the sentence spells them | Coat only a spelling that matches the stores' exactly; compute the coat over the stores' spelling and cut it to the word | Sentences say "a twig" and "catnip"; coating only exact spellings leaves most of a blurb's resources plain. Laying the dressing across the word as written keeps each dressing's look on every spelling, where a cut coat would stop a blend partway |
| What beside the name wears the coat | Nothing: figures, separators, and blanks wear the line's colour | The whole component, figure and name | A column of figures in mixed colours reads as several columns |
| A shortened name's colours | Each kept character keeps its colour; the ellipsis wears the replaced character's | Recomputed over the drawn text; the ellipsis in `dim` | A store should look like itself at every width, and a dim ellipsis would be a second meaning on the one glyph that already says "cut" |
| A kitten's portrait | Painted in its true fur and eye colours on block cells, outline in the inherited foreground | No portrait, the fur in words alone; fur colours held to the mid-luminance band; a painted ground behind the portrait | Words alone lose the delight the portrait is for. Held to the band, every black cat draws charcoal and every white one grey, which is not the cat. A painted ground breaks the rule that the ground is the terminal's, and in one polarity or the other it is a patch pasted on the pane. The outline in the inherited foreground keeps the shape on any ground at no cost to the fur |
| Where fur colour may go | A portrait's cells and swatch blocks only, never a name, mark, figure, or word | A kitten's name in its fur colour | A name in fur colour is a coat on a kitten, a vocabulary nothing matches against (Which identities wear a coat); and fur colours are outside the band, so text in them would fail the contrast floor |
| Background | Inherit the terminal's | Paint a designed ground; detect and switch | Inheriting is legible in light and dark with no detection, and looks native beside other panes; detection is unreliable under tmux and a wrong guess is worse than none |
| Palette size | Six roles | A fuller set with per-tier ramps | Every added state colour dilutes the signal that a state colour *is* meaning |
| Mood rendering | A word, no colour | A five-step colour ramp | The word already says it unambiguously; a ramp adds a channel that fails for some players and says nothing new to the rest |
| Seasonal accent | Chrome only, never state colours or coats | Tint the whole palette seasonally; no seasonal element | Confined, it is free delight; on state colours it would make the player ask whether a colour meant affordability or season, and on coats a store would change its look four times a year |
| Accent at a season's turn | Changes on the frame the season word changes, with no blend | A gradual blend between the two seasons' accents | A blend needs the frame to remember what it drew, and while it ran the rule would disagree with the season's name beside it |
| Luminance range | Mid-band throughout, coats included | Full range, tuned per background | The consequence of inheriting the ground; also pushes toward the calm the glance contract wants |
| Focus emphasis | Cursor glyph, and reverse where more is needed | A focus colour | A focus tint would put a second, unrelated colour in a row whose marker is already saying something |
| Where an armed row wears `attention` | On its expansion beneath it; none on its own line, whose marker keeps its availability colour | Tint the whole line, marker included; tint the line around a marker left in its availability colour | An orange `●` or `◐` is the same appearance as a green `●` or an amber `◐` to a red-green deficiency, so a tinted marker would put affordability in doubt; a line tinted around its marker carries two state colours saying different things, the arrangement focus is kept off the palette to avoid; the expansion is the arming's own mark and says nothing else |
| What wears `attention` | Each chip whole, the `full` tag alone, a sidebar note's full count, an armed row's expansion; one value everywhere, and nothing else | A distinct hue per surface; lending the role to other notices such as footer notes | One colour for one meaning is what lets an empty cluster and no `full` tag read as calm; a hue per surface is three colours to learn for one idea, and a notice that is not an opportunity dilutes the signal |
| The `★N new` chip's colour | `attention`, as every chip | `milestone`, matching the ★ on the tabs it counts | The cluster is read as one signal; a chip in another colour would make the player ask which colour means "come look", and the chip's own glyph and word already say what it concerns |
| Where a capped store wears `attention` | On the `full` tag alone | Tint the whole sidebar line; tint the fill bar with the tag | A tinted line colours the store's figures, which carry quantity rather than the state, and would cover its coat; the bar's cells can read full before the cap is reached, so only the word marks the state exactly |
| Footer notes | Inherited foreground | `attention` | A note answers the key just pressed, where the player is already looking, and lasts seconds; in `attention` it would be one more orange the glance must rule out, and a refusal in an opportunity's colour reads as a scolding |
| Missing truecolor | Detect and say so once; render unchanged | Assume it; ship a 256-colour fallback | A silent approximation reads as a defect in the game; a fallback palette is an explicit HLD non-goal |

## Open Questions & Future Decisions

### Deferred
1. The exact hex values want checking against real terminals — several popular themes use low-contrast backgrounds that a mid-luminance palette could sit too close to. The *roles* and their separation, and the ring and its clearance, are the design; the five state values, the seven ring values, and the four seasonal accents are a starting point to verify on a real pane, alongside the render leaf's other measurements.
2. Whether `●` and `◐` stay distinct enough. They are the affordable/partial pair, whose colours are the ones a red-green deficiency compresses hardest, so the whole distinction falls to the glyph — and the two differ only by fill. That is a real difference rather than a colour one, but half-filled glyphs render unevenly across fonts, and this wants checking on a real pane before it is trusted. The glyphs belong to the navigation leaf; changing them is a cascade there, not a decision here.
3. Whether `dim` needs to be a true colour at all, or whether the terminal's own dim attribute serves better. The attribute inherits the user's theme and is arguably more native; it is also inconsistently implemented across terminals, which is what makes this worth testing rather than deciding now.

## References

- Parent: `../tui-design.md` (truecolor-only, the thin-shell split)
- Sibling: `navigation/` owns the glyphs and words this palette sits behind — TUI-NAV-009 (markers), TUI-NAV-019 (mood as a word), TUI-NAV-020 (chips), TUI-NAV-021 (single-width glyphs), TUI-NAV-022 (the non-colour channel requirement)
- Sibling: `render/` draws the fields a coat is worn in — the sidebar, a list row's cost field, the count prompt's preview, and the inspector's cost lines (Numbers and Names) — and shortens names with the ellipsis a coat follows (TUI-RENDER-014)
- Sibling: `render/` draws the portrait whose cells this leaf paints (The portrait)
- Consumes: `clowder_core::Season` for the chrome accent; the profile's fur pattern, fur colours, and eye look (kittens leaf: Profiles) for the portrait; `clowder_core::ResourceId` for the coat table; the core's reading of a blurb as text and resource references (curiosity leaf, SIM-CUR-014) for the resources a description names
