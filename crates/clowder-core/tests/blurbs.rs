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
