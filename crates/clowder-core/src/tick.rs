use crate::content;
use crate::events::Event;
use crate::query::{credit, FlowLedger, FlowSource};
use crate::state::{
    season_of_day, AdventureId, GameState, IdeaId, JobId, Location, ResourceId, StructureId,
    EPSILON,
};

impl GameState {
    /// Advance the simulation one tick (250 ms of game time), running the
    /// fixed phase pipeline: calendar/daily processing, production,
    /// consumption, mood, random events, clamp. Live entry point: the
    /// occurrence is journaled.
    // @spec SIM-ENG-004
    // @spec SIM-ENG-028, SIM-ENG-040
    pub fn tick(&mut self) -> Vec<Event> {
        let events = self.tick_replay();
        let tick = self.ticks;
        for event in &events {
            self.journal.record(tick, event);
        }
        events
    }

    /// Every phase of a tick that moves a resource stock, in pipeline order,
    /// returning the tick's mess change (accrual less tidying) and whether
    /// consumption fell short.
    ///
    /// This exists so there is exactly **one** definition of what a tick does
    /// to stocks. `tick_replay` runs it to advance the game and `net_rates`
    /// derives from it to report a rate (SIM-ENG-030), and a derived rate that
    /// re-listed the phases itself would be a second copy of pipeline
    /// structure — silently wrong the moment a phase is added here and not
    /// there, which is the exact drift SIM-ENG-030 forbids. Adding a
    /// stock-moving phase means adding it here, once.
    ///
    /// Every stock move is credited to the one source it is made for, at the
    /// amount it actually moved the stock, into `ledger` where the caller
    /// keeps one (the flows query); a multiplier is never a source, its share
    /// riding inside the move it scales. A tick passes no ledger.
    // @spec SIM-ENG-004, SIM-ENG-030
    // @spec SIM-ENG-063, SIM-ENG-064, SIM-ENG-065
    pub(crate) fn resource_phases(&mut self, ledger: &mut Option<&mut FlowLedger>) -> (f64, bool) {
        // ---- Phase 2: production (structures, jobs, converters) ----
        let mess_change = self.produce(ledger);
        // ---- Phase 2b: dreaming (mystic drift, per tick) ----
        // Only Arch upgrades drive drift, so the arch-free hot path skips it.
        // @spec SIM-UNI-007, SIM-UNI-008
        if !self.arches_bought.is_empty() {
            self.dream_step(ledger);
        }
        // ---- Phase 3: consumption (food, then luxury drain) ----
        // @spec SIM-ECO-020, SIM-ECO-021
        let shortfall = self.consume(ledger);
        (mess_change, shortfall)
    }

    /// Raise the roster high-water if the clowder has grown. A ratchet: what
    /// prestige reveals is having *met* a size, not currently having it.
    /// @spec SIM-ENG-046
    pub(crate) fn ratchet_roster(&mut self) {
        if self.kittens.len() > self.roster_high_water {
            self.roster_high_water = self.kittens.len();
        }
    }

    /// The identical simulation step as [`GameState::tick`], but never
    /// journals. This is the core's own non-journaling entry point —
    /// catch-up replay and any other fast-forward (a long absence, a Deep
    /// Nap) call this instead of `tick`, so recording a replay's millions of
    /// constituent events is not something a caller can do by omission.
    /// A tick has no way to tell, from its own state, whether it is live or
    /// replaying; the distinction is made here, at the two entry points,
    /// rather than inside the pipeline itself.
    // @spec SIM-ENG-028, SIM-ENG-040
    pub fn tick_replay(&mut self) -> Vec<Event> {
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

        // ---- Phases 2, 2b, 3: everything that moves a stock ----
        self.ratchet_roster();
        let (mess_change, shortfall) = self.resource_phases(&mut None);

        // ---- Phase 4: mood, famine flag & mess recompute ----
        // @spec SIM-KIT-013
        self.grumpy = shortfall;
        self.mess = self.next_mess(mess_change);
        // Stirred dreams cool passively each tick (a nap-cadence brake, never a
        // loss), flooring at zero; free when the dream-space is already cool.
        // @spec SIM-DREAM-003
        if self.stirred_dreams > 0.0 {
            let dissip = content::HEAT_DISSIPATION_PER_TICK
                * (1.0 + self.effect(crate::effects::Effect::HeatDissipation));
            self.stirred_dreams = (self.stirred_dreams - dissip).max(0.0);
        }
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

        // Holdings: a resource is revealed once the clamp leaves some of it.
        // @spec SIM-ENG-048, SIM-ENG-054, SIM-ENG-055
        self.record_holdings();

        // Reveal scan: sticky, ordered, event-emitting. We walk only the
        // unrevealed worklists (the complement of `revealed`, canonical order),
        // draining each item as it reveals — no per-tick membership probe of
        // the whole content space. `mem::take` sidesteps the borrow conflict
        // between `retain`'s &mut and `reveal_condition_met`'s &self; the taken
        // vec is empty during the scan, which that predicate never reads.
        // @spec SIM-ENG-023
        let mut newly: Vec<crate::state::ContentId> = Vec::new();
        let mut pending = std::mem::take(&mut self.unrevealed_structs);
        if self.traditions.is_empty() {
            // Hot path (every tick until prestige): the wave-4 reveal loop.
            pending.retain(|&s| {
                let id = crate::state::ContentId::Structure(s);
                if self.reveal_condition_met(id) {
                    self.revealed.insert(id);
                    newly.push(id);
                    events.push(Event::Revealed { id });
                    false
                } else {
                    true
                }
            });
        } else {
            // Post-prestige: price-reduction Traditions lower reveal thresholds.
            // @spec SIM-MOVE-013
            let price_mult = self.global_price_mult();
            pending.retain(|&s| {
                let id = crate::state::ContentId::Structure(s);
                if self.reveal_condition_met_priced(id, price_mult) {
                    self.revealed.insert(id);
                    newly.push(id);
                    events.push(Event::Revealed { id });
                    false
                } else {
                    true
                }
            });
        }
        self.unrevealed_structs = pending;
        // The curiosity gate is one check for the whole tree, hoisted out of
        // the scan (idea reveal = gate + prerequisites); while it's shut, the
        // idea worklist can't move, so skip it entirely.
        if !self.unrevealed_ideas.is_empty() && self.cap(ResourceId::Curiosity) > 0.0 {
            let mut pending = std::mem::take(&mut self.unrevealed_ideas);
            pending.retain(|&i| {
                let id = crate::state::ContentId::Idea(i);
                if self.idea_prereqs_met(i) {
                    self.revealed.insert(id);
                    newly.push(id);
                    events.push(Event::Revealed { id });
                    false
                } else {
                    true
                }
            });
            self.unrevealed_ideas = pending;
        }
        // A price comes into view on the tick its item climbs the ladder; a
        // tick that reveals no item checks no price. @spec SIM-ENG-049, SIM-ENG-054, SIM-ENG-055
        for id in newly {
            self.record_price_of(id);
        }

        // ---- Phase 7: achievements, against the meadow and this tick's events ----
        self.check_achievements(
            crate::achievements::Look::Tick {
                day_closed: day > prev_day,
            },
            &mut events,
        );

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
                events.push(Event::KittenReturned {
                    name: kitten.name.clone(),
                });
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
                events.push(Event::KittenWanderedOff {
                    name: kitten.name.clone(),
                });
            }
        }

        // (d) Arrival check.
        // @spec SIM-KIT-003, SIM-KIT-005, SIM-KIT-007
        let eligible = self.free_slots() >= 1
            && self.stock(ResourceId::Catnip) + EPSILON >= content::ARRIVAL_MIN_CATNIP;
        if eligible {
            // Pity forces success without an RNG draw. @spec SIM-ENG-018
            let success = self.pity >= content::PITY_THRESHOLD || {
                let chance = self.arrival_chance();
                self.rng.chance(chance)
            };
            if success {
                let name = self.draw_name();
                // The trait draw follows the name draw in the fixed schedule.
                // @spec SIM-KIT-023, SIM-ENG-018
                let trait_id = crate::state::KittenTrait::ALL
                    [self.rng.index(crate::state::KittenTrait::ALL.len())];
                // The profile takes no state-RNG draw: it is read off the
                // name and generation. @spec SIM-KIT-031
                let profile = crate::profile::draw_profile(&name, self.generation);
                self.kittens.push(crate::state::Kitten {
                    name: name.clone(),
                    job: None,
                    location: Location::Home,
                    arrived_day: self.day(),
                    job_seq: None,
                    away_seq: None,
                    skills: [0.0; 8],
                    exp: 0.0,
                    rank: 0,
                    trait_id,
                    tinkering: None,
                    profile: Some(profile),
                });
                self.pity = 0;
                events.push(Event::KittenArrived { name });
            } else {
                self.pity = self.pity.saturating_add(1);
            }
        }

        // (e) Year-close hooks, appended to the fixed daily order: the
        // Cogworks convert capped raws into goods — overflow, not waste.
        // @spec SIM-ECO-039
        if self.day().is_multiple_of(content::DAYS_PER_YEAR) {
            self.cogworks_autocraft();
            // The Star Whale's visit ticks down a year; at zero it drifts calmly
            // on, never angry. @spec SIM-SKY-007
            if self.whale_visit_years > 0 {
                self.whale_visit_years -= 1;
            }
            // The Distant Dawn lights when the meadow reaches year 40000 with the
            // loom woven; once lit it never clears. @spec SIM-DREAM-007
            if !self.distant_dawn
                && self.year() >= content::DISTANT_DAWN_YEAR
                && self
                    .dream_looms
                    .contains(&crate::state::DreamLoomId::DistantDawn)
            {
                self.distant_dawn = true;
                events.push(Event::DistantDawn);
            }
        }

        // (f) Promise upkeep: each kept Promise draws 0.0005 dreamcorn/day; one
        // whose upkeep can't be paid goes quiet (slot held), waking when it can.
        // @spec SIM-UNI-009, SIM-UNI-010
        if !self.promises.is_empty() {
            let corn = ResourceId::Dreamcorns.ix();
            for i in 0..self.promises.len() {
                if self.resources[corn] + EPSILON >= content::PROMISE_UPKEEP_DREAMCORN {
                    self.resources[corn] =
                        (self.resources[corn] - content::PROMISE_UPKEEP_DREAMCORN).max(0.0);
                    if self.promises[i].quiet {
                        self.promises[i].quiet = false;
                        events.push(Event::PromiseResumed { index: i });
                    }
                } else if !self.promises[i].quiet {
                    self.promises[i].quiet = true;
                    events.push(Event::PromiseQuieted { index: i });
                }
            }
        }

        // (g) Adventures: track the goal at day close. Tidy Meadow counts
        // consecutive clean-mess days; a completed goal grows the reward pool
        // once and stays active (the twist lifts only at the following Move).
        // @spec SIM-ADV-004
        if let Some(a) = self.active_adventure {
            if a == AdventureId::TidyMeadow {
                if self.mess <= EPSILON {
                    self.tidy_progress = self.tidy_progress.saturating_add(1);
                } else {
                    self.tidy_progress = 0;
                }
            }
        }
        self.complete_adventure_if_met(false, events);
    }

    /// The Cogworks year-close auto-craft: 2% per Cogworks (max 50%) of each
    /// raw standing at ≥98% of its cap converts through its single-input
    /// recipe, once per year, unjammed, no RNG. Inputs leave at face value;
    /// output rides the craft ratio.
    // @spec SIM-ECO-039
    fn cogworks_autocraft(&mut self) {
        let cogworks = self.structures[StructureId::Cogworks.ix()];
        if cogworks == 0 {
            return;
        }
        let share =
            (content::COGWORKS_CONVERT_PER * cogworks as f64).min(content::COGWORKS_CONVERT_MAX);
        for (raw, craft) in content::COGWORKS_AUTOCRAFT {
            if !self.craft_unlocked(craft) {
                continue;
            }
            let cap = self.cap(raw);
            if !cap.is_finite() || cap <= 0.0 {
                continue;
            }
            let stock = self.stock(raw);
            if stock + EPSILON < content::COGWORKS_THRESHOLD * cap {
                continue;
            }
            let amount = share * stock;
            let per_batch = content::craft_def(craft).inputs[0].1;
            let batches = amount / per_batch;
            let output = content::craft_output(craft);
            self.resources[raw.ix()] = (stock - amount).max(0.0);
            let boosted = batches * (1.0 + self.craft_ratio_for(craft));
            let ix = output.ix();
            self.resources[ix] = (self.resources[ix] + boosted).min(self.cap(output));
        }
    }

    /// Phase 2: structures, then jobs, then converters, accumulating in fixed
    /// enum order so float results are deterministic. Implements the
    /// production stack's normative layer order — structures take only the ☆
    /// layers (base, per-source ratio, season); jobs additionally take mood,
    /// zoomies, and — material tier only — the global layers, dimmed by any
    /// energy deficit; mess multiplies catnip growth last. Returns the tick's
    /// mess accrual less its tidying (applied in phase 4).
    // @spec SIM-ENG-021, SIM-ECO-023, SIM-ECO-025, SIM-ECO-032, SIM-ECO-036
    pub(crate) fn produce(&mut self, ledger: &mut Option<&mut FlowLedger>) -> f64 {
        // A déjà-vu day pauses production entirely — a strange half-remembered
        // day where the meadow simply holds still (no growth, no mess).
        // @spec SIM-DREAM-004
        if self.deja_vu_until > self.ticks {
            return 0.0;
        }
        let season = self.season();
        let winter_eased = self.ideas.contains(&IdeaId::SeasonalAwareness);
        // Layer-6 inputs, frozen at phase start (before converters draw fuel).
        // Layers 7–8 (Elder Wisdom, the Long Sun) are ×1.0 until their waves.
        let (deficit, surplus) = self.energy_ratios();
        // Layers 6 (global), 7 (Elder Wisdom), and 8 (the Long Sun) combine
        // multiplicatively into one raw-tier multiplier whose whole non-base
        // part is dimmed uniformly by the energy deficit — the SIM-ENG-021 revision
        // that also closes the wave-4 gap where the Long Sun escaped the dimmer.
        // Each layer is 1.0 (skipped) on the common hot path.
        // @spec SIM-ENG-021, SIM-MOVE-007, SIM-SUN-004
        let global_layer = 1.0 + self.effect(crate::effects::Effect::GlobalProductionRatio);
        let long_sun_layer = if self.rites.is_empty() {
            1.0
        } else {
            1.0 + self.long_sun_bonus()
        };
        let wisdom_layer = if self.elder_wisdom > 0.0 || self.passed_down_wisdom > 0.0 {
            // The Distant Dawn era lifts the Passed-Down cap +100% → +400%.
            // @spec SIM-MOVE-009, SIM-DREAM-007
            let passed_cap = if self.distant_dawn { 4.0 } else { 1.0 };
            1.0 + crate::effects::limited_dr(
                0.01 * self.elder_wisdom,
                2.0 * self.nine_lives_ratio(),
            ) + crate::effects::limited_dr(0.01 * self.passed_down_wisdom, passed_cap)
        } else {
            1.0
        };
        let combined_layers = global_layer * wisdom_layer * long_sun_layer;
        let raw_mult = 1.0 + (combined_layers - 1.0) * deficit;
        // The outdoor channel (cosmic outposts) takes the global layers at a
        // reduced transfer, times its own per-source ratio. Computed lazily on
        // the first outdoor structure with raw output — free (no scan, no effect
        // sum) until then, the outpost-free hot path. @spec SIM-SKY-004, SIM-ENG-021
        let outdoor_raw_mult = 1.0 + (combined_layers - 1.0) * deficit * self.outdoor_transfer();
        let mut outdoor_ratio: Option<f64> = None;
        // Adventure twists on production: Nap Time halves job (kitten) output;
        // Tidy Meadow raises mess accrual, never the tidying. Both 1.0 with no
        // active adventure.
        // @spec SIM-ADV-002
        let job_twist = if self.active_adventure == Some(AdventureId::NapTime) {
            0.5
        } else {
            1.0
        };
        let tidy_twist = if self.active_adventure == Some(AdventureId::TidyMeadow) {
            content::TIDY_MESS_MULT
        } else {
            1.0
        };
        // What scales the mess made, and never the tidying: the twist, and
        // the mess-accrual effects (Tidy Paws). @spec SIM-ECO-052
        let tidy_mult = tidy_twist * (1.0 + self.effect(crate::effects::Effect::MessAccrualRatio));
        // The mystic tier's own multiplier on Unicorn Meadow output — NOT a
        // global layer. Only Unicorn Meadows produce unicorns, so the effect
        // scan is skipped entirely when none are built. @spec SIM-UNI-002
        let unicorns_mult = if self.structures[StructureId::UnicornMeadow.ix()] > 0 {
            1.0 + self.effect(crate::effects::Effect::UnicornsRatio)
        } else {
            1.0
        };
        let mess_catnip = self.mess_catnip_factor();
        // Adventure rewards (Quiet Retreat / Long Winter seasonal catnip; Nap
        // Time master-skill multiplier) flow through the effect-key system and
        // are LimitedDR-capped. A single array scan gates both, so the hot path
        // (no rewards earned) draws nothing: the seasonal multiply is skipped
        // and the master multiplier stays 1.0 with no effect() sum.
        // @spec SIM-ADV-005
        let season_reward = self.adventure_rewards.iter().any(|&x| x != 0.0);
        let master_mult = if season_reward {
            1.0 + self.effect(crate::effects::Effect::MasterSkillMultiplier)
        } else {
            1.0
        };

        // ---- Structures (☆ layers only) ----
        for s in StructureId::ALL {
            let count = self.structures[s.ix()];
            if count == 0 {
                continue;
            }
            let def = content::structure_def(s);
            for (r, amount) in def.productions {
                let mut flow = amount * count as f64;
                flow *= 1.0
                    + self.effect(crate::effects::Effect::ResourceRatio(*r))
                    + self.effect(crate::effects::Effect::StructureRatio(s));
                if def.seasonal {
                    // @spec SIM-ECO-019
                    flow *= content::season_patch_modifier(season, winter_eased);
                }
                if def.surplus_scaled {
                    flow *= surplus;
                }
                // SIM-ENG-021 REVISED: the global layers (6+8) now reach raw-tier
                // STRUCTURE production, not just jobs — except a structure that is
                // itself a global-production source never amplifies its own
                // faucet (the Star Basin's stardust). Mystic/transient/luxury
                // flows skip the global layers; unicorns take their own mystic
                // multiplier instead. @spec SIM-ENG-021, SIM-SUN-004, SIM-UNI-002
                if content::tier(*r) == content::Tier::Raw {
                    if def.outdoor {
                        // The outdoor channel: the global layers at a reduced
                        // transfer, times the outdoor per-source ratio (summed
                        // once, on demand). @spec SIM-SKY-004, SIM-ENG-021
                        let oratio = *outdoor_ratio.get_or_insert_with(|| {
                            1.0 + self.effect(crate::effects::Effect::OutdoorProductionRatio)
                        });
                        flow *= outdoor_raw_mult * oratio;
                    } else if def.global_ratio == 0.0 {
                        flow *= raw_mult;
                    }
                } else if *r == ResourceId::Unicorns {
                    flow *= unicorns_mult;
                }
                if *r == ResourceId::Catnip {
                    // @spec SIM-ECO-038
                    flow *= mess_catnip;
                }
                // Per-season, per-resource Adventure reward (Quiet Retreat +spring
                // catnip, Long Winter +winter catnip). @spec SIM-ADV-005
                if season_reward {
                    flow *=
                        1.0 + self.effect(crate::effects::Effect::SeasonResourceRatio(season, *r));
                }
                self.resources[r.ix()] += flow;
                credit(ledger, FlowSource::Structure(s), *r, flow);
            }
        }

        // Refinement-granted extra outputs on non-converter structures (the
        // Story Press). Converter hosts (the Hearth's Bellows and Ember
        // Beds) add theirs throttle-scaled in run_converters.
        // @spec SIM-CUR-010
        for (s, r) in content::STRUCTURE_OUTPUT_ADD_TARGETS {
            if content::structure_def(s).converter.is_some() {
                continue;
            }
            let count = self.structures[s.ix()];
            if count == 0 {
                continue;
            }
            let add = self.effect(crate::effects::Effect::StructureOutputAdd(s, r));
            if add != 0.0 {
                self.resources[r.ix()] += add * count as f64;
                credit(ledger, FlowSource::Structure(s), r, add * count as f64);
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
            let Some((r, per_worker)) = content::job_def(j).yield_per_tick else {
                continue; // the Tinkerer crafts in the background (below)
            };
            // Production resolves per kitten in roster order: each Home
            // worker contributes base × (1 + job-ratio effects) × its skill
            // tier × the Elder's own-job leadership; the aggregate layers
            // (per-resource ratio, mood, zoomies, global, mess) are identical
            // per kitten and stay aggregate.
            // @spec SIM-KIT-020, SIM-KIT-022
            let job_ratio = 1.0 + self.effect(crate::effects::Effect::JobRatio(j));
            let mut base_sum = 0.0;
            for (ix, k) in self.kittens.iter().enumerate() {
                if k.location != Location::Home || k.job != Some(j) {
                    continue;
                }
                let mut per = per_worker
                    * job_ratio
                    * (1.0 + content::skill_tier_bonus_scaled(k.skills[j as usize], master_mult));
                if self.elder == Some(ix) {
                    per *= ((k.rank as f64 + 1.0) / content::ELDER_RANK_DIVISOR).max(1.0);
                }
                base_sum += per;
            }
            if base_sum == 0.0 {
                continue;
            }
            let mut flow = base_sum;
            flow *= 1.0 + self.effect(crate::effects::Effect::ResourceRatio(r));
            flow *= mood * zoomies;
            if content::tier(r) == content::Tier::Raw {
                // Layers 6 (global), 7 (Elder Wisdom), 8 (the Long Sun) all
                // reach raw jobs. @spec SIM-ENG-021, SIM-MOVE-007, SIM-SUN-004
                flow *= raw_mult;
            }
            if r == ResourceId::Catnip {
                flow *= mess_catnip;
            }
            // Per-season, per-resource Adventure reward (Quiet Retreat +spring
            // catnip, Long Winter +winter catnip) reaches job output too.
            // @spec SIM-ADV-005
            if season_reward {
                flow *= 1.0 + self.effect(crate::effects::Effect::SeasonResourceRatio(season, r));
            }
            // Nap Time halves every job's output (structures are untouched).
            // @spec SIM-ADV-002
            flow *= job_twist;
            self.resources[r.ix()] += flow;
            credit(ledger, FlowSource::Job(j), r, flow);
        }

        // ---- Skills: employed Home kittens learn; other skills fade ----
        // Grumpiness slows paws, never learning; loafing and Away kittens
        // neither accrue nor decay.
        // @spec SIM-KIT-019
        let xp_gain =
            content::SKILL_XP_PER_TICK * (1.0 + self.effect(crate::effects::Effect::SkillXpRatio));
        let decay_removed = self
            .refinements
            .contains(&crate::state::RefinementId::GroupLessons);
        for k in &mut self.kittens {
            if k.location != Location::Home {
                continue;
            }
            let Some(job) = k.job else { continue };
            k.skills[job as usize] += xp_gain;
            k.exp += xp_gain;
            if !decay_removed {
                for j in JobId::ALL {
                    if j != job {
                        let s = &mut k.skills[j as usize];
                        *s = (*s - content::SKILL_DECAY_PER_TICK).max(0.0);
                    }
                }
            }
        }

        // ---- Converters (throttled), then Tinkerers, accruing mess ----
        let (made, tidied) = self.run_converters(ledger);
        self.run_tinkerers(ledger);
        made * tidy_mult - tidied
    }

    /// The mess after a tick whose accrual less tidying is `change`: less the
    /// tick's dissipation, never below zero, so tidying banks nothing.
    // @spec SIM-ECO-037
    pub(crate) fn next_mess(&self, change: f64) -> f64 {
        (self.mess + change - content::MESS_DISSIPATION * self.mess).max(0.0)
    }

    /// Bound Home Tinkerers craft their recipes in the background, in roster
    /// order: 0.0017 batches/tick × (1 + tinkerer-speed effects) × rank-tier
    /// effectiveness, inputs drawn through the engine throttle at face value,
    /// output riding the craft ratio. No mood or zoomies factor. An Away
    /// Tinkerer's recipe idles; its binding (and seat) persist.
    // @spec SIM-ECO-043, SIM-KIT-026
    fn run_tinkerers(&mut self, ledger: &mut Option<&mut FlowLedger>) {
        let speed = 1.0 + self.effect(crate::effects::Effect::TinkererSpeed);
        for ix in 0..self.kittens.len() {
            let k = &self.kittens[ix];
            if k.location != Location::Home || k.job != Some(JobId::Tinkerer) {
                continue;
            }
            let Some(recipe) = k.tinkering else { continue };
            let def = content::craft_def(recipe);
            let effectiveness = (1.0
                - content::TINKER_TIER_PENALTY
                    * (def.tier as f64 - (k.rank as f64 + 1.0)).max(0.0))
            .max(content::TINKER_EFFECTIVENESS_FLOOR);
            let rate = content::TINKER_BATCHES_PER_TICK * speed * effectiveness;
            let demands: Vec<(ResourceId, f64)> = def
                .inputs
                .iter()
                .map(|(r, amount)| (*r, amount * rate))
                .collect();
            let fraction = self.throttle_fraction(&demands);
            if fraction <= 0.0 {
                continue; // a starved Tinkerer idles quietly, still bound
            }
            let src = FlowSource::Craft(recipe);
            for (r, demand) in &demands {
                let rix = r.ix();
                let before = self.resources[rix];
                self.resources[rix] = (before - demand * fraction).max(0.0);
                credit(ledger, src, *r, self.resources[rix] - before);
            }
            let output = content::craft_output(recipe);
            let boosted = rate * fraction * (1.0 + self.craft_ratio_for(recipe));
            let oix = output.ix();
            let before = self.resources[oix];
            self.resources[oix] = (before + boosted).min(self.cap(output));
            credit(ledger, src, output, self.resources[oix] - before);
        }
    }

    /// Run every converter through the engine throttle and total the tick's
    /// mess, as the mess made and the mess tidied away: converters make or
    /// tidy in proportion to their throughput, everything else per copy.
    // @spec SIM-ECO-029, SIM-ECO-050
    fn run_converters(&mut self, ledger: &mut Option<&mut FlowLedger>) -> (f64, f64) {
        let (mut made, mut tidied) = (0.0, 0.0);
        let mut tally = |per_copy: f64, copies: f64| {
            if per_copy > 0.0 {
                made += per_copy * copies;
            } else {
                tidied -= per_copy * copies;
            }
        };
        for s in StructureId::ALL {
            let count = self.structures[s.ix()];
            if count == 0 {
                continue;
            }
            let def = content::structure_def(s);
            // Tidy effects can flip a structure from making mess to tidying
            // it (the Tidy-Up Rota). @spec SIM-ECO-050, SIM-CUR-010
            let per_copy_mess =
                def.mess + self.effect(crate::effects::Effect::StructureMessDelta(s));
            let Some(converter) = def.converter else {
                tally(per_copy_mess, count as f64);
                continue;
            };
            let demands: Vec<(ResourceId, f64)> = converter
                .inputs
                .iter()
                .map(|(r, d)| (*r, d * count as f64))
                .collect();
            let fraction = self.throttle_fraction(&demands);
            tally(per_copy_mess, count as f64 * fraction);
            if fraction <= 0.0 {
                continue; // a starved converter idles quietly
            }
            let src = FlowSource::Structure(s);
            for (r, demand) in &demands {
                let ix = r.ix();
                let before = self.resources[ix];
                self.resources[ix] = (before - demand * fraction).max(0.0);
                credit(ledger, src, *r, self.resources[ix] - before);
            }
            for (r, out) in converter.outputs {
                self.resources[r.ix()] += out * count as f64 * fraction;
                credit(ledger, src, *r, out * count as f64 * fraction);
            }
            // Refinement-granted extra outputs ride the same throttle (the
            // Bellows and Ember Beds stoke the Hearth). @spec SIM-CUR-010
            for (target, r) in content::STRUCTURE_OUTPUT_ADD_TARGETS {
                if target != s {
                    continue;
                }
                let add = self.effect(crate::effects::Effect::StructureOutputAdd(s, r));
                if add != 0.0 {
                    self.resources[r.ix()] += add * count as f64 * fraction;
                    credit(ledger, src, r, add * count as f64 * fraction);
                }
            }
            // The Treat Bakery's outputs ride the pep *cap* (mpr), not a
            // recipe table. @spec SIM-ECO-030
            if s == StructureId::TreatBakery {
                let mpr = self.cap(ResourceId::Pep) * content::BAKERY_MPR_PER_PEP_CAP;
                let feathers = content::BAKERY_FEATHERS_MPR * mpr * count as f64 * fraction;
                let bells = content::BAKERY_BELLS_MPR * mpr * count as f64 * fraction;
                self.resources[ResourceId::Feathers.ix()] += feathers;
                self.resources[ResourceId::Bells.ix()] += bells;
                credit(ledger, src, ResourceId::Feathers, feathers);
                credit(ledger, src, ResourceId::Bells, bells);
            }
        }
        (made, tidied)
    }

    /// Pooled consumption; returns whether this tick was a shortfall. Food
    /// first, then the luxury drain — which floors at zero and never
    /// shortfalls: an empty shelf just drops its happiness bonus.
    // @spec SIM-ECO-020, SIM-ECO-021, SIM-ECO-026
    pub(crate) fn consume(&mut self, ledger: &mut Option<&mut FlowLedger>) -> bool {
        // A déjà-vu day pauses consumption too, so the dreamy pause never causes
        // a shortfall (Tenet 1). @spec SIM-DREAM-004
        if self.deja_vu_until > self.ticks {
            return false;
        }
        let home = self
            .kittens
            .iter()
            .filter(|k| k.location == Location::Home)
            .count();
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
        credit(
            ledger,
            FlowSource::Kittens,
            ResourceId::Catnip,
            self.resources[ix] - stock,
        );

        // Luxury drain, per Home kitten, eased by demand effects and raised
        // by an active jamboree (celebrations consume), floored at zero.
        // An empty shelf drains nothing — skip the effect sum entirely.
        // @spec SIM-ECO-026, SIM-KIT-025
        if content::LUXURY_DRAINS
            .iter()
            .any(|(r, _)| self.resources[r.ix()] > 0.0)
        {
            let jamboree = if self.jamboree_active() {
                content::JAMBOREE_LUXURY_DEMAND
            } else {
                0.0
            };
            let scale =
                (1.0 + self.effect(crate::effects::Effect::LuxuryDemandRatio) + jamboree).max(0.0);
            for (r, per_kitten) in content::LUXURY_DRAINS {
                let ix = r.ix();
                let drain = per_kitten * home as f64 * scale;
                let before = self.resources[ix];
                self.resources[ix] = (before - drain).max(0.0);
                credit(ledger, FlowSource::Kittens, r, self.resources[ix] - before);
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

        // A jamboree ends at its expiry the same way (checked every tick);
        // the window resets to none and a fresh one may be hosted after.
        // @spec SIM-KIT-025
        if self.jamboree_until != 0 && self.ticks >= self.jamboree_until {
            self.jamboree_until = 0;
            events.push(Event::JamboreeEnded);
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
            events.push(Event::ZoomiesStarted {
                until_tick: self.zoomies_until,
            });
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

        // Mystic day-boundary rolls, appended AFTER the sky rolls: a prism
        // descent, then a rainbow shower. Both draw only when a Rainbow Arch
        // structure or an Arch upgrade exists (else the chances are 0), keeping
        // the arch-free hot path free of effect scans.
        // @spec SIM-UNI-006, SIM-UNI-005
        if self.structures[StructureId::RainbowArch.ix()] > 0 || !self.arches_bought.is_empty() {
            let descent = self.prism_descent_chance();
            if descent > 0.0 && self.rng.chance(descent) {
                self.resources[ResourceId::Prisms.ix()] += 1.0;
                events.push(Event::PrismDescended);
            }
            let shower = self.rainbow_shower_chance();
            if shower > 0.0 && self.rng.chance(shower) {
                self.resources[ResourceId::Rainbows.ix()] += content::RAINBOW_SHOWER_AMOUNT;
            }
        }

        // Déjà-vu roll — appended AFTER the mystic rolls, at season boundaries
        // only (the SIM-ENG-018 schedule extension). Guarded so the chest/loom-free
        // hot path draws nothing. On a hit the day pauses production and
        // consumption and gathers reverie (a deterministic RNG draw).
        // @spec SIM-DREAM-004, SIM-ENG-018
        if self.day().is_multiple_of(content::DAYS_PER_SEASON)
            && (self.structures[StructureId::MemoryChest.ix()] > 0 || !self.dream_looms.is_empty())
        {
            let chance = self.effect(crate::effects::Effect::DejaVuChance);
            if chance > 0.0 && self.rng.chance(chance) {
                self.deja_vu_until = self.ticks + content::TICKS_PER_DAY;
                let amount = (content::DEJA_VU_REVERIE_BASE
                    + self.rng.uniform() * content::DEJA_VU_REVERIE_SPREAD)
                    * (1.0 + self.effect(crate::effects::Effect::ReverieYield));
                self.resources[ResourceId::Reverie.ix()] += amount;
                events.push(Event::DejaVuDay);
            }
        }
    }

    /// Per-tick dreaming: the Memory Arch upgrade turns the clowder's Rainbow
    /// Arch *structures* into a dreamcorn engine — dream-drift accrues per tick
    /// as 0.000001 × Rainbow Arch structures × (1 + dream-boost effects),
    /// throttled ×0.25 while any dreamcorn already dreams; at ≥ 1 a prism naps
    /// into a dreamcorn — never the last prism. A Sky Arch adds a steady prism
    /// drift.
    // @spec SIM-UNI-007, SIM-UNI-008
    fn dream_step(&mut self, ledger: &mut Option<&mut FlowLedger>) {
        use crate::state::ArchId;
        if self.arches_bought.is_empty() {
            return; // no Sky/Memory Arch: no drift, the common hot-path case
        }
        if self.arches_bought.contains(&ArchId::SkyArch) {
            self.resources[ResourceId::Prisms.ix()] += content::SKY_ARCH_PRISM_DRIFT;
            credit(
                ledger,
                FlowSource::Arch(ArchId::SkyArch),
                ResourceId::Prisms,
                content::SKY_ARCH_PRISM_DRIFT,
            );
        }
        // The Memory Arch upgrade ENABLES the engine; the drift RATE scales with
        // the buildable Rainbow Arch structure count, so the engine grows as the
        // player raises more arches (not a flat once-bought trickle).
        if !self.arches_bought.contains(&ArchId::MemoryArch) {
            return;
        }
        let arches = self.structures[StructureId::RainbowArch.ix()] as f64;
        let throttle = if self.stock(ResourceId::Dreamcorns) > EPSILON {
            content::DREAM_DRIFT_THROTTLE
        } else {
            1.0
        };
        let boost = 1.0 + self.effect(crate::effects::Effect::DreamBoost);
        self.dream_drift += content::DREAM_DRIFT_PER_ARCH * arches * boost * throttle;
        // A prism must stay awake to keep the dream going (the cozy floor).
        while self.dream_drift >= 1.0 && self.stock(ResourceId::Prisms) >= 2.0 - EPSILON {
            self.dream_drift -= 1.0;
            self.resources[ResourceId::Prisms.ix()] -= 1.0;
            self.resources[ResourceId::Dreamcorns.ix()] += 1.0;
            let src = FlowSource::Arch(ArchId::MemoryArch);
            credit(ledger, src, ResourceId::Prisms, -1.0);
            credit(ledger, src, ResourceId::Dreamcorns, 1.0);
        }
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
        let event_chance = content::SKY_EVENT_CHANCE_PER_LEDGE
            * ledges as f64
            * (1.0 + self.effect(crate::effects::Effect::SkyEventChanceRatio));
        if !self.rng.chance(event_chance) {
            return;
        }
        let observe_chance = content::SKY_AUTO_OBSERVE_PER_LEDGE
            * ledges as f64
            * (1.0 + self.effect(crate::effects::Effect::AutoObserveChanceRatio));
        if self.rng.chance(observe_chance) {
            // Someone happened to be looking up.
            let seen = self.grant_sky_observation(true);
            events.push(seen);
        } else {
            // Observable for a game year from this day close.
            self.pending_sky_event = Some(self.ticks + content::SKY_WINDOW_TICKS);
            events.push(Event::SkyEventVisible);
        }
    }

    /// The yields of observing a sky event: curiosity scaled by its
    /// per-source ratios, plus a skymap, both clamped to their caps.
    // @spec SIM-ENG-025
    /// Returns the observation's event: how far each stock rose once its cap
    /// applied, and whether it was made by chance. @spec SIM-ENG-071
    pub(crate) fn grant_sky_observation(&mut self, by_chance: bool) -> Event {
        let before = self.resources;
        let curiosity = content::SKY_OBSERVE_CURIOSITY
            * (1.0 + self.effect(crate::effects::Effect::ResourceRatio(ResourceId::Curiosity)));
        let ix = ResourceId::Curiosity.ix();
        self.resources[ix] = (self.resources[ix] + curiosity).min(self.cap(ResourceId::Curiosity));
        let ix = ResourceId::Skymaps.ix();
        self.resources[ix] =
            (self.resources[ix] + content::SKY_OBSERVE_SKYMAPS).min(self.cap(ResourceId::Skymaps));
        let rose = |r: ResourceId| (self.resources[r.ix()] - before[r.ix()]).max(0.0);
        Event::SkyObserved {
            curiosity: rose(ResourceId::Curiosity),
            skymaps: rose(ResourceId::Skymaps),
            by_chance,
        }
    }

    /// Draw a name from the pool without replacement, cycling with Roman
    /// numeral suffixes once exhausted. The remaining pool is derived from the
    /// roster — no separate stored state — and every rostered name is taken,
    /// ignoring case, whether or not the pool still holds it, so a pool that
    /// changes between versions renames nobody and repeats nobody.
    // @spec SIM-KIT-002, SIM-KIT-030
    fn draw_name(&mut self) -> String {
        let taken: std::collections::BTreeSet<String> =
            self.kittens.iter().map(|k| k.name.to_lowercase()).collect();
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
                .filter(|name| !taken.contains(&name.to_lowercase()))
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
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
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
