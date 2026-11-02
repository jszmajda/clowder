//! What an idea promises, a rule reads it for: the rows of the idea tree whose
//! unlocks once went unread, and what a save from before those gates keeps.
//!
//! @spec SIM-CUR-009, SIM-TRADE-007, SIM-UNI-016, SIM-SKY-012, SIM-DREAM-006
//! @spec SAVE-031

use crate::common::*;

use clowder_core::content;
use clowder_core::*;

fn blurb_of(i: IdeaId) -> &'static str {
    content::idea_def(i).blurb
}

fn meadow() -> GameState {
    let mut s = GameState::new(30_030);
    for r in ResourceId::ALL {
        s.resources[r.ix()] = 1.0e9;
    }
    grant(&mut s, StructureId::Pantry, 200);
    for r in ResourceId::ALL {
        let cap = s.cap(r);
        s.resources[r.ix()] = if cap.is_finite() { cap } else { 1.0e9 };
    }
    s
}

// ---- Scout Picnicking opens the picnics ----

/// @spec SIM-CUR-009, SIM-TRADE-007
#[test]
fn a_picnic_waits_on_scout_picnicking_and_not_on_the_trade_surface() {
    let mut s = meadow();
    s.ideas.insert(IdeaId::GiftEconomy);
    assert!(!s.command_gate_open(&Command::ScoutPicnic));
    assert_eq!(
        s.apply(Command::ScoutPicnic).unwrap_err(),
        CommandError::Locked
    );

    s.ideas.insert(IdeaId::ScoutPicnicking);
    assert!(s.command_gate_open(&Command::ScoutPicnic));
    s.resources[ResourceId::Pep.ix()] = 1000.0;
    s.apply(Command::ScoutPicnic)
        .expect("the idea opens the picnics");
}

// ---- Dream Tending opens the arches that dream ----

const THAT_DREAM: [ArchId; 2] = [ArchId::MemoryArch, ArchId::EverdreamArch];

/// @spec SIM-CUR-009, SIM-UNI-016
#[test]
fn the_arches_that_dream_wait_on_dream_tending() {
    let mut s = meadow();
    s.ideas.insert(IdeaId::ParadeRoutes);
    for a in THAT_DREAM {
        assert_eq!(
            s.availability(ContentId::Arch(a)),
            Availability::Hidden,
            "{a:?}"
        );
        assert_eq!(
            s.apply(Command::BuyArch(a)).unwrap_err(),
            CommandError::Locked,
            "{a:?}"
        );
    }

    s.ideas.insert(IdeaId::DreamTending);
    for a in THAT_DREAM {
        assert_ne!(
            s.availability(ContentId::Arch(a)),
            Availability::Hidden,
            "{a:?}"
        );
        assert_ne!(
            s.apply(Command::BuyArch(a)).err(),
            Some(CommandError::Locked),
            "{a:?}"
        );
    }
}

/// The other six need only what they always did.
/// @spec SIM-UNI-016
#[test]
fn the_other_arch_upgrades_need_parade_routes_alone() {
    let mut s = meadow();
    s.ideas.insert(IdeaId::ParadeRoutes);
    let others = ArchId::ALL.into_iter().filter(|a| !THAT_DREAM.contains(a));
    assert_eq!(others.clone().count(), 6);
    for a in others {
        assert_ne!(
            s.availability(ContentId::Arch(a)),
            Availability::Hidden,
            "{a:?}"
        );
    }
}

/// An upgrade already bought goes on doing what it does whatever the idea says.
/// @spec SIM-UNI-016, SIM-UNI-007
#[test]
fn a_memory_arch_already_owned_goes_on_dreaming() {
    let mut s = meadow();
    s.ideas.insert(IdeaId::ParadeRoutes);
    s.arches_bought.insert(ArchId::MemoryArch);
    grant(&mut s, StructureId::RainbowArch, 200);
    let before = s.dream_drift;
    run_ticks(&mut s, 1);
    assert!(s.dream_drift > before || s.stock(ResourceId::Dreamcorns) > 0.0);
}

// ---- Hushfelt Seeking opens the hushfelt sources ----

const SOURCES: [(StructureId, SiteId); 2] = [
    (StructureId::HushfeltGrotto, SiteId::TheBrook),
    (StructureId::WhisperPost, SiteId::WhisperingWoods),
];

/// @spec SIM-CUR-009, SIM-SKY-012, SIM-SKY-003
#[test]
fn a_hushfelt_source_needs_both_its_site_and_hushfelt_seeking() {
    for (structure, site) in SOURCES {
        let mut site_alone = meadow();
        site_alone.sites_reached.insert(site);
        assert!(!site_alone.structure_unlocked(structure), "{structure:?}");
        assert_eq!(
            site_alone.apply(Command::Build(structure)).unwrap_err(),
            CommandError::Locked,
            "{structure:?}"
        );

        let mut idea_alone = meadow();
        idea_alone.ideas.insert(IdeaId::HushfeltSeeking);
        assert!(!idea_alone.structure_unlocked(structure), "{structure:?}");

        let mut both = site_alone.clone();
        both.ideas.insert(IdeaId::HushfeltSeeking);
        assert!(both.structure_unlocked(structure), "{structure:?}");
    }
}

/// Every other outpost needs its site alone.
/// @spec SIM-SKY-003, SIM-SKY-012
#[test]
fn every_other_outpost_needs_its_site_alone() {
    for site in SiteId::ALL {
        let outpost = content::site_def(site).outpost;
        if SOURCES.iter().any(|(s, _)| *s == outpost) {
            continue;
        }
        let mut s = meadow();
        assert!(!s.structure_unlocked(outpost), "{outpost:?}");
        s.sites_reached.insert(site);
        assert!(s.structure_unlocked(outpost), "{outpost:?}");
    }
}

// ---- Dream Loomcraft opens the Dream Loom ----

/// The loom opens on the idea that promises it, ahead of Deep Nap Theory, which
/// follows it in the tree.
/// @spec SIM-CUR-009, SIM-DREAM-006, SIM-ENG-032
#[test]
fn the_dream_loom_waits_on_dream_loomcraft_and_nothing_later() {
    let loom = DreamLoomId::ALL[0];
    let mut s = meadow();
    assert_eq!(
        s.availability(ContentId::DreamLoom(loom)),
        Availability::Hidden
    );
    assert_eq!(
        s.apply(Command::BuyDreamLoom(loom)).unwrap_err(),
        CommandError::Locked,
        "nor can a command get past what the row hides"
    );

    assert!(
        content::idea_def(IdeaId::DeepNapTheory)
            .prereqs
            .contains(&IdeaId::DreamLoomcraft),
        "fixture: Deep Nap Theory follows Dream Loomcraft"
    );
    s.ideas.insert(IdeaId::DreamLoomcraft);
    assert_ne!(
        s.availability(ContentId::DreamLoom(loom)),
        Availability::Hidden
    );
    s.apply(Command::BuyDreamLoom(loom))
        .expect("the idea opens the loom");
}

// ---- The ceremonies open with warmth, and the rows say so ----

/// The ceremonies are the one surface with its own reason to wait: warmth having
/// somewhere to be kept. The row that gives it says so, and the row that follows
/// makes no claim on it.
/// @spec SIM-CUR-009
#[test]
fn the_ceremonies_open_with_a_sun_room_and_their_row_says_so() {
    let mut s = meadow();
    grant(&mut s, StructureId::SunRoom, 1);
    assert!(
        s.cap(ResourceId::Warmth) > 0.0,
        "fixture: warmth has a home"
    );
    assert!(!s.ideas.contains(&IdeaId::SolarDevotion));
    assert!(s.command_gate_open(&Command::Bask));

    assert!(
        blurb_of(IdeaId::SunSalutations).contains("ceremonies"),
        "{}",
        blurb_of(IdeaId::SunSalutations)
    );
    assert!(
        !blurb_of(IdeaId::SolarDevotion).contains("ceremonies"),
        "{}",
        blurb_of(IdeaId::SolarDevotion)
    );
}

/// A blurb names what the idea opens, in words the player meets.
/// @spec SIM-CUR-009
#[test]
fn dream_tending_says_which_arches_it_opens() {
    let blurb = blurb_of(IdeaId::DreamTending);
    for a in THAT_DREAM {
        assert!(
            blurb.contains(content::arch_def(a).name),
            "{a:?} should be named in: {blurb}"
        );
    }
}

// ---- SAVE-031: a save from before these gates keeps what it could reach ----

const V10_FIXTURE: &str = include_str!("../fixtures/save_v10.json");

fn loaded(json: &str) -> GameState {
    load_string(json, 1_000_000).expect("the save loads").state
}

/// What the committed fixture could reach, it keeps: each idea is held, revealed,
/// and opens what it gates, none of it paid for.
/// @spec SAVE-031
#[test]
fn a_v10_save_is_granted_the_ideas_for_the_surfaces_it_could_reach() {
    let v: serde_json::Value = serde_json::from_str(V10_FIXTURE).unwrap();
    assert_eq!(v["save_version"], 10, "fixture: written at version 10");
    let mut s = loaded(V10_FIXTURE);
    for i in [
        IdeaId::ScoutPicnicking,
        IdeaId::DreamTending,
        IdeaId::HushfeltSeeking,
    ] {
        assert!(s.ideas.contains(&i), "{i:?} granted");
        assert!(s.revealed.contains(&ContentId::Idea(i)), "{i:?} revealed");
    }
    assert!(s.command_gate_open(&Command::ScoutPicnic));
    assert_ne!(
        s.availability(ContentId::Arch(ArchId::MemoryArch)),
        Availability::Hidden
    );
    assert!(s.structure_unlocked(StructureId::HushfeltGrotto));
    s.resources[ResourceId::Pep.ix()] = 1000.0;
    s.apply(Command::ScoutPicnic)
        .expect("the picnics stay open to this save");
}

/// Set a save's version, and remove anything from it a test names.
fn as_version(json: &str, version: u32, ideas: &[&str], sites: &[&str]) -> String {
    let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
    v["save_version"] = serde_json::json!(version);
    let state = &mut v["state"];
    state["ideas"]
        .as_array_mut()
        .unwrap()
        .retain(|i| !ideas.contains(&i.as_str().unwrap()));
    state["sites_reached"]
        .as_array_mut()
        .unwrap()
        .retain(|i| !sites.contains(&i.as_str().unwrap()));
    v.to_string()
}

/// A grant answers only to what that save could reach.
/// @spec SAVE-031
#[test]
fn a_v10_save_is_granted_only_what_it_could_reach() {
    let s = loaded(&as_version(
        V10_FIXTURE,
        10,
        &["GiftEconomy", "ParadeRoutes"],
        &["TheBrook"],
    ));
    for i in [
        IdeaId::ScoutPicnicking,
        IdeaId::DreamTending,
        IdeaId::HushfeltSeeking,
    ] {
        assert!(!s.ideas.contains(&i), "{i:?} was out of this save's reach");
    }

    let picnics_only = loaded(&as_version(
        V10_FIXTURE,
        10,
        &["ParadeRoutes"],
        &["TheBrook"],
    ));
    assert!(picnics_only.ideas.contains(&IdeaId::ScoutPicnicking));
    assert!(!picnics_only.ideas.contains(&IdeaId::DreamTending));
    assert!(!picnics_only.ideas.contains(&IdeaId::HushfeltSeeking));
}

/// A save from this version has met these gates, so it is granted nothing.
/// @spec SAVE-031
#[test]
fn a_current_save_is_granted_nothing() {
    let mut s = GameState::new(30_031);
    s.ideas.insert(IdeaId::GiftEconomy);
    s.ideas.insert(IdeaId::ParadeRoutes);
    s.sites_reached.insert(SiteId::TheBrook);
    let s = loaded(&save_string(&s, 1_000_000));
    for i in [
        IdeaId::ScoutPicnicking,
        IdeaId::DreamTending,
        IdeaId::HushfeltSeeking,
    ] {
        assert!(!s.ideas.contains(&i), "{i:?} must be researched");
    }
}

/// The grant changes nothing else: the same meadow at either version, past the
/// ideas and their reveals.
/// @spec SAVE-031
#[test]
fn a_grant_changes_nothing_else_in_the_save() {
    let mut old = loaded(V10_FIXTURE);
    let mut new = loaded(&as_version(V10_FIXTURE, SAVE_VERSION, &[], &[]));
    for i in [
        IdeaId::ScoutPicnicking,
        IdeaId::DreamTending,
        IdeaId::HushfeltSeeking,
    ] {
        old.ideas.remove(&i);
        old.revealed.remove(&ContentId::Idea(i));
        new.revealed.remove(&ContentId::Idea(i));
    }
    assert_eq!(save_string(&old, 0), save_string(&new, 0));
}

/// A migrated save stays migrated: a resave is at this version, and loads as is.
/// @spec SAVE-031
#[test]
fn a_migrated_save_resaves_at_this_version() {
    let s = loaded(V10_FIXTURE);
    let resaved: serde_json::Value = serde_json::from_str(&save_string(&s, 1_000_000)).unwrap();
    assert_eq!(resaved["save_version"], SAVE_VERSION);
    const { assert!(SAVE_VERSION >= 11) };
}
