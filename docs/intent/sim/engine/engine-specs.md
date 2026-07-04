# Engine Specs (SIM-ENG)

## Time & Calendar

- [x] **SIM-ENG-001**: The system shall advance the simulation in discrete ticks (each representing 250 ms of game time), deriving all calendar state from the tick counter alone: day = ticks/10, season = (day/25) mod 4 in the order Spring, Summer, Autumn, Winter, year = day/100.
- [x] **SIM-ENG-002**: When a tick's counter increment crosses a day boundary, the system shall run daily processing before that tick's production phase, in the fixed order: day-close clock update, kitten returns, kitten wanders, arrival check.
- [x] **SIM-ENG-003**: When a tick crosses a season boundary, the system shall emit a SeasonChanged event, and when it crosses a year boundary, a NewYear event; no boundary events fire at game start (tick 0 is mid-day-0).

## Tick Pipeline & Commands

- [x] **SIM-ENG-004**: The system shall execute tick phases in the fixed order: calendar/daily processing, production, consumption, mood update, random events, cap clamping.
- [x] **SIM-ENG-005**: The system shall apply commands only between ticks, and a command shall either fully apply or return a typed error having mutated nothing (RNG state included).
- [x] **SIM-ENG-006**: When multiple command-rejection reasons apply simultaneously, the system shall report the first matching error in the precedence order: UnknownId, Locked, AlreadyResearched, NoIdleKittens, NoWorkersAssigned, StorageFull, InsufficientResources.

## Determinism

- [x] **SIM-ENG-007**: Given the same RNG seed and the same command sequence applied at the same tick counts, the system shall produce bit-identical serialized game state.
- [x] **SIM-ENG-008**: The system shall draw all randomness from a single serializable PCG-32 generator stored in the game state, and no simulation code shall read wall-clock time.
- [x] **SIM-ENG-015**: The system shall use saturating arithmetic for the pity, famine-day, and recovery-day counters, so counter overflow is impossible regardless of play or catch-up length.

## Events

- [x] **SIM-ENG-009**: The system shall report simulation occurrences as typed events (carrying names, ids, and values — never prose) returned from tick() and from command application.
- [x] **SIM-ENG-010**: The system shall emit a KittenMood event only when the mood tier changes (tiers defined in SIM-KIT; entering or leaving Grumpy is a tier change).

## Zoomies

- [x] **SIM-ENG-011**: While zoomies are inactive, on each day-boundary tick during the random-events phase (after production), the system shall start zoomies with 4% probability, setting the active window and emitting ZoomiesStarted; zoomies may restart on the same tick their previous window expired.
- [x] **SIM-ENG-012**: While zoomies are active, the system shall multiply job production by 1.25 — or by 1.5 whenever the zoomies-enhancing idea (SIM-CUR) is researched — for exactly 120 boosted production ticks, or 240 where that idea was already researched when the zoomies started (magnitude is live; duration is fixed at start); zoomies stack multiplicatively with the mood multiplier, including while Grumpy.
- [x] **SIM-ENG-013**: During each tick's random-events phase, if the zoomies window has expired, the system shall reset it to inactive and emit ZoomiesEnded before rolling any new zoomies start.
- [x] **SIM-ENG-018**: On a day-boundary tick, the system shall consume RNG draws in the fixed order: arrival roll (no draw when pity forces success or the check is ineligible), zoomies start roll, flavor roll, flavor template draw, flavor kitten draw — later draws occurring only when their preceding roll fired.

## Initial State & Derived Queries

- [x] **SIM-ENG-016**: When a new game starts, the system shall initialize an empty meadow — zero stocks, no structures, no kittens, no researched ideas, tick 0 — with the RNG seeded from a caller-supplied seed.
- [x] **SIM-ENG-017**: Where the season-forecast idea (SIM-CUR) is researched, the system shall expose the upcoming season as a queryable derived value.

## Flavor Events

- [x] **SIM-ENG-014**: On each day-boundary tick during the random-events phase, the system shall emit a flavor event with 8% probability, selecting uniformly among eligible templates (needs-kitten templates are eligible only while at least one kitten is Home) and drawing a random Home kitten when the template requires one; flavor events shall have no mechanical effect on game state.
