---
parent: sim
prefix: SIM-MOVE
---

# Prestige — The Great Meadow Move

## Context and Design Philosophy

Prestige in clowder is **The Great Meadow Move**: when a clowder has grown as large as its meadow can hold, it relocates to a bigger, brighter meadow — and everything it learned comes along. This is the inspiration's reset/paragon/karma loop, redesigned so that a Move is a *celebration and a graduation*, never an apocalypse. Nothing the player loves is destroyed: the current kittens become honored **Elders of the old meadow**, archived by generation in the **Family Album** (kept forever, never deleted); chosen kittens ride along in **Traveling Baskets**; a fraction of the pantry travels in the **Memory Chest**; and three permanent legacies — **Elder Wisdom**, **Warm Memories**, and **Passed-Down Wisdom** — brighten every future meadow.

This is the wave the player-owner most looks forward to: the prestige unlocks are the delight centerpiece, so the **Traditions** shop is deep, generous, and full of visible "someday" milestones.

The leaf owns the Move itself, the three legacy currencies and their formulas, the Traditions shop (including the Nine Lives ladder), the Family Album, Traveling Baskets, and the Memory Chest carryover. It contributes Elder Wisdom (production stack layer 7) and Tradition effects through the effect-key system, and it persists its legacy state across every Move.

## The Move

`GreatMeadowMove` is available once the clowder has at least 71 Home-or-Away kittens (the inspiration's paragon threshold — a Move needs a real clowder). On a Move:

1. **Earn the legacies** (below) from the clowder's size and the years elapsed.
2. **Archive the generation.** Every current kitten is copied into the Family Album as a slim memorial — its name, trait, rank, and the current (pre-increment) generation number — archived, never deleted. The kittens themselves become "Elders of the old meadow" in the Album's telling. (Skills are not memorialized; they travel only in the Baskets.)
3. **Pack the Traveling Baskets.** Up to `basket_capacity` kittens (a Tradition-raised count, default 0 → raised by the Traveling Baskets tradition) are carried into the new meadow, keeping their names, traits, ranks, and skills.
4. **Fill the Memory Chest.** `memory_chest_ratio` (Tradition-raised, default 0) of each resource stock travels along.
5. **Move in.** The meadow resets to a fresh `GameState` — zero structures, the carried kittens (Traveling Baskets) placed Home, Memory-Chest resources, generation + 1 — *except* the persistent legacy state (Elder Wisdom, Warm Memories, Passed-Down Wisdom, serenity + enlightenment tier from ceremonies, purchased Traditions, and the Family Album), which carry over untouched.

A Move never reduces any legacy currency and never empties the Family Album. It is the one reset in the game, and it takes nothing the player is attached to.

## The Three Legacies

| Legacy | Earned on Move | Passive effect |
|---|---|---|
| **Elder Wisdom** | `+= max(0, kittens − 70)`; plus +1 per game-millennium elapsed (a slow trickle) | +1% production per point, LimitedDR-capped at +200% × nine_lives_ratio (production stack layer 7); +0.1% storage per point (linear) |
| **Warm Memories** | `+= karma_gain(kittens)` (piecewise, steeper for bigger clowders — see below) | `warmth_memory = unlimited_dr(warm_memories, 5)` → +1% happiness per point; also feeds the ceremonies' Everwarm Frescoes |
| **Passed-Down Wisdom** | via **Pass It Down** (a one-way conversion of Elder Wisdom → Passed-Down Wisdom, the inspiration's burned paragon) | a *second* production pool, +1% per point capped at +100% (×4 in the Distant Dawn era); +0.05% storage per point |

`karma_gain(k)` = `(k−35) + 3·max(0,k−60) + 4·max(0,k−100) + 5·max(0,k−150) + 10·max(0,k−300) + 15·max(0,k−750)`, accumulated across every Move. Warm Memories is the sqrt-diminishing happiness legacy; Elder Wisdom is the capped-linear production/storage legacy; Passed-Down Wisdom is the burn-tier for very long games.

## Traditions

The prestige shop (`BuyTradition`), Elder-Wisdom-priced, permanent across Moves — the delight centerpiece. ~30 traditions in families (all contribute effect-key values):

| # | Tradition | Cost (Elder Wisdom) | Effect |
|---|---|---|---|
| 0 | Careful Packing | 5 | all structure prices −1% |
| 1 | Traveling Baskets I | 5 | basket_capacity +1 |
| 2 | Traveling Baskets II | 35 | basket_capacity +2 |
| 3 | Traveling Baskets III | 300 | basket_capacity +4 |
| 4 | Memory Chest I | 10 | memory_chest_ratio +1.5% |
| 5 | Memory Chest II | 100 | memory_chest_ratio +3% |
| 6 | Warm Welcome Home | 5 | new meadows start with +50 catnip and a Cozy Cushion |
| 7 | Elder Stories | 25 | craft ratio +5%; story crafts +25% |
| 8 | Golden Ratio | 50 | structure prices −1.6%; build-queue +1 (shell hint) |
| 9 | Grand Proportion | 100 | structure prices −1.8% |
| 10 | Vitruvian Cat | 250 | structure prices −2% |
| 11 | Renaissance | 750 | structure prices −2.25% |
| 12–21 | **Nine Lives I–X** | 500 / 750 / 1250 / 1750 / 2500 / 5000 / 7500 / 15000 / 30000 / 60000 | each +5% to nine_lives_ratio (raises both the slope AND the +200% cap of Elder Wisdom's production, and its storage) — the near-endless Wisdom sink |
| 22 | Good Neighbours | 5 | +10% trade standing everywhere; unlocks colony traditions |
| 23 | Well-Rested | 25 | kitten arrival speed +75% |
| 24 | Bright-Eyed | 400 | kitten arrival speed +150% |
| 25 | Festival Spirit | 25 | jamborees may stack (additive) |
| 26 | Second Wind | 50 | on a Move, one Traveling Basket kitten keeps its job assignment |
| 27 | Forecaster | 25 | the season forecast is always available (no idea needed) |
| 28 | Prism Sight | 125 | prism/rainbow event chances +10% |
| 29 | Adventurous Spirit | 40 | unlocks the Adventures surface (SIM-ADV) |

## The Family Album

An append-only archive: each Move copies the current roster into the Album under a generation number. The Album is never trimmed, never deleted — it is the game's memory of every kitten who ever lived in the clowder. It is queryable (for the shell to render "the ancestors") and persists across every Move. It has no mechanical effect beyond being *kept* — that keeping is the point (Tenet 1, made literal).

## Cozy Notes

- The Move is the one place the inspiration is at its darkest (kittens die/are abandoned, the world is wiped). Here it is the warmest: a graduation where nothing is lost. The Family Album is the mechanical guarantee — the code literally cannot delete a kitten, only archive it.
- Passed-Down Wisdom (the burn) is a *choice to pass the torch*, not a sacrifice: the wisdom lives on in the next generations as a permanent bonus.
- Traditions are never un-bought or disabled.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Reset = the Move | Archive + carry, never wipe | Hard reset (inspiration) | Tenet 1 at its most load-bearing: the reset is the darkest mechanic in the inspiration; the Family Album makes "nothing is lost" literal and enforceable |
| Three legacies | Elder Wisdom (capped-linear), Warm Memories (sqrt), Passed-Down (burn) | Single prestige currency | Three timescales/curves is the inspiration's proven shape and gives the shop real texture |
| Nine Lives ladder | 10 traditions, each +5% to a ratio that raises Wisdom's slope AND cap | Flat Wisdom cap | The near-endless sink that keeps prestige rewarding after dozens of Moves (the Sephirot analog) |
| Traditions deep & generous | ~30 with visible milestones | A handful | The player-owner's stated delight is prestige unlocks; this shop is where that delight lives |

## Open Questions & Future Decisions

### Deferred
1. The Distant Dawn era (year 40000) raising Passed-Down Wisdom caps — lands with the dreamtime wave.
2. Colony-specific and cosmic Traditions — a later content pass; the shop is append-only.
3. Whether the Family Album should surface "a descendant shares an ancestor's trait" flavor — a shell-era delight, not a sim mechanic.

## References

- Parent: `../sim-design.md` (production stack layer 7, cozy inversions — the Move)
- Kitten roster/traits/ranks: `../kittens/`; serenity + enlightenment persistence: `../ceremonies/`; Warm Memories feeds `../ceremonies/` Everwarm Frescoes; adventures surface: `../adventures/`
