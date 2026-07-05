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

        // ---- Phase 2: production (structures, jobs, converters) ----
        let mess_accrual = self.produce();

        // ---- Phase 3: consumption (food, then luxury drain) ----
        // @spec SIM-ECO-020, SIM-ECO-021
        let shortfall = self.consume();

        // ---- Phase 4: mood, famine flag & mess recompute ----
        // @spec SIM-KIT-013
        self.grumpy = shortfall;
        // @spec SIM-ECO-037
        self.mess =
            (self.mess + mess_accrual - content::MESS_DISSIPATION * self.mess).max(0.0);
        let tier_after = self.mood_tier();
        if tier_after != tier_before {
            // @spec SIM-ENG-010
            events.push(Event::KittenMood { tier: tier_after });
        }

        // ---- Phase 5: random events ----
        self.random_events(day > prev_day, &mut events);

        // ---- Phase 6: clamp ----
        // The kite high-water ratchets before caps are read, so a fresh haul
        // widens the Porch the same tick. @spec SIM-ECO-031
        let kites = self.resources[ResourceId::Kites.ix()];
        if kites > self.kites_high_water {
            self.kites_high_water = kites;
        }
        // @spec SIM-ECO-003
        for r in ResourceId::ALL {
            // Fast path: cap extensions are never negative, so a stock at or
            // under its base cap can't need trimming.
            if self.resources[r.ix()] <= content::BASE_CAPS[r.ix()] {
                continue;
            }
            let cap = self.cap(r);
            if self.resources[r.ix()] > cap {
                self.resources[r.ix()] = cap;
            }
        }

        // Reveal scan: sticky, ordered, event-emitting. @spec SIM-ENG-023
        for s in StructureId::ALL {
            let id = crate::state::ContentId::Structure(s);
            if !self.revealed.contains(&id) && self.reveal_condition_met(id) {
                self.revealed.insert(id);
                events.push(Event::Revealed { id });
            }
        }
        // The curiosity gate is one check for the whole tree, hoisted out of
        // the loop (idea reveal = idea_unlocked = gate + prerequisites).
        let curiosity_open = self.cap(ResourceId::Curiosity) > 0.0;
        if curiosity_open {
            for i in crate::state::IdeaId::ALL {
                let id = crate::state::ContentId::Idea(i);
                if !self.revealed.contains(&id) && self.idea_prereqs_met(i) {
                    self.revealed.insert(id);
                    events.push(Event::Revealed { id });
                }
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

    /// Phase 2: structures, then jobs, then converters, accumulating in fixed
    /// enum order so float results are deterministic. Implements the
    /// production stack's normative layer order — structures take only the ☆
    /// layers (base, per-source ratio, season); jobs additionally take mood,
    /// zoomies, and — material tier only — the global layers, dimmed by any
    /// energy deficit; mess multiplies catnip growth last. Returns the tick's
    /// mess accrual (applied in phase 4).
    // @spec SIM-ENG-021, SIM-ECO-023, SIM-ECO-025, SIM-ECO-032, SIM-ECO-036
    fn produce(&mut self) -> f64 {
        let season = self.season();
        let winter_eased = self.ideas.contains(&IdeaId::SeasonalAwareness);
        // Layer-6 inputs, frozen at phase start (before converters draw fuel).
        // Layers 7–8 (Elder Wisdom, the Long Sun) are ×1.0 until their waves.
        let deficit = self.energy_deficit_ratio();
        let global_mult = 1.0 + self.effect(crate::effects::Effect::GlobalProductionRatio) * deficit;
        let surplus = self.energy_surplus_ratio();
        let mess_catnip = self.mess_catnip_factor();

        // ---- Structures (☆ layers only) ----
        for s in StructureId::ALL {
            let count = self.structures[s.ix()];
            if count == 0 {
                continue;
            }
            let def = content::structure_def(s);
            for (r, amount) in def.productions {
                let mut flow = amount * count as f64;
                flow *= 1.0 + self.effect(crate::effects::Effect::ResourceRatio(*r));
                if def.seasonal {
                    // @spec SIM-ECO-019
                    flow *= content::season_patch_modifier(season, winter_eased);
                }
                if def.surplus_scaled {
                    flow *= surplus;
                }
                if *r == ResourceId::Catnip {
                    // @spec SIM-ECO-038
                    flow *= mess_catnip;
                }
                self.resources[r.ix()] += flow;
            }
        }

        // ---- Jobs ----
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
            let Some((r, per_worker)) = content::job_def(j).yield_per_tick else {
                continue; // the Tinkerer crafts in the background — later wave
            };
            let mut flow = per_worker * workers as f64;
            flow *= 1.0 + self.effect(crate::effects::Effect::ResourceRatio(r));
            flow *= mood * zoomies;
            if content::tier(r) == content::Tier::Raw {
                flow *= global_mult;
            }
            if r == ResourceId::Catnip {
                flow *= mess_catnip;
            }
            self.resources[r.ix()] += flow;
        }

        // ---- Converters (throttled), accruing mess as they run ----
        self.run_converters()
    }

    /// Run every converter through the engine throttle and total the tick's
    /// mess: converters dirty in proportion to their throughput, everything
    /// else per active copy.
    // @spec SIM-ECO-029, SIM-ECO-037
    fn run_converters(&mut self) -> f64 {
        let mut mess_accrual = 0.0;
        for s in StructureId::ALL {
            let count = self.structures[s.ix()];
            if count == 0 {
                continue;
            }
            let def = content::structure_def(s);
            let Some(converter) = def.converter else {
                mess_accrual += def.mess * count as f64;
                continue;
            };
            let demands: Vec<(ResourceId, f64)> = converter
                .inputs
                .iter()
                .map(|(r, d)| (*r, d * count as f64))
                .collect();
            let fraction = self.throttle_fraction(&demands);
            mess_accrual += def.mess * count as f64 * fraction;
            if fraction <= 0.0 {
                continue; // a starved converter idles quietly
            }
            for (r, demand) in &demands {
                let ix = r.ix();
                self.resources[ix] = (self.resources[ix] - demand * fraction).max(0.0);
            }
            for (r, out) in converter.outputs {
                self.resources[r.ix()] += out * count as f64 * fraction;
            }
            // The Treat Bakery's outputs ride the pep *cap* (mpr), not a
            // recipe table. @spec SIM-ECO-030
            if s == StructureId::TreatBakery {
                let mpr = self.cap(ResourceId::Pep) * content::BAKERY_MPR_PER_PEP_CAP;
                self.resources[ResourceId::Feathers.ix()] +=
                    content::BAKERY_FEATHERS_MPR * mpr * count as f64 * fraction;
                self.resources[ResourceId::Bells.ix()] +=
                    content::BAKERY_BELLS_MPR * mpr * count as f64 * fraction;
            }
        }
        mess_accrual
    }

    /// Pooled consumption; returns whether this tick was a shortfall. Food
    /// first, then the luxury drain — which floors at zero and never
    /// shortfalls: an empty shelf just drops its happiness bonus.
    // @spec SIM-ECO-020, SIM-ECO-021, SIM-ECO-026
    fn consume(&mut self) -> bool {
        let home = self.kittens.iter().filter(|k| k.location == Location::Home).count();
        if home == 0 {
            return false;
        }
        let demand = content::KITTEN_APPETITE * home as f64;
        let ix = ResourceId::Catnip.ix();
        let stock = self.resources[ix];
        let shortfall = if demand > stock + EPSILON {
            self.resources[ix] = 0.0;
            true
        } else {
            self.resources[ix] = (stock - demand).max(0.0);
            false
        };

        // Luxury drain, per Home kitten, eased by demand effects (floor 0).
        // An empty shelf drains nothing — skip the effect sum entirely.
        if content::LUXURY_DRAINS.iter().any(|(r, _)| self.resources[r.ix()] > 0.0) {
            let scale =
                (1.0 + self.effect(crate::effects::Effect::LuxuryDemandRatio)).max(0.0);
            for (r, per_kitten) in content::LUXURY_DRAINS {
                let ix = r.ix();
                let drain = per_kitten * home as f64 * scale;
                self.resources[ix] = (self.resources[ix] - drain).max(0.0);
            }
        }

        shortfall
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

        // Sky events — appended AFTER the flavor draws, per the extended
        // draw schedule (new subsystems never reorder it). @spec SIM-ENG-018
        self.sky_events(events);
    }

    /// Day-close sky processing: an expired observation window fades
    /// silently; then, where Ledges exist and nothing is pending, roll for an
    /// event and (on a hit) for lucky self-observation.
    // @spec SIM-ENG-025
    fn sky_events(&mut self, events: &mut Vec<Event>) {
        if let Some(expiry) = self.pending_sky_event {
            if self.ticks >= expiry {
                self.pending_sky_event = None; // it simply faded
            }
        }
        let ledges = self.structures[StructureId::StargazingLedge.ix()];
        if ledges == 0 || self.pending_sky_event.is_some() {
            return; // no draw at all: at most one pending event at a time
        }
        if !self.rng.chance(content::SKY_EVENT_CHANCE_PER_LEDGE * ledges as f64) {
            return;
        }
        if self.rng.chance(content::SKY_AUTO_OBSERVE_PER_LEDGE * ledges as f64) {
            // Someone happened to be looking up.
            self.grant_sky_observation();
        } else {
            // Observable until the end of the next day.
            self.pending_sky_event = Some((self.day() + 1) * content::TICKS_PER_DAY);
            events.push(Event::SkyEventVisible);
        }
    }

    /// The yields of observing a sky event: curiosity scaled by its
    /// per-source ratios, plus a skymap, both clamped to their caps.
    // @spec SIM-ENG-025
    pub(crate) fn grant_sky_observation(&mut self) {
        let curiosity = content::SKY_OBSERVE_CURIOSITY
            * (1.0 + self.effect(crate::effects::Effect::ResourceRatio(ResourceId::Curiosity)));
        let ix = ResourceId::Curiosity.ix();
        self.resources[ix] = (self.resources[ix] + curiosity).min(self.cap(ResourceId::Curiosity));
        let ix = ResourceId::Skymaps.ix();
        self.resources[ix] =
            (self.resources[ix] + content::SKY_OBSERVE_SKYMAPS).min(self.cap(ResourceId::Skymaps));
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
