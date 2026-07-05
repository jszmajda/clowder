# Expeditions Specs (SIM-EXP)

- [ ] **SIM-EXP-001**: When Pounce(squads) is applied after the expedition-surface idea (SIM-CUR) is researched, with squads ≥ 1 and 100 × squads pep affordable, the system shall spend the pep and resolve yields in the fixed draw order: feathers, bells, shinies, unicorns; before that idea the command rejects with Locked.
- [ ] **SIM-EXP-002**: The system shall yield feathers as (8 + 6.5 × pounce_ratio) × IrwinHall(squads), bells as (5 + 4 × pounce_ratio) × IrwinHall(Binomial(squads, 0.45 + 0.02 × pounce_ratio) successes), and shinies as (0.5 + 0.5 × pounce_ratio) × IrwinHall(Binomial(squads, 0.25) successes) clamped to cap.
- [ ] **SIM-EXP-003**: Each squad shall independently bring home a unicorn with probability 0.05 (added to the unicorn stock).
- [ ] **SIM-EXP-004**: The system shall compute pounce_ratio as the sum of PounceRatio effects (refinements, the Bold Elder trait), defaulting to 0.
- [ ] **SIM-EXP-005**: A pounce shall emit one PounceReturned event carrying squads and each yield amount.
- [ ] **SIM-EXP-006**: Pounce yields shall land at command time with no timers, cooldowns, or return delays.
