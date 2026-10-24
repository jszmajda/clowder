---
parent: sim
prefix: SIM-SKY
---

# Cosmic — Expeditions Beyond the Meadow

## Context and Design Philosophy

The cosmic leaf is clowder's late-game frontier: once the clowder has mastered its meadow, the boldest kittens begin gazing past the fence — to the Fence Top, the Old Oak, the rooftops, the hills, and eventually the stars. This is the inspiration's Space arc, reimagined as a series of cozy expeditions to ever-more-distant vantage points, each a place kittens set up a little outpost and send back wonders. There is no void of space here, only a widening backyard that reaches, dreamily, all the way to the furthest star.

The leaf owns the expedition-site ladder, the missions that reach each site, the site structures (energy-hungry, contributing an *outdoor* production channel), the skymap economy that fuels reaching, and the hushfelt supply the dreamtime and prestige leaves need. It reads kites (crafted), curiosity, moonmilk, and cometsilk as mission fuels and contributes site effects through the effect-key system.

## The Site Ladder

Reaching a site is a one-time `LaunchExpedition(site)` mission, priced in escalating fuels (curiosity + skymaps + moonmilk + cometsilk, ~×10 per rung, mirroring the inspiration's mission cost curve). Each reached site unlocks its structures. Sites, in order (a site is reachable only once the previous is reached):

One structure per site (the site's outpost), appended to the economy StructureId space (ids 40–52), gated on its site being reached:

| # | Site | Mission fuel (headline) | Outpost structure (id) — effect |
|---|---|---|---|
| 0 | **Fence Top** | 5000 curiosity, 50 skymaps | Lookout Post (40) — +0.02 skymap/t; +500 skymap cap |
| 1 | **Old Oak** | 25000 curiosity, 200 skymaps | Treehouse (41) — houses 5; +outdoor catnip/t |
| 2 | **Rooftops** | 50 moonmilk, 500 skymaps, 2 balloons | Weather Vane (42) — +0.05 skymap/t; +2000 skymap cap; forecast; energy −1 |
| 3 | **Sunny Hill** | 150 moonmilk, 2000 skymaps, 5 balloons | Solar Array (43) — energy +12 (seasonal) |
| 4 | **Meadow's Edge** | 400 moonmilk, 25 cometsilk | Ranger Station (44) — outdoor production +8%/bld; energy −2 |
| 5 | **The Brook** | 1000 moonmilk, 75 cometsilk | Hushfelt Grotto (45) — +hushfelt/t; +hushfelt cap; energy +4 |
| 6 | **Whispering Woods** | 60 cometsilk, 5 echoes | Whisper Post (46) — +hushfelt/t; +echo/t trickle; energy −2 |
| 7 | **The Foothills** | 200 cometsilk, 25 echoes | Cairn Circle (47) — outdoor production +12%/bld |
| 8 | **Cloud Nine** | 500 cometsilk, 100 echoes, 20 balloons | Sky Dock (48) — +0.2 skymap/t; +20000 skymap cap; enables the Star Whale summon; energy −5 |
| 9 | **The Aurora** | 5 Jars of Purrs, 250 echoes, 5 dreamweave | Aurora Lantern (49) — outdoor +25%/bld; +2 Long Sun cap each; +50 Jar cap; energy −10 |
| 10 | **The Milky Way** | 25 Jars, 1000 echoes, 15 dreamweave | Star Nursery (50) — +unicorn/t + prism/t trickle; +100 Jar cap; energy −10 |
| 11 | **The Far Shore** | 100 Jars, 4000 echoes, 40 dreamweave | Far Beacon (51) — outdoor +50%/bld; storage +10%/bld; +500 Jar cap; energy −20 |
| 12 | **The Furthest Star** | 500 Jars, 16000 echoes, 100 dreamweave | Wishing Star (52) — the capstone: outdoor +200%; a large permanent global production bonus; energy −40 |

Site structures are `Build`-able like any structure (they'd be appended to the economy StructureId space, gated by their site) but contribute to a distinct **outdoor** production channel: outdoor materials take the global/wisdom/Long-Sun layers at a *reduced transfer* (the inspiration's space-production dilution — space gets only a fraction of the paragon/SR bonus). Outdoor structures are energy-hungry; the late meadow must over-provision its grid to run them.

## The Skymap Economy

Skymaps (transient, from Stargazing Ledges + the sky-event system, then Lookout Posts + Weather Vanes + Sky Docks) are the reaching fuel. Skymap *storage* is the reach gate: the Stargazing Ledge's +10 each bootstraps the first sites, then the outpost skymap caps (Lookout Post +500, Weather Vane +2000, Sky Dock +20000) carry the ladder outward. Reaching the outer sites is gated on skymap *storage* as much as production — the storage-cap-gating pattern, one last time, at cosmic scale.

## The Star Whale Summon

Sky Dock (site 8) enables the **Skyward Song** ritual (`SummonStarWhale`, curiosity + echo priced, needs the Skyward Song idea): it calls a Star Whale to visit (the colonies leaf reserved the slot). While visiting, the whale trades cosmic goods (skymaps, starshards, echoes) for hushfelt, its contentment (and visit length) raised by feeding it **lullabies** (a small dreamcorn offering, `FeedWhale`). A whale fed nothing drifts onward, content — the colonies leaf's cozy visitor, now given its mechanics.

## Cozy Notes

- The inspiration's space arc is cold and dangerous (vacuum, radiation, the void). Clowder's is warm and near: a backyard that keeps reaching a little further, kittens setting up lookouts and sending home wonders. The "furthest star" is a wish, not a void.
- No expedition ever loses a kitten or a resource beyond its mission cost; there are no failed launches (a mission you can't afford simply isn't available).

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| What lifts and carries the missions | Balloons fuel the missions that climb (Rooftops, Sunny Hill, Cloud Nine); Dreamweave the missions to the farthest sites (The Aurora onward) | The economy's balloons and dreamweave with no use | Each is crafted and has no other place to go; a mission is where getting up high and travelling far are paid for, and each craft is reachable before the first site that asks for it (balloons in band 3 before Rooftops' moonmilk, dreamweave after The Brook's hushfelt and before The Aurora) |
| Sites as a linear reach-ladder | 13 sequential sites, mission-gated | Branching space map | Matches the inspiration's planet ladder; a linear reach keeps the late game legible and the milestones clear |
| Outdoor production channel | Diluted global/wisdom/Long-Sun transfer | Full transfer; no transfer | The inspiration's proven "space gets a fraction" — keeps outdoor structures meaningful without letting the meadow's multipliers trivialize them |
| Hushfelt from The Brook onward | Cosmic-tier resource with cosmic cap extenders | Early hushfelt | Hushfelt gates the Memory Chest and Dreamweave — placing it here makes those late-game systems genuinely late |
| Star Whale mechanics here | Summon + lullaby feeding at Sky Dock | Fully in colonies | The whale is a cosmic visitor; colonies reserved the slot, cosmic gives it a home |

## Open Questions & Future Decisions

### Deferred
1. A per-site flavor-event pool (the lookout sees a shooting star) — a shell-era delight.
2. Whether the Furthest Star unlocks a capstone Adventure or Tradition — decided after play.

## References

- Parent: `../sim-design.md` (cosmic tier, outdoor production channel, cozy inversions)
- Skymaps/kites/moonmilk/cometsilk: `../economy/`; Jars of Purrs/echoes/starshards: `../mystic/`; Star Whale slot: `../colonies/`; lullabies (dreamcorns): `../mystic/`; hushfelt feeds `../prestige/` (Memory Chest) and `../economy/` (Dreamweave)
