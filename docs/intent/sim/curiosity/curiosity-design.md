---
parent: sim
prefix: SIM-CUR
---

# Curiosity

## Context and Design Philosophy

Curiosity is the game's research system: kittens investigate the world and their accumulated curiosity is spent on **Ideas** — one-time unlocks that open structures, jobs, and permanent modifiers. The leaf owns the idea tree (definitions, costs, prerequisites, effects-as-flags). The curiosity *resource* itself (stock, cap, generation) belongs to the economy leaf; this leaf is the spending side.

Design intent: the tree is a ladder, not a maze. v1 has a single mostly-linear track with a few branches, each idea readable as a small joke that also states its effect (Tenet 3: whimsy with clarity — the flavor name goes on the label, the mechanical effect in the description the shell always shows).

## The Idea Tree

Costs in curiosity, plus a **gate craft** from band 1 onward — stories, then anthologies, then schemes (economy leaf recipes priced in *stocked* curiosity, making curiosity storage the true research gate). Prerequisite `—` means available once the curiosity cap is above zero (the first Cardboard Box — the economy leaf's physical gate); before that, all ideas are `Locked`. An unlocked idea the player can't yet afford is `InsufficientResources`, per the engine's error precedence.

The tree is a banded ladder with short forks — each band's gate craft requires the previous band's infrastructure, and each band's ideas unlock the structures whose outputs pay for the next band. Ids are append-only (0–8 are the founding nine, unchanged).

### Band 0 — curiosity only

| # | Idea | Cost | Prereq | Unlocks / effect |
|---|---|---|---|---|
| 0 | Pantry Organization | 15 | — | Pantry |
| 1 | The Scratching Arts | 25 | — | Scratching Post |
| 2 | Groomed to Purrfection | 40 | Pantry Organization | Groomer job |
| 3 | Twig Architecture | 60 | The Scratching Arts | Cat Tree, Twig Pile |
| 4 | Selective Hearing | 80 | Pantry Organization | gather ×2 |
| 5 | Biscuit Making | 120 | Groomed to Purrfection | +10 happiness pts |
| 6 | Seasonal Awareness | 150 | Twig Architecture | winter ×0.25→×0.40; forecast |
| 7 | Zoomie Channeling | 200 | Biscuit Making | zoomies ×1.5/240t |
| 8 | Neighborhood Watch | 250 | Seasonal Awareness | arrival +15% |
| 9 | Couch Spelunking | 120 | Twig Architecture | Couch Mine; Rummage action; Rummager job |
| 10 | Horticulture | 180 | Couch Spelunking | Herb Garden |
| 11 | Emberkeeping | 300 | Couch Spelunking | Hearth |
| 12 | Notetaking | 350 | Horticulture | Study Nook |
| 13 | Cozy Carpentry | 400 | Emberkeeping | Window Perch |
| 14 | Tinkering | 500 | Notetaking | Craft Corner; Wands, Tiles, Lattices, Cairns crafts |
| 15 | Pack Rattery | 650 | Tinkering | Shed; Badges craft |
| 16 | Unicorn Lore | 750 | Notetaking | Unicorn Meadow |
| 17 | Oral Tradition | 900 | Tinkering | Story Circle; Quills craft; charm begins |
| 56 | Pouncing Practice | 200 | Pantry Organization | expedition surface; Pouncer job (band 0; id appended late — ids never reorder) |

### Band 1 — curiosity + stories

| # | Idea | Cost | Prereq | Unlocks / effect |
|---|---|---|---|---|
| 18 | Gift Economy | 1100 | Oral Tradition | Gift Stand; trade surface (colonies leaf) |
| 19 | Storycraft | 1200 + 0 stories | Oral Tradition | Stories craft |
| 20 | Sun Salutations | 1500 + 5 stories | Storycraft | Sun Room; warmth begins |
| 21 | Excavation | 1800 + 8 stories | Storycraft | Gravel Pit; Brass craft |
| 22 | Cogwheels | 2600 + 12 stories | Excavation | Cogworks; Cogs craft |
| 23 | Treat Alchemy | 3200 + 15 stories | Excavation | Treat Bakery |
| 24 | Tea Ceremonies | 3800 + 20 stories | Sun Salutations | Tea House; jamborees (kittens leaf) |
| 25 | Solar Devotion | 4500 + 25 stories | Sun Salutations | Sunbeam Altar; ceremonies surface |
| 26 | Sky Charts | 5200 + 30 stories | Cogwheels | Stargazing Ledge; sky events; skymaps begin |
| 27 | Porch Logistics | 6000 + 35 stories | Sky Charts | Porch; Kites craft |

### Band 2 — curiosity + anthologies

| # | Idea | Cost | Prereq | Unlocks / effect |
|---|---|---|---|---|
| 28 | Anthology Binding | 7000 + 40 stories | Sky Charts | Anthologies craft |
| 29 | Dew Harvest | 8000 + 10 anthologies | Anthology Binding | Dew Collector |
| 30 | Toymaking | 10000 + 15 anthologies | Anthology Binding | Toy Workshop; Tinkerer job |
| 31 | Ceramic Arts | 12000 + 20 anthologies | Dew Harvest | Kiln |
| 32 | Deep Curiosity | 14000 + 25 anthologies | Toymaking | Whisker Lab |
| 33 | Moonlight Arts | 16000 + 30 anthologies | Toymaking | Moonlight Stage |
| 34 | Labyrinth Theory | 18000 + 35 anthologies | Deep Curiosity | Box Labyrinth |
| 35 | Grand Schemes | 20000 + 40 anthologies | Deep Curiosity | Schemes craft |

### Band 3 — curiosity + schemes

| # | Idea | Cost | Prereq | Unlocks / effect |
|---|---|---|---|---|
| 36 | Dynamos | 25000 + 8 schemes | Grand Schemes | Dynamo Wheel |
| 37 | Solar Basking | 28000 + 10 schemes | Grand Schemes | Sunning Deck |
| 38 | Millworks | 32000 + 12 schemes | Solar Basking | Stream Wheel |
| 39 | Moonmetallurgy | 36000 + 15 schemes | Dynamos | Moonsteel, Pavers crafts |
| 40 | Starcatching | 42000 + 20 schemes | Moonmetallurgy | Star Basin; Cometsilk craft |
| 41 | Prism Craft | 48000 + 25 schemes | Starcatching | Prism Spinner |
| 42 | Balloon Craft | 55000 + 30 schemes | Moonmetallurgy | Balloons craft |
| 43 | Yarn Computing | 65000 + 40 schemes | Starcatching | Yarn Computer |

### Band 4 — curiosity + mystic currencies (content lands in waves 4–6)

| # | Idea | Cost | Prereq | Unlocks / effect |
|---|---|---|---|---|
| 44 | Parade Routes | 30000 + 15 schemes | Unicorn Lore, Grand Schemes | Parades, Rainbow Arch structure, refining (mystic leaf) |
| 45 | Pen Pal Networks | 26000 + 10 schemes | Gift Economy, Grand Schemes | Pen Pals (colonies leaf) |
| 46 | Scout Picnicking | 20000 + 5 schemes | Gift Economy, Grand Schemes | Scout Picnics (colonies leaf) |
| 47 | Dream Tending | 90000 + 3 starshards | Parade Routes | dreaming surface (mystic leaf) |
| 48 | Night Court Etiquette | 150000 + 5 echoes | Dream Tending | Promises (mystic leaf) |
| 49 | Hushfelt Seeking | 80000 + 1 echo | Yarn Computing | hushfelt sources (cosmic leaf) |
| 50 | Memory Weaving | 100000 + 3 starshards | Hushfelt Seeking | Memory Chest |
| 51 | Skyward Longing | 90000 + 2 echoes | Yarn Computing, Porch Logistics | cosmic surface (cosmic leaf); Moonmilk craft |
| 52 | Dreamweaving | 120000 + 5 echoes | Memory Weaving | Dreamweave craft |
| 53 | Adventurous Spirit | 40000 + 20 schemes | Grand Schemes | adventures surface (adventures leaf) |
| 54 | Dream Loomcraft | 200000 + 10 starshards | Memory Weaving | Dream Loom (dreamtime leaf) |
| 55 | Deep Nap Theory | 300000 + 10 echoes | Dream Loomcraft | the Deep Nap (dreamtime leaf) |
| 57 | Long Sun Masonry | 120000 + 3 echoes | Solar Devotion, Skyward Longing | Sunstone structure (ceremonies leaf) |
| 58 | Skyward Song | 180000 + 8 echoes | Skyward Longing, Night Court Etiquette | Star Whale visits (colonies/cosmic leaf) |

The DAG must stay acyclic and every idea reachable (property-tested). Refinements (tool/storage/automation ladders) and House Rules (mutually exclusive policies) are this leaf's remaining depth content — designed in the society wave.

Research is a command (`ResearchIdea`): full-cost spend, all-or-nothing, rejected with `Locked` when prerequisites are missing and `AlreadyResearched` when owned. Completed ideas emit `IdeaResearched` and are permanent — ideas are never lost or disabled (cozy invariant: no confiscation).


## Refinements

Purchasable one-time improvements (`BuyRefinement`), priced in curiosity + materials, gated by ideas, each contributing effect-key values. Ladders stack additively on one key — constant small wins with escalating materials. The table is normative (35 refinements); costs are (curiosity, materials):

| # | Refinement | Cost | Gate | Effect |
|---|---|---|---|---|
| 0 | Pebble Combs | 100c, 275 pebbles | CouchSpelunking | Gardener +50% |
| 1 | Button Combs | 200c, 25 buttons | Emberkeeping | Gardener +30% |
| 2 | Brass Combs | 5000c, 75 brass | Cogwheels | Gardener +50% |
| 3 | Moonsteel Combs | 38000c, 10 moonsteel | Moonmetallurgy | Gardener +50% |
| 4 | Pebble Snips | 100c, 500 pebbles | CouchSpelunking | TwigCollector +70% |
| 5 | Button Shears | 200c, 50 buttons | Emberkeeping | TwigCollector +50% |
| 6 | Brass Clippers | 20000c, 75 brass | Cogwheels | TwigCollector +50% |
| 7 | Moonsteel Pruners | 38000c, 10 moonsteel | Moonmetallurgy | TwigCollector +50% |
| 8 | Deep Pockets | 1000c, 50 buttons | PackRattery | Rummager +50% |
| 9 | Sorting Claws | 15000c, 25 brass | Excavation | Rummager +50% |
| 10 | Bolas of Yarn | 1000c, 50 twigs + 250 pebbles | PouncingPractice | pounce ratio +1 |
| 11 | Pounce Pads | 2000c, 750 buttons | Emberkeeping | pounce ratio +2 |
| 12 | Whisker Guards | 10000c, 50 brass | Cogwheels | pounce ratio +0.5 |
| 13 | Moonsteel Mittens | 50000c, 25 moonsteel | Moonmetallurgy | pounce ratio +0.5 |
| 14 | Deep Shelves | 500c, 1000 twigs + 750 pebbles + 50 buttons | PackRattery | +100 twig cap, +150 pebble cap |
| 15 | Stacked Crates | 800c, 25 wands + 10 tiles | PackRattery | +100 twig cap, +150 pebble cap, +25 button cap |
| 16 | Labeled Jars | 60000c, 25 moonstone + 200 brass | Moonmetallurgy | +50 button cap, +30 ember cap, +10 moonstone cap |
| 17 | Catnip Cellars | 50000c, 125 brass + 5 schemes | Dynamos | Pantries also hold +125 catnip each |
| 18 | Bigger Baskets | 15000c, 50 badges + 25 brass | Cogwheels | +200 pebble cap, +50 button cap |
| 19 | Bellows | 5000c, 35 wands + 2000 pebbles | Emberkeeping | Hearth embers +0.002/t |
| 20 | Ember Beds | 35000c, 5 anthologies | CeramicArts | Hearth embers +0.003/t |
| 21 | Story Press | 7500c, 45 cogs | Cogwheels | Cogworks +0.005 stories/t each |
| 22 | Better Bindings | 25000c, 5 anthologies | AnthologyBinding | Stories/Anthologies/Schemes craft +25% |
| 23 | Field Notes | 250c, 100 pebbles | Notetaking | Observer +30% |
| 24 | Star Diaries | 20000c, 5 kites | SkyCharts | sky-event chance ×2 |
| 25 | Patient Eyes | 25000c, 10 kites | SkyCharts | auto-observe chance ×2 |
| 26 | Tidy-Up Rota | 10000c, 15 stories | Toymaking | Toy Workshop mess 2 → −2 |
| 27 | Group Lessons | 45000c, 10 anthologies | DeepCuriosity | skill decay removed |
| 28 | Encouraging Mews | 20000c, 5 anthologies | Toymaking | skill XP +50% |
| 29 | Nimble Paws | 30000c, 10 cogs + 5 anthologies | Toymaking | Tinkerer speed +100% |
| 30 | Patient Paws | 75000c, 10 schemes | Dynamos | Tinkerer speed +100% |
| 31 | Clockwork Paws | 150000c, 25 schemes | YarnComputing | Tinkerer speed +200% |
| 32 | Warm Welcome Mat | 4000c, 20 stories | TeaCeremonies | arrival chance +5% (inside the cap) |
| 33 | Thicker Blankets | 8000c, 50 quills | TeaCeremonies | +1 happiness point per Hearth |
| 34 | Dew Funnels | 30000c, 40 lattices | DewHarvest | Dew Collector +50% |



## House Rules

Mutually exclusive pairs, charm-priced (`AdoptRule`), each permanently blocking its sibling for the current meadow (a Great Meadow Move clears adopted rules — a new meadow, new habits):

| Pair | Cost | A | B |
|---|---|---|---|
| 0/1 | 150 charm | **Early Birds**: catnip production +10% | **Night Owls**: curiosity production +10% |
| 2/3 | 1500 charm | **Open Meadow**: arrival chance +10% (inside cap) | **Cozy Burrows**: +5 happiness points |
| 4/5 | 4000 charm | **Sharing Circle**: luxury drain −25% | **Personal Stashes**: all caps +10% |
| 6/7 | 15000 charm | **Loud Purring**: charm production +20% | **Quiet Contemplation**: warmth production +20% |

Adopting a rule whose sibling is adopted rejects with Locked. Rules are effect-key sources like everything else. Personal Stashes composes as cap = (base + flat extensions) × (1 + cap-ratio effects): infinite caps stay infinite, and a zero cap stays zero — the Cardboard-Box gate does not open by percentage.

Refinement material costs may tease one band ahead of their gate idea (Pebble Combs wants more pebbles than the gate band can store) — reachability is property-tested at gate band + 1.

## Effect Plumbing

Effects are **flags and modifier values exposed by pure lookup** (`ideas.has(IdeaId)` and derived modifier accessors), consumed inside the owning leaf's math. This leaf never mutates other leaves' state; researching an idea changes *lookups*, not stocks or rosters. In v1 every idea touches its own distinct knob, so no composition question arises; if future ideas ever share a knob, the owning leaf's formula must state the composition explicitly at that knob (rates compose multiplicatively, percentage-point bonuses sum) rather than relying on a global rule.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Tree shape | Banded ladder with short forks — 57 ideas over five bands, each band gated by the previous band's crafted currency | Wide branching tree; flat shop | Bands make every idea a visible milestone while gate crafts turn research into an infrastructure problem, not a wait |
| Effects as flags/lookups | Pure `has(idea)` + derived modifiers | Event-driven effect application, direct state mutation on research | Idempotent under save/load and catch-up; no ordering bugs; each owning leaf keeps its own math legible |
| Ideas are permanent | No respec, no disable | Respec mechanic | Nothing the player earns is ever taken (Tenet 1); respec adds decision anxiety to a cozy game |
| Cost curve | 15 → 300,000 across bands, roughly ×1.4 steps within a band, gate crafts escalating between bands | Flat costs; steeper exponential | Within-band steps stay session-scale; between-band jumps are infrastructure projects |

## Open Questions & Future Decisions

### Deferred
1. Tier-2 ideas that interact with roadmap systems (ceremonies, expeditions) — land with those segments.
2. Whether Seasonal Awareness's forecast belongs in core state or as a pure derived flag the shell reads — trivially the latter in v1; revisit if forecasts gain mechanics.

## References

- Parent: `../sim-design.md` (curiosity → economy/kittens contract: flags, never reach-ins)
- Curiosity resource generation/cap: `../economy/economy-design.md`
