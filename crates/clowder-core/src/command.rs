use crate::content;
use crate::events::Event;
use crate::state::{
    AdventureId, ArchId, ArchivedKitten, ColonyId, CraftId, DreamLoomId, GameState, IdeaId, JobId,
    Kitten, Location, Promise, RefinementId, ResourceId, RiteId, RuleId, SiteId, StructureId,
    TraditionId, EPSILON, RESOURCE_COUNT, STRUCTURE_COUNT,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Command {
    GatherCatnip,
    Forage,
    /// Idea-gated pebble foraging. @spec SIM-ECO-027
    Rummage,
    Build(StructureId),
    AssignJob(JobId),
    UnassignJob(JobId),
    ResearchIdea(IdeaId),
    /// One batch of a recipe. @spec SIM-ECO-033
    Craft(CraftId),
    /// Bank a pending sky event's yields before its window closes.
    /// @spec SIM-ENG-025
    ObserveSky,
    SetSpeed(f64),
    /// Send squads beyond the meadow: 100 pep each, instant loot.
    /// @spec SIM-EXP-001
    Pounce(u32),
    /// 800 pep + 1500 charm + 40 quills buy 100 days of celebration.
    /// @spec SIM-KIT-025
    HostJamboree,
    /// Spend a kitten's exp (plus shinies) to raise its rank.
    /// @spec SIM-KIT-021
    Promote(usize),
    /// Name a kitten the Elder (replacing any prior one, freely).
    /// @spec SIM-KIT-022
    AppointElder(usize),
    // @spec SIM-CUR-010
    BuyRefinement(RefinementId),
    // @spec SIM-CUR-011
    AdoptRule(RuleId),
    /// Bind the newest unbound Home Tinkerer to a recipe (seat permitting).
    /// @spec SIM-ECO-043
    AssignTinkerer(CraftId),
    /// Release the most recently assigned bound holder of a recipe.
    /// @spec SIM-ECO-043
    UnassignTinkerer(CraftId),
    // ---- Wave 4: ceremonies (SIM-SUN) ----
    /// Offer warmth up into devotion. @spec SIM-SUN-001
    Bask,
    /// Settle devotion into permanent serenity. @spec SIM-SUN-002
    Reverie,
    /// Buy (or restack) a Sunbeam Rite. @spec SIM-SUN-005
    BuyRite(RiteId),
    /// Spend serenity to raise the Enlightenment tier. @spec SIM-SUN-008
    Enlighten,
    // ---- Wave 4: mystic (SIM-UNI) ----
    /// Hold a Unicorn Parade: unicorns → arch-scaled rainbows. @spec SIM-UNI-001
    HoldParade,
    /// Buy a Rainbow Arch upgrade. @spec SIM-UNI-005
    BuyArch(ArchId),
    /// Refine 25 prisms → 1 starshard. @spec SIM-UNI-003
    RefineStarshard,
    /// Refine 25 starshards → 1 echo. @spec SIM-UNI-003
    RefineEcho,
    /// Condense 10000 rainbows → 1 Jar of Purrs. @spec SIM-UNI-004
    CondenseJar,
    /// Make a Promise to the Night Court. @spec SIM-UNI-009
    MakePromise,
    // ---- Wave 4: colonies (SIM-TRADE) ----
    /// Send `count` caravans to a colony. @spec SIM-TRADE-001
    Trade(ColonyId, u32),
    /// Deepen a Pen Pal correspondence. @spec SIM-TRADE-006
    SendPenPal(ColonyId),
    /// Scout for the next colony. @spec SIM-TRADE-007
    ScoutPicnic,
    // ---- Wave 5: prestige / the Great Meadow Move (SIM-MOVE) ----
    /// Relocate the whole clowder to a fresh, brighter meadow. @spec SIM-MOVE-001
    GreatMeadowMove,
    /// Convert all Elder Wisdom into Passed-Down Wisdom (one-way). @spec SIM-MOVE-009
    PassItDown,
    /// Buy a permanent Tradition with Elder Wisdom. @spec SIM-MOVE-010
    BuyTradition(TraditionId),
    // ---- Wave 5: adventures (SIM-ADV) ----
    /// Arm (or, with None, disarm) an Adventure for the next Move. @spec SIM-ADV-001
    DeclareAdventure(Option<AdventureId>),
    // ---- Wave 6: cosmic (SIM-SKY) / dreamtime (SIM-DREAM) ----
    /// Reach the next cosmic site, spending its mission fuel. @spec SIM-SKY-001
    LaunchExpedition(SiteId),
    /// Call a Star Whale to visit (Sky Dock + Skyward Song). @spec SIM-SKY-006
    SummonStarWhale,
    /// Feed the visiting whale a dreamcorn lullaby. @spec SIM-SKY-007
    FeedWhale,
    /// Skip the meadow forward `years` game-years. @spec SIM-DREAM-001
    DeepNap(u32),
    /// Weave a Dream Loom upgrade with reverie. @spec SIM-DREAM-006
    BuyDreamLoom(DreamLoomId),
}

/// Why a command was rejected. Listed in precedence order: when several
/// reasons apply, the earliest variant is reported.
// @spec SIM-ENG-006
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    UnknownId,
    Locked,
    AlreadyResearched,
    NoIdleKittens,
    /// Every Tinkerer seat (Toy Workshops) is taken. @spec SIM-ENG-006
    NoSeats,
    NoWorkersAssigned,
    StorageFull,
    InsufficientResources {
        missing: Vec<(ResourceId, f64)>,
    },
}

impl GameState {
    /// Apply a command between ticks. All-or-nothing: a rejected command has
    /// mutated nothing, RNG included. An ACCEPTED command may draw randomness
    /// (Pounce, Trade, ScoutPicnic, DeepNap do); a REJECTED command never draws.
    /// Determinism holds because every draw goes through the single ordered RNG.
    // @spec SIM-ENG-005
    // @spec SIM-ENG-028
    pub fn apply(&mut self, cmd: Command) -> Result<Vec<Event>, CommandError> {
        let events = self.apply_inner(cmd)?;
        // An applied command leaves every holding and every stated price
        // recorded; a refused one returned above and revealed nothing.
        // @spec SIM-ENG-050, SIM-ENG-054
        self.record_reveals();
        let tick = self.ticks;
        for event in &events {
            self.journal.record(tick, event);
        }
        Ok(events)
    }

    /// The price of one unit of a count-taking command whose price is fixed
    /// per unit — a Pounce (per squad) or a Trade (per caravan, the Star
    /// Whale's own exchange included) — regardless of the count the command
    /// happens to carry. Answers nothing for a count-taking command priced
    /// otherwise (a Deep Nap's price escalates with its length), for a
    /// command that takes no count, and for a command still locked, decided
    /// by the same checks `apply` makes before any resource is costed — so a
    /// caller never states a price for something not yet reached.
    // @spec SIM-ENG-047
    pub fn unit_price(&self, cmd: &Command) -> Option<Vec<(ResourceId, f64)>> {
        match *cmd {
            Command::Pounce(_) => {
                if !self.ideas.contains(&IdeaId::PouncingPractice) {
                    return None;
                }
                Some(vec![(ResourceId::Pep, content::POUNCE_PEP_PER_SQUAD)])
            }
            Command::Trade(colony, _) => {
                if colony == ColonyId::StarWhales {
                    if self.whale_visit_years == 0 {
                        return None;
                    }
                    return Some(vec![(
                        ResourceId::Hushfelt,
                        content::WHALE_HUSHFELT_PER_CARAVAN,
                    )]);
                }
                if !self.discovered.contains(&colony) {
                    return None;
                }
                let def = content::colony_def(colony);
                let mut price = vec![
                    (ResourceId::Pep, content::TRADE_PEP_PER_CARAVAN),
                    (ResourceId::Shinies, content::TRADE_SHINIES_PER_CARAVAN),
                ];
                // A colony may buy in Pep or Shinies itself, which the trade
                // fee already names — merge rather than list the same
                // component twice.
                match price.iter_mut().find(|(r, _)| *r == def.buy.0) {
                    Some((_, amount)) => *amount += def.buy.1,
                    None => price.push(def.buy),
                }
                Some(price)
            }
            _ => None,
        }
    }

    /// The command dispatch itself, kept apart from journaling so a rejected
    /// command (an `Err` here) never reaches `apply`'s recording step.
    // @spec SIM-ENG-005
    fn apply_inner(&mut self, cmd: Command) -> Result<Vec<Event>, CommandError> {
        match cmd {
            Command::GatherCatnip => self.gather(),
            Command::Forage => self.forage(),
            Command::Rummage => self.rummage(),
            Command::Build(s) => self.build(s),
            Command::AssignJob(j) => self.assign(j),
            Command::UnassignJob(j) => self.unassign(j),
            Command::ResearchIdea(i) => self.research(i),
            Command::Craft(c) => self.craft(c),
            Command::ObserveSky => self.observe_sky(),
            Command::SetSpeed(v) => self.set_speed(v),
            Command::Pounce(squads) => self.pounce(squads),
            Command::HostJamboree => self.host_jamboree(),
            Command::Promote(ix) => self.promote(ix),
            Command::AppointElder(ix) => self.appoint_elder(ix),
            Command::BuyRefinement(r) => self.buy_refinement(r),
            Command::AdoptRule(r) => self.adopt_rule(r),
            Command::AssignTinkerer(c) => self.assign_tinkerer(c),
            Command::UnassignTinkerer(c) => self.unassign_tinkerer(c),
            Command::Bask => self.bask(),
            Command::Reverie => self.reverie(),
            Command::BuyRite(r) => self.buy_rite(r),
            Command::Enlighten => self.enlighten(),
            Command::HoldParade => self.hold_parade(),
            Command::BuyArch(a) => self.buy_arch(a),
            Command::RefineStarshard => self.refine_starshard(),
            Command::RefineEcho => self.refine_echo(),
            Command::CondenseJar => self.condense_jar(),
            Command::MakePromise => self.make_promise(),
            Command::Trade(c, n) => self.trade(c, n),
            Command::SendPenPal(c) => self.send_pen_pal(c),
            Command::ScoutPicnic => self.scout_picnic(),
            Command::GreatMeadowMove => self.great_meadow_move(),
            Command::PassItDown => self.pass_it_down(),
            Command::BuyTradition(t) => self.buy_tradition(t),
            Command::DeclareAdventure(id) => self.declare_adventure(id),
            Command::LaunchExpedition(site) => self.launch_expedition(site),
            Command::SummonStarWhale => self.summon_star_whale(),
            Command::FeedWhale => self.feed_whale(),
            Command::DeepNap(years) => self.deep_nap(years),
            Command::BuyDreamLoom(id) => self.buy_dream_loom(id),
        }
    }

    /// Always legal; clamps silently. Speed is boundary cadence, not a rule.
    // @spec SIM-ENG-024
    fn set_speed(&mut self, v: f64) -> Result<Vec<Event>, CommandError> {
        let clamped = if v.is_finite() {
            v.clamp(crate::state::SPEED_MIN, crate::state::SPEED_MAX)
        } else {
            1.0
        };
        self.speed = clamped;
        Ok(vec![Event::SpeedChanged { speed: clamped }])
    }

    // @spec SIM-ECO-004, SIM-ECO-006
    fn gather(&mut self) -> Result<Vec<Event>, CommandError> {
        let cap = self.cap(ResourceId::Catnip);
        if self.stock(ResourceId::Catnip) + EPSILON >= cap {
            return Err(CommandError::StorageFull);
        }
        let amount = if self.ideas.contains(&IdeaId::SelectiveHearing) {
            content::GATHER_YIELD * 2.0
        } else {
            content::GATHER_YIELD
        };
        let ix = ResourceId::Catnip.ix();
        self.resources[ix] = (self.resources[ix] + amount).min(cap);
        Ok(Vec::new())
    }

    // @spec SIM-ECO-005, SIM-ECO-006
    fn forage(&mut self) -> Result<Vec<Event>, CommandError> {
        let twig_cap = self.cap(ResourceId::Twigs);
        if self.stock(ResourceId::Twigs) + EPSILON >= twig_cap {
            return Err(CommandError::StorageFull);
        }
        let catnip = self.stock(ResourceId::Catnip);
        if catnip + EPSILON < content::FORAGE_COST {
            return Err(CommandError::InsufficientResources {
                missing: vec![(ResourceId::Catnip, content::FORAGE_COST - catnip)],
            });
        }
        self.resources[ResourceId::Catnip.ix()] = (catnip - content::FORAGE_COST).max(0.0);
        let twigs = ResourceId::Twigs.ix();
        self.resources[twigs] = (self.resources[twigs] + content::FORAGE_YIELD).min(twig_cap);
        Ok(Vec::new())
    }

    /// Rummage the couch: Locked before its idea, then +0.5 pebbles under the
    /// same StorageFull rule as every manual action.
    // @spec SIM-ECO-027
    fn rummage(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::CouchSpelunking) {
            return Err(CommandError::Locked);
        }
        let cap = self.cap(ResourceId::Pebbles);
        if self.stock(ResourceId::Pebbles) + EPSILON >= cap {
            return Err(CommandError::StorageFull);
        }
        let ix = ResourceId::Pebbles.ix();
        self.resources[ix] = (self.resources[ix] + content::RUMMAGE_YIELD).min(cap);
        Ok(Vec::new())
    }

    /// One craft batch: all-or-nothing inputs, craft-ratio-boosted output.
    // @spec SIM-ECO-033
    fn craft(&mut self, c: CraftId) -> Result<Vec<Event>, CommandError> {
        if !self.craft_unlocked(c) {
            return Err(CommandError::Locked);
        }
        let def = content::craft_def(c);
        self.spend(def.inputs)?;
        let output = content::craft_output(c);
        let batch = 1.0 * (1.0 + self.craft_ratio_for(c));
        let ix = output.ix();
        self.resources[ix] = (self.resources[ix] + batch).min(self.cap(output));
        Ok(Vec::new())
    }

    /// Observe a pending, unexpired sky event for its yields; anything else
    /// (nothing pending, or a faded event) is Locked and a perfect no-op.
    // @spec SIM-ENG-025
    fn observe_sky(&mut self) -> Result<Vec<Event>, CommandError> {
        match self.pending_sky_event {
            Some(expiry) if self.ticks < expiry => {
                self.grant_sky_observation();
                self.pending_sky_event = None;
                Ok(Vec::new())
            }
            _ => Err(CommandError::Locked),
        }
    }

    // @spec SIM-ECO-007
    fn build(&mut self, s: StructureId) -> Result<Vec<Event>, CommandError> {
        if !self.structure_unlocked(s) {
            return Err(CommandError::Locked);
        }
        let costs = self.next_cost(s);
        self.spend(&costs)?;
        self.structures[s.ix()] += 1;
        Ok(vec![Event::StructureBuilt {
            id: s,
            count: self.structures[s.ix()],
        }])
    }

    // @spec SIM-KIT-008
    fn assign(&mut self, j: JobId) -> Result<Vec<Event>, CommandError> {
        if !self.job_unlocked(j) {
            return Err(CommandError::Locked);
        }
        // Most recently arrived idle Home kitten: the roster is append-ordered
        // by arrival, so scan from the back.
        let Some(ix) = self
            .kittens
            .iter()
            .rposition(|k| k.location == Location::Home && k.job.is_none())
        else {
            return Err(CommandError::NoIdleKittens);
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        let kitten = &mut self.kittens[ix];
        kitten.job = Some(j);
        kitten.job_seq = Some(seq);
        Ok(vec![Event::JobAssigned {
            name: kitten.name.clone(),
            job: j,
        }])
    }

    // @spec SIM-KIT-009
    fn unassign(&mut self, j: JobId) -> Result<Vec<Event>, CommandError> {
        let Some(ix) = self
            .kittens
            .iter()
            .enumerate()
            .filter(|(_, k)| k.location == Location::Home && k.job == Some(j))
            .max_by_key(|(_, k)| k.job_seq)
            .map(|(ix, _)| ix)
        else {
            return Err(CommandError::NoWorkersAssigned);
        };
        let kitten = &mut self.kittens[ix];
        kitten.job = None;
        kitten.job_seq = None;
        // A Tinkerer leaving the job releases its recipe binding (and seat).
        // @spec SIM-ECO-043
        kitten.tinkering = None;
        Ok(vec![Event::JobUnassigned {
            name: kitten.name.clone(),
            job: j,
        }])
    }

    /// Research spends curiosity and any gate-craft components together,
    /// all-or-nothing.
    // @spec SIM-CUR-003, SIM-CUR-006, SIM-CUR-007
    fn research(&mut self, i: IdeaId) -> Result<Vec<Event>, CommandError> {
        if !self.idea_unlocked(i) {
            return Err(CommandError::Locked);
        }
        if self.ideas.contains(&i) {
            return Err(CommandError::AlreadyResearched);
        }
        let def = content::idea_def(i);
        // The Curious Elder discounts the curiosity price at spend time;
        // gate crafts stay full price. @spec SIM-KIT-024
        let curiosity = def.cost * (1.0 + self.effect(crate::effects::Effect::IdeaCostRatio));
        let mut costs = vec![(ResourceId::Curiosity, curiosity)];
        costs.extend_from_slice(def.gate);
        self.spend(&costs)?;
        self.ideas.insert(i);
        // Cross-leaf bootstraps granted at research time (no RNG):
        match i {
            // The first Rainbow Arch structure arrives free, so the Parade loop
            // bootstraps without a chicken-and-egg. @spec SIM-UNI-011
            IdeaId::ParadeRoutes => self.structures[StructureId::RainbowArch.ix()] += 1,
            // The trade surface hands over the first colony free, forever.
            // @spec SIM-TRADE-008
            IdeaId::GiftEconomy => {
                for c in [
                    ColonyId::Otters,
                    ColonyId::Raccoons,
                    ColonyId::Hedgehogs,
                    ColonyId::Rabbits,
                ] {
                    if self.discovered.insert(c) {
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(vec![Event::IdeaResearched { id: i }])
    }

    /// Squads scamper beyond the meadow and come back at once with loot:
    /// affordability is checked before any draw (a rejected pounce leaves
    /// the RNG untouched), then yields resolve in the fixed draw order —
    /// feathers, bells, shinies (cap-clamped), unicorns.
    // @spec SIM-EXP-001, SIM-EXP-002, SIM-EXP-003, SIM-EXP-005, SIM-EXP-006
    fn pounce(&mut self, squads: u32) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::PouncingPractice) {
            return Err(CommandError::Locked);
        }
        if squads == 0 {
            return Err(CommandError::UnknownId); // a zero-squad pounce is no pounce
        }
        self.spend(&[(
            ResourceId::Pep,
            content::POUNCE_PEP_PER_SQUAD * squads as f64,
        )])?;

        // Gentle Paws: the squads still trot out (pep spent), but come home with
        // nothing — no yields, no RNG draws. @spec SIM-ADV-002
        if self.active_adventure == Some(AdventureId::GentlePaws) {
            return Ok(vec![Event::PounceReturned {
                squads,
                feathers: 0.0,
                bells: 0.0,
                shinies: 0.0,
                unicorns: 0,
            }]);
        }

        // @spec SIM-EXP-004
        let ratio = self.effect(crate::effects::Effect::PounceRatio);
        let feathers = (content::POUNCE_FEATHERS_BASE + content::POUNCE_FEATHERS_PER_RATIO * ratio)
            * self.irwin_hall(squads);
        let bell_hits = self.binomial(
            squads,
            content::POUNCE_BELLS_CHANCE + content::POUNCE_BELLS_CHANCE_PER_RATIO * ratio,
        );
        let bells = (content::POUNCE_BELLS_BASE + content::POUNCE_BELLS_PER_RATIO * ratio)
            * self.irwin_hall(bell_hits);
        let shiny_hits = self.binomial(squads, content::POUNCE_SHINIES_CHANCE);
        let shinies_rolled = (content::POUNCE_SHINIES_BASE
            + content::POUNCE_SHINIES_PER_RATIO * ratio)
            * self.irwin_hall(shiny_hits);
        let unicorns = self.binomial(squads, content::POUNCE_UNICORN_CHANCE);

        self.resources[ResourceId::Feathers.ix()] += feathers;
        self.resources[ResourceId::Bells.ix()] += bells;
        // Shinies clamp to cap at yield time; the event reports the amount
        // that actually landed.
        let cap = self.cap(ResourceId::Shinies);
        let held = self.stock(ResourceId::Shinies);
        let shinies = shinies_rolled.min((cap - held).max(0.0));
        self.resources[ResourceId::Shinies.ix()] = held + shinies;
        self.resources[ResourceId::Unicorns.ix()] += unicorns as f64;

        Ok(vec![Event::PounceReturned {
            squads,
            feathers,
            bells,
            shinies,
            unicorns,
        }])
    }

    /// Irwin–Hall: the sum of n uniform draws (mean n/2, bell-shaped).
    // @spec SIM-EXP-002
    fn irwin_hall(&mut self, n: u32) -> f64 {
        (0..n).map(|_| self.rng.uniform()).sum()
    }

    /// Binomial(n, p) as n Bernoulli draws — the schedule stays fixed given n.
    fn binomial(&mut self, n: u32, p: f64) -> u32 {
        (0..n).filter(|_| self.rng.chance(p)).count() as u32
    }

    /// Host a jamboree: gated by its idea, one at a time ("already
    /// celebrating" is Locked), all-or-nothing bill, 100 days of festivity.
    // @spec SIM-KIT-025
    fn host_jamboree(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::TeaCeremonies)
            || self.jamboree_active()
            || self.active_adventure == Some(AdventureId::QuietRetreat)
        {
            return Err(CommandError::Locked);
        }
        self.spend(&[
            (ResourceId::Pep, content::JAMBOREE_PEP),
            (ResourceId::Charm, content::JAMBOREE_CHARM),
            (ResourceId::Quills, content::JAMBOREE_QUILLS),
        ])?;
        self.jamboree_until = self.ticks + content::JAMBOREE_TICKS;
        Ok(vec![Event::JamboreeStarted {
            until_tick: self.jamboree_until,
        }])
    }

    /// Promote a kitten: 500 × 1.75^rank exp from its own pool plus
    /// 25 × (rank+1) shinies, both all-or-nothing.
    // @spec SIM-KIT-021
    fn promote(&mut self, ix: usize) -> Result<Vec<Event>, CommandError> {
        let Some(kitten) = self.kittens.get(ix) else {
            return Err(CommandError::UnknownId);
        };
        let rank = kitten.rank;
        let exp_cost = content::PROMOTE_EXP_BASE * content::PROMOTE_EXP_RATIO.powi(rank as i32);
        let shinies_cost = content::PROMOTE_SHINIES_STEP * (rank as f64 + 1.0);
        let shinies = self.stock(ResourceId::Shinies);
        let mut missing = Vec::new();
        if shinies + EPSILON < shinies_cost {
            missing.push((ResourceId::Shinies, shinies_cost - shinies));
        }
        if kitten.exp + EPSILON < exp_cost || !missing.is_empty() {
            return Err(CommandError::InsufficientResources { missing });
        }
        self.resources[ResourceId::Shinies.ix()] = (shinies - shinies_cost).max(0.0);
        let kitten = &mut self.kittens[ix];
        kitten.exp = (kitten.exp - exp_cost).max(0.0);
        kitten.rank += 1;
        Ok(vec![Event::KittenPromoted {
            name: kitten.name.clone(),
            rank: kitten.rank,
        }])
    }

    /// Name a kitten the Elder. Free, replaceable anytime; the game itself
    /// never removes one.
    // @spec SIM-KIT-022
    fn appoint_elder(&mut self, ix: usize) -> Result<Vec<Event>, CommandError> {
        let Some(kitten) = self.kittens.get(ix) else {
            return Err(CommandError::UnknownId);
        };
        // Naming the sitting Elder again changes nothing: accepted, silent.
        // @spec SIM-ENG-058
        if self.elder == Some(ix) {
            return Ok(Vec::new());
        }
        self.elder = Some(ix);
        Ok(vec![Event::ElderAppointed {
            name: kitten.name.clone(),
        }])
    }

    /// Buy a refinement: Locked before its gate idea, purchasable once,
    /// curiosity + materials spent together, all-or-nothing, permanent.
    // @spec SIM-CUR-010
    fn buy_refinement(&mut self, r: RefinementId) -> Result<Vec<Event>, CommandError> {
        let def = content::refinement_def(r);
        if !self.ideas.contains(&def.gate) {
            return Err(CommandError::Locked);
        }
        if self.refinements.contains(&r) {
            return Err(CommandError::AlreadyResearched);
        }
        let mut costs = vec![(ResourceId::Curiosity, def.cost)];
        costs.extend_from_slice(def.materials);
        self.spend(&costs)?;
        self.refinements.insert(r);
        Ok(vec![Event::RefinementBought { id: r }])
    }

    /// Adopt a house rule: charm all-or-nothing; the sibling rule is
    /// permanently Locked for this meadow, in both directions.
    // @spec SIM-CUR-011
    fn adopt_rule(&mut self, r: RuleId) -> Result<Vec<Event>, CommandError> {
        let def = content::rule_def(r);
        if self.rules.contains(&def.blocks) {
            return Err(CommandError::Locked);
        }
        if self.rules.contains(&r) {
            return Err(CommandError::AlreadyResearched);
        }
        self.spend(&[(ResourceId::Charm, def.cost_charm)])?;
        self.rules.insert(r);
        Ok(vec![Event::RuleAdopted { id: r }])
    }

    /// Bind the most recently arrived unbound Home Tinkerer to a recipe.
    /// Error precedence: Locked (the recipe), NoIdleKittens (no unbound Home
    /// Tinkerer), NoSeats (bindings would exceed the Toy Workshop count —
    /// an absent Tinkerer still holds its seat).
    // @spec SIM-ECO-043, SIM-ENG-006
    fn assign_tinkerer(&mut self, c: CraftId) -> Result<Vec<Event>, CommandError> {
        if !self.craft_unlocked(c) {
            return Err(CommandError::Locked);
        }
        let Some(ix) = self.kittens.iter().rposition(|k| {
            k.location == Location::Home && k.job == Some(JobId::Tinkerer) && k.tinkering.is_none()
        }) else {
            return Err(CommandError::NoIdleKittens);
        };
        let seats = self.structures[StructureId::ToyWorkshop.ix()];
        let bound = self
            .kittens
            .iter()
            .filter(|k| k.tinkering.is_some())
            .count() as u32;
        if bound + 1 > seats {
            return Err(CommandError::NoSeats);
        }
        self.kittens[ix].tinkering = Some(c);
        Ok(Vec::new())
    }

    /// Release the bound Home holder of this recipe with the highest
    /// job_seq — the later assignee unbinds first.
    // @spec SIM-ECO-043
    fn unassign_tinkerer(&mut self, c: CraftId) -> Result<Vec<Event>, CommandError> {
        let Some(ix) = self
            .kittens
            .iter()
            .enumerate()
            .filter(|(_, k)| k.location == Location::Home && k.tinkering == Some(c))
            .max_by_key(|(_, k)| k.job_seq)
            .map(|(ix, _)| ix)
        else {
            return Err(CommandError::NoWorkersAssigned);
        };
        self.kittens[ix].tinkering = None;
        Ok(Vec::new())
    }

    // ---- Ceremonies (SIM-SUN) ----

    /// Offer warmth up into devotion: devotion += warmth × (1 + reverence),
    /// warmth zeroed, peak_devotion ratcheted. An empty sunpuddle rejects.
    // @spec SIM-SUN-001
    fn bask(&mut self) -> Result<Vec<Event>, CommandError> {
        if self.silent_sun() {
            return Err(CommandError::Locked);
        }
        let warmth = self.stock(ResourceId::Warmth);
        if warmth <= EPSILON {
            return Err(CommandError::InsufficientResources {
                missing: vec![(ResourceId::Warmth, content::GATHER_YIELD)],
            });
        }
        self.devotion += warmth * (1.0 + self.reverence());
        self.resources[ResourceId::Warmth.ix()] = 0.0;
        self.peak_devotion = self.peak_devotion.max(self.devotion);
        Ok(vec![Event::Basked {
            devotion: self.devotion,
        }])
    }

    /// Settle devotion into permanent serenity (needs the Reverie rite):
    /// serenity += devotion / 1e6 × (tier + 1)² × 1.01, devotion → a 0.01 spark;
    /// peak_devotion is untouched.
    // @spec SIM-SUN-002
    fn reverie(&mut self) -> Result<Vec<Event>, CommandError> {
        if self.silent_sun() {
            return Err(CommandError::Locked);
        }
        if !self.rites.contains(&RiteId::TheReverie) {
            return Err(CommandError::Locked);
        }
        if self.devotion <= EPSILON {
            return Err(CommandError::InsufficientResources {
                missing: vec![(ResourceId::Warmth, 1.0)],
            });
        }
        let tier = (self.enlightenment_tier as f64 + 1.0).powi(2);
        self.serenity += self.devotion / 1.0e6 * tier * 1.01;
        self.devotion = 0.01;
        Ok(vec![Event::Reveried {
            serenity: self.serenity,
        }])
    }

    /// Buy or (post-Enlightenment) restack a Sunbeam Rite. Gated on devotion's
    /// persisted high-water; warmth+shinies(+charm) all-or-nothing; the ×2.5
    /// restack ramp scales the cost by the copies already held.
    // @spec SIM-SUN-005, SIM-SUN-006, SIM-SUN-007
    fn buy_rite(&mut self, r: RiteId) -> Result<Vec<Event>, CommandError> {
        if self.silent_sun() {
            return Err(CommandError::Locked);
        }
        let def = content::rite_def(r);
        if self.peak_devotion + EPSILON < def.devotion_gate {
            return Err(CommandError::Locked);
        }
        let count = self.rite_counts[r.ix()];
        let restackable =
            self.rites.contains(&RiteId::Enlightenment) && content::rite_restackable(r);
        if count >= 1 && !restackable {
            return Err(CommandError::AlreadyResearched);
        }
        let ramp = content::RITE_RESTACK_RATIO.powi(count as i32);
        let discount = 1.0 + self.effect(crate::effects::Effect::RiteCostRatio);
        let costs: Vec<(ResourceId, f64)> = def
            .cost
            .iter()
            .map(|(res, amt)| (*res, amt * ramp * discount))
            .collect();
        self.spend(&costs)?;
        self.rites.insert(r);
        self.rite_counts[r.ix()] += 1;
        Ok(vec![Event::RiteBought { id: r }])
    }

    /// Spend the marginal serenity for the next Enlightenment tier.
    // @spec SIM-SUN-008
    fn enlighten(&mut self) -> Result<Vec<Event>, CommandError> {
        if self.silent_sun() {
            return Err(CommandError::Locked);
        }
        if !self.rites.contains(&RiteId::Enlightenment) {
            return Err(CommandError::Locked);
        }
        let t = self.enlightenment_tier;
        let cost = enlighten_total_cost(t + 1) - enlighten_total_cost(t);
        if self.serenity + EPSILON < cost {
            return Err(CommandError::InsufficientResources {
                missing: vec![(ResourceId::Reverie, cost - self.serenity)],
            });
        }
        self.serenity = (self.serenity - cost).max(0.0);
        self.enlightenment_tier += 1;
        Ok(vec![Event::Enlightened {
            tier: self.enlightenment_tier,
        }])
    }

    // ---- Mystic (SIM-UNI) ----

    /// Hold a Unicorn Parade: 2500 unicorns → 2500 × arches × (1 + parade
    /// ratio) rainbows. No RNG.
    // @spec SIM-UNI-001
    fn hold_parade(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::ParadeRoutes) {
            return Err(CommandError::Locked);
        }
        self.spend(&[(ResourceId::Unicorns, content::PARADE_UNICORN_COST)])?;
        let arches = self.structures[StructureId::RainbowArch.ix()] as f64;
        let ratio = 1.0 + self.effect(crate::effects::Effect::ParadeRatio);
        let rainbows = content::PARADE_RAINBOWS_PER_ARCH * arches * ratio;
        self.resources[ResourceId::Rainbows.ix()] += rainbows;
        Ok(vec![Event::ParadeHeld { rainbows }])
    }

    /// Buy a Rainbow Arch upgrade (once each), gated by the Parade Routes idea.
    // @spec SIM-UNI-005
    fn buy_arch(&mut self, a: ArchId) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::ParadeRoutes) {
            return Err(CommandError::Locked);
        }
        if self.arches_bought.contains(&a) {
            return Err(CommandError::AlreadyResearched);
        }
        self.spend(content::arch_def(a).cost)?;
        self.arches_bought.insert(a);
        Ok(vec![Event::ArchBought { id: a }])
    }

    /// Refine 25 prisms → 1 × (1 + starshard-refine) starshards.
    // @spec SIM-UNI-003
    fn refine_starshard(&mut self) -> Result<Vec<Event>, CommandError> {
        self.spend(&[(ResourceId::Prisms, content::REFINE_INPUT)])?;
        let yield_ = 1.0 + self.effect(crate::effects::Effect::StarshardRefine);
        self.resources[ResourceId::Starshards.ix()] += yield_;
        Ok(vec![Event::StarshardRefined])
    }

    /// Refine 25 starshards → 1 × (1 + echo-refine) echoes.
    // @spec SIM-UNI-003
    fn refine_echo(&mut self) -> Result<Vec<Event>, CommandError> {
        self.spend(&[(ResourceId::Starshards, content::REFINE_INPUT)])?;
        let yield_ = 1.0 + self.effect(crate::effects::Effect::EchoRefine);
        self.resources[ResourceId::Echoes.ix()] += yield_;
        Ok(vec![Event::EchoRefined])
    }

    /// Condense 10000 rainbows → 1 Jar of Purrs (needs the Everdream Arch,
    /// clamped to the Jar cap).
    // @spec SIM-UNI-004
    fn condense_jar(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.arches_bought.contains(&ArchId::EverdreamArch) {
            return Err(CommandError::Locked);
        }
        self.spend(&[(ResourceId::Rainbows, content::JAR_RAINBOW_COST)])?;
        let ix = ResourceId::JarsOfPurrs.ix();
        self.resources[ix] = (self.resources[ix] + 1.0).min(self.cap(ResourceId::JarsOfPurrs));
        Ok(vec![Event::JarCondensed])
    }

    /// Make a Promise: spend echoes, fill a slot. Slots come from Everdream
    /// Arches; the Night Court surface (idea) must be open.
    // @spec SIM-UNI-009
    fn make_promise(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::NightCourtEtiquette) {
            return Err(CommandError::Locked);
        }
        let slots = if self.arches_bought.contains(&ArchId::EverdreamArch) {
            content::PROMISE_SLOTS_PER_EVERDREAM
        } else {
            0
        };
        if self.promises.len() >= slots {
            return Err(CommandError::NoSeats);
        }
        self.spend(&[(ResourceId::Echoes, content::PROMISE_ECHO_COST)])?;
        self.promises.push(Promise { quiet: false });
        Ok(vec![Event::PromiseMade])
    }

    // ---- Colonies (SIM-TRADE) ----

    /// Send `count` caravans to a discovered colony. Cost (50 pep + 15 shinies +
    /// buy-resource per caravan) is affordable-checked and spent BEFORE any RNG
    /// draw; each caravan then resolves against standing.
    // @spec SIM-TRADE-001, SIM-TRADE-002, SIM-TRADE-004
    fn trade(&mut self, colony: ColonyId, count: u32) -> Result<Vec<Event>, CommandError> {
        if count == 0 {
            return Err(CommandError::UnknownId);
        }
        // The Star Whale is a visitor, not a caravan route: its exchange runs
        // only while it visits. @spec SIM-SKY-007, SIM-TRADE-001
        if colony == ColonyId::StarWhales {
            return self.whale_exchange(count);
        }
        if !self.discovered.contains(&colony) {
            return Err(CommandError::Locked);
        }
        let def = content::colony_def(colony);
        let n = count as f64;
        // Affordability is checked (and spent) before any draw, so a rejected
        // trade leaves the RNG untouched.
        self.spend(&[
            (ResourceId::Pep, content::TRADE_PEP_PER_CARAVAN * n),
            (ResourceId::Shinies, content::TRADE_SHINIES_PER_CARAVAN * n),
            (def.buy.0, def.buy.1 * n),
        ])?;

        let st = self.standing(colony);
        let season = self.season();
        let trade_ratio = 1.0 + self.effect(crate::effects::Effect::TradeRatio);
        let kites = self.stock(ResourceId::Kites);
        let penpal = self.penpal[colony.ix()];
        for _ in 0..count {
            // Shy colonies nap with probability −standing; warm/neutral trade.
            let traded = st >= 0.0 || self.rng.chance(1.0 + st);
            if !traded {
                continue; // a nap: the caravan travelled, cost already paid
            }
            let bonus = st > 0.0 && self.rng.chance(st.min(1.0));
            let bonus_mult = if bonus {
                content::TRADE_BONUS_MULT
            } else {
                1.0
            };
            for sell in def.sells {
                if penpal < sell.min_penpal || !self.rng.chance(sell.chance) {
                    continue;
                }
                let season_mult = if sell.season == Some(season) {
                    1.0 + sell.season_bonus
                } else {
                    1.0
                };
                let fuzz = 1.0 + (self.rng.uniform() * 2.0 - 1.0) * content::TRADE_FUZZ_WIDTH;
                self.resources[sell.resource.ix()] +=
                    sell.amount * trade_ratio * season_mult * bonus_mult * fuzz;
            }
            // Foxes' kite-scaled moonstone — the early metal path.
            // @spec SIM-TRADE-004
            if colony == ColonyId::Foxes {
                let chance = content::FOX_MOONSTONE_CHANCE_BASE
                    + content::FOX_MOONSTONE_CHANCE_PER_KITE * kites;
                if self.rng.chance(chance) {
                    let amount = (content::FOX_MOONSTONE_AMOUNT_BASE
                        + content::FOX_MOONSTONE_AMOUNT_PER_KITE * kites)
                        * (1.0 + st).max(0.0);
                    let fuzz = 1.0 + (self.rng.uniform() * 2.0 - 1.0) * content::TRADE_FUZZ_WIDTH;
                    self.resources[ResourceId::Moonstone.ix()] +=
                        amount * trade_ratio * bonus_mult * fuzz;
                }
            }
        }
        Ok(vec![Event::TradeReturned {
            colony,
            caravans: count,
        }])
    }

    /// Deepen a Pen Pal correspondence: charm base × 1.15^level. Star Whales
    /// take no Pen Pals.
    // @spec SIM-TRADE-006
    fn send_pen_pal(&mut self, colony: ColonyId) -> Result<Vec<Event>, CommandError> {
        if colony == ColonyId::StarWhales
            || !self.ideas.contains(&IdeaId::PenPalNetworks)
            || !self.discovered.contains(&colony)
        {
            return Err(CommandError::Locked);
        }
        let level = self.penpal[colony.ix()];
        let cost = content::PENPAL_BASE_CHARM * content::PENPAL_CHARM_RATIO.powi(level as i32);
        self.spend(&[(ResourceId::Charm, cost)])?;
        self.penpal[colony.ix()] += 1;
        Ok(vec![Event::PenPalSent {
            colony,
            level: self.penpal[colony.ix()],
        }])
    }

    /// Spend 1000 pep to scout the next colony; a quiet day refunds 950.
    // @spec SIM-TRADE-007
    fn scout_picnic(&mut self) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::GiftEconomy) {
            return Err(CommandError::Locked);
        }
        self.spend(&[(ResourceId::Pep, content::SCOUT_PEP_COST)])?;
        let found = self
            .rng
            .chance(content::SCOUT_FIND_CHANCE)
            .then(|| self.scout_target())
            .flatten();
        match found {
            Some(colony) => {
                self.discovered.insert(colony);
                Ok(vec![Event::ColonyDiscovered { colony }])
            }
            None => {
                // A quiet day: they mostly stayed home. Most of the pep returns.
                self.resources[ResourceId::Pep.ix()] += content::SCOUT_PEP_REFUND;
                Ok(Vec::new())
            }
        }
    }

    /// The next colony a successful picnic discovers, by deterministic
    /// prerequisite priority: a random undiscovered first-tier colony, else the
    /// eligible Foxes, Deer, then Owls. @spec SIM-TRADE-007
    fn scout_target(&mut self) -> Option<ColonyId> {
        let first_tier: Vec<ColonyId> = [
            ColonyId::Otters,
            ColonyId::Raccoons,
            ColonyId::Hedgehogs,
            ColonyId::Rabbits,
        ]
        .into_iter()
        .filter(|c| !self.discovered.contains(c))
        .collect();
        if !first_tier.is_empty() {
            return Some(first_tier[self.rng.index(first_tier.len())]);
        }
        let charm = self.stock(ResourceId::Charm);
        let kites = self.stock(ResourceId::Kites);
        let curiosity_cap = self.cap(ResourceId::Curiosity);
        if !self.discovered.contains(&ColonyId::Foxes)
            && charm + EPSILON >= content::SCOUT_FOX_CHARM
        {
            return Some(ColonyId::Foxes);
        }
        if !self.discovered.contains(&ColonyId::Deer) && kites >= 1.0 - EPSILON {
            return Some(ColonyId::Deer);
        }
        if !self.discovered.contains(&ColonyId::Owls)
            && kites + EPSILON >= content::SCOUT_OWL_KITES
            && curiosity_cap > content::SCOUT_OWL_CURIOSITY_CAP
        {
            return Some(ColonyId::Owls);
        }
        None
    }

    /// The Silent Sun adventure twist: ceremonies unavailable. @spec SIM-ADV-002
    pub(crate) fn silent_sun(&self) -> bool {
        self.active_adventure == Some(AdventureId::SilentSun)
    }

    // ---- Wave 5: prestige / the Great Meadow Move (SIM-MOVE) ----

    /// The one reset in the game: earn the legacies, archive the generation,
    /// pack the baskets and the Memory Chest, and move into a fresh meadow —
    /// preserving the legacy set (Elder Wisdom, Warm Memories, Passed-Down
    /// Wisdom, serenity, enlightenment, Traditions, the Family Album, the
    /// Adventures state, generation, speed, and the never-reseeded RNG). It
    /// draws no randomness and takes nothing the player is attached to.
    /// @spec SIM-MOVE-001, SIM-MOVE-002, SIM-MOVE-003, SIM-MOVE-004,
    ///       SIM-MOVE-005, SIM-MOVE-006, SIM-MOVE-008, SIM-ADV-002
    fn great_meadow_move(&mut self) -> Result<Vec<Event>, CommandError> {
        let roster_len = self.kittens.len();
        if roster_len < content::MEADOW_MOVE_MIN_KITTENS {
            // A Move needs a real clowder (Home and Away both count). Nothing
            // is mutated on rejection.
            return Err(CommandError::InsufficientResources {
                missing: Vec::new(),
            });
        }

        // ---- 1. Snapshot everything the reset would erase ----
        let old_ticks = self.ticks;
        let old_stocks = self.resources;
        // Reverie is a permanent dreamtime currency: preserved in full across a
        // Move (carved out of the resource reset). @spec SIM-MOVE-004
        let kept_reverie = self.stock(ResourceId::Reverie);
        let ratio = self.memory_chest_ratio(); // reads OLD structures + traditions
        let gen = self.generation;
        let millennia = (old_ticks / 1_000_000) as f64;
        let cap = self.basket_capacity() as usize;
        let second_wind = self.traditions.contains(&TraditionId::SecondWind);
        let old_armed = self.armed_adventure;
        let old_active = self.active_adventure;
        let old_done = self.adventure_done;
        let old_roster = std::mem::take(&mut self.kittens);

        // ---- 2. Earn the legacies (never decreasing) ----
        self.elder_wisdom += (roster_len as f64 - 70.0).max(0.0) + millennia;
        self.warm_memories += karma_gain(roster_len as f64);

        // ---- 3. Archive every kitten under the current (pre-increment) gen ----
        for k in &old_roster {
            self.family_album.push(ArchivedKitten {
                name: k.name.clone(),
                trait_id: k.trait_id,
                rank: k.rank,
                generation: gen,
            });
        }

        // ---- 4. Pack the Traveling Baskets: most-senior first, normalized ----
        let mut sorted = old_roster;
        sorted.sort_by_key(|k| k.arrived_day);
        let basket: Vec<Kitten> = sorted
            .into_iter()
            .take(cap)
            .enumerate()
            .map(|(i, k)| {
                let keep_job = second_wind && i == 0;
                Kitten {
                    name: k.name,
                    job: if keep_job { k.job } else { None },
                    location: Location::Home,
                    arrived_day: 0,
                    job_seq: None,
                    away_seq: None,
                    skills: k.skills,
                    exp: k.exp,
                    rank: k.rank,
                    trait_id: k.trait_id,
                    tinkering: None,
                }
            })
            .collect();

        // ---- 5. Fill the Memory Chest (a fraction of each pre-reset stock) ----
        let mut carried = [0.0f64; RESOURCE_COUNT];
        for r in ResourceId::ALL {
            carried[r.ix()] = ratio * old_stocks[r.ix()];
        }

        // ---- 6. Move in: reset the meadow in place (NOT GameState::new, which
        // would reseed the RNG); the legacy set below is left untouched. ----
        self.structures = [0; STRUCTURE_COUNT];
        self.ideas.clear();
        self.refinements.clear();
        self.rules.clear();
        self.resources = [0.0; RESOURCE_COUNT];
        self.kittens = basket;
        self.devotion = 0.0;
        self.peak_devotion = 0.0;
        self.rite_counts = [0; 11];
        self.rites.clear();
        self.dream_drift = 0.0;
        self.arches_bought.clear();
        self.promises.clear();
        self.discovered.clear();
        self.penpal = [0; 8];
        self.elder = None;
        self.mess = 0.0;
        self.grumpy = false;
        self.pity = 0;
        self.famine_days = 0;
        self.recovery_days = 0;
        self.next_seq = 0;
        self.zoomies_until = 0;
        self.jamboree_until = 0;
        self.kites_high_water = 0.0;
        self.pending_sky_event = None;
        self.ticks = 0;
        self.tidy_progress = 0;
        // Wave-6 meadow-local state resets; sites_reached, dream_looms, the
        // distant_dawn flag, and reverie are the preserved legacies (untouched).
        // @spec SIM-MOVE-004
        self.stirred_dreams = 0.0;
        self.whale_visit_years = 0;
        self.whale_contentment = 0;
        self.deja_vu_until = 0;
        self.revealed.clear();
        // @spec SIM-ENG-052
        self.revealed_resources.clear();
        self.generation = gen + 1;
        self.rebuild_unrevealed();

        // ---- 7. Adventures: armed → active; a completed twist lifts, an
        // unfinished one carries over. @spec SIM-ADV-002, SIM-ADV-004 ----
        self.active_adventure = if let Some(a) = old_armed {
            Some(a)
        } else if old_done {
            None
        } else {
            old_active
        };
        self.armed_adventure = None;
        self.adventure_done = false;

        // ---- 8. Place the carried resources, then the Warm Welcome Home gift ----
        for r in ResourceId::ALL {
            self.resources[r.ix()] = carried[r.ix()];
        }
        // Reverie is preserved in full, not fractioned into the Memory Chest.
        // @spec SIM-MOVE-004
        self.resources[ResourceId::Reverie.ix()] = kept_reverie;
        if self.traditions.contains(&TraditionId::WarmWelcomeHome) {
            self.resources[ResourceId::Catnip.ix()] += content::WARM_WELCOME_CATNIP;
            self.structures[StructureId::CozyCushion.ix()] += 1;
        }

        Ok(vec![Event::MeadowMoved {
            generation: self.generation,
        }])
    }

    /// Convert all Elder Wisdom into Passed-Down Wisdom (one-way). @spec SIM-MOVE-009
    fn pass_it_down(&mut self) -> Result<Vec<Event>, CommandError> {
        self.passed_down_wisdom += self.elder_wisdom;
        self.elder_wisdom = 0.0;
        Ok(vec![Event::Passed {
            passed_down_wisdom: self.passed_down_wisdom,
        }])
    }

    /// Buy a Tradition once with Elder Wisdom: AlreadyResearched on a re-buy,
    /// InsufficientResources when unaffordable, all-or-nothing, permanent.
    /// @spec SIM-MOVE-010, SIM-MOVE-012
    fn buy_tradition(&mut self, t: TraditionId) -> Result<Vec<Event>, CommandError> {
        if self.traditions.contains(&t) {
            return Err(CommandError::AlreadyResearched);
        }
        let cost = content::tradition_def(t).cost_wisdom;
        if self.elder_wisdom + EPSILON < cost {
            return Err(CommandError::InsufficientResources {
                missing: Vec::new(),
            });
        }
        self.elder_wisdom -= cost;
        self.traditions.insert(t);
        Ok(vec![Event::TraditionBought { id: t }])
    }

    /// Arm (or disarm, with None) an Adventure for the next Move — once the
    /// Adventures surface is open (the Adventurous Spirit tradition or idea).
    /// @spec SIM-ADV-001
    fn declare_adventure(&mut self, id: Option<AdventureId>) -> Result<Vec<Event>, CommandError> {
        let unlocked = self.traditions.contains(&TraditionId::AdventurousSpirit)
            || self.ideas.contains(&IdeaId::AdventurousSpirit);
        if !unlocked {
            return Err(CommandError::Locked);
        }
        self.armed_adventure = id;
        Ok(vec![Event::AdventureDeclared { id }])
    }

    // ---- Wave 6: cosmic (SIM-SKY) ----

    /// Reach the next unreached site: surface-gated (Skyward Longing), strictly
    /// in ladder order, spending the exact mission fuel all-or-nothing. An
    /// out-of-order or surface-locked launch is Locked; an unaffordable one is
    /// InsufficientResources.
    // @spec SIM-SKY-001
    fn launch_expedition(&mut self, site: SiteId) -> Result<Vec<Event>, CommandError> {
        if !self.site_reachable(site) {
            return Err(CommandError::Locked);
        }
        self.spend(content::site_def(site).mission_fuel)?;
        self.sites_reached.insert(site);
        Ok(vec![Event::SiteReached { site }])
    }

    /// Summon a Star Whale: needs the Sky Dock built and the Skyward Song idea;
    /// spends curiosity + echoes; the visit lasts 5 years + 1 per contentment.
    // @spec SIM-SKY-006
    fn summon_star_whale(&mut self) -> Result<Vec<Event>, CommandError> {
        if self.structures[StructureId::SkyDock.ix()] < 1
            || !self.ideas.contains(&IdeaId::SkywardSong)
        {
            return Err(CommandError::Locked);
        }
        self.spend(&[
            (ResourceId::Curiosity, content::WHALE_SUMMON_CURIOSITY),
            (ResourceId::Echoes, content::WHALE_SUMMON_ECHOES),
        ])?;
        self.whale_visit_years = content::WHALE_BASE_VISIT_YEARS + self.whale_contentment;
        Ok(vec![Event::WhaleSummoned])
    }

    /// Feed the visiting whale a dreamcorn lullaby: contentment (and the visit)
    /// rise. Needs a visiting whale and a dreamcorn.
    // @spec SIM-SKY-007
    fn feed_whale(&mut self) -> Result<Vec<Event>, CommandError> {
        if self.whale_visit_years == 0 {
            return Err(CommandError::Locked);
        }
        self.spend(&[(ResourceId::Dreamcorns, content::WHALE_LULLABY_DREAMCORNS)])?;
        self.whale_contentment += 1;
        self.whale_visit_years += 1;
        Ok(vec![Event::WhaleFed {
            contentment: self.whale_contentment,
        }])
    }

    /// The visit-gated hushfelt exchange: spend hushfelt for cosmic goods
    /// (skymaps, starshards, echoes), scaled by contentment; Locked with no
    /// visit. No pep or shinies change paws.
    // @spec SIM-SKY-007
    fn whale_exchange(&mut self, count: u32) -> Result<Vec<Event>, CommandError> {
        if self.whale_visit_years == 0 {
            return Err(CommandError::Locked);
        }
        let n = count as f64;
        self.spend(&[(
            ResourceId::Hushfelt,
            content::WHALE_HUSHFELT_PER_CARAVAN * n,
        )])?;
        let scale = n * (1.0 + self.whale_contentment as f64);
        let skymaps = ResourceId::Skymaps.ix();
        self.resources[skymaps] = (self.resources[skymaps]
            + content::WHALE_SKYMAPS_PER_CARAVAN * scale)
            .min(self.cap(ResourceId::Skymaps));
        self.resources[ResourceId::Starshards.ix()] +=
            content::WHALE_STARSHARDS_PER_CARAVAN * scale;
        self.resources[ResourceId::Echoes.ix()] += content::WHALE_ECHOES_PER_CARAVAN * scale;
        Ok(vec![Event::TradeReturned {
            colony: ColonyId::StarWhales,
            caravans: count,
        }])
    }

    // ---- Wave 6: dreamtime (SIM-DREAM) ----

    /// The Deep Nap: idea-gated, bounded to max_nap_length, spends the
    /// escalating starshard cost, then fast-forwards the meadow through the
    /// standard tick() (pure — the identical path as offline catch-up, no added
    /// yields). Accrues stirred-dreams heat and returns one summarized report,
    /// folded the same way offline absence folds its report and retained in
    /// the journal alongside it. @spec SIM-DREAM-001, SIM-DREAM-002, SIM-DREAM-007, SIM-DREAM-008
    fn deep_nap(&mut self, years: u32) -> Result<Vec<Event>, CommandError> {
        if !self.ideas.contains(&IdeaId::DeepNapTheory) {
            return Err(CommandError::Locked);
        }
        if years < 1 || years > self.max_nap_length() {
            return Err(CommandError::InsufficientResources {
                missing: Vec::new(),
            });
        }
        // Affordability is checked (and the cost spent) before any tick draw.
        let cost = self.starshard_cost(years);
        let held = self.stock(ResourceId::Starshards);
        if held + EPSILON < cost {
            return Err(CommandError::InsufficientResources {
                missing: vec![(ResourceId::Starshards, cost - held)],
            });
        }
        self.resources[ResourceId::Starshards.ix()] = (held - cost).max(0.0);

        // Fast-forward through the pure tick(), folding away the per-tick event
        // stream (like the save.rs catch-up loop); the advancing counter alone
        // delivers any Move-time trickle. distant_dawn lights inside tick() at
        // the year-40000 boundary (with the loom) and persists.
        let was_dawn = self.distant_dawn;
        let ticks = years as u64 * 1000;
        let before = self.resources;
        let mut report = crate::save::OfflineReport {
            elapsed_seconds: ticks / content::TICKS_PER_SECOND,
            truncated: false, // a nap is bounded by max_nap_length; it never hits the catch-up cap
            ..crate::save::OfflineReport::default()
        };
        // @spec SIM-ENG-028, SIM-ENG-040
        for _ in 0..ticks {
            for event in self.tick_replay() {
                crate::save::fold_event(&mut report, event);
            }
        }
        for r in ResourceId::ALL {
            report.resource_deltas[r.ix()] = self.resources[r.ix()] - before[r.ix()];
        }

        // Heat accrues after the replay so it survives dissipation (a nap warms
        // the dream-space by years × heat_per_year × (1 − heat efficiency)).
        let heat_eff = self.effect(crate::effects::Effect::HeatEfficiency);
        self.stirred_dreams += years as f64 * content::HEAT_PER_YEAR * (1.0 - heat_eff).max(0.0);

        // Retained in the journal by the same route offline absence uses, so
        // the returned summary and the retained one are one report.
        // @spec SIM-DREAM-008
        self.journal.record_away(self.ticks, &report);

        let mut events = vec![Event::Napped {
            years,
            report: Box::new(report),
        }];
        if self.distant_dawn && !was_dawn {
            events.push(Event::DistantDawn);
        }
        Ok(events)
    }

    /// Weave a Dream Loom upgrade: reverie (and, for Distant Dawn, Jars) spent
    /// all-or-nothing, once each. @spec SIM-DREAM-006
    fn buy_dream_loom(&mut self, id: DreamLoomId) -> Result<Vec<Event>, CommandError> {
        if self.dream_looms.contains(&id) {
            return Err(CommandError::AlreadyResearched);
        }
        self.spend(content::dream_loom_def(id).cost)?;
        self.dream_looms.insert(id);
        Ok(vec![Event::DreamLoomBought { id }])
    }

    /// All-or-nothing spend with epsilon-tolerant affordability.
    // @spec SIM-ECO-022
    fn spend(&mut self, costs: &[(ResourceId, f64)]) -> Result<(), CommandError> {
        // A cost list may name the same component twice (a colony that buys
        // in the same resource its trade fee already spends, say) — checked
        // against the combined total, never against each entry alone, or a
        // stock short of the true total could slip past two individually
        // satisfied checks.
        let mut totals: Vec<(ResourceId, f64)> = Vec::new();
        for (r, amount) in costs {
            match totals.iter_mut().find(|(tr, _)| tr == r) {
                Some((_, total)) => *total += amount,
                None => totals.push((*r, *amount)),
            }
        }
        let missing: Vec<(ResourceId, f64)> = totals
            .iter()
            .filter(|(r, amount)| self.stock(*r) + EPSILON < *amount)
            .map(|(r, amount)| (*r, amount - self.stock(*r)))
            .collect();
        if !missing.is_empty() {
            return Err(CommandError::InsufficientResources { missing });
        }
        for (r, amount) in costs {
            let ix = r.ix();
            self.resources[ix] = (self.resources[ix] - amount).max(0.0);
        }
        Ok(())
    }
}

/// The Enlightenment cumulative cost to reach tier t:
/// total_cost(t) = inverse_unlimited_dr(e^t / 10, 0.1) = 0.1 · y · (y+1) / 2,
/// with y = e^t / 10 — the brutally exponential ladder. @spec SIM-SUN-008
pub(crate) fn enlighten_total_cost(t: u32) -> f64 {
    let y = (t as f64).exp() / 10.0;
    0.1 * y * (y + 1.0) / 2.0
}

/// The piecewise Warm Memories gain from a Move, steeper for bigger clowders.
/// @spec SIM-MOVE-008
fn karma_gain(k: f64) -> f64 {
    (k - 35.0)
        + 3.0 * (k - 60.0).max(0.0)
        + 4.0 * (k - 100.0).max(0.0)
        + 5.0 * (k - 150.0).max(0.0)
        + 10.0 * (k - 300.0).max(0.0)
        + 15.0 * (k - 750.0).max(0.0)
}
