---
parent: high-level-design
prefix: SIM
---

# Simulation (sub-HLD)

> **Spoiler notice:** this document and everything beneath `docs/intent/sim/` describe game content. The player-owner discovers content by playing; see `AGENTS.md § Spoiler Policy`.

## Context and Design Philosophy

The simulation is the whole game: every rule, number, name, and event lives in `clowder-core`, headless and deterministic. The TUI is a window onto it. This sub-HLD holds what its child leaves share — the tick pipeline, the determinism contract, the cozy invariants, the effect-resolution architecture, the currency-tier taxonomy, and the ownership boundaries between leaves — and owns no EARS of its own.

Clowder targets **Kittens-Game-scale depth**: a dozen interlocking subsystems, each feeding a multiplier layer into a shared production stack, paced by storage-cap gating over months of slow-burn play. The mechanical skeleton is adapted from Kittens Game's actual source (its curve shapes, stack order, and gating patterns), with clowder's own names, numbers, and — everywhere the original is grim — a cozy inversion.

## Load-Bearing Properties

1. **Determinism.** Same RNG seed + same command sequence at the same tick counts ⇒ bit-identical serialized state. One serializable PCG-32 in state; no wall clocks inside the sim; fixed pipeline order; canonical serialization order for every collection; fixed RNG draw schedules. Every stochastic system must have a deterministic replay path (offline catch-up runs the same `tick()`).
2. **The cozy invariants** (HLD Tenet 1, made mechanical):
   - No kitten is ever removed from the roster. Absence is a *location*; the Great Meadow Move archives generations into the Family Album — archived, never deleted.
   - No structure is destroyed against the player's will.
   - No stock falls except by consumption, player-chosen spending, storage-overflow trimming, or player-accepted conversion. Systems that in the inspiration *destroy* things instead *relocate, suspend, or hide* them, reversibly (see § Cozy Inversions).
   - Every negative pressure is a multiplier on gains — moods, seasons, mess, energy deficits — never a confiscation.
3. **Whimsy with clarity** (Tenet 3): every flavor-named entity carries a plain mechanical description in its content definition; the shell always has truth to show one keypress away.

## Cozy Inversions (normative for all leaves)

| Grim original (Kittens Game) | Clowder's version |
|---|---|
| Starvation kills kittens | Shortfall → Grumpy naps; sustained famine → kittens visit the neighbor's yard, return on recovery; the last Home kitten never leaves |
| Sacrificing unicorns for tears | **Unicorn Parades**: unicorns march off in glittering procession; rainbows arrive in gratitude |
| Sacrificing alicorns; corruption breeding necrocorns | **Prisms** occasionally drift into the clowder's shared dreams (**dreaming**), becoming **dreamcorns** — asleep elsewhere, not gone |
| Black Liquid Sorrow (condensed suffering) | **Jars of Purrs** — condensed collective contentment, capped and precious |
| Pacts consuming necrocorns; Fracture destroys pacts, alicorns, and all skills permanently | **Promises** to the Night Court keep dreamcorns drowsy as upkeep; a neglected Promise **goes quiet** (suspends, keeps its slot) and wakes when upkeep resumes — nothing shatters |
| AI apocalypse destroys resources yearly; sale locked | The **Yarn Computer** above a cleverness threshold starts *batting resources under the couch* — a visible hidden stash, fully recovered when its enthusiasm is channeled (research) or its level lowered |
| Reset: kittens die/are abandoned; resources wiped | **The Great Meadow Move**: the clowder's kittens become honored **Elders of the old meadow**, archived by generation in the **Family Album**; **Traveling Baskets** carry chosen kittens along; the Memory Chest carries a fraction of resources |
| Pollution penalties | **Mess**: industry scatters clutter; too much slows catnip growth and makes arrivals shy — tidying (structures, rotas) reverses it; never a loss, only a slowdown |
| Hostile races; failed trades | **Shy colonies**: a failed trade means the caravan found them napping (goods spent, a courtesy gift sometimes returned); standing improves with Pen Pals |

## Time Model & Speed

| Unit | Definition | Real time at speed 1× |
|---|---|---|
| Tick | atomic sim step | 250 ms (4 ticks/s) |
| Day | 10 ticks | 2.5 s |
| Season | 25 days | 62.5 s |
| Year | 4 seasons (100 days) | ~4.2 min |

`speed: f64` lives in `GameState` (canonical, persisted), default 1.0, clamped to [0.25, 8]. The shell drives `4 × speed` ticks per real second; offline catch-up replays `elapsed_seconds × 4 × speed` ticks. Because speed scales attended and away time identically, it can never make presence beat absence (HLD decision 7). Nothing inside `tick()` reads `speed`.

## The Production Stack (normative order)

Per-tick resource flow resolves in fixed multiplier layers, adapted from the inspiration's stack. Layers 4–5 (mood, zoomies) apply to **all job outputs regardless of tier**; layers 6–8 apply only to **material-tier** flows. Structure base production takes layers marked ☆. Transient, luxury, mystic, and dreamtime resources skip layers 6–8 (§ Currency Tiers).

1. **Base rate** — content-table flat rates × count (structures ☆) or × effective workers (jobs).
2. **Per-source ratio** — additive `+X%` effect keys targeting that source (tool ladders, per-building boosts) ☆.
3. **Season/weather** — seasonal modifier where the source is marked seasonal ☆ (catnip patches; weather adds ±0.15 to the seasonal band from year 4).
4. **Mood** — happiness multiplier × Grumpy ×0.25 (jobs only).
5. **Zoomies** — ×1.25/×1.5 while active (jobs only).
6. **Global production ratios** — Dynamo Wheel (+2%/active, amplified by Cogworks), Star Basin (+5%/active). Layers 6–8 apply to **all raw-tier material production** — structures and converters as well as jobs — so the Long Sun and global boosts actually move the late game, whose materials come mostly from structures.
7. **Elder Wisdom** — +1%/point, LimitedDR-capped at +200% (× Nine Lives tradition ratio); Passed-Down Wisdom adds a second capped pool.
8. **The Long Sun** — ceremonies' devotion bonus: `LDR(UDR(devotion, 1000)/100, cap)`, cap 10 base, extendable by Enlightenment-tier content.
9. **Mess & energy** — mess level multiplies catnip growth down (floor ×0.25) and slows arrivals; an energy deficit multiplies layers 6–8's *effects* by `produced/consumed` floored at 0.25. Surplus energy is separately rewarded where content says so (e.g., Moonlight Stage).

**Diminishing-return primitives** (shared, engine-owned): `limited_dr(v, limit)` — linear to 75% of limit, asymptotic after; `unlimited_dr(v, stripe)` — sqrt-shaped unbounded. Any stacking bonus MUST pass through one of these before it can run away; a leaf introducing an unbounded raw sum is a design bug.

## Effect Resolution Architecture

All modifiers flow through one **typed effect-key system** owned by the engine leaf:

```rust
enum Effect {
    ResourceRatio(ResourceId),        // +% on a resource's base production
    StructureRatio(StructureId),      // +% on one structure's output
    JobRatio(JobId),                  // +% on one job's yield
    MaxCap(ResourceId),               // flat cap extension
    MaxCapRatio(ResourceId),          // % cap extension
    GlobalProductionRatio,            // layer 6
    CraftRatio,                       // craft output bonus
    CraftRatioFor(CraftId),           // per-recipe bonus
    HappinessPoints, UnhappinessRatio,
    TradeRatio, StandingRatio,
    PriceRatioDelta(StructureId), GlobalPriceRatioDelta,
    EnergyProduction, EnergyConsumptionRatio,
    ArrivalChanceBonus, KittenGrowthRatio,
    // … extended per wave; every variant is compiler-checked and greppable
}
```

Sources of effect values: structures (× count), refinements, ideas, house rules, rites, arch upgrades, traditions, colony standing, adventure rewards, the Elder's trait. The wonder wave adds effect-key variants — `MaxCapRatio(ResourceId)` (per-resource cap %, folded into `cap()` as `(base + MaxCap) × (1 + CapRatio + MaxCapRatio)`), `PrismDescentChance`, `RainbowShowerChance`, `UnicornsRatio`, `StarshardRefine`/`EchoRefine`, `ParadeRatio`, `StandingFor(ColonyId)`, `TradeRatio` (already present), the per-Altar rite outputs via `StructureOutputAdd`, and the legacy wave's `GlobalPriceRatioDelta` (DR-clamped in `next_cost`), `KittenGrowthRatio` (arrival-speed traditions), a global `StandingRatio` (Good Neighbours), `MasterSkillMultiplier` (Nap Time), `EnergyConsumptionRatio` (Frugal Meadow), season-scoped production ratios (`SeasonResourceRatio(season, resource)` for Quiet Retreat/Long Winter rewards), and the cosmic/dreamtime wave's `OutdoorProductionRatio`, `LongSunCapBonus` (Aurora Lantern), `DejaVuChance`, `ReverieYield`, and the nap knobs `HeatEfficiency`/`HeatDissipation` (Dream Loom) — with constants `heat_per_year`, `starshard_cost(years)` (escalating, heat-coupled), `max_nap_length` (base 100), and the outdoor transfer fraction pinned in the cosmic/dreamtime specs — and new persisted state (devotion, serenity + a monotone peak-devotion, enlightenment_tier, dream_drift, rites-bought/arch-bought sorted sets, Promises with quiet-flags, per-colony pen-pal levels + discovered set; and the legacy wave's elder_wisdom/warm_memories/passed_down_wisdom, generation, traditions set, a slim Family Album (name/trait/rank/profile/generation per archived kitten — not the full Kitten), and the adventure armed/active/completions/reward-pool + Tidy-Meadow goal timer; and the cosmic/dreamtime wave's sites_reached set, the Star Whale visit state (contentment + remaining visit), and dreamtime's stirred_dreams heat scalar, a déjà-vu-in-progress flag, and the dream-loom purchased set — reverie is the existing resource RES 26, and the cosmic outpost structures 40–52 reuse the structure array). All serde-default, BTree/sorted for determinism, appended to the save (v2 zero-fills). The engine exposes `effects(state, key) -> f64` summing all sources in a fixed order; leaves own *which* keys their content emits and *where* each key is consumed. Rejected alternatives are recorded in Decisions & Alternatives.

## Storage-Cap Gating (pacing philosophy)

Prices grow exponentially (`base × ratio^owned`, ceil per component); caps grow linearly via discrete structures and refinements. The real progression question is always "what raises my cap" — this is the master pacing valve, adopted deliberately. Cap-extension classes are targeted (barn-analog vs shed-analog cover different resource sets; specific structures extend specific caps) so each storage tier re-orders priorities. At least one structure inverts the flow (production scaled by a *cap*) to make storage an input. Price-ratio bands are a tuning vocabulary: ~1.10 (want many) → 1.12–1.15 (default) → 1.18–1.25 (global-multiplier structures) → 1.5–1.75 (deliberate walls) → ~2.0+ (housing, population pressure).

## Converters & Energy

**Converters** (structures consuming X/tick to produce Y/tick) starve gracefully: all inputs and outputs scale by the worst-supplied input's fraction — chains degrade proportionally, never flap. Engine owns the throttling primitive.

**Energy** is a late-game soft constraint: producers and consumers are content-table effects; deficit multiplies global-multiplier layers by `prod/cons` floored at 0.25 (never a shutdown); surplus feeds designated bonuses. Active/built split: production effects count *active* copies, storage effects count *built* — toggling is a real decision.

## Tick Pipeline (extended)

1. **Calendar & daily processing** — tick counter; on day boundaries, in order: day-close clock update (from the persisted shortfall flag), returns, wanders, arrival check, **jamboree countdown, colony/whale visit clocks, dream drift, Promise upkeep, yearly hooks on year boundaries** (wisdom trickle, automation-jam reset, Yarn Computer check). No events at game start.
2. **Production** — structures ☆ then jobs, via the production stack; then **converters** (throttled); then **Tinkerer background crafts**.
3. **Consumption** — kittens eat catnip (pooled); **luxuries drain per kitten**; converter fuel already handled in 2.
4. **Mood & flags** — happiness recompute; Grumpy flag; **mess recompute**.
5. **Random events** — zoomies expiry/start, flavor, **windfalls, sky events (observable), unicorn rifts, prism descents, colony arrivals** — all at day boundaries in a fixed draw schedule.
6. **Clamp** — trim stocks to caps.
7. **Achievements** — state conditions at a day close, moment conditions against the tick's events; also after every applied command. Reads the meadow, changes nothing but the record (achievements leaf).

Commands apply only between ticks. Saturating arithmetic on all day counters.

## Currency Tiers

| Tier | Members (leaf that owns definitions) | Properties |
|---|---|---|
| Raw materials (capped) | catnip, twigs, fluff, pebbles, buttons, embers, shinies, dew, moonstone, stardust, hushfelt (economy) | full production stack; caps from structures |
| Transient currencies | pep, curiosity, charm, warmth, skymaps (capped); cleverness (uncapped by design — the Yarn Computer's wit has no ceiling) | exempt from global multipliers (layers 6–8); each earned by its own subsystem |
| Luxuries (uncapped, draining) | feathers, bells, mint (economy; structures, expeditions, and colonies supply) | per-kitten drain; each distinct stocked luxury = +10 happiness points |
| Crafted goods (uncapped) | wands, tiles, badges, brass, cogs, moonsteel, lattices, pavers, quills, stories, anthologies, schemes, cairns, kites, balloons, moonmilk, dreamweave, cometsilk (economy) | made via crafting with craft-ratio bonus; gate currencies for research bands |
| Mystic chain | unicorns, rainbows, prisms, starshards, echoes, dreamcorns, jars of purrs (mystic) | sink-chain with upgradeable exchange rates and event faucets |
| Legacy (persist across Moves) | Elder Wisdom, Warm Memories, Passed-Down Wisdom, serenity (prestige/ceremonies) | prestige currencies at different timescales |
| Dreamtime | reverie, starshards (shared with mystic), stirred dreams (dreamtime) | earned in déjà-vu days / skips |

## Child Leaves and Ownership

| Leaf | Prefix | Owns |
|---|---|---|
| `engine/` | `SIM-ENG` | GameState, command API, tick pipeline, calendar, RNG, determinism, event vocabulary, effect-key system + DR primitives, converter throttling, unlock/visibility queries, speed, random-event scheduling |
| `economy/` | `SIM-ECO` | Facets: `RES` (all stocks/caps/tiers), `BLD` (structures, staged rebuilds, price ramps), `CRAFT` (recipes, craft bonus, manual + Tinkerer crafting), `NRG` (energy), `MESS` (mess) — one leaf; promotion to sub-HLD deliberately deferred (see Decisions) |
| `kittens/` | `SIM-KIT` | Population lifecycle, names, jobs & assignment, happiness (incl. luxury bonus), skills/XP/ranks, the Elder + traits, jamborees, famine/return |
| `curiosity/` | `SIM-CUR` | Ideas tree (research bands + gate-crafts), Refinements (tool/storage/automation ladders), House Rules (mutually exclusive policies) |
| `expeditions/` | `SIM-EXP` | Pouncing squads: pep costs, Irwin-Hall yield rolls, luxury/unicorn/shiny outcomes |
| `ceremonies/` | `SIM-SUN` | warmth → Basking → devotion → Sunrise Reverie → serenity; the Long Sun bonus; Sunbeam Rites ladder; Enlightenment tiers; Dream Cartography (echo-priced) |
| `mystic/` | `SIM-UNI` | Unicorns, Parades, Rainbow Arch ladder, prisms, starshard/echo refining, dreaming & dreamcorns, Promises, Jars of Purrs |
| `colonies/` | `SIM-TRADE` | Eight neighbor colonies, standing/shyness, Pen Pals, Scout Picnics, seasonal trade rates, kite scaling, Star Whales |
| `prestige/` | `SIM-MOVE` | The Great Meadow Move, Elder Wisdom (+shop: Traditions incl. Nine Lives ladder), Warm Memories, Passed-Down Wisdom, Family Album, Traveling Baskets, Memory Chest |
| `adventures/` | `SIM-ADV` | Opt-in variant runs with LDR-capped permanent rewards |
| `cosmic/` | `SIM-SKY` | Expedition sites beyond the meadow, missions, site structures, outdoor production channel, skymap economy |
| `dreamtime/` | `SIM-DREAM` | Starshard skips, stirred dreams & dissipation, Dream Loom, the Deep Nap, déjà-vu days & reverie, the Distant Dawn |
| `achievements/` | `SIM-ACH` | The achievement table, noticing when one is earned, and the record of what was earned and when — pure delight, read by no rule |
| `persistence` (sibling segment, not under sim) | `SAVE` | Format v2, migrations (v1→v2), catch-up × speed, Family Album persistence |

Cross-leaf contract style is unchanged: leaves expose pure per-phase functions and effect-key contributions; the engine sequences; nobody reaches into another leaf's state.

## Build Waves (execution roadmap)

Each wave walks LLD → EARS → edge audit → tests → code for its segments, keeping all prior specs green:

1. **Engine** — effect system, DR primitives, speed, unlock queries, converter throttle, save v2 + v1 migration.
2. **Economy** — full RES/BLD/CRAFT/NRG/MESS content.
3. **Society** — kittens skills/Elder/jamborees/luxuries; expeditions; curiosity tree + refinements + house rules.
4. **Wonder** — ceremonies, mystic, colonies.
5. **Legacy** — prestige (the delight-priority segment), adventures.
6. **Cosmic** — cosmic sites, dreamtime.
7. **Integration** — cross-wave audit, balance property tests, coherence verification.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Effect plumbing at depth | Typed effect-key enum + fixed resolution stack | (a) Hand-coded per-system math (v1 shape); (b) string-keyed dynamic effects (inspiration's shape) | Hand-coding ~40 structures × ~150 refinements is unmaintainable; string keys lose the compiler. Typed keys keep grep + exhaustiveness checks and make every new modifier a compile-checked variant |
| Economy leaf shape | One leaf with RES/BLD/CRAFT/NRG/MESS facets | Promote economy to a sub-HLD with child leaves | The facets are categories of one intent (the material economy); promotion would force a spec-ID re-parent across tests and code mid-expansion for zero behavioral change. Revisit after the expansion lands |
| Speed | Persisted `GameState.speed`, scales catch-up | Shell-side playback rate | Idle-respect: attended fast-play must not beat absence (HLD decision 7) |
| Balance source | Adopt the inspiration's *curve shapes and stack order*, not its numbers | Copy numbers verbatim; invent everything fresh | Its numbers assume its pacing and its harsher survival loop; its shapes (exponential price vs linear caps, DR-capped multiplier layers, rotating gate currencies) are the proven depth skeleton |
| DR discipline | Every stacking bonus passes limited/unlimited DR | Raw additive stacking | The inspiration's demonstrated guard against runaway multiplier growth |
| Grim-mechanic handling | Structural inversion per § Cozy Inversions, preserving the mechanical role | Omit the systems; keep them grim | Omission loses depth (these systems ARE the mid/late game); keeping them grim violates Tenet 1. Inversion keeps the math and changes the story |
| Risk mechanics | Keep risk, redesign failure as suspension/hiding (Promises go quiet; Yarn Computer stashes) | No downsides anywhere | Depth needs tension; Tenet 1 bans loss, not consequence. Suspension is consequence with a road back |

## Open Questions & Future Decisions

### Deferred
1. Whether economy promotes to a sub-HLD after the expansion (revisit at integration wave).
2. Whether adventures unlock via a Tradition (inspiration's pattern) or are visible from first Move — decide in the Legacy wave against Tenet 3.
3. Flavor-event pool growth per wave — each wave should add templates; tune frequency at integration.

## References

- Parent: `docs/high-level-design.md` (tenets, speed decision, depth goal)
- Children: leaves per § Child Leaves table
- Sibling segment: `docs/intent/persistence/`
- Mechanical inspiration: Kittens Game source (github.com/nuclear-unicorn/kittensgame), digested 2026-07-05; curve primitives `limitedDR`/`unlimitedDR`, production stack order, storage-cap gating, price-ratio bands, converter throttling, three-currency prestige
