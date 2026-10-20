---
parent: sim
prefix: SIM-ENG
---

# Engine

## Context and Design Philosophy

The engine is the spine of `clowder-core`: it owns the canonical `GameState`, the command API the shell calls, the tick pipeline that sequences every other leaf's phase functions, the calendar, the RNG, the typed event vocabulary, and — at depth — the shared mechanics every content leaf builds on: the effect-key resolution system, the diminishing-returns primitives, converter throttling, unlock/visibility queries, and the game-speed field. Everything here is pure and deterministic — no IO, no clocks, no threads. The engine owns *machinery*; leaves own *content and values*.

## Effect-Key System

All modifiers in the game flow through one typed effect vocabulary (sub-HLD § Effect Resolution Architecture). The engine owns:

- `enum Effect` — compiler-checked variant per modifier kind (`ResourceRatio(ResourceId)`, `JobRatio(JobId)`, `MaxCap(ResourceId)`, `GlobalProductionRatio`, `CraftRatio`, …). Variants are added per wave; adding one is a source-level change that exhaustiveness-checks every consumer.
- `effects(state, key) -> f64` — sums contributions in a fixed source order: structures (× owned/active count as the key demands) → refinements → ideas → house rules → rites → arch upgrades → traditions → colony standing → adventure rewards → the Elder's trait. Fixed order keeps float accumulation deterministic.
- **DR primitives**: `limited_dr(v, limit)` (linear to 75% of limit, asymptotic above) and `unlimited_dr(v, stripe)` (`(sqrt(1 + 8v/stripe) − 1)/2`). Consumers of stacking sums apply one of these per the sub-HLD's DR discipline.

The production stack (sub-HLD § The Production Stack) is engine code: leaves declare rates and effect contributions; the engine computes per-tick flows in the normative layer order, exempting transient currencies from global layers.

## Converter Throttling

`throttle(inputs: &[(ResourceId, f64)], state) -> f64` returns the worst-supplied input's satisfaction fraction (0..=1); converter phase functions scale all their inputs and outputs by it. Chains degrade proportionally and never flap. A converter at fraction 0 is simply idle — never an error.

## Unlocks & Visibility

Content items (structures, ideas, refinements, house rules, rites, colonies, sites) share one availability model, engine-owned:

- An item **reveals** the first time its gate is satisfied. Revelation is sticky: revealed ids live in a persisted `revealed` set in `GameState`, the scan runs at the end of the clamp phase, and each new revelation emits a `Revealed` event — the "something new is possible" moment.
- **A gate is a reveal.** A structure behind a gate — an idea researched, a site reached — reveals in the first scan after its gate opens, whatever the meadow holds of its price. The gate is already the moment the player is told the structure exists: the idea's blurb names what it unlocks, and a site once reached is where its outpost stands. Holding the row back until a share of its price is in stock would leave a promise visibly unkept, and would hide the one thing the player needs to act on it: what the structure costs, and so what to gather toward. A gate opened by a command reveals what it opens in the next tick's scan, as a researched idea's successors do.
- **The cost ladder paces the ungated.** A structure with no gate — one of the meadow's first buildings, open from the start — reveals the first time every cost component has reached the reveal fraction of its current price (default 30%, content-tunable). Nothing else announces these, so they arrive as the stock to afford them begins to gather, and a fresh meadow is not handed every first building on its first frame.
- `Availability::{Hidden, Teased, Available}` — *Hidden*: not yet revealed; *Teased*: revealed but not currently affordable/unlockable; *Available*: revealed and actionable now. The shell renders Teased items dimmed; presentation is entirely the shell's.
- Purchase-time gating stays the command layer's job (`Locked` per error precedence); Availability is advisory for display.
- **Implementation note.** The per-tick reveal scan walks a derived, non-serialized worklist (the complement of `revealed`, canonical order) rather than probing the whole content space each tick — a hot-path accelerator, rebuilt from `revealed` on load so it can never drift from that serialized truth. Purely a performance concern; the observable reveal behavior is identical to a naive full scan.

### Revealed resources

A resource is not a content item — nothing is built, bought, or researched of it — but it is revealed on the same principle: the stores a player is shown are the stores the meadow has met, and a store once met stays shown. A resource has no Teased state; it is revealed or it is not. It reveals the first time either of these holds:

- **The meadow holds some.** Its stock stands above zero after a tick or an applied command — by any amount, with no dust guard. The rule only ever adds a store, and a sliver is still something the meadow held. A tick reads the stock after its clamp, so a trickle into a store whose cap is zero — a Sunwarden's warmth before any Sun Room stands — clamped away in the tick that made it, is not a holding.
- **A price the player can be shown names it.** The core would state a price with the resource among its components: the price of one more of a content item it reports Teased or Available, whether or not that item is already the player's (the cost query); the per-unit price of a count-taking command, wherever that query answers — a pounce, a trade with a discovered colony, the whale's exchange while a whale visits; or a deep nap's cost while the nap's gate is open. A player reading a need beside a row must be able to find the store it speaks of — its stock, its cap, its rate — so the store is listed from the moment its name is (tenet 3: no decision is lost because a store was not shown). A name already on screen in a price has nothing left to give away.

A resource the meadow has neither held nor been shown in such a price stays unrevealed, so the stores never name what the player has not met. A refused command reveals nothing, even where its refusal names a store the player has never seen: a refusal mutates nothing (Command API).

**Revelation is permanent for the meadow.** Revealed resources live in a persisted `revealed_resources` set in `GameState`, beside the content set and serialized in id order, and nothing short of a Move takes one out: not its stock falling to zero, and not the row whose price named it leaving its list — a house rule closed by its sibling's adoption, a whale's visit ending. A reveal that could un-reveal would take a store out of the list, and out from under the cursor, exactly when the player has spent it and most wants to watch it refill. Like the content set, the set is the meadow's own: a Great Meadow Move empties both with everything else it resets (SIM-MOVE-004), and the meadow that follows reveals afresh, beginning with whatever the Memory Chest carried in.

**The answer is a query** (Derived Queries), and it never says no of a resource the meadow holds: it reads the set and the stock in front of it together, as the roster reach reads its high-water and the roster (SIM-ENG-046). A state built or edited by hand is judged on what it plainly holds, and the recording below only ever confirms an answer the query already gave.

**Where a reveal is recorded.** Each condition is checked where it can first become true, and recorded before control returns to the caller, so no state a caller can observe shows a store that a later spend could un-list, or a priced row whose resources are unrevealed:

- *Holdings* — at the end of every tick's clamp phase, live or replayed, and after every applied command. A tick alone would not do: one command can gain a stock and a later one spend it to nothing within a single tick's interval.
- *Prices* — after every applied command, and on a tick for each content item that tick's own reveal scan reveals. Those are the two ways a price comes into view: a content item climbs the reveal ladder on a tick, and every other gate that puts a priced row in front of the player — an idea researched, a peak of devotion reached by basking, a colony discovered, a whale summoned — opens on a command. A gate that came to open on a tick would owe this check a third site.

A resource's reveal emits no event and enters no journal. The store's row arriving is the whole of the moment, and whatever brought it — a gain, or the item whose price named it — already has its own event or its own row.

**Implementation note.** Which resources a price names is fixed by the item's definition — effects scale a price's amounts, never its components — so the price check reads components and computes no price. The per-tick holdings check walks a derived, non-serialized worklist of the not-yet-revealed resources, rebuilt from the set on load as the content worklists are: one comparison per unrevealed resource, and nothing once every resource is revealed. A tick whose scan reveals no content item checks no price. Catch-up applies no command and reveals content rarely, so the rule adds nothing to the replay hot path the persistence leaf bounds (SAVE-009).

## Game Speed

`speed: f64` in `GameState`, default 1.0, clamped [0.25, 8] on set and on load. A `SetSpeed(f64)` command (always legal, clamps silently, emits `SpeedChanged`) mutates it. Nothing in `tick()` reads it: the shell drives `4 × speed` ticks/second and persistence replays `elapsed × 4 × speed` — speed changes *how many* ticks happen per wall-second, never what a tick does.

## GameState

One serde-serializable struct, the single source of truth:

- `ticks: u64` — monotone tick counter; the calendar is derived from it (no separate stored date to drift).
- `rng: Pcg32` — hand-rolled PCG-32 (64-bit state, 64-bit stream constant); serializable; the only randomness source.
- `resources` — stocks per resource kind (economy leaf defines kinds/caps).
- `structures` — owned count per structure kind.
- `kittens` — the roster (kittens leaf defines the entry shape).
- `ideas` — completed idea set, stored as a sorted set for deterministic iteration.
- `pity: u8` — consecutive failed *eligible* arrival checks (kittens leaf semantics); saturating.
- `famine_days` / `recovery_days: u8` — famine and recovery clocks (kittens leaf semantics); saturating — only threshold crossings matter, so saturation is harmless.
- `grumpy: bool` — set each tick by the mood phase (consumption shortfall occurred); read by the next production phase, and by the next day close as the "did the day end in shortfall" bit for the famine/recovery clocks.
- `next_seq: u64` — monotone counter stamping kitten job assignments and departures (kittens leaf ordering semantics).
- `zoomies_until: u64` — tick until which zoomies are active (active while `ticks < zoomies_until`); 0 when inactive.

Derived values (happiness, effective rates, season) are computed, never stored — stored state is minimal so saves can't go internally inconsistent. A queryable next-season forecast is exposed once the relevant idea is researched (until then the query returns nothing).

A new game (`GameState::new(seed)`) is an empty meadow: zero stocks, no structures, no kittens, no ideas, tick 0. The seed comes from the caller — the shell picks it (typically from the OS clock, which is allowed *outside* the sim); everything after the seed is deterministic.

## Calendar

Derived entirely from `ticks`: day = `ticks / 10`, season index = `(day / 25) % 4` (order: Spring, Summer, Autumn, Winter, starting in Spring), year = `day / 100`. Day boundaries trigger daily sub-steps (arrival, wander/return); season boundaries emit `SeasonChanged`.

## Command API

```rust
pub enum Command {
    GatherCatnip,          // manual: +1 catnip (×2 with the relevant idea)
    Forage,                // manual: spend catnip, gain a twig
    Build(StructureId),
    AssignJob(JobId),      // one idle kitten takes the job
    UnassignJob(JobId),    // one worker returns to loafing
    ResearchIdea(IdeaId),
}

pub fn apply(state: &mut GameState, cmd: Command) -> Result<Vec<Event>, CommandError>;
pub fn tick(state: &mut GameState) -> Vec<Event>;
```

`CommandError` names the reason: `UnknownId`, `Locked` (structure/job/idea not yet unlocked), `AlreadyResearched`, `NoIdleKittens`, `NoWorkersAssigned`, `StorageFull` (a manual action whose yield resource sits at cap — spending a press for a trimmed-away yield would be a clarity failure), `InsufficientResources { missing }`. When several apply, the **first in that list wins** — report the most fundamental blocker, and never leak the cost of content the player hasn't unlocked. Commands either fully apply or return an error — no partial spends. A rejected command never mutates state (RNG included).

## Derived Queries

The shell renders; the core rules. Any question whose answer is a *rule* is answered here, even when the shell could technically assemble it from public state — a frontend that computes affordability, or explains why something is locked, is holding game knowledge the crate split exists to prevent, and it will drift from the real rule the first time the rule changes.

Ten read-only queries carry that boundary:

- **Net rate per resource** — production minus consumption at current state, per second. This must be *derived from the tick pipeline itself*, not re-implemented as a parallel formula. A second copy of the production maths is exactly the shadow math the HLD rejects for offline progress, and it would drift the moment any leaf adds a multiplier.
- **Flows by source per resource** — the net rate taken apart by what moves the stock: for a resource, each source's own signed rate at current state, per second, from the same run of the pipeline the net rate reads. A *source* is what a stock move is made for: a structure, for its productions, its converter's inputs and outputs, and any output an effect grants it; a job, for its workers' yield, all of them together; a craft, for the inputs and output of the tinkerers bound to it; an arch upgrade, for the drift and dreaming it drives, whatever structures its rate is scaled by; and the kittens themselves, for their appetite and the luxuries they use. Each move is credited to its source where the pipeline makes it, at the amount it actually moved the stock once the phase's own floors and caps have applied, so a resource's flows add up to its net rate, and nothing is credited by a formula of its own. A multiplier is not a source, whatever grants it: it shapes the flow it multiplies, and its share is inside that source's figure. The flows are listed in a fixed order — structures, jobs, the kittens, crafts, arch upgrades, each kind in its content table's order — and a source that moved the resource by nothing is left out. Recording the credits is something only this query asks for: a tick, live or replayed, keeps none, so the pipeline's result is the same whether or not anyone asks.
- **What runs on a clock** — every timed state the meadow holds at the current tick, each with how many ticks it has left: a visible sky event waiting to be observed, a Star Whale's visit, a jamboree, the zoomies, and a déjà-vu day, listed in that fixed order. Each is listed exactly while the rule that decides it holds — a sky event while the tick is short of its expiry, a whale while visit years remain, the others while the tick is short of their end — and carries the ticks from now until the first tick on which that rule no longer holds, if nothing is done meanwhile: for a whale, the year close at which its last year runs out. Each state ends by its own rule, and a frontend that worked out an end from the fields would be holding those rules outside the core. The answer is in ticks, not seconds: how long a tick lasts in real time is the shell's cadence, set by the speed (Game Speed), and converting is the shell's to do.
- **Lock reason** — why a given piece of content is not actionable, as typed data rather than prose, drawn from the same gate the command layer enforces. Two sources for one answer is how a screen ends up explaining a rule the command layer no longer applies.
- **Why a tinkerer bind is refused** — for a recipe, why binding a Tinkerer to it would be refused now, as a lock reason drawn from the bind command's own checks in its own precedence: a prerequisite where the recipe is locked, no-one-free where no Tinkerer at home stands without a binding, no-room where the bindings already fill the meadow's tinkerer seats (economy leaf: Tinkerer seats); and none where the bind would be made. The content lock reason for a recipe speaks for making it, and a bind is a second act on the same item with blockers of its own, so it has its own answer rather than a variant of the first; a frontend that judged it from the roster and the seats would hold the bind's rule outside the core.
- **The tinkerers' bench** — the meadow's tinkerer seats; every binding, by the bound kitten's roster position in roster order, with the recipe it is bound to, an away kitten's binding included, since a binding holds its seat while its kitten is away; and the Tinkerers at home holding no binding, by roster position in roster order — the kittens a bind can take. How many seats the meadow has and which kittens a bind can take are both rules: a frontend that counted the seat-giving structures, or picked the bindable kittens out by job, home, and binding, would hold them outside the core and drift the first time either changed.
- **Availability, total over every content kind** — the query answers for anything revealable, not only the kinds that carry a reveal ladder. Kinds without one answer with the two states they have. Totality is the point: the shell renders exactly what it is told (TUI-NAV-053), so a query that silently has no answer for some kinds pushes the branch back into the frontend.
- **Revealed resources** — whether a resource is revealed, and the revealed resources in the order the core lists resources (Unlocks & Visibility: Revealed resources). Which stores exist for the player is a rule — when a store is first met, and that it is never un-met — so a frontend that listed the stores holding a stock above zero would hold a reveal rule of its own, and would drop a store the moment it was spent.
- **Maximum affordable count** — where a closed form exists, so a bulk action can name its count before acting instead of discovering it by attempting and failing.
- **Per-unit price of a count-taking command** — for a command that takes its count as an argument and is priced a fixed amount per unit (a pounce per squad; a trade per caravan to the colony it names, the whale's exchange by its own rate), the price of one unit, per component, with the promise that n units cost exactly n times that price — a caller previewing a count multiplies by the promise the query makes rather than by a pricing rule of its own. It answers nothing for a command priced otherwise, where that promise would not hold — a deep nap's price escalates with its length, and has its own cost query — for a command that takes no count, whose price, if any, is a content item's, and for a command the command layer would refuse as locked, decided by the same checks that command makes before it spends, so a price is never stated for something the meadow has not reached (the lock reason's withholding, applied to prices). A frontend that read these amounts from the content tables instead would hold the command's pricing rule outside the core, and would go stale the first time an effect scaled a unit's price.

## Event Vocabulary

`tick()` and `apply()` return `Vec<Event>` — typed data, no prose:

- `KittenArrived { name }`, `KittenWanderedOff { name }`, `KittenReturned { name }`
- `JobAssigned { name, job }`, `JobUnassigned { name, job }` — assignment names the kitten (kittens leaf: identity makes assignment delightful)
- `SeasonChanged { season }`, `NewYear { year }`
- `StructureBuilt { id, count }`, `IdeaResearched { id }`
- `ZoomiesStarted { until_tick }`, `ZoomiesEnded`
- `KittenMood { mood }` — emitted only on mood-tier transitions (content, not spam)
- `Flavor { template_id, kitten: Option<name> }` — pure-delight events with no mechanical effect; templates are content-table entries the shell renders

Every event carries a **classification** — flavor, mechanical, or milestone. The table below is the classification; it is intent, not an implementation detail, because deciding which moments are worth remembering is a statement about the game.

**A milestone must be bounded over a playthrough by construction.** Milestones are kept permanently, so a recurring occurrence classed as one grows the save without limit — and "recurring" includes things that feel rare in game time but are not: a game-year turns roughly every half-minute of real time at the fastest speed. Milestones are therefore one-shot unlocks, first-of-a-kind moments, discoveries, and the deliberate turning points a player chooses — a turning point being a choice that is made once, or whose price bounds how often it can be made, never an arrangement the player can rearrange at will for nothing. Everything else that changes state is mechanical.

- **Milestone**: an idea researched, a refinement or rite or arch or tradition or loom bought, a rule adopted, the *first* copy of a structure raised, a kitten promoted, a colony discovered, a site reached, an enlightenment tier gained, a whale summoned, a meadow moved, an adventure completed, the distant dawn.
- **Mechanical**: everything else that moves state — including the calendar's turns, every structure after the first of its kind, and every appointment of an Elder.
- **Flavor**: pure delight with no mechanical effect.

**An appointment's event.** Appointing an Elder is free and can be repeated without limit (kittens leaf: the Elder), so nothing bounds how many appointments a playthrough makes, and its event is mechanical: the rolling window keeps it for weeks of ordinary play and then lets it go, while the permanent entries stay the moments reached once. It is mechanical whichever kitten it names and whichever came before — the first appointment in a meadow included — since an Elder is an arrangement the player can change back at any moment, not a turning point. The event is emitted only where the appointment changes who the Elder is: when there was no Elder, or the Elder was the kitten at another roster position — a namesake of the Elder included, since the core tells kittens apart by position, never by name. An appointment naming the position already the Elder's is accepted like any other — it is not refused, since nothing blocks it and a refusal would answer a request for what is already true as though it had failed — but it changes nothing, so it emits no event and the journal gains no entry. An event says something happened in the meadow, and a line saying the Elder is who the Elder already was would be noise in the journal and in every display that reads the events. It is still an applied command, so the reveal recording that follows every applied command runs after it (Unlocks & Visibility: where a reveal is recorded); in a meadow reached by play that recording finds nothing new.

The first-copy rule reads the event's own count rather than its variant alone: raising a thing you have never had is the moment; the eleventh is bookkeeping. It is a property of the vocabulary rather than a judgement the consumer makes: deciding that reaching a new site matters more than a job reassignment is a rule about the game, and a shell that made that call itself would be encoding game rules in the frontend. The classification is what lets a display rank prominence, and what lets the journal below decide what it may never evict.

## The Journal

`GameState` carries an event journal, because the events a player has lived through are part of their game and should survive closing the terminal. It holds a **bounded rolling window** of the most recent thousand entries plus **every milestone entry, permanently**; a thousand is a few weeks of ordinary play, enough that reopening after a break shows continuity while the long arc of the game rides in the permanent entries regardless; each entry carries the tick at which it occurred, so a reader can group by day. When the window is full, the oldest non-milestone entry is evicted — growth is bounded, and the moments worth remembering are never what gets dropped.

**Where an entry is kept is settled when it is recorded.** The journal holds its rolling window and its permanent entries apart, and an entry goes to one or the other by its event's class at the moment it is recorded; nothing moves it afterwards. So a save's journal loads exactly as it was written: an entry kept permanently stays permanent even where its event is not classed a milestone today — as an appointment kept by a save from a version that classed appointments as milestones is not — and no load rewrites, moves, or drops one. The journal is the history the player lived through, and a reclassification decides what is remembered from now on, never what was.

**Catch-up replay does not append.** A year-long absence replays millions of ticks, and recording their events would write an enormous save and slow every load — the exact costs the persistence leaf's replay budget exists to prevent. Replay already folds into an `OfflineReport`; that summary is what enters the journal, and the constituent events are discarded as they always were. Live ticks and applied commands append; replay does not — and *any* bulk fast-forward is replay, a deliberate long sleep as much as an unattended absence.

Two occurrences are deliberately never journaled. A **speed change** is not something that happened in the meadow; it is the pace the meadow is being watched at, and journaling it would make a viewing knob the one way speed leaves a trace in the meadow's history. A **bulk-sleep summary** is already retained as its own away entry, so journaling the event as well would file one sleep under two headings.

The distinction is drawn *inside* the core. A tick cannot tell from its own state whether it is live or replaying, so if the caller decided, the rule would live outside the crate that owns it — invisible to headless tests, and wrong for any frontend that forgot. Instead the ordinary tick journals and replay takes a non-journaling entry point of its own, so journaling a replay is not something a caller can do by omission. This is what keeps a journal in the save file affordable.

## Random Events

Drawn in pipeline phase 5, at day boundaries, from the state RNG. The complete RNG draw schedule on a boundary tick is fixed: arrival roll (phase 1d — skipped entirely, no draw, when pity forces the arrival or the check is ineligible) → zoomies expiry check (no draw; per-tick, `ticks ≥ zoomies_until`) → zoomies start roll → flavor roll → flavor template draw → flavor kitten draw (the last two only when the preceding roll fired). Zoomies may restart on the same boundary tick they expired (expiry is checked first), and may start or run while the clowder is Grumpy — the multipliers stack (0.25 × zoomies), which is intended: zoomies shorten famines, and taking them away from a grumpy clowder would be anti-cozy.

- **Zoomies:** while inactive, daily chance 4% (expected roughly once per 25 game-days — about once a season); sets `zoomies_until = ticks + duration + 1` and emits `ZoomiesStarted`. The `+1` is deliberate: the start happens in phase 5 *after* that tick's production, so the window must extend one past the arithmetic end for exactly `duration` production ticks to be boosted — the stated duration is never quietly one short. The multiplier applies while `ticks < zoomies_until`; when phase 5 finds `ticks ≥ zoomies_until`, it resets the field to 0 and emits `ZoomiesEnded`. Base: ×1.25 for 120 ticks (30 s); the relevant idea raises this to ×1.5 for 240 ticks. **Magnitude is live, duration is fixed at start** — researching the idea mid-zoomies upgrades the multiplier immediately but never rewrites `zoomies_until` (the player only ever gains).
- **Sky events:** where Stargazing Ledges exist, each day close rolls for one, and a hit is either seen by someone who happened to be looking up — its yields granted at once — or left visible for the player to observe (SIM-ENG-025). At most one visible event waits at a time, and no roll is made while one waits. A visible event stays observable for one game year from the day close that raised it — a few minutes of real time at normal speed — and then simply fades, costing nothing. An observation — the player's, or one made by chance — emits `SkyObserved`, carrying how far it raised curiosity and skymaps, as the stocks moved once their caps applied, and whether it was made by chance. A display can then say the sky was watched and what it gave; without the event an observation would show only as two stocks rising, and one made by chance would pass unseen altogether. The length is set by the glance: the player looks at the pane between other work, and an opportunity gone before the next glance is one only a player watching the pane can take, which turns attended play from a bonus into an obligation. Like everything a tick does, the window runs on game time, so it is shorter in real time at a faster speed (Game Speed).
- **Flavor:** daily chance 8%. Templates are typed *ambient* or *needs-kitten*; the draw selects uniformly among currently eligible templates (needs-kitten templates are eligible only while at least one kitten is Home), then draws a kitten if the template wants one. Zero mechanical effect by construction.

## Determinism Contract

Same seed + same command sequence applied at the same tick counts ⇒ bit-identical serialized state. Enforced by: single RNG, fixed pipeline order, the boundary-tick RNG draw schedule above, no float operations whose order varies (accumulate flows in a fixed resource order), commands applied only between ticks, and **canonical serialization order for every GameState collection** — resources and structures as enum-indexed arrays (or BTreeMaps), the roster as an append-ordered Vec, ideas as a sorted set. "Bit-identical" is tested on serialized bytes; an unordered map anywhere in GameState would falsify it.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Effect summation order | Fixed source-class order (structures→refinements→ideas→rules→rites→arches→traditions→standing→adventures→trait) | Arbitrary/iteration order | Float addition isn't associative; a fixed order is what makes bit-identical determinism survive content growth |
| Speed placement | `GameState.speed`, read only at the boundary (shell cadence, catch-up), never inside `tick()` | Speed as a tick()-internal multiplier on rates | Rate-multiplying would change what a tick *means*, breaking replay equivalence between live and catch-up play and every balance test |
| Availability model | Engine-owned Hidden/Teased/Available advisory queries | Per-leaf ad-hoc visibility; shell-side inference | One reveal model keeps the "silhouette teaser" pacing consistent across all content kinds, and the shell out of the rules business |
| When a gated structure reveals | When its gate opens, whatever the stock; the cost ladder kept for ungated structures alone | The cost ladder for every structure, gated or not; a lower reveal fraction tuned per gated structure; a journal line at research time saying the structure will appear later | Under the ladder, an idea whose blurb names a structure opens it and nothing new appears, so the promise reads as broken, and the price that would say what to gather stays hidden. Per-structure fractions leave every new gated structure to repeat the same override. A journal line explains the wait but still withholds the price |
| When a resource is revealed | First held, or first named in a price the core would state; kept in a persisted set and never un-revealed within a meadow | Revealed while its stock is above zero, judged by whoever lists the stores; revealed only once held; every resource revealed from the first tick | A stock is not a reveal: a store spent to nothing would leave the list as it is most wanted, and take the cursor's row with it. Waiting for a holding shows a price's need with no store to read a stock, a cap, or a rate from. Revealing everything names stores months before the meadow meets them |
| Resource reveals and the Move | Emptied with the content set; the new meadow reveals afresh | Kept across Moves, as something the clowder has learned | The Move's legacy set is the prestige leaf's to enumerate (SIM-MOVE-004), and structures and ideas already reveal afresh under it; a holding or a price re-reveals each store the moment the new meadow has a use for it, so no decision waits on a store that is missing |
| How flows are attributed to sources | Each stock move credited to its source where the pipeline makes it, in the probe run the net rate already uses | The net rate recomputed with each source removed, the difference taken as its share; a per-source formula beside the pipeline | Removing a source and differencing runs the pipeline once per source, and the shares stop adding up to the net rate wherever multipliers compound or a throttle redistributes a scarce input. A per-source formula is the shadow math the net rate already refuses. Crediting at the move is the pipeline's own arithmetic, so the flows cannot disagree with the net rate they decompose |
| Multipliers in flows by source | Folded into the flow of the source they multiply; never a source of their own | A line of its own for each booster, carrying its share of the flows it raises | Where several multipliers compound on one flow, a booster's share has no unique answer — any split is a convention the rules never apply — and a booster with a line of its own would make the flows add up to more than the net rate |
| Calendar storage | Derived from `ticks` | Stored day/season fields advanced by tick | Derived can't drift or double-advance during catch-up |
| Command atomicity | All-or-nothing, error enum | Partial application, panics | Shell can render precise "why not" hints; sim state can't half-change |
| RNG | PCG-32, hand-rolled, serialized with state | `rand` crate StdRng, reseeding on load | Stable serde round-trip across versions; ~30 lines; reseeding on load would break catch-up determinism |
| Events carry data, tier-gated | Typed enum; mood emitted on transitions only | Emit-every-tick, string messages in core | Tests assert on events; transition-gating keeps a week's catch-up report bounded |
| Event history | Bounded ring in `GameState` + permanent milestones | Shell-owned session history; unbounded log; milestones only | History in the save survives a quit, which is what makes a report the player hasn't read safe; unbounded turns every autosave into a growing rewrite over a months-long game |
| Event classification | A property of the event vocabulary, queryable | The shell decides which events are prominent | "This moment matters" is a rule about the game; a shell deciding it would hold game knowledge the split forbids |
| An appointment's class | Mechanical, every appointment that changes the Elder | Milestone; milestone for the first appointment in a meadow only, as the first copy of a structure is | Appointments are free and unlimited, so as milestones they grow the permanent journal without bound (SIM-ENG-042). A first-in-meadow milestone would be bounded, one per Move, but the Elder is an arrangement the player rearranges at will rather than a moment reached once, and the first appointment is the same free choice as every later one |
| Appointing the kitten already the Elder | Accepted, changing nothing: no event, no journal entry | A refusal naming it as already the Elder; accepted with an event like any appointment | A refusal answers a press for what is already true as a failure, and puts a note where the player pressed though nothing is wrong. An event for no change writes a line that says nothing happened as though something had, and with the key held writes one ten times a second |
| A journal entry whose event's class changes | Stays where it was recorded; no load moves it | Re-sorting a save's entries by the current classification on load | The journal is the player's lived history; re-sorting would evict, without a word, entries the player was promised were permanent |
| Price of a count-taking command | The price of one unit, promising that n units cost n times it, answered only where that holds | A query taking a count and returning its total; leaving the caller to the content tables | The one-unit price is what a caller shows before any count is chosen, and the promise covers every count from it; where a price is not fixed per unit the query stays silent rather than state a rule it does not keep. Amounts read from the tables hold the pricing rule outside the core |
| Replay and the journal | Replay events fold into the report and never append | Append replay events like any other | A year of replay is millions of events; appending them would blow up the save and the load budget in one step |
| How long a visible sky event stays observable | One game year from the day close that raised it | Until the end of the next day; until the end of the season after; until observed, never fading | The next day is a few seconds of real time at normal speed and under one at the fastest, gone between two glances, so only a player watching the pane could ever observe one. A season is about a minute at normal speed and seconds at the fastest, still shorter than the gap between glances at a pane kept beside other work. Never fading makes an event to catch into a stock to collect; a year keeps the window closing while leaving minutes to see it in |
| Daily randomness cadence | Random events roll at day boundaries only | Per-tick probabilities | Fewer RNG draws keeps catch-up fast and probabilities legible ("8%/day", not "0.0083%/tick") |

## Open Questions & Future Decisions

### Resolved
1. ✅ Zoomies frequency: daily 4% ≈ once per 25 game-days (about once a season) — frequent enough to delight, rare enough to feel like an event.

### Deferred

## References

- Parent: `../sim-design.md` (pipeline order, determinism rationale, cozy invariants)
- Consumes phase functions from: `../economy/`, `../kittens/`, `../curiosity/`
