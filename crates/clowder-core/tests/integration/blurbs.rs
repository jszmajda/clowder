//! Every purchasable item says what it does: rites, arch upgrades,
//! traditions, and dream looms each carry a blurb, as ideas, refinements, and
//! house rules do.

use clowder_core::content::{self, blurb_parts, BlurbPart};
use clowder_core::*;

// ---------------------------------------------------------------------------
// SIM-SUN-009, SIM-UNI-012, SIM-MOVE-014, SIM-DREAM-012 — every purchasable
// item says what it does
// ---------------------------------------------------------------------------

/// Every rite, arch upgrade, tradition, and dream loom carries a blurb: a
/// joke and its effect, each sentence closed, every mark naming a resource.
/// @spec SIM-SUN-009, SIM-UNI-012, SIM-MOVE-014, SIM-DREAM-012
#[test]
fn every_purchasable_item_says_what_it_does() {
    let mut blurbs: Vec<(String, &str)> = Vec::new();
    for r in RiteId::ALL {
        blurbs.push((format!("{r:?}"), content::rite_def(r).blurb));
    }
    for a in ArchId::ALL {
        blurbs.push((format!("{a:?}"), content::arch_def(a).blurb));
    }
    for t in TraditionId::ALL {
        blurbs.push((format!("{t:?}"), content::tradition_def(t).blurb));
    }
    for l in DreamLoomId::ALL {
        blurbs.push((format!("{l:?}"), content::dream_loom_def(l).blurb));
    }
    for (item, blurb) in blurbs {
        let text = content::blurb_text(blurb);
        assert!(
            text.matches(". ").count() + text.matches("? ").count() + text.matches(": ").count()
                >= 1,
            "{item}: a joke, then the effect: {text:?}"
        );
        assert!(text.ends_with('.'), "{item}: {text:?}");
        let marks = blurb.matches('{').count();
        let resources = blurb_parts(blurb)
            .iter()
            .filter(|p| matches!(p, BlurbPart::Resource(..)))
            .count();
        assert_eq!(marks, resources, "{item}: every mark names a resource");
    }
}

// ---------------------------------------------------------------------------
// SIM-ADV-006 — every Adventure says what it asks and what it gives
// ---------------------------------------------------------------------------

/// A reward figure as an Adventure's blurb states it: a ratio as the
/// percentage it adds or trims, and a pounce ratio or a long-sun extension as
/// the amount itself, as a trait's blurb and a structure's statement put them.
fn reward_figure(key: Effect, x: f64) -> String {
    let shown = |v: f64| format!("{}", (v * 1000.0).round() / 1000.0);
    match key {
        Effect::PounceRatio | Effect::LongSunCapBonus => shown(x.abs()),
        _ => format!("{}%", shown(x.abs() * 100.0)),
    }
}

/// Every Adventure carries a blurb: a joke, then its twist, its goal by the
/// names its rows carry, and its reward — a stacking reward's per-completion
/// amount and cap as the core's own figures, each a bound, or the name of the
/// tradition a one-shot unlock opens. An Adventure whose completion adds
/// nothing states no reward.
/// @spec SIM-ADV-006
#[test]
fn every_adventure_says_its_twist_goal_and_reward() {
    use content::{adventure_def, idea_def, structure_def};

    for a in AdventureId::ALL {
        let def = adventure_def(a);
        let text = content::blurb_text(def.blurb);
        assert!(
            text.matches(". ").count() >= 2,
            "{a:?}: a joke, then the twist and the goal: {text:?}"
        );
        assert!(text.ends_with('.'), "{a:?}: {text:?}");
        let marks = def.blurb.matches('{').count();
        let resources = blurb_parts(def.blurb)
            .iter()
            .filter(|p| matches!(p, BlurbPart::Resource(..)))
            .count();
        assert_eq!(marks, resources, "{a:?}: every mark names a resource");
        assert!(
            text.contains("While it lasts, "),
            "{a:?} states its twist: {text:?}"
        );
        assert!(text.contains("Goal: "), "{a:?} states its goal: {text:?}");

        let rewards = def.reward_per != 0.0 || def.unlocks.is_some();
        assert_eq!(
            text.contains("Finishing it "),
            rewards,
            "{a:?}: a reward is stated exactly where the core applies one: {text:?}"
        );
        if let Some(t) = def.unlocks {
            let name = spaced(&format!("{t:?}"));
            assert!(
                text.contains(&format!("Finishing it opens the {name} tradition")),
                "{a:?}: a one-shot unlock names what it opens: {text:?}"
            );
        } else if rewards {
            let per = reward_figure(def.reward_key, def.reward_per);
            let cap = reward_figure(def.reward_key, def.reward_cap);
            assert!(
                text.contains(&format!("up to {per}"))
                    && text.contains(&format!("at most {cap} in all")),
                "{a:?}: one completion adds up to {per}, every completion at most {cap}: {text:?}"
            );
        }
    }

    // A goal that asks for a structure or an idea names it as its row does.
    let names = |a: AdventureId| content::blurb_text(adventure_def(a).blurb);
    for (a, wanted) in [
        (
            AdventureId::LongWinter,
            vec![structure_def(StructureId::SunbeamAltar).name],
        ),
        (
            AdventureId::NapTime,
            vec![structure_def(StructureId::YarnComputer).name],
        ),
        (
            AdventureId::FrugalMeadow,
            vec![
                structure_def(StructureId::DynamoWheel).name,
                structure_def(StructureId::StreamWheel).name,
                structure_def(StructureId::StarBasin).name,
            ],
        ),
        (
            AdventureId::SilentSun,
            vec![structure_def(StructureId::MemoryChest).name],
        ),
        (
            AdventureId::GentlePaws,
            vec![idea_def(IdeaId::PenPalNetworks).name],
        ),
    ] {
        for name in wanted {
            assert!(
                names(a).contains(name),
                "{a:?}'s goal names {name}: {:?}",
                names(a)
            );
        }
    }

    // A twist figure the core holds as a constant is the one the blurb states.
    let frugal = names(AdventureId::FrugalMeadow);
    let twist = reward_figure(Effect::EnergyConsumptionRatio, content::FRUGAL_MEADOW_TWIST);
    assert!(
        frugal.contains(&format!("{twist} more")),
        "{frugal:?} states {twist}"
    );
    assert_eq!(
        content::TIDY_MESS_MULT,
        2.0,
        "fixture: the twist the blurb words as twice"
    );
    assert!(names(AdventureId::TidyMeadow).contains("twice as fast"));
}

/// An identifier spelled as words, as the shell names a tradition's row.
fn spaced(tag: &str) -> String {
    let mut out = String::new();
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}
