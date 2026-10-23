# Colonies Specs (SIM-TRADE)

## Trade

- [x] **SIM-TRADE-001**: When Trade(colony, count) is applied for a discovered *tradeable* colony (colonies 0–6) with count ≥ 1 and 50 pep + 15 shinies + the colony's buy-resource affordable per caravan, the system shall spend those inputs per caravan and resolve each caravan's outcome; an undiscovered colony rejects with Locked and an unaffordable trade with InsufficientResources. Trade with the Star Whale (colony 7) follows the cosmic leaf's visit-gated hushfelt exchange (SIM-SKY-007), not this caravan model.
- [x] **SIM-TRADE-002**: For each caravan, the system shall roll success against standing (nap chance proportional to −standing while shy; bonus-trade chance proportional to standing while warm), yielding on success the colony's sell basket × (1 + trade-ratio effects) × (1 + in-season bonus), or on a nap only the caravan's cost with a possible courtesy gift; a bonus outcome yields ×1.25.
- [x] **SIM-TRADE-003**: The system shall define exactly the seven tradeable colonies of the colonies design's table — start standings, buy resources, and sell baskets with amounts, chances, and best-season bonuses; the Star Whale (colony 7) reserves a visitor slot only, its exchange being a visit rather than a caravan route and specified with the rest of the sky in SIM-SKY-006 and SIM-SKY-007.
- [x] **SIM-TRADE-004**: The system shall scale Foxes' moonstone by Kites: chance 0.15 + 0.0035 × Kites, amount (1.5 + 0.03 × Kites) × (1 + fox standing bonus).
- [x] **SIM-TRADE-009**: When a Trade is applied and accepted, whether to a colony (SIM-TRADE-001) or as the whale's exchange (SIM-SKY-007), the system shall emit one TradeReturned event carrying the colony, the caravans sent, the number of them that napped, and each resource whose stock the trade raised with the amount that stock rose by, in the core's resource order, a resource the trade did not raise being left out.

## What a Neighbour Has Shown You

- [ ] **SIM-TRADE-010**: When a caravan of an accepted Trade to a tradeable colony does not nap (SIM-TRADE-002), the system shall record that colony as reached, whatever the caravan's draws bring, and record as learned for it every good the caravan's draws brought home at an amount above zero, whether or not its store could hold it — Foxes' moonstone only on a caravan whose own roll (SIM-TRADE-004) brought some — keeping both for the rest of the meadow; a napping caravan and the Star Whale's exchange shall record nothing.
- [ ] **SIM-TRADE-011**: When a Great Meadow Move resets the meadow, the system shall forget every colony's reach and learned goods, as it forgets every colony's discovery.
- [ ] **SIM-TRADE-012**: The system shall expose, for a colony, a read-only offers query reporting whether a caravan has reached it; and, taking every good it offers, those its Pen Pal level does not yet reach included, in a fixed order — its sell basket in the order the colony's own table lists it, then, for Foxes, moonstone — its learned goods in that order, and its unseen goods, those it has not been recorded as learning, in that order, each with whether the colony has it to trade at its current Pen Pal level, Foxes' moonstone wanting no level; and whether it tells of its unseen goods by name, which it does exactly while its standing (SIM-TRADE-005) is above zero.

## Standing & Pen Pals

- [x] **SIM-TRADE-005**: The system shall compute a colony's standing as its start value plus its Pen Pal contribution plus any dedicated per-colony standing effects (the Charming Elder and Gift Stands raise trade ratio, a separate knob); a shy colony (negative standing) shall only ever warm toward neutral and never become an enemy.
- [x] **SIM-TRADE-006**: When SendPenPal(colony) is applied with charm base × 1.15^level affordable, the system shall spend the charm and raise the colony's Pen Pal level, adding +1% non-guaranteed success chance per level (soft-capped at +75%) and raising its minimum-level goods; Star Whales accept no Pen Pals.

## Discovery

- [x] **SIM-TRADE-007**: When ScoutPicnic is applied after the trade surface (Gift Economy idea) opens and with 1000 pep, the system shall discover the next colony by deterministic prerequisite priority (Foxes need charm ≥ 1500; Deer need ≥ 1 Kite; Owls need ≥ 100 Kites and curiosity cap > 125000; else a random first-tier colony), finding nobody half the time and refunding 950 pep on such a quiet day — the chance is named here because every other number in this spec is.
- [x] **SIM-TRADE-008**: The system shall discover the first colony free when the trade surface opens, and never undiscover a colony.
