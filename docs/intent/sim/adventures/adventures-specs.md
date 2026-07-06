# Adventures Specs (SIM-ADV)

- [ ] **SIM-ADV-001**: When DeclareAdventure(id) is applied after the Adventures surface is unlocked (the Adventurous Spirit prestige tradition, or the same-named curiosity idea — either opens the surface), the system shall arm that Adventure for the next Move, replacing any previously armed one; DeclareAdventure(None) disarms; before the surface is unlocked it rejects with Locked.
- [ ] **SIM-ADV-002**: On a Move, the system shall make the armed Adventure active for the new meadow (applying its twist) and clear the armed slot; a meadow with no armed Adventure Moves normally.
- [ ] **SIM-ADV-003**: The system shall define exactly the adventures of the adventures design's catalogue — twist, goal, and reward — with each twist being a slowdown or removal of options only, never a deletion of kittens, structures, or resources.
- [ ] **SIM-ADV-004**: While an Adventure is active and its goal is met, the system shall complete it — incrementing its completion count and growing its reward pool by the reward amount under a per-reward LimitedDR cap — and lift the twist at the following Move; a never-completed Adventure carries no penalty.
- [ ] **SIM-ADV-005**: The system shall apply Adventure rewards through the typed effect-key system, permanently and cumulatively (LimitedDR-capped per reward), surviving every subsequent Move.
