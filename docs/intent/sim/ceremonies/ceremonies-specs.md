# Ceremonies Specs (SIM-SUN)

## Currencies & Offerings

- [ ] **SIM-SUN-001**: When Bask is applied, the system shall add warmth × (1 + reverence) to devotion and set warmth to zero, where reverence = unlimited_dr(serenity, 0.1) × 0.1; with zero warmth it rejects with InsufficientResources.
- [ ] **SIM-SUN-002**: When Reverie is applied after the Reverie rite is bought, the system shall add devotion / 1e6 × (enlightenment_tier + 1)² × 1.01 to serenity and reset devotion to a spark (0.01); before that rite, or with negligible devotion, it rejects with Locked or InsufficientResources respectively.
- [ ] **SIM-SUN-003**: The system shall persist serenity and the enlightenment tier across the Great Meadow Move (SIM-MOVE), and neither shall ever decrease.

## The Long Sun

- [ ] **SIM-SUN-004**: While the Long Sun rite is bought, the system shall contribute a production multiplier of limited_dr(unlimited_dr(devotion, 1000) / 100, cap) at production-stack layer 8 to material flows only, where cap = 10 + 0.05 × enlightenment_tier per Sunstone; before the rite the contribution is 1.0 (no bonus).

## Rites

- [ ] **SIM-SUN-005**: The system shall define exactly the eleven Sunbeam Rites of the ceremonies design's ladder — warmth+shinies costs, devotion gates, and effects — each unlockable once devotion has ever reached its gate and buyable via BuyRite (Locked before the gate, InsufficientResources when unaffordable).
- [ ] **SIM-SUN-006**: While the Enlightenment rite is bought, the system shall make rites 0–7 re-buyable with a ×2.5 price ramp per additional copy, their effects stacking; rites 5, 8, 9, and 10 shall remain single-purchase.
- [ ] **SIM-SUN-007**: The system shall apply every rite's effect through the typed effect-key system, in the engine's fixed source order; no rite effect shall ever be removed or disabled once bought.

## Enlightenment

- [ ] **SIM-SUN-008**: When Enlighten is applied with enough serenity for the next tier (total_cost(t) = inverse_unlimited_dr(e^t / 10, 0.1)), the system shall spend the serenity cost and increment the enlightenment tier, raising the Long Sun cap and the Reverie conversion square; otherwise it rejects with InsufficientResources.
