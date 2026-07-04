# Kittens Specs (SIM-KIT)

## Roster & Identity

- [ ] **SIM-KIT-001**: The system shall never remove a kitten from the roster; a kitten's absence is represented by the location NeighborsYard, never by deletion.
- [ ] **SIM-KIT-002**: When a kitten arrives, the system shall draw its name without replacement from the name pool via the state RNG, appending Roman-numeral suffixes (II, III, …) once the pool is exhausted, so every name in a save is unique.

## Arrival

- [ ] **SIM-KIT-003**: The system shall check for kitten arrival once per day close, only when at least one housing slot is free and catnip stock is at least 50, admitting at most one kitten per day.
- [ ] **SIM-KIT-004**: When an eligible arrival check runs, the system shall admit a kitten with probability 15% + 5 percentage points per full 100 catnip in stock, capped at 60%, plus 15 percentage points on top of that cap where the arrival-boosting idea (SIM-CUR) is researched (effective maximum 75%).
- [ ] **SIM-KIT-005**: If four consecutive eligible arrival checks fail, then the system shall make the next eligible check succeed automatically; ineligible days pause the failure count (neither extending nor resetting it), and any arrival resets it.
- [ ] **SIM-KIT-006**: The system shall count housing so that every rostered kitten holds a slot regardless of location (free slots = capacity − roster size), so a returning kitten never finds the house full and an arrival never displaces an absentee.
- [ ] **SIM-KIT-007**: When a kitten arrives, the system shall add it at Home with no job, emit a KittenArrived event carrying its name, and reset the pity counter.

## Jobs

- [ ] **SIM-KIT-008**: When AssignJob is applied for an unlocked job with at least one idle Home kitten, the system shall assign the most recently arrived idle Home kitten, stamp its job_seq from the monotone counter, and emit an event naming the kitten; with no idle Home kitten it shall reject with NoIdleKittens.
- [ ] **SIM-KIT-009**: When UnassignJob is applied, the system shall return the Home kitten with the highest job_seq for that job to loafing; if no Home kitten holds the job (including when its only holders are Away), it shall reject with NoWorkersAssigned.
- [ ] **SIM-KIT-010**: The system shall count effective workers for a job as the number of Home kittens holding it; Away kittens retain their job title but are never counted.

## Happiness & Mood

- [ ] **SIM-KIT-011**: The system shall compute happiness each tick (never storing it) as 100% − 3 points per Home kitten beyond the fifth + 5 points per Scratching Post + 1 point per loafing Home kitten (at most 5 such points) + 10 points where the happiness idea (SIM-CUR) is researched, floored at 25% with no upper cap.
- [ ] **SIM-KIT-012**: The system shall apply a mood multiplier of happiness/100 to job production, further multiplied by 0.25 while the clowder is Grumpy.
- [ ] **SIM-KIT-013**: While the most recent consumption phase was a shortfall (SIM-ECO: pooled demand exceeded stock), the clowder shall be Grumpy; Grumpiness clears on the first tick whose consumption phase was not a shortfall.
- [ ] **SIM-KIT-014**: The system shall classify mood into tiers — Blissful (happiness ≥ 120), Content (≥ 90), Restless (≥ 60), Fussy (< 60) — with Grumpy overriding all tiers while active.

## Famine & the Neighbor's Yard

- [ ] **SIM-KIT-015**: At each day close, the system shall update exactly one famine clock: a day that ended in shortfall increments famine_days and resets recovery_days; a day that did not increments recovery_days and resets famine_days (both saturating).
- [ ] **SIM-KIT-016**: At each day close where famine_days is at least 3 (including the close where it reaches 3), the system shall move the most recently arrived Home kitten to NeighborsYard, stamp its away_seq, and emit a KittenWanderedOff event — unless it is the only kitten at Home: the last Home kitten never wanders.
- [ ] **SIM-KIT-017**: At each day close where recovery_days is at least 1, catnip stock is above zero, and any kitten is at NeighborsYard, the system shall return the kitten with the lowest away_seq (an absent stamp sorting lowest) to Home, clear its away_seq, and emit a KittenReturned event.
- [ ] **SIM-KIT-018**: The system shall update the famine and recovery clocks from the engine's persisted shortfall flag (the Grumpy bit as it stood at the end of the previous tick), not from a fresh reading of the stock.
