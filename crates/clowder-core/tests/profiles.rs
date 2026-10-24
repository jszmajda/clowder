//! Kitten profiles — what each kitten looks like and loves — and the facts
//! the simulation holds about a kitten that bear on play.
//!
//!   * SIM-KIT-031..034 — a profile is drawn at arrival from the kitten's name
//!     and generation alone, read by no rule, and kept for life.
//!   * SIM-KIT-035..039 — the tables a profile draws from, and a fur in words.
//!   * SIM-KIT-040, 041 — each trait's name and blurb, and the facts query.
//!   * SIM-MOVE-001, 002 — the album and the baskets keep profiles.
//!   * SAVE-027, 028 — profiles are saved, and a save from before them draws
//!     its kittens' profiles on load.

mod common;

use std::collections::BTreeSet;

use clowder_core::content::{self, blurb_parts, BlurbPart};
use clowder_core::*;
use common::*;

/// A Home kitten with no job, as a save from before profiles holds it.
fn bare(name: &str, arrived_day: u64) -> Kitten {
    Kitten {
        name: name.to_string(),
        job: None,
        location: Location::Home,
        arrived_day,
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

/// Every favourite kind's label and entries.
fn favourite_tables() -> Vec<(&'static str, &'static [&'static str])> {
    FavouriteKind::ALL
        .iter()
        .map(|k| {
            let def = content::favourite_kind_def(*k);
            (def.label, def.entries)
        })
        .collect()
}

/// Whether `phrase` holds `name` as a whole word or a run of whole words,
/// ignoring case.
fn contains_words(phrase: &str, name: &str) -> bool {
    let words = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect()
    };
    let (hay, needle) = (words(phrase), words(name));
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle.as_slice())
}

// ---------------------------------------------------------------------------
// SIM-KIT-031, 032 — drawn at arrival, from the name and generation alone
// ---------------------------------------------------------------------------

/// Every kitten that arrives is given the profile its name and the current
/// generation draw.
/// @spec SIM-KIT-031
#[test]
fn an_arrival_brings_the_profile_its_name_and_generation_draw() {
    for (seed, generation) in [(31u64, 0u32), (32, 3)] {
        let mut s = GameState::new(seed);
        s.generation = generation;
        adopt(&mut s, 6);
        for k in &s.kittens {
            assert_eq!(
                k.profile.as_ref(),
                Some(&draw_profile(&k.name, generation)),
                "{}",
                k.name
            );
        }
    }
}

/// The draw is a fixed function of the name and the generation: the same two
/// draw the same profile every time; another generation, or another name,
/// draws its own.
/// @spec SIM-KIT-032
#[test]
fn the_draw_is_a_fixed_function_of_name_and_generation() {
    assert_eq!(draw_profile("Biscuit", 0), draw_profile("Biscuit", 0));
    let names: Vec<&str> = content::NAME_POOL.iter().take(40).copied().collect();
    let by_name: BTreeSet<String> = names
        .iter()
        .map(|n| format!("{:?}", draw_profile(n, 0)))
        .collect();
    assert!(
        by_name.len() >= 39,
        "forty names draw forty profiles, near enough"
    );
    let by_generation: BTreeSet<String> = (0..20)
        .map(|g| format!("{:?}", draw_profile("Biscuit", g)))
        .collect();
    assert!(
        by_generation.len() >= 19,
        "namesakes in other generations differ"
    );
}

/// The state RNG plays no part: two meadows with different seeds that admit a
/// kitten of the same name in the same generation give it the same profile.
/// @spec SIM-KIT-032
#[test]
fn the_state_rng_plays_no_part_in_a_profile() {
    let pool = &content::NAME_POOL;
    let last = pool[pool.len() - 1];
    let mut drawn = Vec::new();
    for seed in [1u64, 2, 3, 99] {
        let mut s = GameState::new(seed);
        // Every pool name but one is rostered (away), so the next arrival must
        // draw the last.
        s.kittens = pool[..pool.len() - 1]
            .iter()
            .map(|n| {
                let mut k = bare(n, 1);
                k.location = Location::NeighborsYard;
                k
            })
            .collect();
        adopt(&mut s, 1);
        let k = s.kittens.last().unwrap();
        assert_eq!(k.name, last, "fixture: the one name left is drawn");
        drawn.push(k.profile.clone().expect("an arrival has a profile"));
    }
    assert!(drawn.windows(2).all(|w| w[0] == w[1]), "{drawn:?}");
}

/// Each slot's colour comes from that slot's list and is never a colour an
/// earlier slot took; the eyes and every favourite come from their tables.
/// @spec SIM-KIT-032
#[test]
fn every_draw_comes_from_its_table() {
    let mut patterns = BTreeSet::new();
    let (mut counts, mut all_kinds, mut poses) =
        (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
    for (i, name) in content::NAME_POOL.iter().enumerate() {
        let p = draw_profile(name, (i % 5) as u32);
        let def = content::fur_pattern_def(p.fur.pattern);
        assert_eq!(
            p.fur.colours.len(),
            def.slots.len(),
            "{name}: one colour per slot"
        );
        for (slot, c) in p.fur.colours.iter().enumerate() {
            assert!(
                def.slots[slot].contains(c),
                "{name}: slot {slot} took {c:?}"
            );
            assert!(
                !p.fur.colours[..slot].contains(c),
                "{name}: slot {slot} repeats an earlier slot's {c:?}"
            );
        }
        // Four to six favourites, each of its own kind, in kind order, each
        // from its kind's table.
        assert!(
            (4..=6).contains(&p.favourites.len()),
            "{name}: {}",
            p.favourites.len()
        );
        let kinds: Vec<FavouriteKind> = p.favourites.iter().map(|f| f.kind).collect();
        let mut ordered = kinds.clone();
        ordered.sort();
        ordered.dedup();
        assert_eq!(kinds, ordered, "{name}: distinct kinds, in kind order");
        for f in &p.favourites {
            let def = content::favourite_kind_def(f.kind);
            assert!(
                def.entries.contains(&f.entry.as_str()),
                "{name}: {:?} {:?}",
                f.kind,
                f.entry
            );
        }
        patterns.insert(p.fur.pattern);
        counts.insert(p.favourites.len());
        all_kinds.extend(kinds);
        poses.insert(p.pose);
    }
    assert_eq!(
        patterns.len(),
        FurPattern::ALL.len(),
        "every pattern turns up"
    );
    assert_eq!(
        counts.len(),
        3,
        "four, five, and six favourites all turn up"
    );
    assert_eq!(
        all_kinds.len(),
        FavouriteKind::ALL.len(),
        "every kind turns up"
    );
    assert_eq!(poses.len(), Pose::ALL.len(), "every pose turns up");
}

/// Kittens differ in which kinds of favourites they have, not only in what
/// they are.
/// @spec SIM-KIT-032
#[test]
fn kittens_are_known_for_different_things() {
    let sets: BTreeSet<Vec<FavouriteKind>> = content::NAME_POOL
        .iter()
        .take(60)
        .map(|n| {
            draw_profile(n, 0)
                .favourites
                .iter()
                .map(|f| f.kind)
                .collect()
        })
        .collect();
    assert!(
        sets.len() >= 50,
        "sixty kittens, {} sets of kinds",
        sets.len()
    );
}

// ---------------------------------------------------------------------------
// SIM-KIT-033 — no rule reads a profile
// ---------------------------------------------------------------------------

/// Two meadows that differ only in their kittens' profiles stay identical,
/// profiles aside, through a long run of ticks and commands.
/// @spec SIM-KIT-033
#[test]
fn no_rule_reads_a_profile() {
    let mut a = GameState::new(77);
    adopt(&mut a, 8);
    grant(&mut a, StructureId::CozyCushion, 6);
    let mut b = a.clone();
    for (i, k) in b.kittens.iter_mut().enumerate() {
        k.profile = Some(draw_profile("Somebody Else", i as u32 + 40));
    }
    assert_ne!(a.kittens[0].profile, b.kittens[0].profile, "fixture");
    for s in [&mut a, &mut b] {
        give(s, ResourceId::Catnip, 400.0);
        let _ = s.apply(Command::AssignJob(JobId::Gardener));
        let _ = s.apply(Command::AssignJob(JobId::Gardener));
        let _ = s.apply(Command::AppointElder(2));
        run_days(s, 150);
        let _ = s.apply(Command::UnassignJob(JobId::Gardener));
        run_days(s, 150);
    }
    let strip = |s: &GameState| {
        let mut s = s.clone();
        for k in &mut s.kittens {
            k.profile = None;
        }
        save_string(&s, 1_000)
    };
    assert_eq!(strip(&a), strip(&b));
}

// ---------------------------------------------------------------------------
// SIM-KIT-034 — kept for life
// ---------------------------------------------------------------------------

/// Wandering off and coming home, promotion, and becoming the Elder leave a
/// profile as it was.
/// @spec SIM-KIT-034
#[test]
fn a_profile_survives_everything_a_kitten_does() {
    let mut s = GameState::new(5);
    adopt(&mut s, 4);
    let before: Vec<Option<Profile>> = s.kittens.iter().map(|k| k.profile.clone()).collect();
    // A famine sends kittens next door; a full pantry brings them home.
    give(&mut s, ResourceId::Catnip, 0.0);
    run_days(&mut s, 8);
    assert!(
        s.kittens
            .iter()
            .any(|k| k.location == Location::NeighborsYard),
        "fixture: someone wandered"
    );
    give(&mut s, ResourceId::Catnip, 500.0);
    run_days(&mut s, 8);
    s.kittens[1].exp = 10_000.0;
    give(&mut s, ResourceId::Shinies, 1_000.0);
    s.apply(Command::Promote(1)).expect("promotion");
    s.apply(Command::AppointElder(1)).expect("appointment");
    let after: Vec<Option<Profile>> = s.kittens.iter().map(|k| k.profile.clone()).collect();
    assert_eq!(after, before);
}

/// A stored profile is read back as stored, not drawn again: a profile no
/// draw would give a kitten of that name survives a save and a load.
/// @spec SIM-KIT-034, SAVE-027
#[test]
fn a_stored_profile_is_kept_not_redrawn() {
    let mut s = GameState::new(6);
    adopt(&mut s, 2);
    let odd = draw_profile("Not This Kitten", 9);
    s.kittens[0].profile = Some(odd.clone());
    let back = load_string(&save_string(&s, 1_000), 1_000).unwrap().state;
    assert_eq!(back.kittens[0].profile.as_ref(), Some(&odd));
    assert_eq!(back.kittens[1].profile, s.kittens[1].profile);
}

// ---------------------------------------------------------------------------
// SIM-MOVE-001, 002 — the album and the baskets keep profiles
// ---------------------------------------------------------------------------

/// A Move memorializes each kitten with its profile, and each kitten it
/// carries keeps its own.
/// @spec SIM-MOVE-001, SIM-MOVE-002, SIM-KIT-034
#[test]
fn a_move_keeps_every_profile() {
    let mut s = GameState::new(404);
    s.kittens = (0..71)
        .map(|i| {
            let name = content::NAME_POOL[i];
            let mut k = bare(name, 1 + i as u64);
            k.profile = Some(draw_profile(name, 0));
            k
        })
        .collect();
    s.traditions.insert(TraditionId::TravelingBasketsI);
    let profiles: Vec<Option<Profile>> = s.kittens.iter().map(|k| k.profile.clone()).collect();
    s.apply(Command::GreatMeadowMove).expect("a Move");
    let archived: Vec<Option<Profile>> = s.family_album.iter().map(|a| a.profile.clone()).collect();
    assert_eq!(
        archived, profiles,
        "each memorial keeps its kitten's profile"
    );
    assert_eq!(s.kittens.len(), 1, "fixture: one basket");
    assert_eq!(
        s.kittens[0].profile, profiles[0],
        "the carried kitten keeps its own"
    );
}

// ---------------------------------------------------------------------------
// SIM-KIT-035..039 — the tables, and a fur in words
// ---------------------------------------------------------------------------

/// Every pattern has one to three slots, each of which keeps a colour to take
/// whatever the slots before it took; its words mark only slots it has.
/// @spec SIM-KIT-035
#[test]
fn every_pattern_can_always_fill_its_slots() {
    for p in FurPattern::ALL {
        let def = content::fur_pattern_def(p);
        assert!((1..=3).contains(&def.slots.len()), "{p:?}");
        assert!(!def.words.trim().is_empty(), "{p:?} has words");
        for k in 0..10 {
            if def.words.contains(&format!("{{{k}}}")) {
                assert!(k < def.slots.len(), "{p:?} marks slot {k}, which it lacks");
            }
        }
        // Every choice the earlier slots can make leaves each slot a colour.
        let mut prefixes: Vec<Vec<FurColour>> = vec![vec![]];
        for (slot, list) in def.slots.iter().enumerate() {
            assert!(!list.is_empty(), "{p:?} slot {slot}");
            let mut next = Vec::new();
            for taken in &prefixes {
                let left: Vec<FurColour> = list
                    .iter()
                    .copied()
                    .filter(|c| !taken.contains(c))
                    .collect();
                assert!(!left.is_empty(), "{p:?} slot {slot} after {taken:?}");
                for c in left {
                    let mut t = taken.clone();
                    t.push(c);
                    next.push(t);
                }
            }
            prefixes = next;
        }
    }
}

/// Every eye look names both eyes and has words, and at least one look has
/// two different eyes.
/// @spec SIM-KIT-036
#[test]
fn every_eye_look_names_both_eyes() {
    let mut odd = 0;
    for l in EyeLook::ALL {
        let def = content::eye_look_def(l);
        assert!(!def.words.trim().is_empty(), "{l:?}");
        if def.left != def.right {
            odd += 1;
        }
    }
    assert!(odd >= 1, "some kitten has odd eyes");
}

/// At least twelve kinds, each with a short label of its own and at least
/// sixteen short, printable, distinct entries.
/// @spec SIM-KIT-037
#[test]
fn the_favourite_tables_are_full_and_tidy() {
    let tables = favourite_tables();
    assert!(tables.len() >= 12, "{} kinds", tables.len());
    let labels: BTreeSet<&str> = tables.iter().map(|(l, _)| *l).collect();
    assert_eq!(labels.len(), tables.len(), "every kind has its own label");
    for (label, table) in tables {
        let n = label.chars().count();
        assert!((1..=20).contains(&n), "{label:?} is {n} characters");
        assert!(
            label.chars().all(|c| c.is_ascii_graphic() || c == ' '),
            "{label:?}"
        );
        assert!(table.len() >= 16, "{label}: {} entries", table.len());
        let mut seen = BTreeSet::new();
        for e in table {
            let n = e.chars().count();
            assert!((1..=40).contains(&n), "{label}: {e:?} is {n} characters");
            assert!(
                e.chars().all(|c| c.is_ascii_graphic() || c == ' '),
                "{label}: {e:?} is printable ASCII"
            );
            assert_eq!(e.trim(), *e, "{label}: {e:?} has no loose spaces");
            assert!(seen.insert(e.to_lowercase()), "{label}: {e:?} twice");
        }
    }
}

/// No favourite names anything else in the game, as a word or a run of words.
/// @spec SIM-KIT-038
#[test]
fn no_favourite_names_a_thing_in_the_game() {
    let vocabulary = game_vocabulary();
    for (label, table) in favourite_tables() {
        for name in &vocabulary {
            assert!(
                !contains_words(label, name),
                "the label {label:?} names {name:?}"
            );
        }
        for e in table {
            for name in &vocabulary {
                assert!(
                    !contains_words(e, name),
                    "{label}: {e:?} names {name:?}, a thing in the game"
                );
            }
        }
    }
}

/// A fur in words is its pattern's words with each slot mark filled in.
/// @spec SIM-KIT-039
#[test]
fn a_fur_reads_as_its_pattern_words_filled_in() {
    for p in FurPattern::ALL {
        let def = content::fur_pattern_def(p);
        // The first colour each slot can take, distinct as the draw keeps them.
        let mut colours: Vec<FurColour> = Vec::new();
        for list in def.slots {
            let c = list.iter().copied().find(|c| !colours.contains(c)).unwrap();
            colours.push(c);
        }
        let mut want = def.words.to_string();
        for (k, c) in colours.iter().enumerate() {
            want = want.replace(&format!("{{{k}}}"), content::fur_colour_name(*c));
        }
        let fur = Fur {
            pattern: p,
            colours,
        };
        let words = fur_words(&fur);
        assert_eq!(words, want, "{p:?}");
        assert!(!words.contains('{'), "{p:?}: {words:?} left a mark");
    }
}

/// Every fur colour has a name.
/// @spec SIM-KIT-039
#[test]
fn every_fur_colour_has_a_name() {
    let names: BTreeSet<&str> = FurColour::ALL
        .iter()
        .map(|c| content::fur_colour_name(*c))
        .collect();
    assert_eq!(names.len(), FurColour::ALL.len(), "names are distinct");
    assert!(names.iter().all(|n| !n.is_empty()));
}

// ---------------------------------------------------------------------------
// SIM-KIT-040 — each trait as the player reads it
// ---------------------------------------------------------------------------

/// The numbers a blurb states, as written: maximal runs of digits and dots.
fn figures(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || (c == '.' && !run.is_empty()) {
            run.push(c);
        } else if !run.is_empty() {
            out.push(run.trim_end_matches('.').to_string());
            run.clear();
        }
    }
    out
}

/// A figure as a blurb writes it: no trailing zeros, no trailing point.
fn written(v: f64) -> String {
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A meadow whose one kitten, holding `t`, is the Elder.
fn led_by(t: KittenTrait) -> GameState {
    let mut s = GameState::new(8);
    let mut k = bare("Pip", 1);
    k.trait_id = t;
    s.kittens = vec![k];
    s.elder = Some(0);
    s
}

/// Each trait has its name — None read as Easygoing — and a blurb beginning
/// `As Elder: ` whose marks all name resources.
/// @spec SIM-KIT-040
#[test]
fn every_trait_has_a_name_and_an_as_elder_blurb() {
    for t in KittenTrait::ALL {
        let def = content::trait_def(t);
        let want = if t == KittenTrait::None {
            "Easygoing".to_string()
        } else {
            format!("{t:?}")
        };
        assert_eq!(def.name, want);
        assert!(
            def.blurb.starts_with("As Elder: "),
            "{t:?}: {:?}",
            def.blurb
        );
        let marks = def.blurb.matches('{').count();
        let resources = blurb_parts(def.blurb)
            .iter()
            .filter(|p| matches!(p, BlurbPart::Resource(..)))
            .count();
        assert_eq!(marks, resources, "{t:?}: every mark names a resource");
    }
}

/// Every figure a trait's blurb states is the figure its effect applies, and
/// a craft trait's blurb marks exactly the goods its effect reaches.
/// @spec SIM-KIT-040
#[test]
fn every_trait_blurb_states_its_effect_exactly() {
    let cases: [(KittenTrait, Effect, f64); 7] = [
        (KittenTrait::Crafty, Effect::CraftRatio, 100.0),
        (
            KittenTrait::Brassy,
            Effect::CraftRatioFor(CraftId::Badges),
            100.0,
        ),
        (
            KittenTrait::Alchemical,
            Effect::CraftRatioFor(CraftId::Pavers),
            100.0,
        ),
        (KittenTrait::Charming, Effect::TradeRatio, 100.0),
        (KittenTrait::Bold, Effect::PounceRatio, 1.0),
        (KittenTrait::Curious, Effect::IdeaCostRatio, 100.0),
        (KittenTrait::Serene, Effect::RiteCostRatio, 100.0),
    ];
    for (t, key, scale) in cases {
        let s = led_by(t);
        let applied = s.effect(key);
        assert!(applied != 0.0, "fixture: {t:?} moves {key:?}");
        let text = content::blurb_text(content::trait_def(t).blurb);
        assert_eq!(
            figures(&text),
            vec![written(applied.abs() * scale)],
            "{t:?}: {text:?}"
        );
    }
    let calm = content::blurb_text(content::trait_def(KittenTrait::None).blurb);
    assert!(
        figures(&calm).is_empty(),
        "Easygoing states no figure: {calm:?}"
    );

    for t in [KittenTrait::Brassy, KittenTrait::Alchemical] {
        let s = led_by(t);
        let marked: BTreeSet<String> = blurb_parts(content::trait_def(t).blurb)
            .into_iter()
            .filter_map(|p| match p {
                BlurbPart::Resource(r, _) => Some(format!("{r:?}")),
                BlurbPart::Text(_) => None,
            })
            .collect();
        let reached: BTreeSet<String> = CraftId::ALL
            .iter()
            .filter(|c| s.effect(Effect::CraftRatioFor(**c)) != 0.0)
            .map(|c| format!("{c:?}"))
            .collect();
        assert_eq!(marked, reached, "{t:?}");
    }
}

// ---------------------------------------------------------------------------
// SIM-KIT-041 — the facts query
// ---------------------------------------------------------------------------

/// Trait and rank are the kitten's own; a missing kitten has no facts.
/// @spec SIM-KIT-041
#[test]
fn facts_report_the_trait_and_rank() {
    let mut s = GameState::new(9);
    let mut k = bare("Pip", 12);
    k.trait_id = KittenTrait::Bold;
    k.rank = 3;
    s.kittens = vec![k];
    let f = s.kitten_facts(0).expect("a rostered kitten");
    assert_eq!(f.trait_id, KittenTrait::Bold);
    assert_eq!(f.rank, 3);
    assert!(s.kitten_facts(1).is_none());
}

/// Best at: the job with the most skill, ties to the earliest job, with its
/// tier's bonus; nothing while every skill is zero.
/// @spec SIM-KIT-041
#[test]
fn facts_report_the_job_it_knows_best() {
    let mut s = GameState::new(10);
    s.kittens = vec![bare("Pip", 1)];
    assert_eq!(s.kitten_facts(0).unwrap().best_at, None, "no skill yet");

    let jobs = JobId::ALL;
    let (first, second) = (jobs[1], jobs[3]);
    for (xp, bonus) in [
        (40.0, 0.0),
        (100.0, 0.0125),
        (640.0, 0.025),
        (1200.0, 0.045),
        (2500.0, 0.075),
        (5000.0, 0.125),
        (9000.0, 0.1875),
    ] {
        s.kittens[0].skills = [0.0; 8];
        s.kittens[0].skills[second as usize] = xp;
        s.kittens[0].skills[first as usize] = xp / 2.0;
        let best = s.kitten_facts(0).unwrap().best_at.expect("some skill");
        assert_eq!(best.job, second, "{xp}");
        assert!((best.bonus - bonus).abs() < 1e-12, "{xp}: {}", best.bonus);
    }
    s.kittens[0].skills[first as usize] = 9000.0;
    assert_eq!(
        s.kitten_facts(0).unwrap().best_at.unwrap().job,
        first,
        "ties go first"
    );
}

/// Since: the day, season, and year it arrived — or carried along, for a
/// kitten whose arrival day is 0.
/// @spec SIM-KIT-041
#[test]
fn facts_report_when_it_joined() {
    let mut s = GameState::new(11);
    s.kittens = vec![bare("Pip", 12), bare("Mo", 130), bare("Carried", 0)];
    assert_eq!(
        s.kitten_facts(0).unwrap().arrival,
        Arrival::Day {
            day: 12,
            season: Season::ALL[0],
            year: 0
        }
    );
    assert_eq!(
        s.kitten_facts(1).unwrap().arrival,
        Arrival::Day {
            day: 130,
            season: Season::ALL[1],
            year: 1
        }
    );
    assert_eq!(s.kitten_facts(2).unwrap().arrival, Arrival::CarriedAlong);
}

// ---------------------------------------------------------------------------
// SAVE-027, 028 — saved, and drawn for a save from before profiles
// ---------------------------------------------------------------------------

const V7_FIXTURE: &str = include_str!("fixtures/save_v7.json");

/// Kitten and memorial profiles survive a save and a load, under a version
/// newer than v7.
/// @spec SAVE-027
#[test]
fn profiles_are_saved() {
    const { assert!(SAVE_VERSION >= 9, "a version step carries them") };
    let mut s = GameState::new(12);
    adopt(&mut s, 3);
    s.family_album.push(ArchivedKitten {
        name: "Old Friend".into(),
        trait_id: KittenTrait::Serene,
        rank: 1,
        generation: 0,
        profile: Some(draw_profile("Old Friend", 0)),
    });
    let back = load_string(&save_string(&s, 1_000), 1_000).unwrap().state;
    assert_eq!(back.kittens, s.kittens);
    assert_eq!(back.family_album, s.family_album);
}

/// A v7 save's memorials and kittens draw the profiles their names and
/// generations draw — save a kitten the album remembers from the generation
/// before, which takes that memorial's profile.
/// @spec SAVE-028, SAVE-013
#[test]
fn a_v7_save_draws_its_kittens_profiles() {
    assert!(V7_FIXTURE.contains("\"save_version\": 7"));
    let s = load_string(V7_FIXTURE, 1_760_400_000).unwrap().state;
    assert_eq!(s.generation, 1, "fixture: a second generation");
    for a in &s.family_album {
        assert_eq!(
            a.profile.as_ref(),
            Some(&draw_profile(&a.name, a.generation)),
            "memorial {}",
            a.name
        );
    }
    let remembered: Vec<&ArchivedKitten> = s
        .family_album
        .iter()
        .filter(|a| a.generation == 0)
        .collect();
    let mut carried = 0;
    for k in &s.kittens {
        let want = match remembered.iter().find(|a| a.name == k.name) {
            Some(a) => {
                carried += 1;
                a.profile.clone()
            }
            None => Some(draw_profile(&k.name, s.generation)),
        };
        assert_eq!(k.profile, want, "kitten {}", k.name);
    }
    assert_eq!(carried, 1, "fixture: one kitten came along");
}

/// A memorial the album also holds from the generation before takes that
/// memorial's profile, so a kitten carried twice looks the same on each page.
/// @spec SAVE-028
#[test]
fn a_kitten_carried_twice_looks_the_same_on_every_page() {
    let mut s = GameState::new(13);
    s.generation = 2;
    for generation in [0, 1] {
        s.family_album.push(ArchivedKitten {
            name: "Wanderer".into(),
            trait_id: KittenTrait::Curious,
            rank: 0,
            generation,
            profile: None,
        });
    }
    s.kittens = vec![bare("Wanderer", 0), bare("Newcomer", 3)];
    let back = load_string(&save_string(&s, 1_000), 1_000).unwrap().state;
    let first = Some(draw_profile("Wanderer", 0));
    assert_eq!(back.family_album[0].profile, first);
    assert_eq!(back.family_album[1].profile, first);
    assert_eq!(back.kittens[0].profile, first);
    assert_eq!(back.kittens[1].profile, Some(draw_profile("Newcomer", 2)));
}

/// Migrating a v7 save adds a profile to each kitten and memorial and changes
/// nothing else, the state RNG included.
/// @spec SAVE-028, SAVE-013
#[test]
fn a_v7_save_migrates_with_everything_else_exact() {
    let before: serde_json::Value = serde_json::from_str(V7_FIXTURE).unwrap();
    let at = before["saved_at_unix"].as_u64().unwrap();
    let sd = begin_load(V7_FIXTURE, at)
        .expect("a v7 save loads")
        .set_down();
    let resaved: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &sd.state,
        at,
        sd.owed_ticks,
        Some(&sd.report_so_far),
    ))
    .unwrap();
    assert_eq!(resaved["save_version"], SAVE_VERSION);
    let (was, now) = (
        before["state"].as_object().unwrap(),
        resaved["state"].as_object().unwrap(),
    );
    assert_eq!(
        was.keys().collect::<Vec<_>>(),
        now.keys().collect::<Vec<_>>(),
        "no state field is added or lost"
    );
    for (key, old) in was {
        if key == "kittens" || key == "family_album" {
            let (old, new) = (old.as_array().unwrap(), now[key].as_array().unwrap());
            assert_eq!(old.len(), new.len(), "{key}");
            for (o, n) in old.iter().zip(new) {
                let mut n = n.as_object().unwrap().clone();
                assert!(
                    n.remove("profile").is_some_and(|p| !p.is_null()),
                    "{key}: {o}"
                );
                assert_eq!(o.as_object().unwrap(), &n, "{key}");
            }
        } else {
            assert_eq!(old, &now[key], "migrating a v7 save changed `{key}`");
        }
    }
}

/// A hand-edited save of the current version that holds a kitten with no
/// profile gets one on load, by the same rule: physics, not an error.
/// @spec SAVE-028
#[test]
fn a_hand_edited_save_missing_a_profile_gets_one() {
    let mut s = GameState::new(14);
    adopt(&mut s, 2);
    let mut v: serde_json::Value = serde_json::from_str(&save_string(&s, 1_000)).unwrap();
    v["state"]["kittens"][1]
        .as_object_mut()
        .unwrap()
        .remove("profile");
    let back = load_string(&v.to_string(), 1_000).unwrap().state;
    assert_eq!(back.kittens[0].profile, s.kittens[0].profile);
    assert_eq!(
        back.kittens[1].profile,
        Some(draw_profile(&s.kittens[1].name, s.generation))
    );
}

const V8_FIXTURE: &str = include_str!("fixtures/save_v8.json");

/// A v8 save's kittens and memorials keep the fur and eyes they stored and
/// gain favourites of varied kinds and a pose, as their names and
/// generations draw them.
/// @spec SAVE-029
#[test]
fn a_v8_save_keeps_its_kittens_fur_and_eyes() {
    assert!(V8_FIXTURE.contains("\"save_version\": 8"));
    let before: serde_json::Value = serde_json::from_str(V8_FIXTURE).unwrap();
    let s = load_string(V8_FIXTURE, 1_760_500_000).unwrap().state;
    let stored =
        |list: &str, i: usize, field: &str| before["state"][list][i]["profile"][field].clone();
    for (i, k) in s.kittens.iter().enumerate() {
        let p = k.profile.as_ref().expect("a profile");
        assert_eq!(
            serde_json::to_value(&p.fur).unwrap(),
            stored("kittens", i, "fur"),
            "{}",
            k.name
        );
        assert_eq!(
            serde_json::to_value(p.eyes).unwrap(),
            stored("kittens", i, "eyes"),
            "{}",
            k.name
        );
        assert!((4..=6).contains(&p.favourites.len()), "{}", k.name);
    }
    for (i, a) in s.family_album.iter().enumerate() {
        let p = a.profile.as_ref().expect("a profile");
        assert_eq!(
            serde_json::to_value(&p.fur).unwrap(),
            stored("family_album", i, "fur")
        );
        assert_eq!(
            Some(p),
            Some(&Profile {
                fur: p.fur.clone(),
                eyes: p.eyes,
                ..draw_profile(&a.name, a.generation)
            }),
            "memorial {}: favourites and pose as its draw gives them",
            a.name
        );
    }
}

/// Migrating a v8 save changes nothing but its profiles' favourites and
/// poses.
/// @spec SAVE-029, SAVE-013
#[test]
fn a_v8_save_migrates_with_everything_else_exact() {
    let before: serde_json::Value = serde_json::from_str(V8_FIXTURE).unwrap();
    let at = before["saved_at_unix"].as_u64().unwrap();
    let sd = begin_load(V8_FIXTURE, at)
        .expect("a v8 save loads")
        .set_down();
    let resaved: serde_json::Value = serde_json::from_str(&save_string_carrying(
        &sd.state,
        at,
        sd.owed_ticks,
        Some(&sd.report_so_far),
    ))
    .unwrap();
    assert_eq!(resaved["save_version"], SAVE_VERSION);
    let (was, now) = (
        before["state"].as_object().unwrap(),
        resaved["state"].as_object().unwrap(),
    );
    assert_eq!(
        was.keys().collect::<Vec<_>>(),
        now.keys().collect::<Vec<_>>()
    );
    for (key, old) in was {
        if key == "kittens" || key == "family_album" {
            let (old, new) = (old.as_array().unwrap(), now[key].as_array().unwrap());
            assert_eq!(old.len(), new.len(), "{key}");
            for (o, n) in old.iter().zip(new) {
                let (mut o, mut n) = (
                    o.as_object().unwrap().clone(),
                    n.as_object().unwrap().clone(),
                );
                let (op, np) = (o.remove("profile").unwrap(), n.remove("profile").unwrap());
                assert_eq!(o, n, "{key}: everything but the profile is kept");
                assert_eq!(op["fur"], np["fur"], "{key}: the fur is kept");
                assert_eq!(op["eyes"], np["eyes"], "{key}: the eyes are kept");
            }
        } else {
            assert_eq!(old, &now[key], "migrating a v8 save changed `{key}`");
        }
    }
}
