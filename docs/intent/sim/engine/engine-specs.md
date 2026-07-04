# Engine Specs (SIM-ENG)

## Time & Calendar

- [ ] **SIM-ENG-001**: The system shall advance the simulation in discrete ticks (each representing 250 ms of game time), deriving all calendar state from the tick counter alone: day = ticks/10, season = (day/25) mod 4 in the order Spring, Summer, Autumn, Winter, year = day/100.
- [ ] **SIM-ENG-002**: When a tick's counter increment crosses a day boundary, the system shall run daily processing before that tick's production phase, in the fixed order: day-close clock update, kitten returns, kitten wanders, arrival check.
- [ ] **SIM-ENG-003**: When a tick crosses a season boundary, the system shall emit a SeasonChanged event, and when it crosses a year boundary, a NewYear event; no boundary events fire at game start (tick 0 is mid-day-0).

## Tick Pipeline & Commands

- [ ] **SIM-ENG-004**: The system shall execute tick phases in the fixed order: calendar/daily processing, production, consumption, mood update, random events, cap clamping.
- [ ] **SIM-ENG-005**: The system shall apply commands only between ticks, and a command shall either fully apply or return a typed error having mutated nothing (RNG state included).
- [ ] **SIM-ENG-006**: When multiple command-rejection reasons apply simultaneously, the system shall report the first matching error in the precedence order: UnknownId, Locked, AlreadyResearched, NoIdleKittens, NoWorkersAssigned, StorageFull, InsufficientResources.

## Determinism

- [ ] **SIM-ENG-007**: Given the same RNG seed and the same command sequence applied at the same tick counts, the system shall produce bit-identical serialized game state.
- [ ] **SIM-ENG-008**: The system shall draw all randomness from a single serializable PCG-32 generator stored in the game state, and no simulation code shall read wall-clock time.
- [ ] **SIM-ENG-015**: The system shall use saturating arithmetic for the pity, famine-day, and recovery-day counters, so counter overflow is impossible regardless of play or catch-up length.

## Events

- [ ] **SIM-ENG-009**: The system shall report simulation occurrences as typed events (carrying names, ids, and values — never prose) returned from tick() and from command application.
- [ ] **SIM-ENG-010**: The system shall emit a KittenMood event only when the mood tier changes (tiers defined in SIM-KIT; entering or leaving Grumpy is a tier change).

## Zoomies

- [ ] **SIM-ENG-011**: While zoomies are inactive, at each day close the system shall start zoomies with 4% probability, setting the active window and emitting ZoomiesStarted.
- [ ] **SIM-ENG-012**: While zoomies are active (ticks < zoomies_until), the system shall multiply job production by 1.25 — or by 1.5 whenever the zoomies-enhancing idea (SIM-CUR) is researched — with the window lasting 120 ticks, or 240 ticks where that idea was already researched when the zoomies started (magnitude is live; duration is fixed at start).
- [ ] **SIM-ENG-013**: On the first tick at or past zoomies expiry, the system shall reset the zoomies window to inactive and emit ZoomiesEnded.

## Initial State & Derived Queries

- [ ] **SIM-ENG-016**: When a new game starts, the system shall initialize an empty meadow — zero stocks, no structures, no kittens, no researched ideas, tick 0 — with the RNG seeded from a caller-supplied seed.
- [ ] **SIM-ENG-017**: Where the season-forecast idea (SIM-CUR) is researched, the system shall expose the upcoming season as a queryable derived value.

## Flavor Events

- [ ] **SIM-ENG-014**: At each day close, the system shall emit a flavor event with 8% probability, selecting uniformly among eligible templates (needs-kitten templates are eligible only while at least one kitten is Home) and drawing a random Home kitten when the template requires one; flavor events shall have no mechanical effect on game state.
