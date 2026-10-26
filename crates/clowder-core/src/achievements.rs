//! Achievements: noticing when one is earned, and listing them.
//!
//! The record is written here and read by no rule of the simulation, and
//! noticing draws nothing from the state RNG — an achievement is a memory of
//! the meadow, never a lever in it.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use crate::content::{self, AchievementDef};
use crate::events::Event;
use crate::state::{
    AchievementId, ColonyId, CraftId, Earned, EyeLook, FavouriteKind, FurColour, FurPattern,
    GameState, IdeaId, Location, MoodTier, Pose, ResourceId, SiteId, StructureId, EPSILON,
};

/// What a condition reads, which decides when it is checked.
enum Condition {
    /// The meadow as it stands, by its summaries: checked at every day close
    /// and after every command that emits an event.
    State(fn(&GameState) -> bool),
    /// The meadow as it stands, kitten by kitten: checked after every command
    /// that emits an event, and at a day close only when a kitten arrived or a
    /// year turned.
    Kittens(fn(&GameState) -> bool),
    /// Something that just happened: checked against each event emitted.
    Moment(fn(&Event) -> bool),
}

/// The table's conditions sorted by what they read, each kind in table
/// order, so a check walks only the kinds it is due.
struct ByKind {
    state: Vec<(AchievementId, fn(&GameState) -> bool)>,
    kittens: Vec<(AchievementId, fn(&GameState) -> bool)>,
    moments: Vec<(AchievementId, fn(&Event) -> bool)>,
}

fn by_kind() -> &'static ByKind {
    static BY_KIND: OnceLock<ByKind> = OnceLock::new();
    BY_KIND.get_or_init(|| {
        let mut by_kind = ByKind {
            state: Vec::new(),
            kittens: Vec::new(),
            moments: Vec::new(),
        };
        for id in AchievementId::ALL {
            match condition(id) {
                Condition::State(holds) => by_kind.state.push((id, holds)),
                Condition::Kittens(holds) => by_kind.kittens.push((id, holds)),
                Condition::Moment(is) => by_kind.moments.push((id, is)),
            }
        }
        by_kind
    })
}

/// When a check runs.
pub(crate) enum Look {
    /// At the end of a tick, which may or may not have closed a day.
    Tick { day_closed: bool },
    /// After an applied command that emitted an event, a speed change aside.
    Command,
}

/// Each achievement's condition, exactly as the table states it.
/// @spec SIM-ACH-006
fn condition(id: AchievementId) -> Condition {
    use AchievementId::*;
    use Condition::{Kittens, Moment, State};
    match id {
        AFaceAtTheDoor => State(|s| !s.kittens.is_empty()),
        AProperClowder => State(|s| s.kittens.len() >= 10),
        WallToWallWhiskers => State(|s| s.kittens.len() >= 50),
        TooManyToName => State(|s| s.kittens.len() >= 100),
        FullHouse => State(|s| {
            let beds = s.housing_capacity() as usize;
            beds >= 10 && s.kittens.len() >= beds
        }),
        OnceInABlueMoon => Kittens(|s| {
            profiles(s).any(|p| {
                p.fur
                    .colours
                    .iter()
                    .any(|c| content::SURPRISING_COLOURS.contains(c))
            })
        }),
        OddOneOut => Kittens(|s| {
            profiles(s).any(|p| matches!(p.eyes, EyeLook::OddBlueCopper | EyeLook::OddBlueGreen))
        }),
        StrikeAPose => Kittens(|s| {
            let poses: BTreeSet<Pose> = profiles(s).map(|p| p.pose).collect();
            poses.len() == Pose::ALL.len()
        }),
        ACoatOfManyColours => Kittens(|s| {
            let patterns: BTreeSet<FurPattern> = profiles(s).map(|p| p.fur.pattern).collect();
            patterns.len() == FurPattern::ALL.len()
        }),
        SnackBuddies => Kittens(|s| {
            let mut foods = BTreeSet::new();
            profiles(s)
                .flat_map(|p| &p.favourites)
                .filter(|f| f.kind == FavouriteKind::Food)
                .any(|f| !foods.insert(f.entry.as_str()))
        }),
        MatchingOutfits => Kittens(|s| {
            let mut furs: BTreeSet<(FurPattern, &[FurColour])> = BTreeSet::new();
            profiles(s).any(|p| !furs.insert((p.fur.pattern, p.fur.colours.as_slice())))
        }),
        SecondsPlease => Kittens(|s| s.kittens.iter().any(|k| carries_a_numeral(&k.name))),
        WiseWhiskers => State(|s| s.elder.is_some()),
        MovingUp => Kittens(|s| s.kittens.iter().any(|k| k.rank >= 1)),
        SeniorPurrsonnel => Kittens(|s| s.kittens.iter().any(|k| k.rank >= 5)),
        PawsOfExperience => Kittens(|s| {
            s.kittens
                .iter()
                .any(|k| k.skills.iter().any(|&x| x >= 9000.0))
        }),
        EveryonesOnBreak => Kittens(|s| {
            s.kittens
                .iter()
                .filter(|k| k.location == Location::Home)
                .count()
                >= 10
                && s.kittens.iter().all(|k| k.job.is_none())
        }),
        VisitingTheNeighbours => Moment(|e| matches!(e, Event::KittenWanderedOff { .. })),
        HomeAgain => Moment(|e| matches!(e, Event::KittenReturned { .. })),
        PureBliss => State(|s| s.mood_tier() == MoodTier::Blissful),
        // The Grumpy tier is the famine flag itself (SIM-KIT-014).
        Hangry => State(|s| s.grumpy),
        TheZoomies => Moment(|e| matches!(e, Event::ZoomiesStarted { .. })),
        PartyAnimals => State(|s| s.jamboree_active()),
        HomeImprovement => State(|s| structures_standing(s) >= 1),
        Sprawl => State(|s| structures_standing(s) >= 100),
        KittenMetropolis => State(|s| structures_standing(s) >= 500),
        Handmade => State(|s| {
            CraftId::ALL
                .iter()
                .any(|c| s.resources[content::craft_output(*c).ix()] > EPSILON)
        }),
        // Bound by a command that tells of nothing, so read daily.
        BusyPaws => State(|s| s.kittens.iter().any(|k| k.tinkering.is_some())),
        WhatAMess => State(|s| s.mess_level() >= 1),
        LightsFlicker => State(|s| {
            let used = s.energy_consumption();
            used > EPSILON && used > s.energy_production() + EPSILON
        }),
        HummingAlong => State(|s| {
            let made = s.energy_production();
            made >= 20.0 && made + EPSILON >= s.energy_consumption()
        }),
        ABrightIdea => State(|s| !s.ideas.is_empty()),
        WellRead => State(|s| s.ideas.len() >= 20),
        KnowItAll => State(|s| {
            s.ideas.len() >= IdeaId::ALL.len() && IdeaId::ALL.iter().all(|i| s.ideas.contains(i))
        }),
        RefinedTastes => State(|s| s.refinements.len() >= 10),
        HouseRules => State(|s| !s.rules.is_empty()),
        Pounce => Moment(|e| matches!(e, Event::PounceReturned { .. })),
        TheGreatHunt => {
            Moment(|e| matches!(e, Event::PounceReturned { squads, .. } if *squads >= 50))
        }
        FollowedHome => State(|s| s.resources[ResourceId::Unicorns.ix()] + EPSILON >= 1.0),
        OpenForBusiness => Moment(|e| matches!(e, Event::TradeReturned { .. })),
        ShhTheyreSleeping => {
            Moment(|e| matches!(e, Event::TradeReturned { napped, .. } if *napped > 0))
        }
        DearNeighbour => State(|s| s.penpal.iter().any(|&l| l >= 1)),
        BestFriendsForever => State(|s| s.penpal.iter().any(|&l| l >= 10)),
        KnowTheNeighbourhood => State(|s| {
            ColonyId::ALL
                .iter()
                .filter(|&&c| c != ColonyId::StarWhales)
                .all(|c| s.discovered.contains(c))
        }),
        Sunbather => State(|s| !s.rites.is_empty()),
        Enlightened => State(|s| s.enlightenment_tier >= 1),
        GlitterParade => Moment(|e| matches!(e, Event::ParadeHeld { .. })),
        SweetDreams => State(|s| s.resources[ResourceId::Dreamcorns.ix()] + EPSILON >= 1.0),
        BottledContentment => Moment(|e| matches!(e, Event::JarCondensed)),
        OverTheFence => State(|s| !s.sites_reached.is_empty()),
        WishUponAStar => State(|s| {
            let furthest = SiteId::ALL[SiteId::ALL.len() - 1];
            s.sites_reached.contains(&furthest)
        }),
        WhaleWatching => State(|s| s.whale_visit_years > 0),
        AVeryLongNap => Moment(|e| matches!(e, Event::Napped { .. })),
        GoodMorning => State(|s| s.distant_dawn),
        MovingDay => State(|s| s.generation >= 1),
        // Generations count from zero: the fifth is generation 4.
        FamilyTree => State(|s| s.generation >= 4),
        Remembered => State(|s| s.family_album.len() >= 100),
        OffOnAnAdventure => State(|s| s.active_adventure.is_some()),
        TheOldWays => State(|s| !s.traditions.is_empty()),
        AWholeYear => State(|s| s.year() >= 1),
        OldMeadow => State(|s| s.year() >= 100),
    }
}

fn profiles(s: &GameState) -> impl Iterator<Item = &crate::state::Profile> {
    s.kittens.iter().filter_map(|k| k.profile.as_ref())
}

fn structures_standing(s: &GameState) -> u64 {
    StructureId::ALL
        .iter()
        .map(|id| u64::from(s.structures[id.ix()]))
        .sum()
}

/// A name the pool gave out a second time: a pool name, then a space and a
/// Roman numeral, and not itself a pool name.
fn carries_a_numeral(name: &str) -> bool {
    let in_pool = |n: &str| content::NAME_POOL.iter().any(|p| p.eq_ignore_ascii_case(n));
    name.rsplit_once(' ').is_some_and(|(base, tail)| {
        !tail.is_empty() && tail.chars().all(|c| "IVXLCDM".contains(c)) && in_pool(base)
    }) && !in_pool(name)
}

impl GameState {
    /// Notice what has just been earned: record each with the generation and
    /// the meadow's day, and emit it. State conditions are read when `look`
    /// says the meadow is due a look — every day close and every command that
    /// tells of something, and for those that walk the roster only a day close
    /// that brought a kitten or turned the year — and moment conditions against
    /// `events`, the events just emitted. Draws nothing, and changes nothing but the record.
    /// @spec SIM-ACH-002, SIM-ACH-003, SIM-ACH-004
    pub(crate) fn check_achievements(&mut self, look: Look, events: &mut Vec<Event>) {
        let (state_due, kittens_due) = match look {
            Look::Command => (true, true),
            Look::Tick { day_closed } => (
                day_closed,
                day_closed
                    && events
                        .iter()
                        .any(|e| matches!(e, Event::KittenArrived { .. } | Event::NewYear { .. })),
            ),
        };
        // The quiet tick, nearly every tick: nothing to look at.
        if !state_due && events.is_empty() {
            return;
        }
        if self.achievements.len() == AchievementId::ALL.len() {
            return;
        }
        let mut held = [false; AchievementId::ALL.len()];
        for id in self.achievements.keys() {
            held[*id as usize] = true;
        }
        let by_kind = by_kind();
        let mut earned = Vec::new();
        let meadow_due = [(state_due, &by_kind.state), (kittens_due, &by_kind.kittens)];
        for (due, kind) in meadow_due {
            if due {
                earned.extend(
                    kind.iter()
                        .filter(|&&(id, holds)| !held[id as usize] && holds(self))
                        .map(|&(id, _)| id),
                );
            }
        }
        if !events.is_empty() {
            earned.extend(
                by_kind
                    .moments
                    .iter()
                    .filter(|&&(id, is)| !held[id as usize] && events.iter().any(is))
                    .map(|&(id, _)| id),
            );
        }
        // Announced in table order.
        earned.sort();
        let when = Earned {
            generation: self.generation,
            day: self.day(),
        };
        for id in earned {
            self.achievements.insert(id, when);
            events.push(Event::AchievementEarned { id });
        }
    }

    /// Every achievement in table order, with its definition and, where it
    /// has been earned, when. @spec SIM-ACH-005
    pub fn achievement_list(
        &self,
    ) -> Vec<(AchievementId, &'static AchievementDef, Option<Earned>)> {
        AchievementId::ALL
            .iter()
            .map(|&id| {
                (
                    id,
                    content::achievement_def(id),
                    self.achievements.get(&id).copied(),
                )
            })
            .collect()
    }
}
