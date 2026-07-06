# Prestige Specs (SIM-MOVE)

## The Move

- [ ] **SIM-MOVE-001**: When GreatMeadowMove is applied with at least 71 rostered kittens, the system shall archive every current kitten into the Family Album under the next generation number (name, traits, rank, skills preserved), never deleting any; with fewer than 71 kittens it rejects with InsufficientResources.
- [ ] **SIM-MOVE-002**: On a Move, the system shall carry up to basket_capacity kittens (Tradition-raised, default 0) into the new meadow as Home kittens keeping their names, traits, ranks, and skills, selecting the most-senior (lowest arrived_day) first.
- [ ] **SIM-MOVE-003**: On a Move, the system shall carry memory_chest_ratio (Tradition-raised, default 0) of each resource stock into the new meadow.
- [ ] **SIM-MOVE-004**: On a Move, the system shall preserve EXACTLY this legacy set and reset everything else to new-game defaults: preserved — Elder Wisdom, Warm Memories, Passed-Down Wisdom, serenity, enlightenment tier, purchased Traditions (and derived nine_lives_ratio), the Family Album, the Adventures state (reward pool, completions, and the armed→active transition), the game-speed setting, the RNG (continues, never reseeded), and generation (incremented); reset to defaults — structures, ideas, refinements, house rules, resources (before the Memory Chest is placed), the kitten roster (before Traveling Baskets are placed), devotion, rites, Rainbow Arches and the whole mystic chain, Promises, colony discovery/Pen Pals, the Elder appointment, mess, and the tick counter.
- [ ] **SIM-MOVE-005**: A Move shall never decrease any legacy currency and never remove an entry from the Family Album.

## The Legacies

- [ ] **SIM-MOVE-006**: On a Move, the system shall add max(0, kittens − 70) to Elder Wisdom, plus one per game-millennium elapsed since the last Move.
- [ ] **SIM-MOVE-007**: The system shall contribute Elder Wisdom to production stack layer 7 as +1% per point, limited_dr-capped at (2.0 × nine_lives_ratio), and to storage as +0.1% per point (linear); the cap and slope scale with the Nine Lives traditions.
- [ ] **SIM-MOVE-008**: On a Move, the system shall add karma_gain(kittens) = (k−35) + 3·max(0,k−60) + 4·max(0,k−100) + 5·max(0,k−150) + 10·max(0,k−300) + 15·max(0,k−750) to Warm Memories (accumulated across Moves), and expose warm_memory = unlimited_dr(Warm Memories, 5) as +1% happiness per point.
- [ ] **SIM-MOVE-009**: When PassItDown is applied, the system shall convert all Elder Wisdom into Passed-Down Wisdom (one-way), which contributes a second production pool of +1% per point capped at +100% and storage of +0.05% per point.

## Traditions

- [ ] **SIM-MOVE-010**: The system shall define exactly the traditions of the prestige design's shop table — Elder-Wisdom costs and effects — buyable once each via BuyTradition (permanent across Moves; InsufficientResources when unaffordable), their effects flowing through the effect-key system.
- [ ] **SIM-MOVE-011**: The Nine Lives traditions shall each add 0.05 to nine_lives_ratio (default 1.0), raising both the slope and the +200% cap of Elder Wisdom's production contribution.
- [ ] **SIM-MOVE-012**: The system shall apply Tradition effects (price reductions, basket capacity, memory-chest ratio, arrival speed, and the rest) through the typed effect-key system, and no Tradition shall ever be un-bought or disabled.
