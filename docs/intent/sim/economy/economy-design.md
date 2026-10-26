---
parent: sim
prefix: SIM-ECO
---

# Economy

## Context and Design Philosophy

The economy is the material world of the game: what exists, what it costs, what produces what. One leaf, five facets — `RES` (resources), `BLD` (structures), `CRAFT` (crafting), `NRG` (energy), `MESS` (mess) — because these are categories of a single intent: the material loop that every other subsystem feeds multipliers into. It consumes worker counts and the mood multiplier from the kittens leaf, unlock flags from the curiosity leaf, and the engine's effect/DR/throttle machinery; it defines every stock, cap, price, recipe, and flow rate.

Balance philosophy: the early game is hand-fed, the mid game is delegated, the late game is orchestrated. Prices grow exponentially, caps grow linearly through discrete purchases (the sub-HLD's storage-cap gating), and every negative pressure is a slowdown. Numbers below are normative; `content.rs` mirrors them and property tests enforce the reachability and tier rules mechanically.

## RES — Resources

The full id space (append-only; serialized as an enum-indexed array, so **new ids only ever append** — v1 saves zero-fill forward). Base cap `∞` = uncapped (clamp skips it).

| # | Resource | Tier | Base cap | Notes |
|---|---|---|---|---|
| 0 | Catnip | raw | 200 | food & early currency |
| 1 | Twigs | raw | 50 | building material |
| 2 | Fluff | raw | 25 | soft material |
| 3 | Curiosity | transient | 0 | idea currency; cap 0 until a Cardboard Box exists |
| 4 | Pebbles | raw | 60 | mineral analog |
| 5 | Buttons | raw | 40 | sturdy metal analog |
| 6 | Embers | raw | 30 | fuel; Hearth byproduct, more from Gravel Pits |
| 7 | Shinies | raw | 15 | precious; trade & promotion currency |
| 8 | Dew | raw | 0 | fluid fuel; cap 0 until Dew Collectors |
| 9 | Moonstone | raw | 8 | rare metal; first from trade, later Kilns |
| 10 | Stardust | raw | 0 | cosmic fuel; cap 0 until Star Basins |
| 11 | Hushfelt | raw | 0 | the softest felt, found where it's very quiet; cosmic-tier |
| 12 | Pep | transient | 100 | action currency: pounces, trades, picnics, jamborees |
| 13 | Charm | transient | 80 | culture analog |
| 14 | Warmth | transient | 0 | ceremony currency; cap 0 until Sun Rooms |
| 15 | Skymaps | transient | 0 | sky-exploration currency; cap 0 until Stargazing Ledges |
| 16 | Feathers | luxury | ∞ | drains per kitten; +10 happiness while stocked |
| 17 | Bells | luxury | ∞ | ditto |
| 18 | Mint | luxury | ∞ | ditto; from trade (Rabbits) |
| 19 | Unicorns | mystic | ∞ | |
| 20 | Rainbows | mystic | ∞ | arrive in gratitude for Parades |
| 21 | Prisms | mystic | ∞ | rare descents |
| 22 | Starshards | mystic | ∞ | refined from prisms |
| 23 | Echoes | mystic | ∞ | echoes of the First Clowder |
| 24 | Dreamcorns | mystic | ∞ | prisms asleep in the shared dream |
| 25 | JarsOfPurrs | mystic | 0 | condensed contentment; capped by mystic content (wave 4) |
| 26 | Reverie | dreamtime | ∞ | earned only in déjà-vu days |
| 27 | Wands | crafted | ∞ | |
| 28 | Tiles | crafted | ∞ | |
| 29 | Badges | crafted | ∞ | |
| 30 | Brass | crafted | ∞ | |
| 31 | Cogs | crafted | ∞ | |
| 32 | Moonsteel | crafted | ∞ | |
| 33 | Lattices | crafted | ∞ | |
| 34 | Pavers | crafted | ∞ | |
| 35 | Quills | crafted | ∞ | from feathers |
| 36 | Stories | crafted | ∞ | band-1 gate currency |
| 37 | Anthologies | crafted | ∞ | band-2 gate currency |
| 38 | Schemes | crafted | ∞ | band-3 gate currency |
| 39 | Cairns | crafted | ∞ | monument material |
| 40 | Kites | crafted | ∞ | trade & sky capacity |
| 41 | Balloons | crafted | ∞ | lift for the sky missions that go up high (cosmic leaf) |
| 42 | Moonmilk | crafted | ∞ | cosmic propellant |
| 43 | Dreamweave | crafted | ∞ | late-game fabric, carried on the missions to the farthest sites (cosmic leaf) |
| 44 | Cometsilk | crafted | ∞ | stardust textile |
| 45 | Cleverness | transient | ∞ | the Yarn Computer's accumulating wit |

**Tier rules.** *Raw* resources take the full production stack. *Transient* currencies and *luxuries* and *mystic/dreamtime* resources skip global layers 6–8 (each is grown by its own subsystem's multipliers instead). *Crafted* goods are made, not produced, so the stack doesn't apply. Luxuries drain per Home kitten per tick — feathers 0.001, bells 0.0007, mint 0.0001, scaled by (1 + LuxuryDemandRatio effects, Gift Stands −4% each) — and each *distinct* stocked luxury grants +10 happiness points (kittens leaf consumes). Stocks are f64 with the 1e-9 dust guard everywhere zero-vs-nonzero matters.

## BLD — Structures

Cost of copy k+1 = `ceil(base × ratio^k)` per component. A gated structure reveals when its gate opens; an ungated one when the meadow holds 30% of each cost component (engine leaf: Unlocks & Visibility). Effects are effect-key contributions; "active" = the on-count for global/energy effects (built count for caps). Gates name ideas from the curiosity tree.

| # | Structure | Base cost | Ratio | Effects | Gate |
|---|---|---|---|---|---|
| 0 | Catnip Patch | 10 catnip | 1.12 | +0.125 catnip/t, seasonal | — |
| 1 | Cozy Cushion | 25 catnip, 5 twigs | 1.15 | houses 1; +25 pep cap | — |
| 2 | Cardboard Box | 8 twigs | 1.15 | +50 curiosity cap; +0.005 curiosity/t | — |
| 3 | Pantry | 20 twigs | 1.15 | +250 catnip cap | PantryOrganization |
| 4 | Twig Pile | 50 catnip, 10 twigs | 1.15 | +40 twig cap | TwigArchitecture |
| 5 | Scratching Post | 15 twigs, 5 fluff | 1.15 | +5 happiness pts | ScratchingArts |
| 6 | Cat Tree | 30 twigs, 10 fluff | 1.15 | houses 3; +10 fluff cap; +75 pep cap | TwigArchitecture |
| 7 | Window Perch | 40 catnip, 8 twigs | 1.15 | houses 1; +0.001 curiosity/t; +25 pep cap | CozyCarpentry |
| 8 | Herb Garden | 60 pebbles | 1.12 | catnip production +3%/bld; +0.00015 mint/t (a corner of every garden grows it) | Horticulture |
| 9 | Couch Mine | 90 twigs | 1.15 | +0.01 pebbles/t; +0.001 shinies/t (what turns up under the couch); pebble production +20%/bld | CouchSpelunking |
| 10 | Gravel Pit | 80 tiles, 15 brass | 1.15 | pebble production +35%/bld; +0.004 embers/t; mess 0.25 | Excavation |
| 11 | Hearth | 45 pebbles, 25 twigs | 1.15 | converter −0.05 twigs −0.10 pebbles → +0.02 buttons/t, +0.001 embers/t; +2 happiness pts; mess 0.1 | Emberkeeping |
| 12 | Kiln | 60 tiles, 150 dew, 30 brass | 1.15 | converter −0.5 pebbles −0.024 dew → +0.15 buttons/t, +0.0005 moonstone/t; mess 1.0 | CeramicArts |
| 13 | Shed | 2 wands, 3 tiles | 1.15 | caps: twigs+150, pebbles+200, buttons+25, embers+30, moonstone+10, shinies+5 | PackRattery |
| 14 | Porch | 40 tiles, 60 badges, 4 lattices | 1.15 | caps: catnip+1000, twigs+400, pebbles+500, buttons+80, embers+50, moonstone+25, shinies+12; all Porch caps ×(1+LDR(0.01·kite high-water, 2.25)) — the multiplier ratchets on the most kites ever held, so spending kites never shrinks storage (cozy: no confiscation-by-clamp) | PorchLogistics |
| 15 | Study Nook | 30 twigs, 45 pebbles, 80 curiosity | 1.15 | curiosity production +20%/bld; +200 curiosity cap | Notetaking |
| 16 | Stargazing Ledge | 250 buttons, 800 curiosity, 20 tiles, 40 lattices | 1.10 | curiosity production +25%/bld; +800 curiosity cap; +10 skymap cap; sky-event chance +0.2%/bld; auto-observe +1%/bld | SkyCharts |
| 17 | Whisker Lab | 1200 curiosity, 60 tiles, 40 brass | 1.10 | curiosity production +35%/bld; +1200 curiosity cap; energy −1 | DeepCuriosity |
| 18 | Craft Corner | 80 twigs, 300 pebbles | 1.15 | craft ratio +6%/bld | Tinkering |
| 19 | Toy Workshop | 250 buttons, 12 pavers, 18 moonstone | 1.15 | craft ratio +5%/bld; +1 Tinkerer seat; energy −2; mess 2 (Tidy rota flips to −2) | Toymaking |
| 20 | Gift Stand | 400 twigs, 150 pebbles, 8 shinies | 1.15 | trade ratio +1.5%/bld; luxury demand −4%/bld | GiftEconomy |
| 21 | Story Circle | 160 twigs, 900 pebbles, 3 quills | 1.15 | crowding relief 0.048/bld (pooled with Stages, LDR cap 0.9 — kittens leaf consumes); +0.005 charm/t; +40 charm cap | OralTradition |
| 22 | Sun Room | 1600 pebbles, 200 charm, 200 quills | 1.15 | +0.04 charm/t; +0.004 warmth/t; +75 warmth cap; +60 charm cap (a sunny room invites conversation) | SunSalutations |
| 23 | Sunbeam Altar | 40 shinies, 20 tiles, 12 badges, 8 stories | 1.15 | +0.08 charm/t; +120 warmth cap; +100 charm cap; +100 shinies cap (offerings collect); hosts Rites (ceremonies leaf) | SolarDevotion |
| 24 | Treat Bakery | 4000 pebbles, 150 shinies, 150 badges | 1.15 | converter −0.004 shinies −0.6 pep → feathers ×1.25·mpr, bells ×0.3·mpr where mpr = pep_cap × 0.007/100 | TreatAlchemy |
| 25 | Tea House | 800 twigs, 600 charm, 4 mint, 300 quills | 1.5 | jamboree ratio +1%/bld; jamboree arrival +0.1%/bld | TeaCeremonies |
| 26 | Dew Collector | 40 brass, 20 cogs, 20 lattices | 1.15 | +0.02 dew/t; +1200 dew cap | DewHarvest |
| 27 | Cogworks | 50 brass, 15 cogs | 1.25 | Dynamo boost +15%/bld; energy +3; yearly auto-craft of capped raws (98% rule); mess 0.5 | Cogwheels |
| 28 | Dynamo Wheel | 4 cogs, 8 moonsteel, 1 scheme | 1.25 | −0.05 dew/t; global production +2%/active ×(1+0.15·Cogworks) × fuel fraction; energy +5 × fuel fraction; mess 5 (a starved wheel spins down proportionally) | Dynamos |
| 29 | Sunning Deck | 60 moonstone | 1.15 | energy +2 (winter ×0.75, summer ×1.25) | SolarBasking |
| 30 | Stream Wheel | 120 moonstone, 25 pavers | 1.15 | energy +5 | Millworks |
| 31 | Star Basin | 150 moonstone, 1200 badges, 12 pavers, 6 schemes | 1.15 | global production +5%/active; +0.0003 stardust/t; +60 stardust cap; energy +10 | Starcatching |
| 32 | Prism Spinner | 250 moonstone, 6 stardust, 30 pavers | 1.15 | converter −0.004 moonstone → +0.0006 stardust/t; energy −2 | PrismCraft |
| 33 | Box Labyrinth | 30 brass, 90 tiles | 1.15 | +1500 curiosity cap; energy −2 | LabyrinthTheory |
| 34 | Moonlight Stage | 250 buttons, 20 moonstone | 1.15 | crowding relief 0.075/bld (pooled with Circles, LDR cap 0.9); +0.25 charm/t × clamp(production/max(consumption,1), 0, 1.75) — an unpowered stage is dark by design | MoonlightArts |
| 35 | Memory Chest | 700 hushfelt, 60000 curiosity, 1 starshard, 25 schemes | 1.25 | Move carryover +1.5%/bld; déjà-vu chance +1%/season; energy −20 | MemoryWeaving |
| 36 | Yarn Computer | 30 stardust, 60000 curiosity | 1.15 | +0.02 cleverness/t; energy −(2·n + n²/2) total at n copies | YarnComputing |
| 37 | Unicorn Meadow | 2 unicorns | 1.75 | +0.001 unicorns/t (mystic leaf multiplies) | UnicornLore |
| 38 | Rainbow Arch | 5000 rainbows, 25 cairns | 1.15 | count is the Parade exchange rate; hosts Arch upgrades; +0.02%/day prism-descent chance each (mystic leaf consumes); first copy granted free with the Parade Routes idea | ParadeRoutes |
| 39 | Sunstone | 8 moonstone, 50 cairns, 1 echo | 1.15 | Long Sun cap +0.05 per enlightenment tier each (ceremonies leaf consumes); energy −1 | LongSunMasonry |
| 53 | Broom Cupboard | 30 wands, 10 brass | 1.15 | mess −1 (tidies) | — |
| 54 | Clockwork Broom | 30 cogs, 12 moonsteel | 1.15 | mess −5 (tidies); energy −2 | — |

No staged rebuilds in this pass: every structure is its own entry (see Decisions). The table steps from 39 to 53 because ids 40–52 are the cosmic outposts described below. The tidying structures 53–54 carry no gate (see MESS). Structures 38–39 are cross-leaf: they live in this table (the economy owns the StructureId space, costs, and ramps) but their effects are defined and consumed by the mystic and ceremonies leaves respectively — the same split as the Sunbeam Altar hosting Rites. Structures 40–52 are the cosmic outposts (Lookout Post … Wishing Star): they too live in the StructureId space, but they are gated on their *site being reached* (a cosmic-leaf state, not an idea), their production flows through the cosmic **outdoor** channel (diluted global/wisdom/Long-Sun transfer), and their effects are owned by the cosmic leaf. Their build costs escalate off cosmic materials (moonmilk, cometsilk, hushfelt, Jars) per the cosmic design.

## CRAFT — Crafting

`Craft(id)` makes one batch: output = 1 × (1 + CraftRatio effects). All-or-nothing input spend, epsilon-tolerant. Recipes (inputs per batch):

| Craft | Inputs |
|---|---|
| Wands | 15 twigs |
| Tiles | 25 pebbles |
| Badges | 12 buttons |
| Brass | 10 buttons + 10 embers |
| Cogs | 8 brass |
| Moonsteel | 1 moonstone + 8 brass |
| Lattices | 5 wands |
| Pavers | 250 tiles + 3 brass |
| Quills | 18 feathers |
| Stories | 40 charm + 3 quills |
| Anthologies | 1000 curiosity + 5 stories |
| Schemes | 2500 curiosity + 3 anthologies |
| Cairns | 3 wands + 5 tiles + 1 badge |
| Kites | 3 skymaps + 15 badges + 10 lattices |
| Balloons | 125 moonsteel + 20 kites + 1 scheme |
| Moonmilk | 750 dew |
| Dreamweave | 100 hushfelt + 250 moonsteel |
| Cometsilk | 25 stardust |

Each recipe carries a **tier** for Tinkerer work (SIM-KIT-026's rank comparison): tier 1 — Wands, Tiles, Badges, Quills, Cairns; tier 2 — Brass, Lattices, Stories; tier 3 — Cogs, Pavers, Anthologies, Kites; tier 4 — Moonsteel, Schemes, Moonmilk; tier 5 — Balloons, Dreamweave, Cometsilk.

Craft unlocks ride the idea tree (a craft is `Locked` until its granting idea). Gate-currency pattern: stories (band 1), anthologies (band 2), schemes (band 3) enter idea costs — since anthologies/schemes cost *stocked curiosity*, curiosity **storage** becomes the real research gate, forcing the Ledge/Lab/Labyrinth loop. Tinkerer background crafting (kittens leaf, wave 3) consumes these same recipes.

**Tinkerer seats.** A Tinkerer works a recipe only while bound to it (SIM-ECO-043), and every binding takes a seat. Seats are a structure field, as housing is: a structure's definition carries the tinkerer seats each copy gives — the Toy Workshop's one, in the BLD table, and none for any other structure — and the meadow's seats are the sum over structures of the copies held times the seats each gives. A binding holds its seat whether its kitten is at home or away: a visiting Tinkerer keeps its recipe and returns to it, so its seat waits for it. The seats stand in the definition rather than in the bind's rule because the definition is what the player is shown — the inspector states every field a structure's definition carries — so a seat the rule alone granted would be a rule no screen could state, and a second structure that seats tinkerers would be a rule change rather than a table entry. Who holds the seats and who waits for one are reported by the engine's bench query (engine leaf: Derived Queries).

**When the Tinkerer commands exist.** AssignTinkerer and UnassignTinkerer exist for a meadow exactly while the Tinkerer job does — while its granting idea is researched — and their gate (SIM-ENG-045) says so. Before then no kitten can hold the job, so a bind could never be made, and a gate open from the start would put a tinkerer count row under every recipe from the meadow's first craft onward, each offering an act no press can take.

## NRG — Energy

`EnergyProduction` and consumption are effect sums over *active* structures. Deficit: `deficit_ratio = clamp(production/consumption, 0.25, 1.0)` multiplies the contributions of stack layers 6–8 (global production, Wisdom, Long Sun). Never a shutdown. Surplus: `surplus_ratio = min(production/max(consumption, 1), 1.75)` feeds only effects that name it (Moonlight Stage). Sunning Deck output is seasonal (winter ×0.75, summer ×1.25) — the grid breathes with the year.


The core reports the meadow's energy structure by structure: for each built structure that makes or uses energy, in structure order, its signed share — what its copies make, seasonal and fuel-scaled as production is, or what they use, the Yarn Computer's escalation and any consumption ratio included — so the shares add up to the production and consumption above and a frontend can say where the energy goes without a rule of its own (SIM-ECO-048).

## MESS — Mess

A structure's mess per copy is its BLD-table mess plus the tidy effects that name it. Where that sum is positive, each built copy makes that much mess per tick; where it is negative, each copy tidies that much away. A converter makes or tidies in proportion to its throughput (its throttle fraction). The tick's **mess accrual** is all the mess made; its **tidying** is all the mess tidied away. The two stay separate sums, because a rule that scales one must not scale the other: a twist that makes mess accrue faster (adventures leaf) scales the accrual and leaves the tidying as it is (SIM-ECO-050).

Each tick, mess changes by accrual − tidying − dissipation, where dissipation is 1e-4 × mess, and it never falls below zero: a meadow cannot be tidier than clean, and tidying banks nothing against mess made later. With no tidying, mess settles where accrual equals dissipation (mess = accrual / 1e-4). Mess level = floor(log10(mess / 1e4)) when mess ≥ 1e4, else none. Level effects (all slowdowns): L0+: catnip growth ×0.8 per level step, floored at ×0.25; L1+: arrival chance ×0.75; L2+: −5 happiness points per level above 1. Mess never destroys anything.

Tidying comes from three sources, each at the scale of the mess it meets:

- The **Broom Cupboard** (−1 per copy) is built from wands and brass. Brass is also what the first serious mess-makers, the Gravel Pit and the Cogworks, are built from, so tidying turns up when they do.
- The **Tidy-Up Rota** refinement (curiosity leaf) turns each Toy Workshop's 2 into −2.
- The **Clockwork Broom** (−5 per copy, energy −2) is built from cogs and moonsteel, so it turns up with the Dynamo Wheel, the meadow's biggest mess-maker, and runs on the energy the wheel supplies.

The two tidying structures carry no gate. Each reveals when the meadow holds 30% of its price, as any ungated structure does, so a way back from mess opens as soon as the meadow holds what builds it.

Tidying can always outpace mess-making: at 20 copies of every structure, with every tidy effect in the content tables applied, the meadow tidies more per tick than it makes (SIM-ECO-051). The figures are 160 tidied (Broom Cupboards 20, rota'd Toy Workshops 40, Clockwork Brooms 100) against 137 made (Hearths 2, Gravel Pits 5, Kilns 20, Cogworks 10, Dynamo Wheels 100).

The core reports mess as a frontend needs to state it: the amount, its level (or none), how fast it is changing per game second at current state, and what the level is slowing: the catnip-growth multiplier, the arrival multiplier, and the happiness points it costs (SIM-ECO-049). The rate is how far the next tick would move the mess: accrual less tidying and dissipation, stopping at zero. A clean meadow that tidies more than it makes is holding steady, not falling.

## Jobs (extended roster)

New JobId entries land with this wave as data; assignment mechanics are unchanged (SIM-KIT). Yields per effective worker per tick, mood-multiplied:

| Job | Yield/tick | Gate |
|---|---|---|
| **Rummager** | +0.02 pebbles | CouchSpelunking |
| **Pouncer** | +0.06 pep | PouncingPractice |
| **Sunwarden** | +0.0015 warmth | SolarDevotion |
| **Tinkerer** | background crafting (society wave); no passive yield | Toymaking |

## Manual Actions

| Action | Effect | Gate |
|---|---|---|
| Gather catnip | +1 catnip (×2 with SelectiveHearing) | — |
| Forage | −10 catnip, +1 twig | — |
| Rummage | +0.5 pebbles | CouchSpelunking |

StorageFull rejection when the yield resource is at cap (unchanged rule).

## Consumption

(Unchanged from v1: pooled 0.085 catnip/tick per Home kitten, shortfall semantics, dust guard.) Luxury drain runs in the same consumption phase, after food: pooled per-luxury deduction, floored at zero — an empty luxury shelf simply drops its happiness bonus (never a shortfall/Grumpy trigger).

## Balance Anchors (tests enforce)

- One patch in summer feeds ~1.5 kittens; a gardener ~1.75 (unchanged).
- Reachability (SIM-ECO-024): structures gated in bands 0–2 keep their first 20 copies within achievable caps; band 3+ structures keep their first 5 copies (first copy for band 4) — the plateau bites later the deeper the tier. **Declared walls are exempt from their band's horizon and use the 5-copy rule instead**: the Tea House (ratio 1.5) and the Treat Bakery are deliberate few-copy structures, in the sub-HLD's price-band vocabulary. The Yarn Computer is a **declared monument** (first-copy horizon): one is a project, a second is a lifestyle, and curiosity storage is not obliged to keep up. Hushfelt's cap extender is owed by the cosmic leaf and exempt until it lands.
- Every craft's inputs are producible before or at the band where the craft unlocks (no dead recipes) — **except the expedition/colony faucets** (feathers, bells, mint, shinies, unicorns), owed by the society and wonder waves and exempted from this wave's property test with an explicit list.
- Global-layer exemption: a Dynamo Wheel never changes pep/charm/warmth/curiosity/skymap/luxury/mystic income.
- Pounce-vs-bakery: with fewer than 10 Pouncers, an unthrottled Treat Bakery out-drinks pep income — the shinies input throttles it first in practice; the tension is intended (see Decisions).
- Energy floor: no deficit can push layers 6–8 below ×0.25.
- Mess is reversible: at 20 copies of everything, tidying outpaces mess-making (SIM-ECO-051; MESS has the figures).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Energy before the band-3 producers | The Cogworks, the one band-1 producer, gives enough energy that one covers a band-2 consumer or two | Band-2 consumers with only a +1 producer; band-2 consumers that use no energy | Every band-2 energy consumer arrives a band before any other producer, so a +1 Cogworks at a 1.25 price ramp leaves a meadow in deficit for the whole band, with the deficit cutting the global layers it has no other way to restore. Consumers that use no energy until later would remove the trade-off the band introduces energy to teach |
| The Kiln's price | Tiles, dew, and brass, no moonstone | A little moonstone, bought from a neighbour | The Kiln is the meadow's own moonstone source; pricing it in moonstone leaves a meadow without the neighbour that sells it with no way to build the building that makes it |
| Mint's supply | A trickle from every Herb Garden, trade with the Rabbits on top | Trade alone | Mint is a luxury every kitten uses up, and a single half-chance trade cannot keep a grown clowder's mint stocked; feathers and bells already have a structure source in the Treat Bakery |
| Every crafted good has a use (SIM-ECO-047) | Balloons and Dreamweave each fuel sky missions (cosmic leaf) | Crafts kept for flavour with nothing to spend them on | A recipe with no use is labour the player can only waste, and its Tinkerer bindings and trait bonuses with it |
| Where shinies come from | A steady trickle from every Couch Mine, with pouncing as the active bonus on top | Pouncing alone; a new structure of their own | Shinies price every caravan, every promotion, and the rites, so a supply that only active play brings in makes trading an obligation to keep pouncing and lets a converter drain them while the player is away (Tenet 2). The Couch Mine is already built early and is where lost glinting things would be found; a structure of its own would be one more thing to learn for a currency the player needs from the first trade |
| Id-space growth | Append-only enums; arrays zero-fill on load | Re-orderable ids; map-keyed stocks | Append-only keeps every old save's indexes valid and serialization canonical |
| Staged rebuilds | None — separate structures | KG-style deltagrades (pasture→solar farm) | Staging recycles UI slots but resets counts and complicates save/undo semantics; a TUI-era decision, revisit after the shell exists |
| Buttons' early source | Hearth converter (twigs+pebbles→buttons) | Mining-only; trade-only | Mirrors the inspiration's smelter so the early loop teaches converters before the Kiln tier |
| Moonstone gating | Pre-Kiln moonstone only via trade (wave 4) | Native early source | The inspiration's titanium pattern: trade opens the metal tier early for those who invest in it, the Kiln arrives later |
| Gate crafts | stories/anthologies/schemes cost stocked curiosity | Direct curiosity prices only | Makes curiosity *storage* the research gate — the self-reinforcing infrastructure loop that gives the mid-game its shape |
| Cleverness as a resource | Ordinary transient resource | Hidden non-resource accumulator | Keeps Yarn Computer state visible, serializable, and normalizable like everything else |
| Global-source self-boost | A structure's own raw output is exempt from the global ratio it contributes | Let it amplify its own faucet | Avoids a self-referential loop and keeps the Star Basin's stardust legible; the boost still applies from every other global source |
| Where tinkerer seats come from | A per-copy field of the structure definition, summed over the meadow's structures | The bind's rule counting the one structure that seats tinkerers | The definition is what the inspector states, so a field is a seat the player reads on the structure's own row; a rule naming a structure is stated nowhere a player looks, and a second seat-giver would mean changing the rule |
| When the Tinkerer commands exist | While the Tinkerer job is unlocked | Always, the command refusing until a Tinkerer stands free | An always-open gate lists a tinkerer count row under every recipe from the meadow's first craft, long before any kitten can take the job, every one of them an act no press can take |
| Where tidying comes from | Two tidying structures, one at the scale of band 1's industry and one at the Dynamo Wheel's, beside the Tidy-Up Rota | (a) Refinements alone; (b) a tidying job; (c) a single tidying structure | A refinement is bought once and cannot grow with the industry it answers. A job spends kittens and tidies at the mood's mercy, turning mess into a tax on the clowder. A single structure priced for band 1 cannot keep up with Dynamo Wheels within a sensible copy count; priced for band 3, it leaves band 1's mess with no answer |
| Tidying structures' gates | None: each reveals when the meadow holds 30% of its price | An idea gate on each | Mess is a slowdown the meadow makes for itself, and its remedy should open as soon as the meadow holds what builds it, not after a research detour. The price materials already pace it, since brass and moonsteel each arrive with an idea |
| Mess below zero | Floored at zero; tidying banks nothing | Let mess run negative as credit | Negative mess means nothing on screen, and a banked credit would let one burst of building prepay a future industry, removing the trade-off between making and tidying |
| Accrual and tidying | Kept as two sums, the mess made and the mess tidied away | One net change per tick | A rule that makes mess accrue faster would, applied to a net figure, also make a tidy meadow tidy faster: the opposite of what it is for |
| Treat Bakery pep draw vs pounce budget | Contention by design (the inspiration's mint pattern) | Pep-reserve floor; auto-toggling | Choosing between passive luxury income and active expedition bursts is a real decision; structure on/off toggling is an integration-wave obligation that will hand the player the lever |

## Open Questions & Future Decisions

### Deferred
1. Fluff sources beyond Groomers (seasonal shedding) — with the crafting segment's tier 2.
2. Whether Forage should batch-scale late (a refinement) — playtest first.
3. Cogworks yearly auto-craft jam semantics (the inspiration jams until New Year) — decide in wave 3 with Tinkerers.

## References

- Parent: `../sim-design.md` (tiers, stack, gating philosophy, cozy inversions)
- Consumes: engine effect/DR/throttle machinery; kittens worker counts & mood; curiosity unlock flags
- Idea names referenced here are defined in `../curiosity/curiosity-design.md`
- **Cross-wave obligations:** the colonies leaf owes moonstone trade rates that carry band 3's pacing (the Kiln's native trickle is deliberately slow); the kittens leaf owes the luxury-happiness term and the crowding-relief LDR term in its happiness formula; the cosmic leaf owes a hushfelt cap extender; mint is trade-only, the Rabbits selling it (colonies leaf).
