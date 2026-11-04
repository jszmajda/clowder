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
    State(MeadowCheck),
    /// The meadow as it stands, kitten by kitten: checked after every command
    /// that emits an event, and at a day close only when a kitten arrived or a
    /// year turned.
    Kittens(MeadowCheck),
    /// Something that just happened: checked against each event emitted.
    Moment(MomentCheck),
}

/// The table's conditions sorted by what they read, each kind in table
/// order, so a check walks only the kinds it is due.
struct ByKind {
    state: Vec<(AchievementId, MeadowCheck)>,
    kittens: Vec<(AchievementId, MeadowCheck)>,
    moments: Vec<(AchievementId, MomentCheck)>,
}

type MeadowCheck = fn(&GameState) -> bool;
type MomentCheck = fn(&Event) -> bool;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{DAYS_PER_YEAR, TICKS_PER_DAY};
    use crate::save::OfflineReport;
    use crate::state::{
        AdventureId, ArchivedKitten, Favourite, Fur, Kitten, KittenTrait, Profile, RefinementId,
        RiteId, RuleId, TraditionId,
    };

    /// What a row's condition is held to: meadows that meet it and meadows just
    /// short of it, or events that are it and events that are not.
    enum Case {
        Meadow(Vec<GameState>, Vec<GameState>),
        Moment(Vec<Event>, Vec<Event>),
    }

    fn fresh() -> GameState {
        GameState::new(1)
    }

    fn with(edit: impl FnOnce(&mut GameState)) -> GameState {
        let mut s = fresh();
        edit(&mut s);
        s
    }

    fn kitten(name: &str) -> Kitten {
        Kitten {
            name: name.to_string(),
            job: None,
            location: Location::Home,
            arrived_day: 0,
            job_seq: None,
            away_seq: None,
            skills: [0.0; 8],
            exp: 0.0,
            rank: 0,
            trait_id: KittenTrait::None,
            tinkering: None,
            profile: None,
        }
    }

    fn roster(n: usize) -> Vec<Kitten> {
        (0..n).map(|i| kitten(&format!("K{i}"))).collect()
    }

    fn with_roster(n: usize) -> GameState {
        with(|s| s.kittens = roster(n))
    }

    /// One kitten, changed.
    fn one_kitten(edit: impl FnOnce(&mut Kitten)) -> GameState {
        with(|s| {
            let mut k = kitten("K");
            edit(&mut k);
            s.kittens = vec![k];
        })
    }

    /// The plainest look a pattern allows: each slot's first colour no earlier
    /// slot took.
    fn plain_look(pattern: FurPattern) -> Profile {
        let mut colours: Vec<FurColour> = Vec::new();
        for slot in content::fur_pattern_def(pattern).slots {
            let free = slot.iter().find(|c| !colours.contains(c));
            colours.push(*free.expect("a slot has a free colour"));
        }
        Profile {
            fur: Fur { pattern, colours },
            eyes: EyeLook::Green,
            favourites: Vec::new(),
            pose: Pose::ALL[0],
        }
    }

    /// One kitten per profile.
    fn dressed(looks: Vec<Profile>) -> GameState {
        with(|s| {
            s.kittens = looks
                .into_iter()
                .enumerate()
                .map(|(i, look)| Kitten {
                    profile: Some(look),
                    ..kitten(&format!("K{i}"))
                })
                .collect();
        })
    }

    fn looks(edit: impl FnOnce(&mut Profile)) -> Profile {
        let mut p = plain_look(FurPattern::ALL[0]);
        edit(&mut p);
        p
    }

    fn favouring(kind: FavouriteKind, entry: &str) -> Profile {
        looks(|p| {
            p.favourites = vec![Favourite {
                kind,
                entry: entry.to_string(),
            }]
        })
    }

    fn built(n: u32) -> GameState {
        with(|s| s.structures[StructureId::ALL[0].ix()] = n)
    }

    fn powered(makers: u32, users: u32) -> GameState {
        with(|s| {
            s.structures[StructureId::Cogworks.ix()] = makers;
            s.structures[StructureId::BoxLabyrinth.ix()] = users;
        })
    }

    fn beds_and_kittens(beds: u32, kittens: usize) -> GameState {
        with(|s| {
            s.structures[StructureId::CozyCushion.ix()] = beds;
            assert_eq!(s.housing_capacity(), beds, "fixture: a cushion is a bed");
            s.kittens = roster(kittens);
        })
    }

    fn at_tick(ticks: u64) -> GameState {
        with(|s| s.ticks = ticks)
    }

    fn year(n: u64) -> u64 {
        n * DAYS_PER_YEAR * TICKS_PER_DAY
    }

    fn stocked(r: ResourceId, amount: f64) -> GameState {
        with(|s| s.resources[r.ix()] = amount)
    }

    fn album(n: usize) -> GameState {
        with(|s| {
            s.family_album = (0..n)
                .map(|i| ArchivedKitten {
                    name: format!("K{i}"),
                    trait_id: KittenTrait::None,
                    rank: 0,
                    generation: 0,
                    profile: None,
                })
                .collect();
        })
    }

    fn pounce(squads: u32) -> Event {
        Event::PounceReturned {
            squads,
            feathers: 0.0,
            bells: 0.0,
            shinies: 0.0,
            unicorns: 0,
        }
    }

    fn trade(napped: u32) -> Event {
        Event::TradeReturned {
            colony: ColonyId::ALL[0],
            caravans: 1,
            napped,
            goods: Vec::new(),
        }
    }

    /// Exhaustive on purpose: a new row of the table stops this compiling until it
    /// has a case.
    fn case(id: AchievementId) -> Case {
        use AchievementId::*;
        use Case::{Meadow, Moment};
        let tradeable: Vec<ColonyId> = ColonyId::ALL
            .into_iter()
            .filter(|c| *c != ColonyId::StarWhales)
            .collect();
        match id {
            AFaceAtTheDoor => Meadow(vec![with_roster(1)], vec![with_roster(0)]),
            AProperClowder => Meadow(vec![with_roster(10)], vec![with_roster(9)]),
            WallToWallWhiskers => Meadow(vec![with_roster(50)], vec![with_roster(49)]),
            TooManyToName => Meadow(vec![with_roster(100)], vec![with_roster(99)]),
            FullHouse => Meadow(
                vec![beds_and_kittens(10, 10), beds_and_kittens(12, 13)],
                vec![beds_and_kittens(10, 9), beds_and_kittens(9, 9)],
            ),
            OnceInABlueMoon => Meadow(
                vec![dressed(vec![looks(|p| {
                    p.fur.colours = vec![content::SURPRISING_COLOURS[0]; p.fur.colours.len()]
                })])],
                vec![dressed(vec![looks(|_| {})]), with_roster(1)],
            ),
            OddOneOut => Meadow(
                vec![
                    dressed(vec![looks(|p| p.eyes = EyeLook::OddBlueCopper)]),
                    dressed(vec![looks(|p| p.eyes = EyeLook::OddBlueGreen)]),
                ],
                vec![
                    dressed(vec![looks(|p| p.eyes = EyeLook::Blue)]),
                    dressed(vec![looks(|p| p.eyes = EyeLook::Copper)]),
                    with_roster(1),
                ],
            ),
            StrikeAPose => {
                let posed = |poses: &[Pose]| {
                    dressed(poses.iter().map(|&pose| looks(|p| p.pose = pose)).collect())
                };
                Meadow(
                    vec![posed(&Pose::ALL)],
                    vec![posed(&Pose::ALL[..Pose::ALL.len() - 1])],
                )
            }
            ACoatOfManyColours => {
                let furs = |patterns: &[FurPattern]| {
                    dressed(patterns.iter().map(|&f| plain_look(f)).collect())
                };
                Meadow(
                    vec![furs(&FurPattern::ALL)],
                    vec![furs(&FurPattern::ALL[..FurPattern::ALL.len() - 1])],
                )
            }
            SnackBuddies => {
                let other = FavouriteKind::ALL
                    .into_iter()
                    .find(|k| *k != FavouriteKind::Food)
                    .expect("fixture: a kind beside food");
                Meadow(
                    vec![dressed(vec![
                        favouring(FavouriteKind::Food, "toast"),
                        favouring(FavouriteKind::Food, "toast"),
                    ])],
                    vec![
                        dressed(vec![
                            favouring(FavouriteKind::Food, "toast"),
                            favouring(FavouriteKind::Food, "milk"),
                        ]),
                        dressed(vec![favouring(other, "toast"), favouring(other, "toast")]),
                        dressed(vec![favouring(FavouriteKind::Food, "toast")]),
                    ],
                )
            }
            MatchingOutfits => {
                let pattern = FurPattern::ALL
                    .into_iter()
                    .find(|f| content::fur_pattern_def(*f).slots[0].len() > 1)
                    .expect("fixture: a pattern whose slot offers a choice");
                let twin = plain_look(pattern);
                let mut near = twin.clone();
                near.fur.colours[0] = content::fur_pattern_def(pattern).slots[0][1];
                Meadow(
                    vec![dressed(vec![twin.clone(), twin.clone()])],
                    vec![dressed(vec![twin.clone(), near]), dressed(vec![twin])],
                )
            }
            SecondsPlease => {
                let pool = content::NAME_POOL[0];
                Meadow(
                    vec![
                        one_kitten(|k| k.name = format!("{pool} II")),
                        one_kitten(|k| k.name = format!("{pool} XIV")),
                    ],
                    vec![
                        one_kitten(|k| k.name = pool.to_string()),
                        one_kitten(|k| k.name = "Nobody Anyone II".to_string()),
                        one_kitten(|k| k.name = format!("{pool} Junior")),
                    ],
                )
            }
            WiseWhiskers => Meadow(
                vec![with(|s| {
                    s.kittens = roster(1);
                    s.elder = Some(0);
                })],
                vec![with_roster(1)],
            ),
            MovingUp => Meadow(vec![one_kitten(|k| k.rank = 1)], vec![one_kitten(|_| {})]),
            SeniorPurrsonnel => Meadow(
                vec![one_kitten(|k| k.rank = 5)],
                vec![one_kitten(|k| k.rank = 4)],
            ),
            PawsOfExperience => Meadow(
                vec![one_kitten(|k| k.skills[3] = 9_000.0)],
                vec![one_kitten(|k| k.skills[3] = 8_999.5)],
            ),
            EveryonesOnBreak => {
                let out = |n: usize, edit: fn(&mut Vec<Kitten>)| {
                    with(|s| {
                        s.kittens = roster(n);
                        edit(&mut s.kittens);
                    })
                };
                Meadow(
                    vec![
                        out(10, |_| {}),
                        out(11, |k| k[10].location = Location::NeighborsYard),
                    ],
                    vec![
                        out(9, |_| {}),
                        out(10, |k| k[0].job = Some(crate::state::JobId::ALL[0])),
                        // The roster's second clause reads the whole roster.
                        out(11, |k| {
                            k[10].location = Location::NeighborsYard;
                            k[10].job = Some(crate::state::JobId::ALL[0]);
                        }),
                        out(10, |k| k[0].location = Location::NeighborsYard),
                    ],
                )
            }
            VisitingTheNeighbours => Moment(
                vec![Event::KittenWanderedOff { name: "K".into() }],
                vec![Event::KittenReturned { name: "K".into() }],
            ),
            HomeAgain => Moment(
                vec![Event::KittenReturned { name: "K".into() }],
                vec![Event::KittenWanderedOff { name: "K".into() }],
            ),
            PureBliss => {
                let glad = |posts: u32, grumpy: bool| {
                    with(|s| {
                        s.kittens = roster(1);
                        s.structures[StructureId::ScratchingPost.ix()] = posts;
                        s.grumpy = grumpy;
                    })
                };
                Meadow(vec![glad(4, false)], vec![glad(0, false), glad(4, true)])
            }
            Hangry => Meadow(vec![with(|s| s.grumpy = true)], vec![fresh()]),
            TheZoomies => Moment(
                vec![Event::ZoomiesStarted { until_tick: 5 }],
                vec![Event::ZoomiesEnded],
            ),
            PartyAnimals => Meadow(vec![with(|s| s.jamboree_until = 50)], vec![fresh()]),
            HomeImprovement => Meadow(vec![built(1)], vec![built(0)]),
            Sprawl => Meadow(vec![built(100)], vec![built(99)]),
            KittenMetropolis => Meadow(vec![built(500)], vec![built(499)]),
            Handmade => Meadow(
                CraftId::ALL
                    .into_iter()
                    .map(|c| stocked(content::craft_output(c), 1.0))
                    .collect(),
                vec![fresh(), stocked(ResourceId::Catnip, 1.0)],
            ),
            BusyPaws => Meadow(
                vec![one_kitten(|k| k.tinkering = Some(CraftId::ALL[0]))],
                vec![one_kitten(|_| {})],
            ),
            WhatAMess => {
                let unit = content::MESS_LEVEL_UNIT;
                Meadow(
                    vec![with(|s| s.mess = unit * 10.0)],
                    vec![with(|s| s.mess = unit * 9.9), with(|s| s.mess = unit)],
                )
            }
            LightsFlicker => Meadow(
                vec![powered(1, 3), powered(0, 1)],
                vec![powered(0, 0), powered(2, 1), powered(1, 0)],
            ),
            HummingAlong => Meadow(
                vec![powered(7, 0), powered(7, 10)],
                vec![powered(6, 0), powered(7, 11)],
            ),
            ABrightIdea => Meadow(
                vec![with(|s| {
                    s.ideas.insert(IdeaId::ALL[0]);
                })],
                vec![fresh()],
            ),
            WellRead => {
                let read =
                    |n: usize| with(|s| s.ideas = IdeaId::ALL[..n].iter().copied().collect());
                Meadow(vec![read(20)], vec![read(19)])
            }
            KnowItAll => {
                let read =
                    |n: usize| with(|s| s.ideas = IdeaId::ALL[..n].iter().copied().collect());
                Meadow(
                    vec![read(IdeaId::ALL.len())],
                    vec![read(IdeaId::ALL.len() - 1)],
                )
            }
            RefinedTastes => {
                let bought = |n: usize| {
                    with(|s| s.refinements = RefinementId::ALL[..n].iter().copied().collect())
                };
                Meadow(vec![bought(10)], vec![bought(9)])
            }
            HouseRules => Meadow(
                vec![with(|s| {
                    s.rules.insert(RuleId::ALL[0]);
                })],
                vec![fresh()],
            ),
            Pounce => Moment(vec![pounce(1)], vec![Event::JarCondensed]),
            TheGreatHunt => Moment(vec![pounce(50), pounce(51)], vec![pounce(49)]),
            FollowedHome => Meadow(
                vec![stocked(ResourceId::Unicorns, 1.0)],
                vec![stocked(ResourceId::Unicorns, 0.5), fresh()],
            ),
            OpenForBusiness => Moment(vec![trade(0), trade(1)], vec![Event::JarCondensed]),
            ShhTheyreSleeping => Moment(vec![trade(1), trade(3)], vec![trade(0)]),
            DearNeighbour => Meadow(vec![with(|s| s.penpal[0] = 1)], vec![fresh()]),
            BestFriendsForever => Meadow(
                vec![with(|s| s.penpal[0] = 10)],
                vec![with(|s| s.penpal[0] = 9)],
            ),
            KnowTheNeighbourhood => {
                let met = |colonies: &[ColonyId]| {
                    with(|s| s.discovered = colonies.iter().copied().collect())
                };
                Meadow(
                    vec![met(&tradeable)],
                    vec![
                        met(&tradeable[..tradeable.len() - 1]),
                        met(&[ColonyId::StarWhales]),
                    ],
                )
            }
            Sunbather => Meadow(
                vec![with(|s| {
                    s.rites.insert(RiteId::ALL[0]);
                })],
                vec![fresh()],
            ),
            Enlightened => Meadow(vec![with(|s| s.enlightenment_tier = 1)], vec![fresh()]),
            GlitterParade => Moment(
                vec![Event::ParadeHeld { rainbows: 1.0 }],
                vec![Event::JarCondensed],
            ),
            SweetDreams => Meadow(
                vec![stocked(ResourceId::Dreamcorns, 1.0)],
                vec![stocked(ResourceId::Dreamcorns, 0.5), fresh()],
            ),
            BottledContentment => Moment(vec![Event::JarCondensed], vec![Event::EchoRefined]),
            OverTheFence => Meadow(
                vec![with(|s| {
                    s.sites_reached.insert(SiteId::ALL[0]);
                })],
                vec![fresh()],
            ),
            WishUponAStar => {
                let last = SiteId::ALL.len() - 1;
                Meadow(
                    vec![with(|s| {
                        s.sites_reached.insert(SiteId::ALL[last]);
                    })],
                    vec![with(|s| {
                        s.sites_reached = SiteId::ALL[..last].iter().copied().collect();
                    })],
                )
            }
            WhaleWatching => Meadow(vec![with(|s| s.whale_visit_years = 1)], vec![fresh()]),
            AVeryLongNap => Moment(
                vec![Event::Napped {
                    years: 1,
                    report: Box::new(OfflineReport::default()),
                }],
                vec![Event::DejaVuDay],
            ),
            GoodMorning => Meadow(vec![with(|s| s.distant_dawn = true)], vec![fresh()]),
            MovingDay => Meadow(vec![with(|s| s.generation = 1)], vec![fresh()]),
            FamilyTree => Meadow(
                vec![with(|s| s.generation = 4)],
                vec![with(|s| s.generation = 3)],
            ),
            Remembered => Meadow(vec![album(100)], vec![album(99)]),
            OffOnAnAdventure => Meadow(
                vec![with(|s| s.active_adventure = Some(AdventureId::ALL[0]))],
                vec![fresh()],
            ),
            TheOldWays => Meadow(
                vec![with(|s| {
                    s.traditions.insert(TraditionId::ALL[0]);
                })],
                vec![fresh()],
            ),
            AWholeYear => Meadow(vec![at_tick(year(1))], vec![at_tick(year(1) - 1)]),
            OldMeadow => Meadow(vec![at_tick(year(100))], vec![at_tick(year(100) - 1)]),
        }
    }

    /// Every row's condition holds for each case that meets it, and for none just
    /// short of it.
    /// @spec SIM-ACH-006
    #[test]
    fn every_condition_holds_for_what_meets_it_and_not_for_what_falls_short() {
        for id in AchievementId::ALL {
            match (condition(id), case(id)) {
                (
                    Condition::State(holds) | Condition::Kittens(holds),
                    Case::Meadow(meets, short),
                ) => {
                    assert!(!meets.is_empty() && !short.is_empty(), "fixture: {id:?}");
                    for (i, s) in meets.iter().enumerate() {
                        assert!(holds(s), "{id:?} should hold for meadow {i} that meets it");
                    }
                    for (i, s) in short.iter().enumerate() {
                        assert!(!holds(s), "{id:?} held for meadow {i} just short of it");
                    }
                }
                (Condition::Moment(is), Case::Moment(meets, short)) => {
                    assert!(!meets.is_empty() && !short.is_empty(), "fixture: {id:?}");
                    for e in &meets {
                        assert!(is(e), "{id:?} should hold for {e:?}");
                    }
                    for e in &short {
                        assert!(!is(e), "{id:?} held for {e:?}");
                    }
                }
                _ => panic!("{id:?}: its case is not of its condition's kind"),
            }
        }
    }

    /// The rows that read a kitten wait for a reason to look: exactly these, as the
    /// spec lists what such a reading is.
    /// @spec SIM-ACH-002
    #[test]
    fn the_conditions_that_read_a_kitten_are_the_ones_that_wait() {
        use AchievementId::*;
        let wait = [
            OnceInABlueMoon,
            OddOneOut,
            StrikeAPose,
            ACoatOfManyColours,
            SnackBuddies,
            MatchingOutfits,
            SecondsPlease,
            MovingUp,
            SeniorPurrsonnel,
            PawsOfExperience,
            EveryonesOnBreak,
        ];
        for id in AchievementId::ALL {
            assert_eq!(
                matches!(condition(id), Condition::Kittens(_)),
                wait.contains(&id),
                "{id:?}"
            );
        }
    }

    /// Every row of the design's table stands in the content: its place, its name,
    /// its kind, and whether it is secret.
    /// @spec SIM-ACH-001, SIM-ACH-006
    #[test]
    fn every_achievement_matches_its_row() {
        const DESIGN: &str =
            include_str!("../../../docs/intent/sim/achievements/achievements-design.md");
        let start = DESIGN.find("## The Achievements").expect("the heading");
        let end = start + DESIGN[start..].find("## Earning").expect("the heading");
        let rows: Vec<Vec<&str>> = DESIGN[start..end]
            .lines()
            .filter(|l| {
                l.strip_prefix("| ")
                    .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
            })
            .map(|l| l.trim_matches('|').split('|').map(str::trim).collect())
            .collect();
        assert_eq!(
            rows.len(),
            AchievementId::ALL.len(),
            "one row per achievement"
        );
        for (n, row) in rows.iter().enumerate() {
            let id = AchievementId::ALL[n];
            assert_eq!(
                row[0].parse::<usize>().unwrap(),
                n,
                "the rows number from 0"
            );
            let def = content::achievement_def(id);
            assert_eq!(def.name, row[1], "row {n}'s name");
            let kind = match condition(id) {
                Condition::State(_) | Condition::Kittens(_) => "state",
                Condition::Moment(_) => "moment",
            };
            assert_eq!(kind, row[2], "{}'s kind", row[1]);
            assert_eq!(def.secret, row[4] == "secret", "{} secret", row[1]);
        }
    }
}
