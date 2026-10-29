//! Tests-first for SIM-ENG-048 to SIM-ENG-056 — revealed resources.
//!
//! A resource is revealed the first time the meadow holds some, or the first
//! time a price the core would state names it, and it stays revealed for as
//! long as its meadow lasts. Which stores exist for the player is therefore a
//! rule, and the core's to answer.
//!
//! The surface these tests address, which the code stage adds:
//!
//!   impl GameState {
//!       /// Revealed or not — never teased.
//!       pub fn resource_revealed(&self, r: ResourceId) -> bool
//!       /// The revealed resources, in the order the core lists resources.
//!       pub fn resources_revealed(&self) -> Vec<ResourceId>
//!   }
//!
//! and one persisted field: the state serializes a `revealed_resources` set,
//! in id order (SIM-ENG-051).
//!
//! **Two things are asserted, and they are kept apart.** The *query* answers
//! for the persisted set and for whatever the meadow holds at the asking
//! (SIM-ENG-053), so it cannot show whether a holding was ever recorded: it
//! says yes either way while the stock lasts. Whether a reveal was *recorded*
//! — and so survives the stock being spent — is read from the state's own
//! serialization (`common::recorded_resources`). A test about when a reveal is
//! recorded reads the record; a test about what a caller is told asks the
//! query.
//!
//! **The price oracle.** "A price the core would state" is assembled in
//! `common::priced_resources` from the three queries SIM-ENG-049 names — the
//! cost of one more of every item the core reports Teased or Available, the
//! per-unit price of every count-taking command, and the deep nap's cost while
//! its gate is open — and from nothing else. No test here reads a content
//! table to decide what a price names.
//!
//! **Hand-built states.** Most fixtures set fields directly, so they start
//! with nothing recorded however much they hold or show. That is what lets a
//! single tick or a single command be seen to make the record: the rule's own
//! wording is that a holding is recorded at the end of every tick and after
//! every applied command, and a price after every applied command — any
//! command, the one that changes nothing included (SIM-ENG-054).

mod common;

use clowder_core::content::MEADOW_MOVE_MIN_KITTENS;
use clowder_core::*;
use common::*;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn kitten(name: &str, job: Option<JobId>) -> Kitten {
    Kitten {
        name: name.to_string(),
        job,
        location: Location::Home,
        arrived_day: 0,
        job_seq: job.map(|_| 1),
        away_seq: None,
        skills: [0.0; 8],
        exp: 0.0,
        rank: 0,
        trait_id: KittenTrait::None,
        tinkering: None,
        profile: None,
    }
}

/// A meadow with room for every store: every structure standing, nobody home.
fn roomy(seed: u64) -> GameState {
    let mut s = GameState::new(seed);
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    for r in ResourceId::ALL {
        assert!(
            s.cap(r) > 0.0,
            "fixture: the roomy meadow should have room for {r:?}"
        );
    }
    s
}

/// The barest meadow that has room for `r`: a new one where a new meadow can
/// hold it, the roomy one otherwise.
fn meadow_with_room_for(r: ResourceId, seed: u64) -> GameState {
    let fresh = GameState::new(seed);
    if fresh.cap(r) > 0.0 {
        fresh
    } else {
        roomy(seed)
    }
}

/// Everything the core can put a price in view for, in view — every structure
/// and idea revealed, every idea known, every colony met, a whale visiting, a
/// meadow that has Moved once — and nothing in store.
fn everything_in_view() -> GameState {
    let mut s = GameState::new(48);
    for b in StructureId::ALL {
        s.structures[b.ix()] = 2;
    }
    s.ideas = IdeaId::ALL.into_iter().collect();
    s.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .chain(IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)))
        .collect();
    s.peak_devotion = 1.0e12;
    s.discovered = ColonyId::ALL.into_iter().collect();
    s.whale_visit_years = 5;
    s.generation = 1;
    // Every adventure finished once, so a tradition one opens is in view.
    s.adventure_completions = [1; AdventureId::ALL.len()];
    s.kittens = vec![kitten("Bramble", None)];

    for id in every_content_id() {
        assert_ne!(
            s.availability(id),
            Availability::Hidden,
            "fixture: {id:?} should be in view in the meadow built to show everything"
        );
    }
    for cmd in per_unit_commands() {
        assert!(
            s.unit_price(&cmd).is_some(),
            "fixture: {cmd:?} should be priced here"
        );
    }
    assert!(
        s.command_gate_open(&Command::DeepNap(1)),
        "fixture: the deep nap's gate should be open"
    );
    assert!(
        held_resources(&s).is_empty(),
        "fixture: nothing is in store"
    );
    s
}

/// As `everything_in_view`, with everything that can be the player's already
/// theirs: every refinement, rite, arch, and loom upgrade taken, a house rule
/// adopted.
fn everything_owned() -> GameState {
    let mut s = everything_in_view();
    s.refinements = RefinementId::ALL.into_iter().collect();
    s.rites = RiteId::ALL.into_iter().collect();
    s.rite_counts = [1; 11];
    s.arches_bought = ArchId::ALL.into_iter().collect();
    s.dream_looms = DreamLoomId::ALL.into_iter().collect();
    s.rules = [RuleId::ALL[0]].into_iter().collect();
    s
}

/// A meadow in which a great many commands succeed: everything in view,
/// every store brimming, idle hands at home.
fn rich_and_open() -> GameState {
    let mut s = everything_in_view();
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    s.kittens = (0..6)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    for k in s.kittens.iter_mut().take(2) {
        k.job = Some(JobId::Tinkerer);
        k.exp = 1.0e9;
    }
    s.devotion = 1.0e6;
    s.serenity = 1.0e12;
    s.elder_wisdom = 1.0e9;
    s
}

/// One meadow per way a price comes into view, each with nothing in store, so
/// that whatever is revealed in it was revealed by a price.
fn priced_meadows() -> Vec<(String, GameState)> {
    let mut out: Vec<(String, GameState)> = vec![
        ("a new meadow".into(), GameState::new(48)),
        (
            "a meadow with everything in view".into(),
            everything_in_view(),
        ),
        (
            "a meadow with everything already the player's".into(),
            everything_owned(),
        ),
    ];

    let mut structures_only = GameState::new(48);
    structures_only.revealed = StructureId::ALL
        .iter()
        .map(|b| ContentId::Structure(*b))
        .collect();
    out.push((
        "a meadow whose structures alone are revealed".into(),
        structures_only,
    ));

    let mut ideas_only = GameState::new(48);
    ideas_only.revealed = IdeaId::ALL.iter().map(|i| ContentId::Idea(*i)).collect();
    out.push(("a meadow whose ideas alone are revealed".into(), ideas_only));

    let mut pounce_only = GameState::new(48);
    pounce_only.ideas.insert(IdeaId::PouncingPractice);
    out.push((
        "a meadow that has learnt only to pounce".into(),
        pounce_only,
    ));

    let mut nap_only = GameState::new(48);
    nap_only.ideas.insert(IdeaId::DeepNapTheory);
    out.push((
        "a meadow that has learnt only to nap deeply".into(),
        nap_only,
    ));

    for c in ColonyId::ALL {
        let mut one = GameState::new(48);
        one.discovered.insert(c);
        out.push((format!("a meadow that has met only {c:?}"), one));
    }

    let mut whale = GameState::new(48);
    whale.whale_visit_years = 3;
    out.push(("a meadow with only a whale visiting".into(), whale));

    for (what, s) in &out {
        assert!(
            held_resources(s).is_empty(),
            "fixture: {what} should hold nothing"
        );
    }
    out
}

/// How many variants `Command` has — paired with `every_command`'s own count,
/// so the enumeration cannot silently miss one.
const COMMAND_VARIANTS: usize = 40;

/// Every command there is: each variant, over every id it can name.
fn every_command() -> Vec<Command> {
    let mut out = vec![
        Command::GatherCatnip,
        Command::Forage,
        Command::Rummage,
        Command::ObserveSky,
        Command::SetSpeed(2.0),
        Command::Pounce(1),
        Command::HostJamboree,
        Command::Promote(0),
        Command::AppointElder(0),
        Command::Bask,
        Command::Reverie,
        Command::Enlighten,
        Command::HoldParade,
        Command::RefineStarshard,
        Command::RefineEcho,
        Command::CondenseJar,
        Command::MakePromise,
        Command::ScoutPicnic,
        Command::GreatMeadowMove,
        Command::PassItDown,
        Command::DeclareAdventure(None),
        Command::SummonStarWhale,
        Command::FeedWhale,
        Command::DeepNap(1),
    ];
    out.extend(StructureId::ALL.map(Command::Build));
    out.extend(JobId::ALL.map(Command::AssignJob));
    out.extend(JobId::ALL.map(Command::UnassignJob));
    out.extend(IdeaId::ALL.map(Command::ResearchIdea));
    out.extend(CraftId::ALL.map(Command::Craft));
    out.extend(CraftId::ALL.map(Command::AssignTinkerer));
    out.extend(CraftId::ALL.map(Command::UnassignTinkerer));
    out.extend(RefinementId::ALL.map(Command::BuyRefinement));
    out.extend(RuleId::ALL.map(Command::AdoptRule));
    out.extend(RiteId::ALL.map(Command::BuyRite));
    out.extend(ArchId::ALL.map(Command::BuyArch));
    out.extend(ColonyId::ALL.map(|c| Command::Trade(c, 1)));
    out.extend(ColonyId::ALL.map(Command::SendPenPal));
    out.extend(TraditionId::ALL.map(Command::BuyTradition));
    out.extend(AdventureId::ALL.map(|a| Command::DeclareAdventure(Some(a))));
    out.extend(SiteId::ALL.map(Command::LaunchExpedition));
    out.extend(DreamLoomId::ALL.map(Command::BuyDreamLoom));

    assert_eq!(
        variants_of(&out).len(),
        COMMAND_VARIANTS,
        "fixture: every_command must name every Command variant"
    );
    out
}

fn variants_of(cmds: &[Command]) -> Vec<std::mem::Discriminant<Command>> {
    let mut variants = Vec::new();
    for c in cmds {
        let d = std::mem::discriminant(c);
        if !variants.contains(&d) {
            variants.push(d);
        }
    }
    variants
}

/// A command every meadow accepts and that moves no stock: the plainest
/// "applied command" there is.
const ANY_COMMAND: Command = Command::SetSpeed(1.0);

fn apply_any(s: &mut GameState) {
    s.apply(ANY_COMMAND)
        .expect("fixture: setting the speed is always accepted");
}

/// The revealed resources as the core lists resources: `ResourceId::ALL`'s
/// order.
fn in_core_order(set: &BTreeSet<ResourceId>) -> Vec<ResourceId> {
    ResourceId::ALL
        .into_iter()
        .filter(|r| set.contains(r))
        .collect()
}

fn union(a: &BTreeSet<ResourceId>, b: &BTreeSet<ResourceId>) -> BTreeSet<ResourceId> {
    a.union(b).copied().collect()
}

/// The components of the prices of the items a tick's reveal scan revealed.
fn prices_revealed_by(s: &GameState, events: &[Event]) -> BTreeSet<ResourceId> {
    let mut out = BTreeSet::new();
    for e in events {
        if let Event::Revealed { id } = e {
            out.extend(price_components(s, *id));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SIM-ENG-048 — revealed by holding
// ---------------------------------------------------------------------------

/// Every resource there is reveals the first time a tick leaves some of it in
/// store — by any amount: where nothing in the meadow would drain it, a stock
/// far below the dust guard every other comparison draws is still a holding.
///
/// @spec SIM-ENG-048
#[test]
fn every_resource_is_revealed_the_first_time_a_tick_leaves_some_in_store() {
    // Far under the 1e-9 guard affordability and zero-tests elsewhere draw.
    const DUST: f64 = 1.0e-12;
    let mut dust_holdings = 0;
    for r in ResourceId::ALL {
        let mut s = meadow_with_room_for(r, 480);
        let quiet = s.kittens.is_empty() && s.structures.iter().all(|n| *n == 0);
        let amount = if quiet { DUST } else { 3.0 };
        assert!(
            recorded_set(&s).is_empty(),
            "fixture: a meadow just built has recorded nothing"
        );
        s.resources[r.ix()] = amount;

        s.tick();

        assert!(
            s.stock(r) > 0.0,
            "fixture: {r:?} should still be in store after one tick of a meadow with room for it"
        );
        if quiet {
            dust_holdings += 1;
            assert_eq!(
                s.stock(r),
                DUST,
                "fixture: nothing in a bare meadow moves {r:?}"
            );
        }
        assert!(
            recorded_set(&s).contains(&r),
            "{r:?} stood at {} after a tick and was not revealed: a stock above zero after a tick \
             is a holding, by any amount and with no dust guard",
            s.stock(r)
        );
        assert!(
            s.resource_revealed(r),
            "{r:?} is held and recorded, and the query says it is not revealed"
        );
    }
    assert!(
        dust_holdings > 0,
        "fixture: some resource should be held as dust in a bare meadow"
    );
}

/// A command that leaves some in store reveals it before it returns: no tick
/// has run, and none is needed.
///
/// @spec SIM-ENG-048
#[test]
fn a_resource_is_revealed_the_first_time_an_applied_command_leaves_some_in_store() {
    let mut s = GameState::new(481);
    assert!(
        recorded_set(&s).is_empty(),
        "a new meadow has revealed nothing"
    );

    s.apply(Command::GatherCatnip)
        .expect("gathering is always possible on an empty meadow");

    let held = held_resources(&s);
    assert!(
        !held.is_empty(),
        "fixture: gathering leaves something in store"
    );
    assert_eq!(s.ticks, 0, "fixture: no tick has run");
    assert_eq!(
        recorded_set(&s),
        held,
        "a command left {held:?} in store and the meadow recorded {:?}: a holding is revealed \
         after an applied command, not at the next tick",
        recorded_set(&s)
    );
}

/// Production into a store whose cap is zero is clamped away in the tick that
/// made it, and is no holding: the tick reads the stock after its clamp.
///
/// @spec SIM-ENG-048
#[test]
fn production_clamped_away_in_the_tick_that_made_it_is_no_holding() {
    let mut s = GameState::new(482);
    s.kittens = vec![kitten("Ember", Some(JobId::Sunwarden))];
    let made = ResourceId::Warmth;
    assert!(
        s.net_rate(made) > 0.0,
        "fixture: the kitten's work should produce {made:?}"
    );
    assert_eq!(
        s.cap(made),
        0.0,
        "fixture: and the meadow should have nowhere to keep it"
    );
    assert!(
        !priced_resources(&s).contains(&made),
        "fixture: no price in view names {made:?}"
    );

    for _ in 0..50 {
        s.tick();
        assert_eq!(s.stock(made), 0.0, "fixture: the clamp leaves none of it");
        assert!(
            !recorded_set(&s).contains(&made),
            "{made:?} was revealed by production the same tick's clamp took away: a tick reads \
             the stock after its clamp"
        );
        assert!(
            !s.resource_revealed(made),
            "the query calls {made:?} revealed; the meadow never held any"
        );
    }
}

/// The same line for every store a new meadow cannot keep: a stock set above
/// a zero cap is gone by the end of the tick, and the tick records nothing.
///
/// @spec SIM-ENG-048
#[test]
fn a_stock_the_clamp_takes_away_is_never_recorded_as_held() {
    let fresh = GameState::new(483);
    let no_room: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .filter(|r| fresh.cap(*r) == 0.0)
        .collect();
    assert!(
        !no_room.is_empty(),
        "fixture: a new meadow should have stores it cannot keep anything in"
    );
    for r in no_room {
        let mut s = fresh.clone();
        s.resources[r.ix()] = 5.0;
        s.tick();
        assert_eq!(s.stock(r), 0.0, "fixture: the clamp takes all of {r:?}");
        assert!(
            !recorded_set(&s).contains(&r),
            "{r:?} was recorded as held though the tick's clamp left none"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-049 — revealed by a price the core would state
// ---------------------------------------------------------------------------

/// Every resource a stated price names is revealed, whatever its stock, and —
/// in a meadow holding nothing — no other is. Quantified over every way a
/// price comes into view: every content item of every kind, a pounce, a trade
/// with every colony and the whale, and the deep nap, each alone and all at
/// once.
///
/// @spec SIM-ENG-049, SIM-ENG-050
#[test]
fn every_resource_a_stated_price_names_is_revealed_whatever_its_stock_and_no_other() {
    let mut items_in_view: BTreeSet<ContentId> = BTreeSet::new();
    let mut commands_priced: Vec<Command> = Vec::new();
    let mut nap_open = false;
    let mut price_only_reveals = 0;

    for (what, meadow) in priced_meadows() {
        let mut s = meadow;
        let priced = priced_resources(&s);
        for id in every_content_id() {
            if s.availability(id) != Availability::Hidden && !price_components(&s, id).is_empty() {
                items_in_view.insert(id);
            }
        }
        for cmd in per_unit_commands() {
            if s.unit_price(&cmd).is_some() && !commands_priced.contains(&cmd) {
                commands_priced.push(cmd);
            }
        }
        nap_open |= s.command_gate_open(&Command::DeepNap(1));

        apply_any(&mut s);

        assert!(
            held_resources(&s).is_empty(),
            "fixture: {what} still holds nothing"
        );
        assert_eq!(
            recorded_set(&s),
            priced,
            "in {what}, the resources revealed are not the resources its stated prices name \
             (left: revealed, right: named in a price)"
        );
        for r in ResourceId::ALL {
            assert_eq!(
                s.resource_revealed(r),
                priced.contains(&r),
                "in {what}, {r:?} stands at nothing and the query's answer disagrees with whether \
                 a stated price names it"
            );
        }
        assert_eq!(s.resources_revealed(), in_core_order(&priced), "in {what}");
        price_only_reveals += priced.len();
    }

    // The quantified set was covered: every item that has a price, every
    // per-unit command, and the nap.
    for id in every_content_id() {
        if !price_components(&everything_in_view(), id).is_empty() {
            assert!(
                items_in_view.contains(&id),
                "fixture: no meadow put {id:?}'s price in view"
            );
        }
    }
    for cmd in per_unit_commands() {
        assert!(
            commands_priced.contains(&cmd),
            "fixture: no meadow priced {cmd:?}"
        );
    }
    assert!(nap_open, "fixture: no meadow opened the deep nap's gate");
    assert!(
        price_only_reveals > 0,
        "fixture: no price revealed anything"
    );
}

/// An item that is already the player's still states a price, and that price
/// still reveals: the row stays listed once the thing is theirs.
///
/// @spec SIM-ENG-049
#[test]
fn the_price_of_an_item_already_the_players_still_reveals_what_it_names() {
    let mut s = everything_owned();
    let owned: Vec<ContentId> = every_content_id()
        .into_iter()
        .filter(|id| {
            let yours = s.lock_reason(*id) == Some(LockReason::AlreadyYours);
            let built = matches!(id, ContentId::Structure(b) if s.structures[b.ix()] > 0);
            yours || built
        })
        .collect();
    for kind in [
        "Structure",
        "Idea",
        "Refinement",
        "Rule",
        "Rite",
        "Arch",
        "DreamLoom",
    ] {
        assert!(
            owned.iter().any(|id| format!("{id:?}").starts_with(kind)),
            "fixture: some {kind} should already be the player's"
        );
    }

    apply_any(&mut s);

    let recorded = recorded_set(&s);
    for id in owned {
        for r in price_components(&s, id) {
            assert_eq!(s.stock(r), 0.0, "fixture: nothing is in store");
            assert!(
                recorded.contains(&r),
                "{id:?} is already the player's and its price names {r:?}, which was not revealed"
            );
        }
    }
}

/// A content item the reveal scan reveals on a tick has its price's resources
/// revealed by that same tick, before any is held.
///
/// @spec SIM-ENG-049, SIM-ENG-054
#[test]
fn an_item_a_ticks_scan_reveals_brings_the_stores_its_price_names_with_it() {
    let mut s = GameState::new(491);
    grant(&mut s, StructureId::StudyNook, 1);
    assert!(recorded_set(&s).is_empty(), "fixture: nothing recorded yet");

    let events = s.tick();

    let named = prices_revealed_by(&s, &events);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Revealed {
                id: ContentId::Idea(_)
            }
        )),
        "fixture: the tick's scan should reveal an idea; events: {events:?}"
    );
    let unheld: Vec<ResourceId> = named
        .iter()
        .copied()
        .filter(|r| s.stock(*r) == 0.0)
        .collect();
    assert!(
        !unheld.is_empty(),
        "fixture: a revealed item's price should name a store standing at nothing"
    );

    let recorded = recorded_set(&s);
    for r in &named {
        assert!(
            recorded.contains(r),
            "the tick revealed an item whose price names {r:?} and did not reveal {r:?} with it"
        );
        assert!(s.resource_revealed(*r));
    }
    // And nothing the rule does not name: what the meadow holds, what the
    // revealed items' prices name, and at most what other stated prices name.
    let allowed = union(&held_resources(&s), &priced_resources(&s));
    for r in &recorded {
        assert!(
            allowed.contains(r),
            "{r:?} was revealed though neither held nor named in a price"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-050 — and by no other route
// ---------------------------------------------------------------------------

/// A meadow that has held nothing and been shown no price has revealed
/// nothing, however long it waits.
///
/// @spec SIM-ENG-050
#[test]
fn a_resource_neither_held_nor_named_in_a_price_is_not_revealed() {
    let mut s = GameState::new(500);
    for step in 0..=200 {
        assert!(
            held_resources(&s).is_empty() && priced_resources(&s).is_empty(),
            "fixture: an idle new meadow"
        );
        assert!(
            recorded_set(&s).is_empty(),
            "after {step} ticks a new meadow has recorded {:?}",
            recorded_set(&s)
        );
        assert!(
            s.resources_revealed().is_empty(),
            "after {step} ticks the query lists {:?}",
            s.resources_revealed()
        );
        for r in ResourceId::ALL {
            assert!(
                !s.resource_revealed(r),
                "after {step} ticks {r:?} is revealed in a meadow that never met it"
            );
        }
        s.tick();
    }
}

/// A refused command reveals nothing — every command, in meadows that refuse
/// them for every kind of reason — even where the refusal itself names a
/// resource the player has never seen.
///
/// @spec SIM-ENG-050
#[test]
fn a_refused_command_reveals_nothing_even_where_its_refusal_names_a_resource() {
    let mut recorded_prices = everything_in_view();
    apply_any(&mut recorded_prices);
    let mut a_little = GameState::new(501);
    a_little.apply(Command::GatherCatnip).unwrap();

    // Built by hand: holding a little and shown a price, none of it recorded
    // — so a refusal that recorded anything would show.
    let mut unrecorded = GameState::new(501);
    unrecorded.resources[ResourceId::Catnip.ix()] = 3.0;
    unrecorded.ideas.insert(IdeaId::PouncingPractice);
    assert!(
        !held_resources(&unrecorded).is_empty() && !priced_resources(&unrecorded).is_empty(),
        "fixture: the hand-built meadow holds something and shows a price"
    );

    let meadows = [
        ("a new meadow", GameState::new(501)),
        ("a meadow that has gathered once", a_little),
        (
            "a meadow with everything in view and nothing in store",
            recorded_prices,
        ),
        ("a meadow built by hand, nothing of it recorded", unrecorded),
    ];

    let mut refused = 0;
    let mut named_an_unrevealed_store = 0;
    for (what, meadow) in meadows {
        for cmd in every_command() {
            let mut s = meadow.clone();
            let before = recorded_resources(&s);
            let listed = s.resources_revealed();
            let Err(err) = s.apply(cmd) else { continue };
            refused += 1;
            if let CommandError::InsufficientResources { missing } = &err {
                for (r, _) in missing {
                    if !meadow.resource_revealed(*r) {
                        named_an_unrevealed_store += 1;
                        assert!(
                            !s.resource_revealed(*r),
                            "in {what}, {cmd:?} was refused for want of {r:?} and the refusal \
                             revealed it: a refused command mutates nothing"
                        );
                    }
                }
            }
            assert_eq!(
                recorded_resources(&s),
                before,
                "in {what}, the refused {cmd:?} changed what is revealed"
            );
            assert_eq!(
                s.resources_revealed(),
                listed,
                "in {what}, the refused {cmd:?} changed what the query lists"
            );
            assert_eq!(
                s, meadow,
                "in {what}, the refused {cmd:?} changed the meadow"
            );
        }
    }
    assert!(
        refused > 100,
        "fixture: only {refused} commands were refused"
    );
    assert!(
        named_an_unrevealed_store > 0,
        "fixture: some refusal should name a resource its meadow has not revealed"
    );
}

/// What a scripted game saw along the way.
struct Played {
    state: GameState,
    /// Every resource the rule has named so far.
    met: BTreeSet<ResourceId>,
    /// Sightings of a revealed store standing at nothing.
    spent_out: usize,
    /// Sightings of a stated price naming a store standing at nothing.
    shown_before_held: usize,
}

/// Play a scripted game on from `start`, a day a round: gather, build and
/// learn and assign whatever it can, try every other command the game has,
/// forage, and let the day pass. After every accepted command and every tick
/// the record is compared with a running union built from the rule alone —
/// holdings after each step, stated prices after each command, and the
/// prices of the items each tick's scan revealed — and with what the meadow
/// holds and shows at that moment.
fn play_checking_every_step(start: GameState, rounds: usize) -> Played {
    let mut s = start;
    let mut expected = recorded_set(&s);
    assert_eq!(
        expected,
        union(&held_resources(&s), &priced_resources(&s)),
        "fixture: the game starts with exactly its holdings and stated prices revealed"
    );
    // Growth first, so the meadow gets somewhere; then everything else the
    // game has, bar the commands that take work or time away.
    let grows = |c: &Command| {
        matches!(
            c,
            Command::Build(_) | Command::ResearchIdea(_) | Command::AssignJob(_)
        )
    };
    let skipped = |c: &Command| {
        matches!(
            c,
            Command::DeepNap(_)
                | Command::SetSpeed(_)
                | Command::UnassignJob(_)
                | Command::UnassignTinkerer(_)
                | Command::GreatMeadowMove
                | Command::GatherCatnip
                | Command::Forage
        )
    };
    let mut script: Vec<Command> = every_command().into_iter().filter(|c| grows(c)).collect();
    script.extend(
        every_command()
            .into_iter()
            .filter(|c| !grows(c) && !skipped(c)),
    );
    script.push(Command::Forage);
    script.push(Command::Forage);

    let check = |s: &GameState, expected: &BTreeSet<ResourceId>, after: &str| {
        let recorded = recorded_set(s);
        assert_eq!(
            &recorded, expected,
            "after {after} at tick {}, the revealed resources differ from what the meadow has \
             held or been shown in a price so far (left: revealed, right: the rule)",
            s.ticks
        );
        for r in union(&held_resources(s), &priced_resources(s)) {
            assert!(
                recorded.contains(&r),
                "after {after} at tick {}, {r:?} is held or named in a stated price and is not \
                 revealed: a caller could observe a store, or a priced row, with no store listed",
                s.ticks
            );
        }
        assert_eq!(
            s.resources_revealed(),
            in_core_order(&recorded),
            "after {after}: the query's list"
        );
    };

    let mut spent_out = 0;
    let mut shown_before_held = 0;
    for round in 0..rounds {
        for _ in 0..40 {
            if s.apply(Command::GatherCatnip).is_ok() {
                expected.extend(held_resources(&s));
                expected.extend(priced_resources(&s));
            }
        }
        check(&s, &expected, "gathering");
        for cmd in &script {
            if s.apply(*cmd).is_ok() {
                expected.extend(held_resources(&s));
                expected.extend(priced_resources(&s));
                check(&s, &expected, &format!("{cmd:?}"));
            }
        }
        for _ in 0..10 {
            let events = s.tick();
            expected.extend(held_resources(&s));
            expected.extend(prices_revealed_by(&s, &events));
            check(&s, &expected, "a tick");
        }
        if round % 10 == 0 {
            spent_out += expected.iter().filter(|r| s.stock(**r) == 0.0).count();
            shown_before_held += priced_resources(&s)
                .iter()
                .filter(|r| s.stock(**r) == 0.0)
                .count();
        }
    }
    Played {
        state: s,
        met: expected,
        spent_out,
        shown_before_held,
    }
}

/// Under play from a new meadow, the revealed resources are at every moment
/// exactly what the two routes have named so far — and no state the game
/// passes through shows a held store or a priced row whose resources are
/// unrevealed.
///
/// @spec SIM-ENG-050, SIM-ENG-054
#[test]
fn under_play_from_a_new_game_the_revealed_resources_are_exactly_what_was_held_or_priced() {
    let played = play_checking_every_step(GameState::new(502), 120);

    let s = &played.state;
    assert!(
        played.met.len() >= 4,
        "fixture: the game should meet several stores, met {:?}",
        played.met
    );
    assert!(
        played.met.len() < ResourceId::ALL.len(),
        "fixture: and should not have met them all"
    );
    assert!(
        !s.ideas.is_empty() && !s.kittens.is_empty(),
        "fixture: the meadow should have grown"
    );
    assert!(
        played.shown_before_held > 0,
        "fixture: some price should have named a store standing at nothing"
    );
}

/// The same, played on from a meadow part-way along — built by hand, its
/// holdings and prices recorded by one command, and organic from there — so
/// the game reaches the gates a new meadow is a long way from: ideas that
/// open commands, crafts, and whole sections.
///
/// @spec SIM-ENG-050, SIM-ENG-054
#[test]
fn under_play_from_a_game_part_way_along_the_revealed_resources_are_exactly_what_was_held_or_priced(
) {
    let mut s = GameState::new(503);
    for b in StructureId::ALL.into_iter().take(28) {
        s.structures[b.ix()] = 3;
    }
    s.kittens = (0..12)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    for r in ResourceId::ALL.into_iter().take(5) {
        s.resources[r.ix()] = s.cap(r).min(400.0);
    }
    apply_any(&mut s);
    let ideas_at_start = s.ideas.len();
    let met_at_start = recorded_set(&s).len();

    let played = play_checking_every_step(s, 80);

    let s = &played.state;
    assert!(
        s.ideas.len() >= ideas_at_start + 5,
        "fixture: the meadow should have learnt several ideas"
    );
    assert!(
        played.met.len() > met_at_start,
        "fixture: and met stores it had not at the start"
    );
    assert!(
        played.met.len() < ResourceId::ALL.len(),
        "fixture: without meeting them all"
    );
    assert!(
        played.spent_out > 0,
        "fixture: some revealed store should have stood at nothing along the way"
    );
    assert!(
        played.shown_before_held > 0,
        "fixture: some price should have named a store standing at nothing"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-051 — revelation is permanent for the meadow
// ---------------------------------------------------------------------------

/// Every resource, once revealed, stays revealed when its stock falls to
/// zero: in the record, and in what the query answers.
///
/// @spec SIM-ENG-051
#[test]
fn every_revealed_resource_stays_revealed_when_its_stock_falls_to_zero() {
    for r in ResourceId::ALL {
        let mut s = meadow_with_room_for(r, 510);
        s.resources[r.ix()] = 3.0;
        s.tick();
        assert!(s.stock(r) > 0.0, "fixture: {r:?} is held after a tick");

        s.resources[r.ix()] = 0.0;

        assert!(
            recorded_set(&s).contains(&r),
            "{r:?} was held, and is missing from the record now that none is left"
        );
        assert!(
            s.resource_revealed(r),
            "{r:?} was spent to nothing and the query no longer calls it revealed: a reveal that \
             can un-reveal takes the store's row out from under the cursor"
        );
        assert!(
            s.resources_revealed().contains(&r),
            "{r:?} left the query's list when it was spent"
        );

        // Neither time nor a command takes it back out.
        for _ in 0..30 {
            s.tick();
            assert!(
                recorded_set(&s).contains(&r),
                "{r:?} was un-revealed by a tick at tick {}",
                s.ticks
            );
        }
        apply_any(&mut s);
        assert!(
            recorded_set(&s).contains(&r) && s.resource_revealed(r),
            "{r:?} was un-revealed by a command"
        );
    }
}

/// The player's own case: a store spent to exactly nothing by a command, with
/// no tick between the gain and the spend, is still revealed.
///
/// @spec SIM-ENG-051, SIM-ENG-054
#[test]
fn a_store_gained_and_spent_to_nothing_between_two_ticks_stays_revealed() {
    let mut s = GameState::new(511);
    for _ in 0..10 {
        s.apply(Command::GatherCatnip).unwrap();
    }
    let spent: Vec<ResourceId> = held_resources(&s).into_iter().collect();
    s.apply(Command::Forage)
        .expect("fixture: ten gathers should pay for one forage");
    assert_eq!(
        s.ticks, 0,
        "fixture: no tick has run between the gain and the spend"
    );
    for r in &spent {
        assert_eq!(
            s.stock(*r),
            0.0,
            "fixture: the forage should spend {r:?} to exactly nothing"
        );
        assert!(
            recorded_set(&s).contains(r),
            "{r:?} was gained and spent before any tick, and was never recorded: a holding is \
             recorded after every applied command, so a spend before the next tick cannot un-list it"
        );
        assert!(s.resource_revealed(*r));
    }
    let gained = held_resources(&s);
    assert!(
        !gained.is_empty(),
        "fixture: the forage leaves something else in store"
    );
    assert_eq!(
        recorded_set(&s),
        union(&gained, &spent.iter().copied().collect())
    );

    for _ in 0..100 {
        s.tick();
    }
    for r in &spent {
        assert!(
            s.resource_revealed(*r),
            "{r:?} un-revealed while it stood at nothing"
        );
    }
}

/// A resource a price named stays revealed when that price is no longer
/// shown — every one of them, when every gate that showed a price is shut.
///
/// @spec SIM-ENG-051
#[test]
fn a_resource_a_price_named_stays_revealed_when_the_price_is_no_longer_shown() {
    let mut s = everything_in_view();
    apply_any(&mut s);
    let once_named = recorded_set(&s);
    assert_eq!(
        once_named,
        priced_resources(&s),
        "fixture: revealed by prices alone"
    );
    assert!(once_named.len() > 10, "fixture: a good many stores named");

    // Every item and command that named them leaves the player's view.
    s.structures = GameState::new(0).structures;
    s.ideas.clear();
    s.revealed.clear();
    s.discovered.clear();
    s.whale_visit_years = 0;
    s.generation = 0;
    s.peak_devotion = 0.0;
    s.kittens.clear();
    assert!(
        priced_resources(&s).is_empty(),
        "fixture: no price is in view now: {:?}",
        priced_resources(&s)
    );

    for r in &once_named {
        assert!(
            s.resource_revealed(*r),
            "{r:?} un-revealed the moment the price naming it left view"
        );
    }
    for _ in 0..40 {
        s.tick();
    }
    apply_any(&mut s);
    assert_eq!(
        recorded_set(&s),
        union(&once_named, &held_resources(&s)),
        "resources a price once named left the record after the price stopped being shown"
    );
    for r in &once_named {
        assert!(
            s.resource_revealed(*r),
            "{r:?} un-revealed after its price stopped being shown"
        );
    }
}

/// A whale's visit ending takes its exchange off the screen and leaves the
/// store its price named where it was.
///
/// @spec SIM-ENG-051
#[test]
fn a_per_unit_price_leaving_view_on_a_tick_leaves_the_store_it_named_revealed() {
    let mut s = GameState::new(512);
    s.whale_visit_years = 1;
    let exchange = Command::Trade(ColonyId::StarWhales, 1);
    let named: Vec<ResourceId> = s
        .unit_price(&exchange)
        .expect("fixture: a visiting whale states a price")
        .into_iter()
        .map(|(r, _)| r)
        .collect();
    apply_any(&mut s);
    for r in &named {
        assert_eq!(s.stock(*r), 0.0, "fixture: the meadow holds none of {r:?}");
        assert!(
            s.resource_revealed(*r),
            "fixture: the whale's price reveals {r:?}"
        );
    }

    let mut waited = 0;
    while s.whale_visit_years > 0 {
        s.tick();
        waited += 1;
        assert!(
            waited < 5_000,
            "fixture: the whale's visit should end within its year"
        );
    }
    assert!(
        s.unit_price(&exchange).is_none(),
        "fixture: no whale, no price"
    );

    for r in &named {
        assert!(
            s.resource_revealed(*r) && recorded_set(&s).contains(r),
            "{r:?} was un-revealed when the whale's visit ended"
        );
    }
}

/// The set is persisted in id order, whatever order the meadow met its
/// stores in, each resource once.
///
/// @spec SIM-ENG-051
#[test]
fn the_revealed_resources_are_persisted_as_a_set_in_id_order() {
    let mut s = GameState::new(513);
    let meet: Vec<ResourceId> = ResourceId::ALL
        .into_iter()
        .rev()
        .step_by(3)
        .filter(|r| s.cap(*r) > 0.0)
        .collect();
    assert!(
        meet.len() >= 8,
        "fixture: several stores to meet, last id first"
    );
    for r in &meet {
        s.resources[r.ix()] = 2.0;
        s.tick();
        s.tick();
        s.resources[r.ix()] = 0.0;
    }

    let written = recorded_resources(&s);
    let mut sorted = written.clone();
    sorted.sort_by_key(|r| r.ix());
    sorted.dedup();
    assert_eq!(
        written, sorted,
        "the revealed resources are written out of id order, or twice"
    );
    assert_eq!(
        written.iter().copied().collect::<BTreeSet<_>>(),
        meet.iter().copied().collect::<BTreeSet<_>>(),
        "the record should hold exactly the stores this meadow met"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-052 — a Great Meadow Move
// ---------------------------------------------------------------------------

/// A Move empties the revealed resources with everything else it resets, and
/// reveals afresh from the meadow that results before the command returns: the
/// stores the Memory Chest filled are revealed from the new meadow's first
/// frame, and the stores only the old meadow met are not.
///
/// @spec SIM-ENG-052
#[test]
fn a_reset_empties_the_revealed_resources_and_reveals_afresh_from_what_results() {
    let mut s = everything_in_view();
    s.kittens = (0..MEADOW_MOVE_MIN_KITTENS + 3)
        .map(|i| kitten(&format!("kitten-{i}"), None))
        .collect();
    s.structures[StructureId::MemoryChest.ix()] = 6;
    let carried = [
        ResourceId::ALL[0],
        ResourceId::ALL[1],
        ResourceId::ALL[4],
        ResourceId::Reverie,
    ];
    for r in carried {
        s.resources[r.ix()] = 4.0e5;
    }
    assert!(
        s.memory_chest_ratio() > 0.0,
        "fixture: the chest carries something across"
    );
    apply_any(&mut s);
    let old = recorded_set(&s);
    assert!(
        old.len() > carried.len() + 5,
        "fixture: the old meadow has revealed a good many stores"
    );

    let events = s
        .apply(Command::GreatMeadowMove)
        .expect("fixture: a clowder this size can Move");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::MeadowMoved { .. })),
        "fixture: the Move happened"
    );
    assert_eq!(s.ticks, 0, "fixture: no tick of the new meadow has run");

    let held = held_resources(&s);
    for r in carried {
        assert!(
            held.contains(&r),
            "fixture: the new meadow should hold some {r:?}"
        );
    }
    let afresh = union(&held, &priced_resources(&s));
    let recorded = recorded_set(&s);
    assert_eq!(
        recorded, afresh,
        "after a Move the revealed resources should be exactly what the new meadow holds or is \
         shown in a price, recorded before the command returns (left: revealed, right: the rule)"
    );
    let forgotten: Vec<&ResourceId> = old.difference(&recorded).collect();
    assert!(
        !forgotten.is_empty(),
        "fixture: the old meadow should have revealed stores the new one has not met"
    );
    for r in forgotten {
        assert!(
            !s.resource_revealed(*r),
            "{r:?} is still revealed after the Move that reset the meadow"
        );
    }
    assert!(
        s.revealed.is_empty(),
        "fixture: the content reveal set is emptied by the same Move"
    );
    assert_eq!(s.resources_revealed(), in_core_order(&afresh));

    // The new meadow's reveals are as permanent as the old one's were.
    for r in carried {
        s.resources[r.ix()] = 0.0;
        assert!(
            s.resource_revealed(r),
            "{r:?}, carried in the chest, un-revealed when the new meadow spent it"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-053 — the queries
// ---------------------------------------------------------------------------

/// The query answers revealed for whatever the meadow holds at the asking,
/// before any tick or command has recorded it — and asking changes nothing.
///
/// @spec SIM-ENG-053
#[test]
fn the_query_answers_revealed_for_a_holding_no_record_has_been_made_of() {
    let fresh = GameState::new(530);
    for r in ResourceId::ALL {
        let mut s = fresh.clone();
        // Held by hand: above its cap where it has none, as dust elsewhere.
        s.resources[r.ix()] = if fresh.cap(r) > 0.0 { 1.0e-12 } else { 4.0 };
        let snapshot = s.clone();

        assert!(
            s.resource_revealed(r),
            "{r:?} stands above zero and the query calls it unrevealed: a state built or edited \
             by hand is judged on what it holds"
        );
        assert_eq!(
            s.resources_revealed(),
            vec![r],
            "the query lists what is held and nothing else"
        );
        for other in ResourceId::ALL {
            if other != r {
                assert!(
                    !s.resource_revealed(other),
                    "{other:?} is revealed in a meadow holding only {r:?}"
                );
            }
        }
        assert_eq!(
            s, snapshot,
            "asking changed the meadow: the queries are read-only"
        );
        assert!(
            recorded_set(&s).is_empty(),
            "and no record was made by asking"
        );
    }
}

/// The list is the persisted set and the holdings together, in the order the
/// core lists resources, and agrees with the one-resource query for every
/// resource — over meadows that reveal by holding, by price, by both, and
/// that have spent what they revealed.
///
/// @spec SIM-ENG-053
#[test]
fn the_list_is_the_recorded_set_and_the_holdings_together_in_the_cores_order() {
    let mut meadows: Vec<(String, GameState)> = priced_meadows();
    for (_, s) in meadows.iter_mut() {
        apply_any(s);
    }
    // Revealed by price, with a scattering of unrecorded holdings on top.
    let mut mixed = everything_in_view();
    apply_any(&mut mixed);
    for r in ResourceId::ALL.into_iter().step_by(4) {
        mixed.resources[r.ix()] = 7.0;
    }
    meadows.push(("prices recorded, holdings not".into(), mixed));
    // Held, recorded, and spent; then other things held and unrecorded.
    let mut spent = roomy(531);
    for r in ResourceId::ALL.into_iter().step_by(3) {
        spent.resources[r.ix()] = 2.0;
    }
    spent.tick();
    for r in ResourceId::ALL {
        spent.resources[r.ix()] = 0.0;
    }
    for r in ResourceId::ALL.into_iter().skip(1).step_by(5) {
        spent.resources[r.ix()] = 1.0;
    }
    meadows.push((
        "held, recorded, spent, and holding other things".into(),
        spent,
    ));
    meadows.push((
        "a brimming meadow nothing has recorded".into(),
        rich_and_open(),
    ));

    let mut recorded_only = 0;
    let mut held_only = 0;
    for (what, s) in &meadows {
        let recorded = recorded_set(s);
        let held = held_resources(s);
        recorded_only += recorded.difference(&held).count();
        held_only += held.difference(&recorded).count();
        let both = union(&recorded, &held);

        let list = s.resources_revealed();
        assert_eq!(
            list,
            in_core_order(&both),
            "in {what}, the query's list is not the recorded set and the holdings together, in \
             the order the core lists resources"
        );
        for r in ResourceId::ALL {
            assert_eq!(
                s.resource_revealed(r),
                both.contains(&r),
                "in {what}, the query's answer for {r:?} disagrees with the record and the stock"
            );
            assert_eq!(
                s.resource_revealed(r),
                list.contains(&r),
                "in {what}, the two queries disagree on {r:?}"
            );
        }
    }
    assert!(
        recorded_only > 0,
        "fixture: some revealed store should stand at nothing"
    );
    assert!(
        held_only > 0,
        "fixture: some held store should be unrecorded"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-054 — where a reveal is recorded
// ---------------------------------------------------------------------------

/// Every tick records the holdings its clamp leaves — the live tick and the
/// replayed one alike, and to the same effect.
///
/// @spec SIM-ENG-054
#[test]
fn every_tick_live_or_replayed_records_the_holdings_its_clamp_leaves() {
    let mut live = roomy(540);
    live.kittens = (0..4)
        .map(|i| kitten(&format!("kitten-{i}"), Some(JobId::ALL[i])))
        .collect();
    for r in ResourceId::ALL.into_iter().step_by(2) {
        live.resources[r.ix()] = 6.0;
    }
    let mut replayed = live.clone();
    assert!(
        recorded_set(&live).is_empty(),
        "fixture: nothing recorded yet"
    );

    for tick in 1..=30 {
        live.tick();
        replayed.tick_replay();
        let held = held_resources(&live);
        assert!(!held.is_empty(), "fixture: the meadow holds things");
        for r in &held {
            assert!(
                recorded_set(&live).contains(r),
                "tick {tick}: the live tick left {r:?} in store and did not record it"
            );
        }
        assert_eq!(
            recorded_resources(&replayed),
            recorded_resources(&live),
            "tick {tick}: a replayed tick recorded different holdings from the live one"
        );
    }
}

/// A catch-up records as it replays: a store the absence filled and emptied
/// again is revealed when the player returns, though it stands at nothing.
///
/// @spec SIM-ENG-054
#[test]
fn a_catch_up_records_the_holdings_of_the_ticks_it_replays() {
    // A kitten with nothing to eat but a morsel: the morsel is gone within the
    // first replayed ticks.
    let mut s = GameState::new(541);
    s.kittens = vec![kitten("Morsel", None)];
    apply_any(&mut s);
    assert!(
        recorded_set(&s).is_empty(),
        "fixture: nothing revealed at the save"
    );
    let food = ResourceId::Catnip;
    let json = {
        let mut v: serde_json::Value = serde_json::from_str(&save_string(&s, 1_000)).unwrap();
        v["state"]["resources"][food.ix()] = serde_json::json!(0.1);
        v.to_string()
    };

    let loaded = load_string(&json, 1_000 + 600)
        .expect("the save loads")
        .state;

    assert!(
        loaded.ticks > s.ticks,
        "fixture: the catch-up replayed ticks"
    );
    assert_eq!(
        loaded.stock(food),
        0.0,
        "fixture: the morsel was eaten while away"
    );
    assert!(
        recorded_set(&loaded).contains(&food),
        "{food:?} was held through replayed ticks and never recorded: a holding is recorded at \
         the end of every tick, live or replayed"
    );
}

/// After every applied command — every one the game has — every holding and
/// every stated price is recorded before the command returns.
///
/// In the brimming meadow every store is held and none recorded, so any
/// accepted command must leave every one of them recorded. In the meadow with
/// everything in view and nothing in store, an accepted command must leave
/// exactly its stated prices and its holdings recorded.
///
/// @spec SIM-ENG-054
#[test]
fn every_applied_command_leaves_every_holding_and_every_stated_price_recorded() {
    let brimming = rich_and_open();
    let bare = everything_in_view();
    let mut accepted: Vec<Command> = Vec::new();

    for cmd in every_command() {
        let mut s = brimming.clone();
        if s.apply(cmd).is_ok() {
            accepted.push(cmd);
            let recorded = recorded_set(&s);
            for r in union(&held_resources(&s), &priced_resources(&s)) {
                assert!(
                    recorded.contains(&r),
                    "{cmd:?} was applied to a brimming meadow and returned with {r:?} held or \
                     priced and not recorded"
                );
            }
        }

        let mut s = bare.clone();
        if s.apply(cmd).is_ok() {
            if !accepted.contains(&cmd) {
                accepted.push(cmd);
            }
            assert_eq!(
                recorded_set(&s),
                union(&held_resources(&s), &priced_resources(&s)),
                "{cmd:?} was applied to a meadow holding nothing and returned with the record \
                 differing from its holdings and stated prices (left: recorded, right: the rule)"
            );
        }
    }

    let variants = variants_of(&accepted).len();
    assert!(
        variants >= 25,
        "fixture: only {variants} of {COMMAND_VARIANTS} command variants were accepted somewhere; \
         accepted: {accepted:?}"
    );
}

/// Put in store, by hand, whatever `cmd` is refused for want of, until it
/// would be accepted. False where it is refused for any other reason.
fn stock_what_it_costs(s: &mut GameState, cmd: Command) -> bool {
    for _ in 0..8 {
        match s.clone().apply(cmd) {
            Ok(_) => return true,
            Err(CommandError::InsufficientResources { missing }) if !missing.is_empty() => {
                for (r, short) in missing {
                    s.resources[r.ix()] += short;
                }
            }
            Err(_) => return false,
        }
    }
    false
}

/// The gates that put a priced row in front of the player open on a command,
/// and the command that opens one reveals the stores its prices name before
/// it returns: an idea researched, a peak of devotion reached by basking, a
/// colony discovered, a whale summoned. Each meadow holds only what its
/// command costs, so a store the new price names is revealed by the price
/// and by nothing else.
///
/// @spec SIM-ENG-054
#[test]
fn a_command_that_opens_a_gate_reveals_the_prices_it_brings_into_view() {
    let mut cases: Vec<(&str, GameState, Command)> = Vec::new();

    let mut learning = GameState::new(542);
    grant(&mut learning, StructureId::StudyNook, 1);
    learning.ideas = IdeaId::ALL
        .into_iter()
        .filter(|i| *i != IdeaId::PouncingPractice)
        .collect();
    learning
        .revealed
        .insert(ContentId::Idea(IdeaId::PouncingPractice));
    cases.push((
        "researching an idea",
        learning,
        Command::ResearchIdea(IdeaId::PouncingPractice),
    ));

    let mut basking = GameState::new(542);
    grant(&mut basking, StructureId::SunRoom, 4);
    basking.resources[ResourceId::Warmth.ix()] = 400.0;
    cases.push(("basking to a peak of devotion", basking, Command::Bask));

    let mut summoning = GameState::new(542);
    grant(&mut summoning, StructureId::SkyDock, 1);
    summoning.ideas.insert(IdeaId::SkywardSong);
    cases.push(("summoning a whale", summoning, Command::SummonStarWhale));

    for seed in 0..64 {
        let mut scouting = GameState::new(seed);
        scouting.ideas.insert(IdeaId::GiftEconomy);
        cases.push(("scouting a picnic", scouting, Command::ScoutPicnic));
    }

    let mut opened: BTreeSet<&str> = BTreeSet::new();
    let mut revealed_at_nothing = 0;
    for (what, meadow, cmd) in cases {
        let mut s = meadow;
        // The prices already in view are recorded; the stock the command will
        // spend is not, and stands by hand.
        let stocked = s.resources;
        s.resources = GameState::new(0).resources;
        apply_any(&mut s);
        s.resources = stocked;
        let before = priced_resources(&s);
        assert_eq!(
            recorded_set(&s),
            before,
            "fixture ({what}): only the stated prices are recorded"
        );
        assert!(
            stock_what_it_costs(&mut s, cmd),
            "fixture: {what} should be affordable"
        );

        s.apply(cmd)
            .unwrap_or_else(|e| panic!("fixture: {what} should be accepted, got {e:?}"));

        let after = priced_resources(&s);
        let brought: Vec<ResourceId> = after.difference(&before).copied().collect();
        if brought.is_empty() {
            assert_eq!(
                what, "scouting a picnic",
                "fixture: {what} should bring a new price into view"
            );
            continue;
        }
        opened.insert(what);
        let recorded = recorded_set(&s);
        for r in brought {
            assert!(
                recorded.contains(&r),
                "{what} brought a price naming {r:?} into view and returned without revealing it"
            );
            if s.stock(r) == 0.0 {
                revealed_at_nothing += 1;
                assert!(
                    s.resource_revealed(r),
                    "{what}: {r:?} is named in a stated price and is unrevealed"
                );
            }
        }
    }
    for what in [
        "researching an idea",
        "basking to a peak of devotion",
        "summoning a whale",
        "scouting a picnic",
    ] {
        assert!(
            opened.contains(what),
            "fixture: {what} should have opened a gate that states a new price"
        );
    }
    assert!(
        revealed_at_nothing >= 4,
        "fixture: the newly stated prices should name stores standing at nothing"
    );
}

// ---------------------------------------------------------------------------
// SIM-ENG-055 — the rule stays off the hot path
// ---------------------------------------------------------------------------

/// A tick whose reveal scan reveals no content item checks no price: a price
/// in view that no command has yet recorded is still unrecorded after it.
/// (Such a state only arises built by hand — in play, the command that brought
/// the price into view recorded it — which is what makes the skipped check
/// visible here.)
///
/// @spec SIM-ENG-055
#[test]
fn a_tick_whose_scan_reveals_nothing_checks_no_price() {
    for replay in [false, true] {
        let mut s = GameState::new(550);
        s.ideas.insert(IdeaId::PouncingPractice);
        let priced = priced_resources(&s);
        assert!(!priced.is_empty(), "fixture: a price is in view");
        assert!(
            held_resources(&s).is_empty(),
            "fixture: and nothing is held"
        );

        for _ in 0..25 {
            let events = if replay { s.tick_replay() } else { s.tick() };
            assert!(
                !events.iter().any(|e| matches!(e, Event::Revealed { .. })),
                "fixture: these ticks' scans should reveal no content item; events: {events:?}"
            );
            assert!(held_resources(&s).is_empty(), "fixture: still nothing held");
            assert!(
                recorded_set(&s).is_empty(),
                "a tick whose reveal scan revealed nothing recorded {:?}: it checked a price",
                recorded_set(&s)
            );
        }

        // The same price, and the command that would have recorded it.
        apply_any(&mut s);
        assert_eq!(
            recorded_set(&s),
            priced,
            "fixture: an applied command records the price in view"
        );
    }
}

// ---------------------------------------------------------------------------
// SIM-ENG-056 — a reveal is not an occurrence
// ---------------------------------------------------------------------------

/// Two copies of one meadow: one with nothing recorded, one with every
/// resource already revealed. Whatever the first reveals, the second cannot.
fn unrevealed_and_all_revealed(s: &GameState) -> (GameState, GameState) {
    let json = save_string(s, 0);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    v["state"]["revealed_resources"] = serde_json::json!([]);
    let unrevealed = load_string(&v.to_string(), 0)
        .expect("fixture: the save loads")
        .state;
    v["state"]["revealed_resources"] = serde_json::to_value(ResourceId::ALL.to_vec()).unwrap();
    let all = load_string(&v.to_string(), 0)
        .expect("fixture: the edited save loads")
        .state;
    assert!(
        recorded_set(&unrevealed).is_empty(),
        "fixture: one copy has revealed nothing"
    );
    assert_eq!(
        recorded_set(&all).len(),
        ResourceId::ALL.len(),
        "fixture: the other has revealed everything"
    );
    (unrevealed, all)
}

/// A resource's reveal emits no event and enters no journal — by a tick or by
/// a command, by a holding or by a price. A meadow that reveals resources and
/// its twin that has none left to reveal report the same events and keep the
/// same journal.
///
/// @spec SIM-ENG-056
#[test]
fn a_resources_reveal_emits_no_event_and_enters_no_journal() {
    enum Step {
        Ticks(u32),
        Apply(Command),
    }
    let mut cases: Vec<(&str, GameState, Step)> = Vec::new();

    // A holding, by ticks: stores filling as the meadow works.
    let mut working = roomy(560);
    working.kittens = (0..4)
        .map(|i| kitten(&format!("kitten-{i}"), Some(JobId::ALL[i])))
        .collect();
    for r in ResourceId::ALL.into_iter().step_by(2) {
        working.resources[r.ix()] = 6.0;
    }
    cases.push(("ticks that leave stores held", working, Step::Ticks(40)));

    // A price, by a tick: the scan reveals ideas.
    let mut nook = GameState::new(560);
    grant(&mut nook, StructureId::StudyNook, 1);
    cases.push(("a tick whose scan reveals ideas", nook, Step::Ticks(1)));

    // A holding, by a command.
    cases.push((
        "a gather",
        GameState::new(560),
        Step::Apply(Command::GatherCatnip),
    ));

    // A price, by a command: every price in view, recorded by the next one.
    cases.push((
        "a command in a meadow with every price in view",
        everything_in_view(),
        Step::Apply(ANY_COMMAND),
    ));

    for (what, meadow, step) in cases {
        let (mut revealing, mut settled) = unrevealed_and_all_revealed(&meadow);
        let all = recorded_resources(&settled);
        match step {
            Step::Ticks(n) => {
                for _ in 0..n {
                    let a = revealing.tick();
                    let b = settled.tick();
                    assert_eq!(
                        a, b,
                        "{what}: a tick that revealed resources returned different events"
                    );
                }
            }
            Step::Apply(cmd) => {
                let a = revealing.apply(cmd).expect("fixture: accepted");
                let b = settled.apply(cmd).expect("fixture: accepted");
                assert_eq!(
                    a, b,
                    "{what}: a command that revealed resources returned different events"
                );
            }
        }
        assert!(
            !recorded_set(&revealing).is_empty(),
            "fixture: {what} should reveal a resource"
        );
        assert_eq!(
            recorded_resources(&settled),
            all,
            "fixture: the twin had nothing left to reveal"
        );
        assert_eq!(
            revealing.journal, settled.journal,
            "{what}: revealing a resource left an entry in the journal"
        );
    }
}
