# Kittens Specs (SIM-KIT)

## Roster & Identity

- [x] **SIM-KIT-001**: The system shall never remove a kitten from the roster; a kitten's absence is represented by the location NeighborsYard, never by deletion.
- [x] **SIM-KIT-002**: When a kitten arrives, the system shall draw its name without replacement from the name pool via the state RNG, appending Roman-numeral suffixes (II, III, …) once the pool is exhausted, so every name in a save is unique.
- [x] **SIM-KIT-028**: The system shall hold a name pool of at least 300 names, each at most 16 characters, made only of ASCII letters with single spaces between words, every word beginning with a capital letter, and no two of which are the same or differ by a single character changed, added, or removed, ignoring case.
- [x] **SIM-KIT-029**: The system shall hold no name in the name pool that equals as a whole, ignoring case, the name of any resource (as the stores spell it, or that spelling with its last word made singular — a final "s" dropped, or a final "ies" read as "y"), structure, job, idea, craft, refinement, house rule, rite, arch, tradition, dream loom, colony, site, adventure, season, or kitten trait, each as the game spells it.
- [x] **SIM-KIT-030**: When a kitten arrives, the system shall count as taken every name already in the roster, Home or Away, compared ignoring case and whether or not the current name pool holds it, and shall leave every rostered kitten's name as it was saved, so a name pool that changes between versions renames no kitten and no draw repeats a rostered name.

## Arrival

- [x] **SIM-KIT-003**: The system shall check for kitten arrival once per day close, only when at least one housing slot is free and catnip stock is at least 50, admitting at most one kitten per day.
- [x] **SIM-KIT-004**: When an eligible arrival check runs, the system shall compute the admission chance as: min(15% + 5pp per full 100 catnip in stock + inside-cap arrival effects, 60%) + on-top effects (the arrival-boosting idea's +15pp), × the mess arrival factor (SIM-ECO-038), × (2 + jamboree-arrival effects) while a jamboree runs, clamped to [0, 1].
- [x] **SIM-KIT-005**: If four consecutive eligible arrival checks fail, then the system shall make the next eligible check succeed automatically; ineligible days pause the failure count (neither extending nor resetting it), and any arrival resets it.
- [x] **SIM-KIT-006**: The system shall count housing so that every rostered kitten holds a slot regardless of location (free slots = capacity − roster size), so a returning kitten never finds the house full and an arrival never displaces an absentee.
- [x] **SIM-KIT-007**: When a kitten arrives, the system shall add it at Home with no job, emit a KittenArrived event carrying its name, and reset the pity counter.

## Jobs

- [x] **SIM-KIT-008**: When AssignJob is applied for an unlocked job with at least one idle Home kitten, the system shall assign the most recently arrived idle Home kitten, stamp its job_seq from the monotone counter, and emit an event naming the kitten; with no idle Home kitten it shall reject with NoIdleKittens.
- [x] **SIM-KIT-009**: When UnassignJob is applied, the system shall return the Home kitten with the highest job_seq for that job to loafing; if no Home kitten holds the job (including when its only holders are Away), it shall reject with NoWorkersAssigned.
- [x] **SIM-KIT-010**: The system shall count effective workers for a job as the number of Home kittens holding it; Away kittens retain their job title but are never counted.

## Happiness & Mood

- [x] **SIM-KIT-011**: The system shall compute happiness each tick (never storing it) as 100% − crowding + happiness-point effects + 1 point per loafing Home kitten (at most 5) + 10 points per distinct stocked luxury (dust-guarded: stock > 1e-9) + 30 × (1 + jamboree-ratio effects) while a jamboree runs + unlimited_dr(Warm Memories, 5) (the prestige leaf's happiness legacy) − the economy leaf's mess penalty, floored at 25% with no upper cap — where crowding = 3 points per Home kitten beyond the fifth × (1 − limited_dr(0.048 × Story Circles + 0.075 × Moonlight Stages, 0.9)).
- [x] **SIM-KIT-012**: The system shall apply a mood multiplier of happiness/100 to job production, further multiplied by 0.25 while the clowder is Grumpy.
- [x] **SIM-KIT-013**: While the most recent consumption phase was a shortfall (SIM-ECO: pooled demand exceeded stock), the clowder shall be Grumpy; Grumpiness clears on the first tick whose consumption phase was not a shortfall.
- [x] **SIM-KIT-014**: The system shall classify mood into tiers — Blissful (happiness ≥ 120), Content (≥ 90), Restless (≥ 60), Fussy (< 60) — with Grumpy overriding all tiers while active.

## Famine & the Neighbor's Yard

- [x] **SIM-KIT-015**: At each day close, the system shall update exactly one famine clock: a day that ended in shortfall increments famine_days and resets recovery_days; a day that did not increments recovery_days and resets famine_days (both saturating).
- [x] **SIM-KIT-016**: At each day close where famine_days is at least 3 (including the close where it reaches 3), the system shall move the most recently arrived Home kitten to NeighborsYard, stamp its away_seq, and emit a KittenWanderedOff event — unless it is the only kitten at Home: the last Home kitten never wanders.
- [x] **SIM-KIT-017**: At each day close where recovery_days is at least 1, catnip stock is above zero, and any kitten is at NeighborsYard, the system shall return the kitten with the lowest away_seq (an absent stamp sorting lowest) to Home, clear its away_seq, and emit a KittenReturned event.
- [x] **SIM-KIT-018**: The system shall update the famine and recovery clocks from the engine's persisted shortfall flag (the Grumpy bit as it stood at the end of the previous tick), not from a fresh reading of the stock.

## Society: Skills, Ranks, the Elder, Jamborees

- [x] **SIM-KIT-019**: While a kitten is Home and employed (Grumpiness slows its paws, not its learning), the system shall add 0.01 × (1 + skill-XP effects) skill XP per tick to its current job's skill and the same amount to its lifetime exp pool, and decay that kitten's every other job skill by 0.001 per tick (floor 0) unless the decay-removing refinement (SIM-CUR) is owned; loafing and Away kittens neither accrue nor decay.
- [x] **SIM-KIT-020**: The system shall multiply each kitten's own job yield by its skill tier: +0% below 100 XP, +1.25% at 100, +2.5% at 500, +4.5% at 1200, +7.5% at 2500, +12.5% at 5000, +18.75% at 9000, resolving production per kitten in roster order.
- [x] **SIM-KIT-021**: When Promote(kitten) is applied with 500 × 1.75^rank exp in that kitten's pool and 25 × (rank+1) shinies in stock, the system shall spend both and increment the kitten's rank; otherwise it rejects with InsufficientResources.
- [x] **SIM-KIT-022**: When AppointElder(kitten) is applied, the system shall make that kitten the Elder (replacing any prior Elder; the game never removes one itself); the Elder's own-job yield is multiplied by max(1, (rank+1)/1.4).
- [x] **SIM-KIT-023**: When a kitten arrives, the system shall roll its trait uniformly over the eight traits (one RNG draw, after the name draw in the fixed schedule); a trait's clowder-wide effect applies only while that kitten is the Elder.
- [x] **SIM-KIT-024**: The system shall apply Elder trait effects exactly as the kittens design's trait list specifies (Crafty, Brassy, Alchemical, Charming, Bold, Curious, Serene, None), through the effect-key system; Charming and Serene are dormant until their consuming subsystems (colonies, rites) land.
- [x] **SIM-KIT-025**: When HostJamboree is applied after the jamboree-unlocking idea (SIM-CUR) is researched, with 800 pep, 1500 charm, and 40 quills affordable and no jamboree running, the system shall spend them and run a jamboree for 100 days: +30 × (1 + jamboree-ratio effects) happiness points, arrival chance × (2 + jamboree-arrival effects), and +0.2 contributed to the luxury-demand ratio (SIM-ECO-026's additive sum); before the idea, or during a jamboree, the command rejects with Locked.
- [x] **SIM-KIT-026**: The system shall reduce a Tinkerer's crafting effectiveness by 15% per recipe tier above rank+1 — effectiveness = 1 − 0.15 × max(0, tier − (rank + 1)), floored at 0.25 — so a rank-r kitten works tier r+1 recipes cleanly.
- [x] **SIM-KIT-027**: Skill XP, exp pools, ranks, and traits shall persist in the save and never decrease except by the specified decay and promotion spends (no mechanic confiscates a kitten's learning).
