---
parent: sim
prefix: SIM-UNI
---

# Mystic

## Context and Design Philosophy

The mystic leaf is the unicorn economy and everything glittering that follows from it — the inspiration's unicorn/ziggurat/alicorn/tears chain, reimagined so nothing is ever sacrificed. Unicorns hold **Parades**; rainbows arrive in gratitude, and **prisms** drift down after the glitter, with now and then one descending on its own; prisms that drift into the clowder's shared dreams become **dreamcorns** (asleep elsewhere, never gone). The leaf owns the mystic currency chain, the Rainbow Arch ladder, the dreaming/dreamcorn system, the Promises to the Night Court, and the Jars of Purrs. It consumes unicorns (economy stock, from Unicorn Meadows and pounces) and contributes multiplier effects through the effect-key system.

## The Currency Chain

| Currency | Gained by | Role |
|---|---|---|
| **unicorns** | Unicorn Meadow (economy, +0.001/t × mystic multipliers), Pounces | held for Parades; Meadow production is multiplied by this leaf's `unicornsRatio` effects |
| **rainbows** | **Parade** (`HoldParade`): 2500 unicorns → 2500 rainbows per Rainbow Arch built | buy Arch upgrades and Rainbow Arches |
| **prisms** | **Parade**: 10 per Rainbow Arch built, beside the rainbows; rare descents (day-boundary event; base chance from each Rainbow Arch *structure* plus Arch *upgrades*); the Sky Arch's steady drift | buy Arch upgrades; refined into starshards; or drift into dreams |
| **starshards** | **refine** 25 prisms → 1 starshard × (1 + shard_refine) | dreamtime currency (skips), gate craft for high research |
| **echoes** | refine 25 starshards → 1 echo | cryptotheology-analog currency (ceremonies/dreamtime) |
| **dreamcorns** | **dreaming**: while dream-drift is positive, prisms fall asleep into dreamcorns | fuel Promises |
| **Jars of Purrs** | condense 10000 rainbows → 1 Jar (capped by mystic content) | the condensed-joy currency; a rare cap raiser |

Every conversion is an *offering that returns more than it takes* or a *transformation* — never a loss. A Parade sends unicorns marching off in glittering procession and rainbows arrive because the neighbourhood loved it; prisms that drift into dreams are dreaming, not destroyed, and a Promise kept keeps them contentedly asleep.

## Rainbow Arches

Arches (economy structure, gated by the Parade Routes idea) are the Parade exchange rate — every copy adds 2500 rainbows and 10 prisms to each Parade. **Researching Parade Routes grants the first Rainbow Arch free** (the SIM-TRADE-008 precedent), so the first Parade already returns rainbows and the arch's base prism-descent chance seeds the prism supply — the loop bootstraps without a chicken-and-egg. Arch upgrades (`BuyArch`, rainbow + prism priced, ~8):

| # | Arch upgrade | Cost | Effect |
|---|---|---|---|
| 0 | Ribbon Arch | 500 rainbows | unicorns +5% (the Rainbow Arch *structure* already grants +0.02%/day base prism descent per copy) |
| 1 | Crystal Arch | 25000 rainbows, 25 prisms | unicorns +10%; +0.05%/day prism descent |
| 2 | Prism Arch | 50000 rainbows, 50 prisms | unicorns +25%; +0.05%/day rainbow shower (a windfall of 5000 rainbows) |
| 3 | Sky Arch | 125000 rainbows, 500 prisms | unicorns +50%; +0.0002 prism/t (a steady drift) |
| 4 | Dreaming Arch | 500 shinies, 1M rainbows, 5000 prisms | unicorns +250%; starshard refine +5% |
| 5 | Starlit Arch | 1250 shinies, 3M rainbows, 25000 prisms | unicorns +500%; starshard refine +10% |
| 6 | Memory Arch | 200 stardust, 6M rainbows, 50000 prisms | turns the clowder's Rainbow Arches into the dreamcorn engine (enables dream-drift; the rate scales with Rainbow Arch structures built) |
| 7 | Everdream Arch | 20 dreamcorns, 1000 cairns | Jars of Purrs cap +2 each; unlocks Jar condensing |

The two upgrades that dream, the Memory Arch and the Everdream Arch, also wait on the Dream Tending idea (curiosity leaf). The other six need Parade Routes alone.

Each arch upgrade carries a **blurb**: a short joke and then its effect in plain words, marking each resource it names as the curiosity leaf's blurbs do (SIM-CUR-014). The shell shows it as what the arch upgrade does, so every one says why it is worth having before it is bought (Tenet 3).

## Pacing

The ladder is paced to the HLD's months-long burn: a few days between steps early on, rising to about a week near the top, at 1× and counting time away (catch-up runs the same rules, so an absence moves the ladder exactly as presence would).

Both of a step's prices come from Parades, so the ladder feeds itself. Every Arch upgrade's unicorn bonus brings more Parades, and every Parade brings rainbows and prisms in proportion to the Rainbow Arches built — a supply that grows with the arch count *and* the unicorn rate, which is the shape of the income the prism prices were sized against (sim: Balance source). The early steps wait on rainbows, which the Rainbow Arches that raise the exchange rate compete for. The later steps wait on prisms; their rainbow prices take about half of what Parades bring while those prisms gather, and the rest goes to more Rainbow Arches, a sink for rainbows for as long as there are cairns to build them. The rare descents and the Sky Arch's drift are the share of the prism supply that needs no Parade.

Prices in materials from outside the mystic chain stay within what a clowder holds at that point in its play. The Memory Arch's stardust fits in four Star Basins' storage, since stardust is capped and a price above the cap is a wall no amount of waiting climbs; the Everdream Arch's dreamcorns are about a week of the dreamcorn engine with thirty-odd Rainbow Arches.

The same prisms feed starshards and echoes, which the ceremonies, curiosity, cosmic, and dreamtime leaves price in. At the top of the ladder the reference clowder's prisms refine into several echoes a day, so those prices draw on a supply that grows with the ladder rather than on the descents alone.

**The reference clowder** that the pacing is held to stands at the Parade Routes idea with its free Rainbow Arch, nineteen Unicorn Meadows, the Dream Tending idea, 15000 cairns, and the ladder's shinies and stardust in hand. It holds a Parade whenever it has the unicorns, buys the next Arch upgrade the moment it can afford it, and otherwise builds Rainbow Arches while it can, keeping back the cairns the Everdream Arch asks. It reaches the top of the ladder in about five weeks.

## Dreaming & Dreamcorns

The Rainbow Arch structure's four rates (the rainbows and the prisms a copy brings to a Parade, the chance it adds to a day's prism descent, and the dream-drift it accrues) are fields of its definition that the rules here read (economy leaf: BLD).

The **Memory Arch upgrade** turns the clowder's Rainbow Arches into a dreamcorn engine: buying it *enables* dreaming, and the drift rate then **scales with the number of Rainbow Arch structures built**, so the engine grows as the player raises more arches. `dream_drift` accrues per tick as `+0.000001 × Rainbow Arch structures`, reduced ×0.25 while any dreamcorn is already dreaming (the inspiration's corruption self-throttle, softened) — *this is a slowdown of the drift rate, never a loss*. When `dream_drift ≥ 1`, one dreaming prism becomes a dreamcorn (`prisms −1, dreamcorns +1`), but **never the last prism** (a prism must stay awake to keep the dream going — the cozy floor). Dreamcorns are the softest, sleepiest resource; they are never consumed except by a Promise's upkeep, and even then they are only *kept asleep*, not spent away.

## Promises to the Night Court

`Promises` (the inspiration's pacts, ~post-Enlightenment): each Promise costs echoes and fills a limited slot (slots from Everdream content), consuming 0.0005 dreamcorn/day as gentle upkeep and granting global production +0.05%. A Promise with insufficient dreamcorn upkeep **goes quiet** — its effect suspends and its slot is held — and wakes automatically when dreamcorns are available again. Nothing shatters, nothing is destroyed; a neglected Promise simply naps until the clowder can keep it. (Contrast the inspiration's Fracture, which permanently destroys pacts, alicorns, and all skills — Clowder has no such state.)

## Jars of Purrs

Condensing 10000 rainbows into 1 Jar of Purrs is the condensed-contentment currency (the inspiration's Black Liquid Sorrow, inverted from suffering to joy). Jars are capped (Everdream Arches raise the cap); overflow rainbows simply aren't condensed. A Jar is precious and does nothing harmful — its late-game role (opening cap ceilings) lands with the cosmic/dreamtime waves.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Parade exchange rate from arches | rainbows_per_parade scales with Arches built | Fixed rate; rate as an upgrade | The inspiration's ziggurat-count-as-tears-rate pattern; makes Arches doubly valuable (rate + upgrades) |
| Where most prisms come from | Each Rainbow Arch adds 10 prisms to every Parade, beside the rare descents and the Sky Arch's drift | Descents alone; cut the prism prices to fit the descents; a steady per-tick prism trickle from each arch | Descents grow only with the arch count, and every arch costs more than the last, so prism income crept up while the prism prices climbed by orders of magnitude: steps stalled for weeks and months. Cutting the prices to fit leaves a ladder that cannot feed itself and starshards and echoes starved. A per-arch trickle grows with the arches but not with the ladder's unicorn bonuses. Per-Parade prisms grow with both, as the income the inspiration pays its matching price column from does |
| Rainbow prices near the top | About half of what Parades bring while the step's prisms gather (Starlit Arch 3M; Memory Arch 6M) | The inspiration's figures, the dearest about a dozen Parades; exactly what Parades bring meanwhile | The inspiration pays that column from a separate source; here both prices come from Parades, so the small figures piled up unspent. Matching Parade yield exactly would leave nothing for the Rainbow Arches that raise the exchange rate |
| How the pacing is checked | A model of the reference clowder that steps from one Parade to the next, reading every rate from the sim's own queries and letting real ticks run the dreaming rule, held to the real tick by a calibration test | Replaying the ladder's weeks in real ticks | Five weeks is twelve million ticks, far past the core suite's few-second budget. Reading the sim's own rates keeps the model from restating rules, and the calibration catches it drifting from the tick |
| What a Promise grants | Global production +0.05% | The same plus storage, warmth, and craft ratios | A quiet Promise suspends its effect, and a suspended storage ratio would trim stocks to the lower cap: confiscation by clamp, which Tenet 1 rules out (the Porch's kite ratchet exists to avoid the same thing). The other channels add no decision at this size |
| Which conversions a multiplier scales | Only the starshard refine, which Arch upgrades feed | A multiplier on the Parade, the echo refine, and the dream drift as well, waiting on content | A multiplier nothing feeds is a rule no test can exercise and a scaling the player can never meet; one arrives with the content that feeds it |
| Dreaming has a floor | The last prism never sleeps | Allow full conversion | Cozy: something must stay awake to dream; also keeps a prism faucet alive |
| Dreamcorn engine scales with Rainbow Arch structures | Memory Arch upgrade enables it; rate = per-arch drift × Rainbow Arch structure count | Flat rate gated on the once-bought Memory Arch upgrade | A once-bought flat trickle never grows, so reaching the dreamcorn endgame (Everdream Arch → Jar cap) took ~10× the design horizon; scaling with the buildable arch count makes the engine a build target that keeps pace with play |
| Promises go quiet, never break | Suspend + auto-resume on upkeep | Permanent destruction (Fracture) | Tenet 1 — depth needs upkeep tension, not catastrophic permanent loss |
| Jars from joy, not sorrow | Condense rainbows (contentment) | Keep the sorrow framing | The single darkest currency in the inspiration becomes the single sweetest here |

## Open Questions & Future Decisions

### Deferred
1. Jars of Purrs' late-game sinks (cap-ceiling raisers) — land with cosmic/dreamtime.
2. Whether a Promise's global effect should scale with the Enlightenment tier (the inspiration's Black-Pyramid × conduit) — decide when Enlightenment tiers are play-tested.

## References

- Parent: `../sim-design.md` (mystic tier, cozy inversions)
- Unicorn stock & Meadow: `../economy/`; Parade Routes idea & Rainbow Arch structure: `../curiosity/` + `../economy/`; echoes also feed `../ceremonies/` and `../dreamtime/`
