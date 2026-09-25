---
parent: tui
prefix: TUI-NAV
---

# Navigation & Input (the UX grammar)

## Context and Design Philosophy

This leaf owns how a human drives clowder from a keyboard: the key model, section navigation, the action model (select / act / repeat / bulk / confirm), the inspector, the glanceable chrome, discoverability, and how events and the offline report surface. It exists as its own leaf because the UX is the part the player *lives in* — it constrains the theme, the render layout, and the runtime, so it is designed first and in full.

**The one-sentence grammar:** sections are digits, items are a cursor, Enter does the thing, Space tells the truth. Every player-facing verb in `clowder_core::Command` is reachable as a *row in a list* (the discoverable backbone) and the frequent ones also carry a single-letter mnemonic (the speed layer). Nothing a player can only reach through a mnemonic or the palette; those are pure accelerators.

The design is bounded by the HLD's tmux-first key constraint: the only usable keys are **plain printable characters, Tab/BackTab, Enter, Esc, Space, Backspace, and the arrow keys** — no Ctrl chords (Ctrl-C is emergency-quit only), no Alt/Meta, no function keys. **Uppercase is treated as the modifier tier** (Shift emits plain printable characters, which tmux never intercepts), recovering most of what chords would have given us.

## The Four Layers

A cold player learns the scheme in order; layer 1 alone plays the whole game.

1. **Backbone (minute one):** `↑`/`↓` move a list cursor, `Enter` performs the focused row's primary action, `Tab` moves to the next section.
2. **Truth (minute two):** `Space` opens the inspector on the focused row — exact costs, rates, effects, and *why* a thing is locked.
3. **Speed (hour one):** digits `1`–`0` jump to sections; per-screen letter mnemonics mirror the hottest rows; `x` acts with a typed count, `X` acts to the max affordable.
4. **Fast travel (week one):** `/` opens a find-anything palette over all *revealed* content.

Why a cursor is the backbone: 53 structures + 59 ideas + 35 refinements cannot be addressed by 26 letters or 10 digits, and progressive reveal inserts rows over time — a per-item hotkey scheme would relabel every key below an insertion (the renumbering hazard). A cursor scales to arbitrarily long, growing lists and is the most discoverable idiom in terminal software.

## The Key Map

### Global (a screen focused, no prompt/overlay open)

| Key | Action |
|---|---|
| `1`–`9`, `0` | Jump to that section (static map — see Navigation). An unrevealed digit is a soft no-op with a footer whisper. |
| `Tab` / `BackTab` | Next / previous *revealed* section |
| `l` / `h` | Synonyms for next / previous section (full hjkl) |
| `→` / `←` | Synonyms for next / previous section |
| `↓` / `↑` | Move the list cursor |
| `j` / `k` | Synonyms for cursor down / up (full hjkl) |
| `[` / `]` | Jump to previous / next group header within the list |
| `Enter` | Focused row's primary action ×1; commits an armed row |
| `Space` (`i` synonym) | Toggle the inspector on the focused row |
| `x` | Open the count prompt for the focused row (digits captured); `Enter` applies ×N |
| `X` | Focused row's action × max affordable |
| `+` / `=` | Increment on count rows (assign a worker/tinkerer) — Enter synonym there |
| `-` / `Backspace` | Decrement on count rows (unassign) |
| `/` | Find-anything palette over revealed content; `Enter` jumps to the top result |
| `e` | Journal overlay: full event scrollback + pinned offline/nap reports |
| `o` | Observe the sky (`ObserveSky`) — meaningful while the `!sky` chip shows; else a footer whisper |
| `<` / `>` | Game speed down / up one notch on the ladder 0.25 → 0.5 → 1 → 2 → 4 → 8 (`SetSpeed`); current speed is always in the header |
| `?` | Help overlay: full key map + glyph legend |
| `q` | Quit immediately (the shell saves first); no confirmation — quitting is always safe |
| `Esc` | Close the topmost transient (prompt, palette, overlay, armed row, banner). On a bare screen: **no-op** |
| `Ctrl-C` | Emergency quit; saves first |

### Prompt contexts (count prompt, palette field)

| Key | Action |
|---|---|
| any printable char / `Space` (palette) | Typed into the query — letters, digits, symbols, and `Space` alike |
| digits `0`–`9` (count prompt) | Typed into the count |
| any other printable char / `Space` (count prompt) | Nothing — no text and no footer, as for any key with no binding in its context; a count is digits only, so there is nothing else for these keys to mean |
| `Backspace` | Delete the last character (nothing on an empty field) — never the global decrement |
| `↑` `↓` `←` `→`, `Tab` / `BackTab` | Nothing — the field holds the keyboard, so the cursor and section beneath it stay put |
| `Enter` | The field's own — the count prompt applies ×N (or cancels on an empty or zero count); the palette jumps to its top result, or does nothing with no query or no match. The press is spent on the field: the row beneath is reached only through the field's own act (the count prompt's ×N), never as a plain `Enter` on it; an armed row beneath is never committed by it; and holding it on never acts on the row the field returns to |
| `Esc` | Cancel; nothing happened |
| `Ctrl-C` | Emergency quit, as everywhere |

In both fields every global single-key binding is suspended, which is how digits stop being section-jumps while typing. The palette is a field under this table, not a full-screen overlay: it captures the speed keys and `q` as query text, where the Journal and help keep them live. A field never expires — it waits, the sim ticking beneath it, until `Esc` closes it or an `Enter` the field acts on does: the count prompt's `Enter` always closes it, the palette's only when there is a result to jump to. Walking away mid-type blocks nothing and loses nothing. While a field is open the hint bar shows that field's keys, even over an armed row beneath it, because the field is what `Enter` will reach.

### Per-screen mnemonics (shown in the hint bar; each mirrors a visible row)

| Section | Keys |
|---|---|
| Meadow | `g` gather · `f` forage · `r` rummage (appears when its row does) |
| Clowder | `p` promote focused kitten · `n` appoint focused kitten Elder (job rows use `+`/`-`) |
| Friends | `p` deepen the pen-pal with the focused colony |
| others | none — rows + Enter suffice; letters are spent only where frequency demands |

**Reserved keys that may never become a screen mnemonic:** `h j k l x e o q` and uppercase `X`, plus the digits and `/ < > ? Space Enter Esc Tab Backspace`. (Full hjkl was the player's choice; `h`/`l` cost two letters, but no screen wants them — mnemonics live on `g f r p n`.)

## Navigation & the Section Graph

Sections map to digits **statically, assigned at design time, never renumbered.** A section that unlocks late slots into its reserved digit; a key the player already knows never changes meaning.

| Digit | Section | Commands it hosts |
|---|---|---|
| `1` | **Meadow** (home) | GatherCatnip, Forage, Rummage, Build; a "stores" group lists every revealed resource as an inspectable row; energy & mess readouts |
| `2` | **Clowder** | AssignJob/UnassignJob, Promote, AppointElder, HostJamboree, Pounce(n) |
| `3` | **Ideas** | ResearchIdea, BuyRefinement, AdoptRule (three groups) |
| `4` | **Crafts** | Craft, AssignTinkerer/UnassignTinkerer |
| `5` | **Sunbeams** | Bask, Reverie, BuyRite, Enlighten |
| `6` | **Wonders** | HoldParade, BuyArch, RefineStarshard, RefineEcho, CondenseJar, MakePromise |
| `7` | **Friends** | Trade(colony, n), SendPenPal, ScoutPicnic |
| `8` | **Sky** | LaunchExpedition, SummonStarWhale, FeedWhale; whale exchange routed here while a whale visits |
| `9` | **Dreams** | DeepNap(years), BuyDreamLoom |
| `0` | **Legacy** | GreatMeadowMove, PassItDown, BuyTradition, DeclareAdventure, Family Album |

Global commands with no section: SetSpeed (`<`/`>`) and ObserveSky (`o`). Journal (`e`) and Help (`?`) are overlays, not sections. All ~40 command variants are covered.

**Growth under progressive reveal.** A tab *exists* the moment its first row is revealed; before that it is absent from the tab bar (tab names are content, so no teasing tabs). Because digits are pre-assigned, a late tab appears in its fixed slot and no learned key shifts. A newly appeared tab shows a `★` badge (glyph, not color) until first visited, and the Journal logs its arrival. `Tab`/`h`/`l`/arrows cycle only revealed tabs.

**Getting home & composition.** Home is `1` (Meadow), one keypress from anywhere. Spatial movement is uniform: `h`/`l`/`←`/`→`/`Tab` across sections, `j`/`k`/`↑`/`↓` within a list, `[`/`]` for long-list group skips, `/` for fast travel. Each screen remembers its focused item by **content identity, not index**, so returning restores your place even if reveals inserted rows.

## The Action Model

Every actionable thing is a row. Row anatomy (one aligned line):

```
▸ ● Sunny catnip patch    ×2    14 catnip
```

`▸` cursor · availability marker · flavor name · owned count · next cost. Markers are **glyph-first, color-second** (CVD-safe): `●` affordable now, `◐` partially affordable, `○` revealed but not affordable, `✓` owned/done for one-shots. `Availability::Teased` rows render dim *with* their marker; Hidden content is absent.

**Repeat:** the cursor does not move on action, so holding `Enter` repeats via terminal key-repeat — but the shell **rate-limits repeated Enter to ~10/second** so behavior is uniform across machines (not hostage to the OS key-repeat rate). Repeat is for ordinary rows only: a heavy row's second Enter must be a *distinct* press, so a held key arms and stops there (TUI-NAV-051). Auto-repeat is a machine typing, not a player deciding, and the armed-row pattern exists precisely to make the deciding press deliberate — letting key-repeat satisfy it would defeat the whole mechanism at 10 commits per second. For the same reason `x` and `X` never reach a heavy row at all.

**×N:** `x` opens a count prompt that captures digits and live-previews total cost; `Enter` fires the command N times, or passes N where the command takes a count (`Pounce(n)`, `Trade(c, n)`, `DeepNap(n)`).

**×max:** `X` loops the command until the core rejects it (all-or-nothing spends make this safe), under a shell-side iteration cap. Where the core exposes a closed-form max, the hint bar names it ("X builds 7").

**Count rows** (jobs, tinkerers): `+`/`Enter` assigns one, `-`/`Backspace` unassigns one, `x`/`X` scale.

**Confirm vs. instant — the armed-row pattern (no modals, ever).** Ordinary purchases are instant; the core's all-or-nothing spend plus the affordability markers make that safe, and a rejection renders as a ~4-second footer flash translating the `CommandError` to plain words ("Not enough catnip — need 12.4 more"), never a dialog. The **heavy, hard-to-undo** actions — GreatMeadowMove, PassItDown, AdoptRule (locks its sibling for the meadow), and DeepNap (after its count prompt) — use an inline two-step: the first `Enter` *arms* the row, expanding it in place to a consequence summary rendered from real state ("Move to a new meadow: N kittens honored as Elders, basket carries M, chest carries P% of stores — Enter to move, Esc to stay"); the second `Enter` commits. Arming auto-expires after ~8 seconds or on any navigation, silently. This is confirmation without blocking: the sim keeps ticking, the rest of the screen keeps rendering, and walking away simply disarms. (`q` is deliberately *not* armed — quitting loses nothing, so it is instant.)

## The Inspector — truth one keypress away (often zero)

- **Wide layouts (≥ ~100 cols):** the inspector is a persistent right-hand pane, live-updating for the focused row. Truth is zero keypresses away.
- **Narrow (80×24):** `Space` swaps the inspector *in place of* the list (Esc or Space returns) — not a popup; the chrome and sim keep rendering.

Contents, rendered from `content.rs` defs + `state.rs` queries: flavor name + blurb + the **plain mechanical statement** ("Produces 0.63 catnip/s per copy, seasonal"); exact next cost with have/need per component, each marked `✓` or short-by-how-much, plus **time-to-afford** at current net rate; owned count, cap contributions, happiness/mess/energy effects, per-worker yield after mood for jobs; and **why-locked** for Teased/locked things, stated plainly from the def tables.

## Glanceability — the persistent chrome

Always on screen, at every size:

1. **Header (1 line):** clowder name/generation · season+day+year · kitten count · **mood as a word** · speed (`×1`) · right-aligned **attention chips**.
2. **Tab bar (1 line):** revealed tabs with digits; `★` on unvisited new tabs.
3. **Resource sidebar (left column):** a curated glance — revealed resources with stock/cap, a 4-cell fill bar, and signed rate; a text `full` tag when capped. Full truth is in Meadow's "stores" rows.
4. **Log ticker** (1 line at 80×24, up to 4 lines wide): latest events, dimmed.
5. **Hint bar (1 line):** context keys.

**The two-second glance contract.** The header's attention-chip cluster is empty when nothing waits. Chips are curated *opportunities*, never obligations, each rendered glyph + count + word (triple-channel, CVD-safe):

- `◦N idle` — kittens with no job *who are in the meadow to be assigned* (free production you're leaving on the table); a kitten visiting the neighbour's yard is not counted, because the chip would then ask for an action the player cannot take
- `!sky` — an observable sky-event window is open (act with `o`; expires harmlessly)
- `★N new` — content revealed since you last looked (clears on visiting)
- `~away` — an unread "while you were away" report waits (clears on reading)

Deliberately excluded: any "something is affordable now" chip — it would never be quiet, and quiet is the whole point. If the chip cluster is empty and no bar reads `full`, nothing needs you.

**Glyph discipline:** UI symbols are single-width geometric/box-drawing characters only (`● ○ ◐ ▸ ▓ ░ ★ · ! ◦ ~`). No emoji anywhere — ambiguous-width glyphs break column math across terminals and tmux. Whimsy lives in the words and the color, not the symbols.

## Discoverability — the first 60 seconds

Cold open: only Meadow exists, one focused row ("Gather catnip"). The hint bar reads, in full:

```
 ↑↓ choose · Enter do · Space details · Tab next · ? all keys
```

The only possible actions teach the backbone. As surface grows, the hint bar adapts by context, most-relevant-first, always ending with `? keys`:

- buildable focused: `Enter build · x amount · X max · Space details`
- job row focused: `+ assign · - unassign · x amount`
- kitten focused: `p promote · n name Elder · Space details`
- armed row: `Enter yes, truly · Esc not today`
- count prompt: `digits amount · Enter go · Esc never mind`
- palette: `type to find · Enter go · Esc never mind`

An open field's line wins over every other context, an armed row's included, since the field holds the keyboard.

The first time a count-taking row is focused, the hint bar surfaces `x amount` — the speed layer is taught lazily, at the moment of relevance. `?` shows the complete key map and glyph legend. There is no manual; the hint bar and `?` are the manual.

## Event Log & Offline Report

- **Ticker + Journal.** Typed `Event`s render into one dim ticker line (or a strip when wide). `e` opens the Journal overlay: full scrollback, `↑↓`/`j`/`k` scroll, `[`/`]` by day, Esc closes. Flavor events are quieter (dim, `·` prefix) than mechanical ones (`▪`); milestones get `★`. Nothing flashes, blocks, or expires unseen — the Journal keeps it all.
- **Offline report.** On load, `OfflineReport` renders as (a) a one-line banner under the tab bar — reverse-video, not color-only: `~ while you were away (2d 4h): +12.4k catnip, 3 kittens arrived · e for the tale · Esc dismiss` — and (b) a pinned Journal entry with the full folded detail. The one line is *curated*, not a summary of the whole struct: elapsed span, the largest resource gains, and arrivals. Wanderings and other absences live in the pinned entry, where they are stated as what they are — a visit that reverses on its own, never a loss or a count going down. The banner is the first thing a returning player sees, so it leads with what grew. The banner dismisses on Esc or first navigation and demands nothing; the pinned entry means dismissing loses nothing. A `truncated` report renders as gentle flavor. `Event::Napped` reports reuse the same path — one mechanism for "time passed in bulk."

## Edge Cases

- **80×24 (floor):** header 1 + tabs 1 + body 20 (sidebar | list) + ticker 1 + hints 1. The body is whatever the chrome leaves — the `render` leaf states it as a formula so the rows always sum to the terminal's height. The inspector becomes Space-toggled; the log becomes a 1-line ticker; the offline banner replaces the group-header line when present. Everything stays reachable; degradation removes *simultaneity*, never capability. Below 80×24, render a calm "this pane is too cozy — 80×24 needed" card; the sim ticks regardless.
- **Very wide (~120+):** three columns — sidebar | list (capped ~60 cols for scannability) | inspector (rest). Extra width feeds the inspector and log.
- **Resize:** layout is a pure function of terminal size, recomputed per frame with breakpoints at 100 and 120 cols; cursor, per-screen memory, armed state, and open overlays survive resize (an armed row that no longer fits re-renders compact, still armed).
- **Keys inside a full-screen overlay:** the Journal and help overlays take the cursor, section, and action keys for their own navigation, suspending those global meanings while open. The speed keys, `q`, `Ctrl-C`, and `Esc` stay live — retiming the sim, leaving, and closing must work from anywhere, and all four are safe wherever they land. (The count prompt and the palette field differ: they capture *all* printable input, which is what stops a digit from jumping sections mid-type.)
- **Esc under tmux `escape-time`:** Esc *only* closes the topmost transient; on a bare screen it is a no-op. No Esc-prefixed sequences exist anywhere, so escape-time coalescing can never form a phantom binding, and a late Esc can at worst close an overlay — never act, quit, or lose anything. Quit is `q` or Ctrl-C; both save.

## Resolved Edge Cases

A self-probe of this leaf's own gaps; each is pinned so an implementer never guesses a load-bearing default.

- **Enter on a Teased/unaffordable row** always *attempts* the command; the core rejects (all-or-nothing), and the rejection renders as the plain-language footer (TUI-NAV-014). There is no separate "can't afford" pre-check in the shell — the core is the single arbiter, and the footer names what's short.
- **A printable key with no binding in the current context** is ignored with no footer noise (only *meaningful-but-not-now* keys, like `o` outside a sky window, get a footer whisper).
- **`x`/`X` on a command that is not count-taking and not usefully repeatable** (e.g. a one-shot rite already owned) is a no-op with a footer ("nothing to repeat here"); on an owned one-shot (`✓`) row, Enter/`x`/`X` are all no-ops ("already yours").
- **Count semantics.** Each row's command is tagged *count-taking* (Pounce, Trade, DeepNap — the count is a real argument: squads, caravans, years) or *repeat-N* (Build, Craft, buys — the shell calls the command N times). `x` passes N to a count-taking command and repeat-calls a repeat-N one. An empty count or `0` cancels. For repeat-N, calls run until one is rejected — partial success is intended (you build as many as you can afford; the core's per-call all-or-nothing keeps each build clean). For count-taking, N is passed through and the core clamps or rejects.
- **The DeepNap sequence is:** `x` → type years → `Enter` sets the count *and arms* the row (showing "nap N years: costs S starshards, wakes in year Y" from real state) → a second `Enter` commits. So the count prompt's Enter arms; the following Enter fires.
- **Armed rows are advisory UI only.** The second Enter re-runs the *real* command through the core, which re-validates against current state — so if the clowder dropped below 71 kittens between arming and committing the Move, the commit simply fails to the footer. Arming another heavy row (or any navigation) disarms the first.
- **Palette results.** The palette's universe is the section lists themselves. A result is an *item* — the thing a row is about — in a revealed section's list. Most items carry one row; some carry several adjacent rows in their section (a craft's make row and its tinkerer count row, a kitten's promote and Elder rows, a colony's trade and pen-pal rows), and such an item is one result. Every item with a selectable row in a revealed section is a result — content, the fixed action rows, the stores rows, kittens, colonies, sites, and the declare-list rows alike — and nothing else is: group headers are never results, and an item with no row (Hidden content, an undiscovered colony, a gated action) is never one. So Teased items are in, because Teased rows are listed, and a kind the core reports only as present or absent (TUI-NAV-053) is in exactly when present. Drawing from the lists means every result lands somewhere and nothing the cursor can reach is missing. Each result shows the name its rows display, beside its section's digit, and each item is its own result — two items sharing a name are never merged, so neither silently vanishes from the list. The results are re-derived from current state as the sim ticks beneath the open palette, so they always match what the sections would show now, and `Enter` takes the top result as listed at the moment of the press.
- **Palette matching and order.** The query matches as typed, spaces included, and every comparison ignores case. A result matches when its name contains the query. Matches rank by closeness — a name equal to the query, then names beginning with it, then names containing it elsewhere — with ties in tab-bar order, then list order. A query that is empty or nothing but spaces is no query: it lists every result in tab-bar then list order and aims at nothing, since a stray `Space` is not a search. The list is drawn from its top, so the result `Enter` takes is always the first one shown and always on screen, however many follow it.
- **Palette pick.** `Enter` closes the palette, moves to the top result's section, and focuses that item's first row there — it performs no row action. With no query or no match, `Enter` does nothing and the palette stays open as typed: there is no aimed result, and a jump the player did not aim is a surprise, not a shortcut; `Esc` is the way out. The jump is navigation, so it does what any other move does: it disarms an armed row, dismisses the report banner, and counts as a visit to the section it lands in. It moves the cursor, not the layout — with the narrow-mode inspector swapped in, the swap stays and shows the landed row, as it follows every other cursor move.
- **A field's accepting Enter** is spent on the field — the count prompt's ×N, the palette's jump. A held or auto-repeating continuation of that press never acts on the row the field returns to; that row acts again only on a distinct Enter, judged as a heavy row's commit is (TUI-NAV-051). The reason is the one that stops repeat at arming: key-repeat is a machine typing, not a player deciding, and without this a held Enter after a palette jump would start buying whatever it landed on.
- **Cursor landing on a section's first visit** (no identity memory yet): the first *Available* (actionable) row, else the first row. If a remembered item has since become Hidden (e.g. structures reset by a Move), the cursor falls back to the same rule.
- **`l`/`h`/Tab/BackTab wrap around** the revealed-section ring (last → first, first → last).
- **Tab bar overflow.** Ten revealed tabs with names do not fit 80 columns, so the tab bar is width-responsive: full `[N Name]` when it fits, else a compact digit strip (`1 2 3̶ 4 …` with the focused and `★`-badged tabs labelled) — never truncated mid-name, never scrolled off. Section names are only ever shown where they fit; the digit is the stable identity.
- **Hint-bar overflow.** When the context keys exceed the width, the shell drops the least-relevant keys first and always keeps `? keys` last; the full set is always available via `?`.
- **Narrow-mode inspector swap:** while the inspector occupies the list's space (80-col mode), `↑`/`↓`/`j`/`k` still move the underlying cursor and the inspector follows it, so the player can browse inspected items; Space or Esc returns to the list view.
- **Breakpoint hysteresis:** the pane/swap inspector breakpoint (~100 cols) uses a small dead-band so a one-column resize wobble never flickers the layout.
- **Attention-chip specifics:** `★N new` clears when the badged tab is *visited*; `~away` clears when its banner is dismissed *or* its pinned Journal entry is opened; a `!sky` chip shows while the window is open (its remaining time lives in the inspector/journal, not the chip); stacked reports (a nap then an offline gap) queue their banners newest-first and all persist as pinned Journal entries.
- **DeclareAdventure(None)** is reachable as a "declare nothing / stand down" row in Legacy (or Esc on the declare list); the Family Album is a browse-only view in Legacy (no command); UnassignTinkerer is the `-` on a Tinkerer count row.

## Core API Dependencies (a small cascade into `clowder-core`)

The shell renders from existing surface except for these read-only additions, which keep rule knowledge in the core (per the engine-owns-visibility decision). They are a separate arrow segment (the sim leaves), flagged here because the navigation segment cannot fully land without them:

1. **`net_rate(ResourceId) -> f64`** — net per-second at current state (production stack minus consumption). Powers sidebar rates and the inspector's time-to-afford. This math currently lives only inside `tick()`.
2. **`lock_reason(content) -> LockReason`** (or equivalent) — the plain "why locked" per content kind. The shell *can* read `prereqs`/`gate`/`StructureGate`, but a core query keeps the rule logic core-side.
3. **Availability coverage** — `ContentId` spans only `Structure | Idea`; crafts, rites, arches, refinements, rules, traditions, looms rely on per-kind `*_unlocked` booleans with no Teased tier. Either extend `ContentId`/`availability()` to those kinds, or document that non-Structure/Idea kinds are two-state (absent/available). The shell's side is already pinned: it renders exactly what the core reports and invents no state the core did not give it (TUI-NAV-053), so whichever way the sim leaves resolve this, the shell needs no branch of its own.
4. **`max_affordable(...) -> u32`** (optional) — lets `X` and the hint bar name a closed-form count; without it, `X` loops-until-error under a cap.

## Decisions & Alternatives

| Decision | Chosen | Rejected | Why |
|---|---|---|---|
| Primary key model | Layered hybrid: cursor+Enter backbone, digit tabs, mnemonics, `/` palette | Per-item hotkeys; vim-modal; palette-primary; cursor-only | Only a cursor scales to 50+ progressively-revealed items; accelerators layer on top and gate no action |
| Section addressing | Digits `1`–`0`, statically pre-assigned | Unlock-order numbering; letter-per-tab; Tab-cycling only | Static map kills the renumbering hazard forever; cycling alone is O(n) travel |
| Modifier substitute | Uppercase as the shift tier (`x`→`X`) | Chord emulation | Shift emits plain printable chars — tmux-safe by construction |
| Count entry | `x` prompt captures digits; `X` = max | Vim count-prefix (`10Enter`) | Prefix digits collide with section jumps, the higher-value use |
| Palette pick | `Enter` takes the top-ranked result (equal name, then prefix, then substring); the palette keeps the count prompt's field keys, arrows included | Arrow-key selection among results | One key contract for both fields; typing more is always the way to a given item, and the ranking puts the likeliest aim first where the player can see it. The palette only accelerates — every item stays reachable in its own section (TUI-NAV-039) |
| Palette scope | Every item in a revealed section's list, one result per item | Content definitions only; one result per row | Fixed actions, kittens, and stores are things a player looks for too, and a universe read from the lists can never offer a result that lands nowhere or miss one the cursor reaches; one result per row would list a multi-row item twice under one name |
| Palette jump under the narrow-mode swap | The swap stays, showing the landed row | Returning to the list view | The jump moves the cursor, not the layout; the player chose the swap, and it already follows every cursor move (TUI-NAV-035) |
| Repeat | Hold Enter, cursor stays; **rate-limited to ~10/s**; never reaches a heavy row | OS key-repeat verbatim; a `.` repeat key; letting repeat commit an armed row | Uniform across machines; no new concept (player choice: rate-limit). Repeat must stop at arming, or holding Enter on the heaviest row in the game commits it in a tenth of a second — the confirm exists to make that press deliberate |
| hjkl | Full hjkl — h/l section prev/next, j/k cursor | j/k only; none | Player's choice; arrows/Tab still work, so nothing depends on the letters |
| Big-action confirm | Inline armed-row, second Enter, auto-disarm ~8s | Blocking modal; type-the-word; long-press | Modal violates idle-respect; terminals have no key-release for long-press; the armed row shows real consequences and expires harmlessly |
| Quit | `q` quits instantly (saves first) | `q` + confirming Enter | Quitting is always safe (autosave + offline accrual); a confirm is ceremony (player's choice) |
| Speed control | `<`/`>` live from any screen, always | Behind a settings row | Fast to reach; current speed always shown (player's choice) |
| Inspector placement | Persistent pane ≥100 cols; Space-swap at 80×24 | Popup overlay; footer strip; separate screen | Zero-keypress truth when space allows; swap keeps chrome + sim visibly alive |
| Esc policy | Close topmost transient only; bare-screen no-op; no Esc-prefixed sequences | Esc = back/home; Esc = quit | Under `escape-time`, a stray/late Esc must be harmless |
| Attention signal | Curated header chips, glyph+count+word, empty = calm; all four chips on | Color-only; per-tab badges only; bell/flash; "affordable" chip | One fixed location; triple-channel survives CVD; excludes the never-quiet "affordable" signal (player kept all four opportunity chips) |
| Event surfacing | Dim ticker + `e` Journal; offline = banner + pinned entry | Log tab; toast popups; blocking splash | Toasts/splash demand attention; ticker+journal is glanceable and lossless |
| Teased rendering | Name shown dim + `○`/`◐` marker; Hidden absent | Mystery silhouettes; hide until affordable | Teased *is* revealed (engine semantics); mystery fights the inspector's why-locked duty |
| Full resource table | Rows in Meadow's "stores" group (inspectable) | A dedicated resources tab | Saves a digit; sidebar stays display-only |
| Glyph set | Single-width geometric/box chars only; no emoji | Emoji flourish | wcwidth ambiguity breaks columns across terminals/tmux |

## Open Questions & Future Decisions

### Resolved
1. ✅ **Journal retention.** The journal is saved state, so it survives closing the game — which is what makes a report the player never got to read safe. It keeps a bounded rolling window of recent events plus every milestone and report permanently; the core owns the ring and the eviction rule (SIM-ENG-027, SIM-ENG-029). Unbounded retention was the alternative, and it turns every autosave of a months-long game into a growing rewrite.

### Deferred
1. Exact widths/breakpoints and per-screen list groupings — refined in the `render` leaf against a real terminal.

## References

- Parent: `../tui-design.md` (loop shape, shell responsibilities)
- Consumes: `clowder_core::{Command, Event, GameState}` and the derived queries in `state.rs`/`save.rs`; the Core API Dependencies above are a cascade into the sim leaves
- Sibling leaves (later): `theme/`, `render/`, `runtime/`
