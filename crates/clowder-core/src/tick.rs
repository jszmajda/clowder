use crate::content;
use crate::events::Event;
use crate::state::{season_of_day, GameState, IdeaId, JobId, Location, ResourceId, StructureId, EPSILON};

impl GameState {
    /// Advance the simulation one tick (250 ms of game time), running the
    /// fixed phase pipeline: calendar/daily processing, production,
    /// consumption, mood, random events, clamp.
    // @spec SIM-ENG-004
    pub fn tick(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let tier_before = self.mood_tier();

        // ---- Phase 1: calendar & daily processing ----
        // @spec SIM-ENG-002
        let prev_day = self.day();
        self.ticks += 1;
        let day = self.day();
        if day > prev_day {
            // @spec SIM-ENG-003
            let season = season_of_day(day);
            if season != season_of_day(prev_day) {
                events.push(Event::SeasonChanged { season });
            }
            let year = day / content::DAYS_PER_YEAR;
            if year > prev_day / content::DAYS_PER_YEAR {
                events.push(Event::NewYear { year });
            }
            self.day_close(&mut events);
        }

        // ---- Phase 2: production ----
        self.produce();

        // ---- Phase 3: consumption ----
        // @spec SIM-ECO-020, SIM-ECO-021
        let shortfall = self.consume();

        // ---- Phase 4: mood & famine flag ----
        // @spec SIM-KIT-013
        self.grumpy = shortfall;
        let tier_after = self.mood_tier();
        if tier_after != tier_before {
            // @spec SIM-ENG-010
            events.push(Event::KittenMood { tier: tier_after });
        }

        // ---- Phase 5: random events ----
        self.random_events(day > prev_day, &mut events);

        // ---- Phase 6: clamp ----
        // @spec SIM-ECO-003
        for r in ResourceId::ALL {
            let cap = self.cap(r);
            if self.resources[r.ix()] > cap {
                self.resources[r.ix()] = cap;
            }
        }

        events
    }

    /// Day-boundary processing, in fixed order: clock update from the
    /// persisted shortfall flag, returns, wanders, arrival check.
    // @spec SIM-ENG-002
    fn day_close(&mut self, events: &mut Vec<Event>) {
        // (a) Exactly one clock advances; the other resets.
        // @spec SIM-KIT-015, SIM-KIT-018
        if self.grumpy {
            self.famine_days = self.famine_days.saturating_add(1); // @spec SIM-ENG-015
            self.recovery_days = 0;
        } else {
            self.recovery_days = self.recovery_days.saturating_add(1);
            self.famine_days = 0;
        }

        // (b) Returns: first out, first back, and only into a stocked pantry.
        // @spec SIM-KIT-017
        if self.recovery_days >= 1 && self.stock(ResourceId::Catnip) > EPSILON {
            let returning = self
                .kittens
                .iter()
                .enumerate()
                .filter(|(_, k)| k.location == Location::NeighborsYard)
                .min_by_key(|(_, k)| k.away_seq.map_or(0, |seq| seq + 1))
                .map(|(ix, _)| ix);
            if let Some(ix) = returning {
                let kitten = &mut self.kittens[ix];
                kitten.location = Location::Home;
                kitten.away_seq = None;
                events.push(Event::KittenReturned { name: kitten.name.clone() });
            }
        }

        // (c) Wanders: newest bonds are loosest; the last Home kitten never goes.
        // @spec SIM-KIT-016
        if self.famine_days >= content::FAMINE_WANDER_DAYS {
            let home: Vec<usize> = self
                .kittens
                .iter()
                .enumerate()
                .filter(|(_, k)| k.location == Location::Home)
                .map(|(ix, _)| ix)
                .collect();
            if home.len() >= 2 {
                let ix = *home.last().expect("len checked");
                let seq = self.next_seq;
                self.next_seq += 1;
                let kitten = &mut self.kittens[ix];
                kitten.location = Location::NeighborsYard;
                kitten.away_seq = Some(seq);
                events.push(Event::KittenWanderedOff { name: kitten.name.clone() });
            }
        }

        // (d) Arrival check.
        // @spec SIM-KIT-003, SIM-KIT-005, SIM-KIT-007
        let eligible =
            self.free_slots() >= 1 && self.stock(ResourceId::Catnip) + EPSILON >= content::ARRIVAL_MIN_CATNIP;
        if eligible {
            // Pity forces success without an RNG draw. @spec SIM-ENG-018
            let success = self.pity >= content::PITY_THRESHOLD || {
                let chance = self.arrival_chance();
                self.rng.chance(chance)
            };
            if success {
                let name = self.draw_name();
                self.kittens.push(crate::state::Kitten {
                    name: name.clone(),
                    job: None,
                    location: Location::Home,
                    arrived_day: self.day(),
                    job_seq: None,
                    away_seq: None,
                });
                self.pity = 0;
                events.push(Event::KittenArrived { name });
            } else {
                self.pity = self.pity.saturating_add(1);
            }
        }
    }

    /// Structures first (season-modified where marked, never mood-modified),
    /// then jobs (mood- and zoomies-modified), accumulating in fixed enum
    /// order so float results are deterministic.
    // @spec SIM-ECO-023
    fn produce(&mut self) {
        let season = self.season();
        let winter_eased = self.ideas.contains(&IdeaId::SeasonalAwareness);
        for s in StructureId::ALL {
            let count = self.structures[s.ix()];
            if count == 0 {
                continue;
            }
            let def = content::structure_def(s);
            if let Some((r, amount)) = def.production {
                let modifier = if def.seasonal {
                    // @spec SIM-ECO-019
                    content::season_patch_modifier(season, winter_eased)
                } else {
                    1.0
                };
                self.resources[r.ix()] += amount * count as f64 * modifier;
            }
        }

        // @spec SIM-ENG-012
        let zoomies = if self.zoomies_active() {
            if self.ideas.contains(&IdeaId::ZoomieChanneling) {
                content::ZOOMIES_CHANNELED_MULT
            } else {
                content::ZOOMIES_MULT
            }
        } else {
            1.0
        };
        let mood = self.mood_multiplier();
        for j in JobId::ALL {
            let workers = self.effective_workers(j);
            if workers == 0 {
                continue;
            }
            let (r, per_worker) = content::job_def(j).yield_per_tick;
            self.resources[r.ix()] += per_worker * workers as f64 * mood * zoomies;
        }
    }

    /// Pooled consumption; returns whether this tick was a shortfall.
    // @spec SIM-ECO-020, SIM-ECO-021
    fn consume(&mut self) -> bool {
        let home = self.kittens.iter().filter(|k| k.location == Location::Home).count();
        if home == 0 {
            return false;
        }
        let demand = content::KITTEN_APPETITE * home as f64;
        let ix = ResourceId::Catnip.ix();
        let stock = self.resources[ix];
        if demand > stock + EPSILON {
            self.resources[ix] = 0.0;
            true
        } else {
            self.resources[ix] = (stock - demand).max(0.0);
            false
        }
    }

    /// Zoomies expiry (every tick), then — at day boundaries only — the fixed
    /// draw sequence: zoomies start, flavor roll, template, kitten.
    // @spec SIM-ENG-018
    fn random_events(&mut self, day_boundary: bool, events: &mut Vec<Event>) {
        // @spec SIM-ENG-013
        if self.zoomies_until != 0 && self.ticks >= self.zoomies_until {
            self.zoomies_until = 0;
            events.push(Event::ZoomiesEnded);
        }

        if !day_boundary {
            return;
        }

        // @spec SIM-ENG-011
        if self.zoomies_until == 0 && self.rng.chance(content::ZOOMIES_DAILY_CHANCE) {
            let duration = if self.ideas.contains(&IdeaId::ZoomieChanneling) {
                content::ZOOMIES_CHANNELED_TICKS
            } else {
                content::ZOOMIES_TICKS
            };
            // +1: the start lands after this tick's production, so the window
            // extends one past the arithmetic end for exactly `duration`
            // boosted production ticks. @spec SIM-ENG-012
            self.zoomies_until = self.ticks + duration + 1;
            events.push(Event::ZoomiesStarted { until_tick: self.zoomies_until });
        }

        // @spec SIM-ENG-014
        if self.rng.chance(content::FLAVOR_DAILY_CHANCE) {
            let any_home = self.kittens.iter().any(|k| k.location == Location::Home);
            let eligible: Vec<usize> = content::FLAVOR_TEMPLATES
                .iter()
                .enumerate()
                .filter(|(_, t)| any_home || !t.needs_kitten)
                .map(|(ix, _)| ix)
                .collect();
            let template = eligible[self.rng.index(eligible.len())];
            let kitten = if content::FLAVOR_TEMPLATES[template].needs_kitten {
                let homes: Vec<&str> = self
                    .kittens
                    .iter()
                    .filter(|k| k.location == Location::Home)
                    .map(|k| k.name.as_str())
                    .collect();
                Some(homes[self.rng.index(homes.len())].to_string())
            } else {
                None
            };
            events.push(Event::Flavor { template, kitten });
        }
    }

    /// Draw a name from the pool without replacement, cycling with Roman
    /// numeral suffixes once exhausted. The remaining pool is derived from the
    /// roster — no separate stored state.
    // @spec SIM-KIT-002
    fn draw_name(&mut self) -> String {
        let taken: std::collections::BTreeSet<&str> =
            self.kittens.iter().map(|k| k.name.as_str()).collect();
        for cycle in 1u32.. {
            let candidates: Vec<String> = content::NAME_POOL
                .iter()
                .map(|base| {
                    if cycle == 1 {
                        (*base).to_string()
                    } else {
                        format!("{} {}", base, roman(cycle))
                    }
                })
                .filter(|name| !taken.contains(name.as_str()))
                .collect();
            if !candidates.is_empty() {
                let ix = self.rng.index(candidates.len());
                return candidates[ix].clone();
            }
        }
        unreachable!("some cycle always has free names")
    }
}

fn roman(n: u32) -> String {
    const VALUES: [(u32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut n = n;
    let mut out = String::new();
    for (value, glyph) in VALUES {
        while n >= value {
            out.push_str(glyph);
            n -= value;
        }
    }
    out
}
