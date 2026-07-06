---
parent: sim
prefix: SIM-UNI
---

# Mystic

## Context and Design Philosophy

The mystic leaf is the unicorn economy and everything glittering that follows from it — the inspiration's unicorn/ziggurat/alicorn/tears chain, reimagined so nothing is ever sacrificed. Unicorns hold **Parades**; rainbows arrive in gratitude; the rarest visitors are **prisms** that occasionally drift into the clowder's shared dreams and become **dreamcorns** (asleep elsewhere, never gone). The leaf owns the mystic currency chain, the Rainbow Arch ladder, the dreaming/dreamcorn system, the Promises to the Night Court, and the Jars of Purrs. It consumes unicorns (economy stock, from Unicorn Meadows and pounces) and contributes multiplier effects through the effect-key system.

## The Currency Chain

| Currency | Gained by | Role |
|---|---|---|
| **unicorns** | Unicorn Meadow (economy, +0.001/t × mystic multipliers), Pounces | held for Parades; Meadow production is multiplied by this leaf's `unicornsRatio` effects |
| **rainbows** | **Parade** (`HoldParade`): 2500 unicorns → rainbows × (Rainbow Arches built) | buy Arch upgrades |
| **prisms** | rare descents (day-boundary event; base chance from each Rainbow Arch *structure* plus Arch *upgrades*) | refined into starshards; or drift into dreams |
| **starshards** | **refine** 25 prisms → 1 starshard × (1 + shard_refine) | dreamtime currency (skips), gate craft for high research |
| **echoes** | refine 25 starshards → 1 echo × (1 + echo-refine effects) | cryptotheology-analog currency (ceremonies/dreamtime) |
| **dreamcorns** | **dreaming**: while dream-drift is positive, prisms fall asleep into dreamcorns | fuel Promises |
| **Jars of Purrs** | condense 10000 rainbows → 1 Jar (capped by mystic content) | the condensed-joy currency; a rare cap raiser |

Every conversion is an *offering that returns more than it takes* or a *transformation* — never a loss. A Parade sends unicorns marching off in glittering procession and rainbows arrive because the neighbourhood loved it; prisms that drift into dreams are dreaming, not destroyed, and a Promise kept keeps them contentedly asleep.

## Rainbow Arches

Arches (economy structure, gated by the Parade Routes idea) are the Parade exchange rate — `rainbows_per_parade = arches_built`. **Researching Parade Routes grants the first Rainbow Arch free** (the SIM-TRADE-008 precedent), so the first Parade already returns rainbows and the arch's base prism-descent chance seeds the prism supply — the loop bootstraps without a chicken-and-egg. Arch upgrades (`BuyArch`, rainbow + prism priced, ~8):

| # | Arch upgrade | Cost | Effect |
|---|---|---|---|
| 0 | Ribbon Arch | 500 rainbows | unicorns +5% (the Rainbow Arch *structure* already grants +0.02%/day base prism descent per copy) |
| 1 | Crystal Arch | 25000 rainbows, 25 prisms | unicorns +10%; +0.05%/day prism descent |
| 2 | Prism Arch | 50000 rainbows, 50 prisms | unicorns +25%; +0.05%/day rainbow shower (a windfall) |
| 3 | Sky Arch | 125000 rainbows, 500 prisms | unicorns +50%; +0.00002 prism/t (a steady drift) |
| 4 | Dreaming Arch | 500 shinies, 1M rainbows, 5000 prisms | unicorns +250%; starshard refine +5% |
| 5 | Starlit Arch | 1250 shinies, 750k rainbows, 25000 prisms | unicorns +500%; starshard refine +10% |
| 6 | Memory Arch | 2500 stardust, 50000 prisms | dream-drift +0.000001/t each (the dreamcorn engine) |
| 7 | Everdream Arch | 100 dreamcorns, 1000 cairns | Jars of Purrs cap +2 each; unlocks Jar condensing |

## Dreaming & Dreamcorns

`dream_drift` accrues per tick from Memory Arches: `+0.000001 × arches × (1 + dream_boost)`, reduced ×0.25 while any dreamcorn is already dreaming (the inspiration's corruption self-throttle, softened) — *this is a slowdown of the drift rate, never a loss*. When `dream_drift ≥ 1`, one dreaming prism becomes a dreamcorn (`prisms −1, dreamcorns +1`), but **never the last prism** (a prism must stay awake to keep the dream going — the cozy floor). Dreamcorns are the softest, sleepiest resource; they are never consumed except by a Promise's upkeep, and even then they are only *kept asleep*, not spent away.

## Promises to the Night Court

`Promises` (the inspiration's pacts, ~post-Enlightenment): each Promise costs echoes and fills a limited slot (slots from Everdream content), consuming 0.0005 dreamcorn/day as gentle upkeep and granting a small global effect (global storage +0.05%, global production +0.05%, warmth +0.1%, craft +0.1%). A Promise with insufficient dreamcorn upkeep **goes quiet** — its effect suspends and its slot is held — and wakes automatically when dreamcorns are available again. Nothing shatters, nothing is destroyed; a neglected Promise simply naps until the clowder can keep it. (Contrast the inspiration's Fracture, which permanently destroys pacts, alicorns, and all skills — Clowder has no such state.)

## Jars of Purrs

Condensing 10000 rainbows into 1 Jar of Purrs is the condensed-contentment currency (the inspiration's Black Liquid Sorrow, inverted from suffering to joy). Jars are capped (Everdream Arches raise the cap); overflow rainbows simply aren't condensed. A Jar is precious and does nothing harmful — its late-game role (opening cap ceilings) lands with the cosmic/dreamtime waves.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Parade exchange rate = arches | rainbows_per_parade scales with Arches built | Fixed rate; rate as an upgrade | The inspiration's ziggurat-count-as-tears-rate pattern; makes Arches doubly valuable (rate + upgrades) |
| Dreaming has a floor | The last prism never sleeps | Allow full conversion | Cozy: something must stay awake to dream; also keeps a prism faucet alive |
| Promises go quiet, never break | Suspend + auto-resume on upkeep | Permanent destruction (Fracture) | Tenet 1 — depth needs upkeep tension, not catastrophic permanent loss |
| Jars from joy, not sorrow | Condense rainbows (contentment) | Keep the sorrow framing | The single darkest currency in the inspiration becomes the single sweetest here |

## Open Questions & Future Decisions

### Deferred
1. Jars of Purrs' late-game sinks (cap-ceiling raisers) — land with cosmic/dreamtime.
2. Whether a Promise's global effect should scale with the Enlightenment tier (the inspiration's Black-Pyramid × conduit) — decide when Enlightenment tiers are play-tested.

## References

- Parent: `../sim-design.md` (mystic tier, cozy inversions)
- Unicorn stock & Meadow: `../economy/`; Parade Routes idea & Rainbow Arch structure: `../curiosity/` + `../economy/`; echoes also feed `../ceremonies/` and `../dreamtime/`
