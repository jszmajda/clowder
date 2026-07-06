---
parent: sim
prefix: SIM-TRADE
---

# Colonies

## Context and Design Philosophy

Colonies are clowder's trade and diplomacy: eight neighbouring animal colonies the clowder sends caravans to, exchanging surplus for goods it can't make yet. This is the inspiration's race/trade/embassy system with the hostility removed — no colony is an enemy, a failed trade means the caravan found them napping (goods spent, sometimes a courtesy gift returned), and standing only ever warms. The leaf owns the colony table, standing/shyness, Pen Pals (embassies), Scout Picnics (exploration), seasonal trade rates, the kite-scaled goods, and the Star Whale visitor arc. It consumes pep (caravan cost) and trade goods, and contributes trade/standing effects through the effect-key system.

## Trade

`Trade(colony, count)` sends `count` caravans, each costing 50 pep + 15 shinies + the colony's buy-resource. Each caravan returns the colony's sell-basket:

```
sold = base × (1 + trade_ratio effects) × (1 + season_bonus if in season) ± width fuzz
```

A caravan **succeeds** with probability based on standing (below); on a *nap* (failure) the pep/shinies/buy-resource are still spent (the caravan travelled) and a small courtesy gift may return. A bonus outcome (positive standing) yields ×1.25.

## The Eight Colonies

Buy → sell [amount @ base-chance, in-season]. Standings warm from their start value via Pen Pals.

| # | Colony | Start standing | Buys | Sells (amount @ chance, best season) |
|---|---|---|---|---|
| 0 | **Otters** | +0.25 (friendly) | pebbles ×1000 | twigs 500 @ 100% (summer +35%); wands 10 @ 25% |
| 1 | **Raccoons** | 0 (neutral) | buttons ×100 | catnip 35000 @ 100% (winter +45%); quills 5 @ 25%, stories 3 @ 15% |
| 2 | **Foxes** | −0.15 (shy) | twigs ×500 | buttons 250 @ 100% (autumn +35%); brass 25 @ 25%, cogs 5 @ 10%; **moonstone** (kite-scaled, below) |
| 3 | **Hedgehogs** | 0 | feathers ×500 | pebbles 1000 @ 100% (spring +25%); tiles 5 @ 75%, pavers 1 @ 10% |
| 4 | **Rabbits** | +0.15 | tiles ×50 | embers 350 @ 100%; dew 100 @ 25% |
| 5 | **Deer** | 0 | shinies ×250 | moonstone 1 @ 95%; stardust 0.1 @ 50% (needs Pen Pal level ≥ 5) |
| 6 | **Owls** | −0.10 (shy) | mint ×500 | curiosity 80 @ 100% (a scholarly colony) |
| 7 | **Star Whales** | visitor (no Pen Pal) | — | see Star Whale arc |

Foxes' moonstone (the inspiration's ship-scaled titanium): chance `0.15 + 0.0035 × Kites`, amount `(1.5 + 0.03 × Kites) × (1 + fox_standing_bonus)` — Kites (crafted) scale the pre-Kiln moonstone supply, making trade the early metal path (the economy leaf's cross-wave obligation).

## Standing & Shyness

`standing = start + Σ standing effects (Pen Pal levels, and any dedicated per-colony standing effects)`. The Charming Elder and Gift Stands raise the *trade ratio* (yield), not standing — distinct knobs. Below zero (shy): the colony can only warm to neutral, and each caravan has a nap chance proportional to −standing. At zero: neutral, base chances. Above zero (warm): a bonus-trade chance proportional to standing. A nap is never punished beyond the caravan's cost — colonies are shy, not hostile.

## Pen Pals

`SendPenPal(colony)` establishes/deepens a correspondence (the inspiration's embassies), charm-priced at `base × 1.15^level`. Each level: +1% success chance on non-guaranteed sells (soft cap +75%), and raises the minimum-level goods a colony will part with. Star Whales take no Pen Pals.

## Scout Picnics

`ScoutPicnic` (the inspiration's "send explorers") spends 1000 pep to discover the next colony; on a quiet day the picnic finds nobody (950 pep refunded — they mostly stayed home). Discovery order is deterministic by prerequisite: Foxes need charm ≥ 1500; Deer need ≥ 1 Kite; Owls need ≥ 100 Kites and curiosity cap > 125000; otherwise a random pick among the first-tier colonies. The first colony arrives free once the trade surface (Gift Economy idea) opens.

## Star Whales

The gentle-giant visitor arc (the inspiration's leviathans, de-fanged): Star Whales drift by when summoned by a Skyward Song (a late ritual), staying 5 years + 1 per contentment. They take no Pen Pal; the clowder feeds them **lullabies** (a small dreamcorn offering) to raise contentment, which lengthens the visit and improves their trades. A whale fed nothing simply drifts onward, content — never angry. They trade rare cosmic goods (skymaps, starshards, echoes) for hushfelt. Full mechanics land with the cosmic wave; this leaf reserves the visitor slot and the contentment currency.

## Decisions & Alternatives

| Decision | Chosen | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| No hostile colonies | Shy (warms to neutral) instead of hostile (capped at neutral, failure chance) | Keep hostile races | Tenet 1 — the *mechanic* (negative standing = failure chance, warmable) is preserved; only "enemy" becomes "shy" |
| Kite-scaled moonstone | Foxes' moonstone scales with Kites built | Flat moonstone trade | The inspiration's ship-scaled-titanium loop; makes Kites doubly useful (storage + metal) and gives band 3 its trade-paced supply |
| Failed trade still costs the caravan | Pep/goods spent on a nap, courtesy gift possible | Full refund on failure | A costless retry removes all standing tension; the courtesy gift keeps it warm |
| Star Whales fed dreamcorns | Lullaby offering raises contentment | Necrocorn-style feeding | Reuses the mystic dreamcorn as a gentle gift rather than the inspiration's grim necrocorn upkeep |

## Open Questions & Future Decisions

### Deferred
1. Star Whale full trade table and the Skyward Song ritual — cosmic wave.
2. Whether a blackcoin-analog speculation mini-market rides the Star Whale visit (the inspiration's leviathan blackcoin) — probably a post-TUI content pass; noted, not committed.

## References

- Parent: `../sim-design.md` (colonies inversion, trade-as-conversion depth pattern)
- Pep/shinies costs & trade goods: `../economy/`; Kites: `../economy/` crafts; Gift Economy / Pen Pal / Scout Picnic ideas: `../curiosity/`; Charming Elder: `../kittens/`; dreamcorns (lullabies): `../mystic/`
