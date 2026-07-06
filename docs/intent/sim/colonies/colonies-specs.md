# Colonies Specs (SIM-TRADE)

## Trade

- [ ] **SIM-TRADE-001**: When Trade(colony, count) is applied for a discovered *tradeable* colony (colonies 0–6) with count ≥ 1 and 50 pep + 15 shinies + the colony's buy-resource affordable per caravan, the system shall spend those inputs per caravan and resolve each caravan's outcome; an undiscovered colony rejects with Locked and an unaffordable trade with InsufficientResources. Trade with the Star Whale (colony 7) follows the cosmic leaf's visit-gated hushfelt exchange (SIM-SKY-007), not this caravan model.
- [x] **SIM-TRADE-002**: For each caravan, the system shall roll success against standing (nap chance proportional to −standing while shy; bonus-trade chance proportional to standing while warm), yielding on success the colony's sell basket × (1 + trade-ratio effects) × (1 + in-season bonus), or on a nap only the caravan's cost with a possible courtesy gift; a bonus outcome yields ×1.25.
- [x] **SIM-TRADE-003**: The system shall define exactly the seven tradeable colonies of the colonies design's table — start standings, buy resources, and sell baskets with amounts, chances, and best-season bonuses; the Star Whale (colony 7) reserves a visitor slot only, its basket deferred [D] to the cosmic wave.
- [x] **SIM-TRADE-004**: The system shall scale Foxes' moonstone by Kites: chance 0.15 + 0.0035 × Kites, amount (1.5 + 0.03 × Kites) × (1 + fox standing bonus).

## Standing & Pen Pals

- [x] **SIM-TRADE-005**: The system shall compute a colony's standing as its start value plus its Pen Pal contribution plus any dedicated per-colony standing effects (the Charming Elder and Gift Stands raise trade ratio, a separate knob); a shy colony (negative standing) shall only ever warm toward neutral and never become an enemy.
- [x] **SIM-TRADE-006**: When SendPenPal(colony) is applied with charm base × 1.15^level affordable, the system shall spend the charm and raise the colony's Pen Pal level, adding +1% non-guaranteed success chance per level (soft-capped at +75%) and raising its minimum-level goods; Star Whales accept no Pen Pals.

## Discovery

- [x] **SIM-TRADE-007**: When ScoutPicnic is applied after the trade surface (Gift Economy idea) opens and with 1000 pep, the system shall discover the next colony by deterministic prerequisite priority (Foxes need charm ≥ 1500; Deer need ≥ 1 Kite; Owls need ≥ 100 Kites and curiosity cap > 125000; else a random first-tier colony), refunding 950 pep on a quiet day that finds nobody.
- [x] **SIM-TRADE-008**: The system shall discover the first colony free when the trade surface opens, and never undiscover a colony.
