# Engine Specs (SIM-ENG)

## Time & Calendar

- [x] **SIM-ENG-001**: The system shall advance the simulation in discrete ticks (each representing 250 ms of game time), deriving all calendar state from the tick counter alone: day = ticks/10, season = (day/25) mod 4 in the order Spring, Summer, Autumn, Winter, year = day/100.
- [x] **SIM-ENG-002**: When a tick's counter increment crosses a day boundary, the system shall run daily processing before that tick's production phase, in the fixed order: day-close clock update, kitten returns, kitten wanders, arrival check.
- [x] **SIM-ENG-003**: When a tick crosses a season boundary, the system shall emit a SeasonChanged event, and when it crosses a year boundary, a NewYear event; no boundary events fire at game start (tick 0 is mid-day-0).

## Tick Pipeline & Commands

- [x] **SIM-ENG-004**: The system shall execute tick phases in the fixed order: calendar/daily processing, production, consumption, mood update, random events, cap clamping.
- [x] **SIM-ENG-005**: The system shall apply commands only between ticks, and a command shall either fully apply or return a typed error having mutated nothing (RNG state included).
- [x] **SIM-ENG-006**: When multiple command-rejection reasons apply simultaneously, the system shall report the first matching error in the precedence order: UnknownId, Locked, AlreadyResearched, NoIdleKittens, NoSeats, NoWorkersAssigned, StorageFull, InsufficientResources (the list grows append-adjacent, never reorders).

## Determinism

- [x] **SIM-ENG-007**: Given the same RNG seed and the same command sequence applied at the same tick counts, the system shall produce bit-identical serialized game state.
- [x] **SIM-ENG-008**: The system shall draw all randomness from a single serializable PCG-32 generator stored in the game state, and no simulation code shall read wall-clock time.
- [x] **SIM-ENG-015**: The system shall use saturating arithmetic for the pity, famine-day, and recovery-day counters, so counter overflow is impossible regardless of play or catch-up length.

## Events

- [x] **SIM-ENG-009**: The system shall report simulation occurrences as typed events (carrying names, ids, and values — never prose) returned from tick() and from command application.
- [ ] **SIM-ENG-026**: The system shall classify every event as flavor, mechanical, or milestone and expose that classification as a query on the event, so a consumer can rank an event's prominence without encoding rules about which event is which.
- [ ] **SIM-ENG-027**: The system shall retain an event journal in game state holding a bounded rolling window of the most recent entries plus every milestone entry permanently, each entry carrying the tick at which it occurred so entries can be grouped by day.
- [ ] **SIM-ENG-028**: The system shall append events to the journal when they arise from a live tick or an applied command, and shall not append events arising from catch-up replay — a long absence is recorded as its folded report, never as its millions of constituent events.
- [ ] **SIM-ENG-029**: When the rolling window is full, the system shall evict its oldest non-milestone entry, so growth is bounded without ever discarding a milestone.

## Derived Queries

- [ ] **SIM-ENG-030**: The system shall expose a per-resource net rate — production less consumption at current state, per game second — computed from the same production and consumption path the tick pipeline runs, never from a separate formula that could diverge from it.
- [ ] **SIM-ENG-034**: The system shall report the net rate free of the speed multiplier, no more reading speed than any other rule does; a caller wanting a real-time estimate applies speed itself, at the same boundary that converts real time into ticks.
- [ ] **SIM-ENG-035**: The system shall report the net rate as advisory, reflecting throttling and current state but carrying no guarantee against a stock reaching a cap or a supply changing before the rate is realised.
- [ ] **SIM-ENG-031**: The system shall expose the reason a piece of content is not currently actionable as typed data rather than prose, determined by the same gate the command layer enforces, so a caller can state the reason without deciding it.
- [ ] **SIM-ENG-036**: The system shall resolve a lock reason by the same precedence the command layer applies when several blockers hold at once, so the reason shown for an item and the error returned by acting on it never name different causes.
- [ ] **SIM-ENG-037**: The system shall withhold from a lock reason any cost or prerequisite detail of content the player has not yet revealed, the reason for an unrevealed item saying only that it is not yet available.
- [ ] **SIM-ENG-032**: The system shall answer an availability query for every revealable content kind, returning the two-state answer for kinds that carry no reveal ladder, so a caller never has to branch on which kind it is holding.
- [ ] **SIM-ENG-038**: The system shall expose the total cost of taking a repeatable action a given number of times from current state, escalation included, so a caller previewing a bulk action does not reimplement the escalation rule outside the core.
- [ ] **SIM-ENG-039**: The system shall expose how far current stock covers an action's next cost as a fraction, so the distinction between partly and not-at-all affordable is drawn by the same rules that gate the action rather than by a caller's own threshold.
- [ ] **SIM-ENG-033**: Where a repeatable action's affordable count has a closed form, the system shall expose that count, so a caller can name it in advance instead of deriving it by attempting the action until it is rejected.
- [x] **SIM-ENG-010**: The system shall emit a KittenMood event only when the mood tier changes (tiers defined in SIM-KIT; entering or leaving Grumpy is a tier change).

## Zoomies

- [x] **SIM-ENG-011**: While zoomies are inactive, on each day-boundary tick during the random-events phase (after production), the system shall start zoomies with 4% probability, setting the active window and emitting ZoomiesStarted; zoomies may restart on the same tick their previous window expired.
- [x] **SIM-ENG-012**: While zoomies are active, the system shall multiply job production by 1.25 — or by 1.5 whenever the zoomies-enhancing idea (SIM-CUR) is researched — for exactly 120 boosted production ticks, or 240 where that idea was already researched when the zoomies started (magnitude is live; duration is fixed at start); zoomies stack multiplicatively with the mood multiplier, including while Grumpy.
- [x] **SIM-ENG-013**: During each tick's random-events phase, if the zoomies window has expired, the system shall reset it to inactive and emit ZoomiesEnded before rolling any new zoomies start.
- [x] **SIM-ENG-018**: On a day-boundary tick, the system shall consume RNG draws in the fixed order: arrival roll (no draw when pity forces success or the check is ineligible), arrival name draw, arrival trait draw (the latter two only on success), zoomies start roll, flavor roll, flavor template draw, flavor kitten draw, sky-event roll, sky auto-observe roll, prism-descent roll, rainbow-shower roll — later draws occurring only when their preceding roll fired; at a season-boundary tick a déjà-vu roll (and, on success, a reverie-amount draw) appends after those. New subsystems append to (never reorder) this schedule.

## Initial State & Derived Queries

- [x] **SIM-ENG-016**: When a new game starts, the system shall initialize an empty meadow — zero stocks, no structures, no kittens, no researched ideas, tick 0, game speed 1.0 — with the RNG seeded from a caller-supplied seed.
- [x] **SIM-ENG-017**: Where the season-forecast idea (SIM-CUR) is researched, the system shall expose the upcoming season as a queryable derived value.

## Effect System & Shared Mechanics

- [x] **SIM-ENG-019**: The system shall resolve every modifier through the typed effect-key system, summing contributions per key in the fixed source-class order (structures, refinements, ideas, house rules, rites, arch upgrades, traditions, colony standing, adventure rewards, Elder trait) so identical states yield bit-identical sums.
- [x] **SIM-ENG-020**: The system shall provide the shared diminishing-returns primitives — limited_dr(v, limit): linear to 75% of limit and asymptotic to limit above it, never reaching limit; unlimited_dr(v, stripe) = (sqrt(1 + 8v/stripe) − 1)/2 — both monotonically increasing.
- [x] **SIM-ENG-021**: The system shall compute per-tick production in the normative layer order (base rate; per-source ratios; season/weather; mood; zoomies; global production ratios; Elder Wisdom capped pool; the Long Sun bonus; mess and energy-deficit multipliers); mood and zoomies apply to all job outputs, and the global layers (6–8: global production ratio, Elder Wisdom, Long Sun) apply to all raw-tier material production — structures and converters included, not only jobs — while non-material resources (transients including cleverness, luxuries, mystic, dreamtime) skip layers 6–8 entirely. A global-source structure's own raw output is not amplified by the global ratio it itself contributes (no self-reference): the Star Basin's stardust trickle takes the global multiplier from *other* sources only. Layer 7 (Elder Wisdom: +1%/point limited_dr-capped at 2.0×nine_lives_ratio, plus Passed-Down Wisdom +1%/point capped at 1.0, or 4.0 in the Distant Dawn) and layer 8 (the Long Sun) are both real production multipliers, both dimmed by the energy deficit ratio exactly as layer 6 is. **Cosmic outpost structures (the `outdoor` channel) take layers 6–8 at a reduced transfer fraction** — only a fraction of the meadow's global/wisdom/Long-Sun bonus reaches outdoor production (the inspiration's space-production dilution) — so the outer frontier is meaningful without the meadow's multipliers trivializing it.
- [x] **SIM-ENG-025**: Where Stargazing Ledges exist, at each day close the system shall roll a sky event with 0.2% × Ledges probability; the event is auto-observed with probability 1% × Ledges (granting 25 curiosity scaled by curiosity ratios, plus 1 skymap) or else emits a SkyEventVisible event observable by an ObserveSky command until the end of the next day for the same yields; an expired event simply fades.
- [x] **SIM-ENG-022**: The system shall throttle every converter by its worst-supplied input's satisfaction fraction, scaling all of that converter's inputs and outputs by the same fraction; a fully starved converter idles without error or event spam.
- [x] **SIM-ENG-023**: The system shall reveal a content item permanently (persisted, emitting a Revealed event) the first time its gate is satisfied and every cost component reaches the reveal fraction (default 30%) of its current price, and shall expose advisory availability — Hidden (unrevealed), Teased (revealed, not actionable), Available (revealed, actionable) — without changing command-layer gating.
- [x] **SIM-ENG-024**: When SetSpeed is applied, the system shall clamp the value to [0.25, 8], store it in game state, and emit a SpeedChanged event; no tick-internal rule shall read the speed (speed only changes how many ticks the boundary drives).

## Flavor Events

- [x] **SIM-ENG-014**: On each day-boundary tick during the random-events phase, the system shall emit a flavor event with 8% probability, selecting uniformly among eligible templates (needs-kitten templates are eligible only while at least one kitten is Home) and drawing a random Home kitten when the template requires one; flavor events shall have no mechanical effect on game state.
