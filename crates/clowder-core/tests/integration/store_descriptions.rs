//! Every store says what it is: a brief description in the content tables,
//! which states no figure and gets ahead of nothing the player has found.

use clowder_core::content;
use clowder_core::*;

/// The most characters a description may run to.
const LONGEST: usize = 90;

/// An identifier spaced into words, as the player reads a resource's name and
/// the name of any item whose definition carries none of its own.
fn spaced(tag: &str) -> String {
    let mut out = String::new();
    for (i, ch) in tag.chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// The spellings a sentence could give a resource's name: as written, and in
/// the singular however its plural is made.
fn spellings(name: &str) -> Vec<String> {
    let lower = name.to_lowercase();
    let mut out = vec![lower.clone()];
    if let Some(stem) = lower.strip_suffix("ies") {
        out.push(format!("{stem}y"));
    }
    if let Some(stem) = lower.strip_suffix("es") {
        out.push(stem.to_string());
    }
    if let Some(stem) = lower.strip_suffix('s') {
        out.push(stem.to_string());
    }
    out
}

/// Whether `text` has `name` as whole words, ignoring case. A word merely
/// beginning with a name is no mention of it: `kitten` does not name a kite.
fn names_whole(text: &str, name: &str) -> bool {
    let text = text.to_lowercase();
    let name = name.to_lowercase();
    text.match_indices(&name).any(|(at, m)| {
        let before = text[..at].chars().last();
        let after = text[at + m.len()..].chars().next();
        before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
    })
}

/// One or two closed sentences, in plain characters, brief enough for a narrow
/// pane; every resource has one.
/// @spec SIM-ECO-061
#[test]
fn every_store_says_what_it_is_briefly() {
    for r in ResourceId::ALL {
        let d = content::resource_description(r);
        assert!(!d.trim().is_empty(), "{r:?} has no description");
        assert!(
            d.chars().count() <= LONGEST,
            "{r:?}: {} characters, past {LONGEST}: {d:?}",
            d.chars().count()
        );
        assert!(d.is_ascii(), "{r:?}: plain characters only: {d:?}");
        assert!(
            d.ends_with(['.', '!', '?']),
            "{r:?}: every sentence is closed: {d:?}"
        );
        let closes = d
            .char_indices()
            .filter(|(i, c)| {
                matches!(c, '.' | '!' | '?') && d[i + 1..].chars().next().is_none_or(|n| n == ' ')
            })
            .count();
        assert!(
            (1..=2).contains(&closes),
            "{r:?}: one or two sentences, found {closes}: {d:?}"
        );
    }
}

/// No two stores read alike, or one store's words would not say what it is.
/// @spec SIM-ECO-061
#[test]
fn no_two_stores_read_alike() {
    for (i, a) in ResourceId::ALL.into_iter().enumerate() {
        for b in ResourceId::ALL.into_iter().skip(i + 1) {
            assert_ne!(
                content::resource_description(a),
                content::resource_description(b),
                "{a:?} and {b:?} read alike"
            );
        }
    }
}

/// A figure in a sentence goes stale, and every figure is the core's to report.
/// @spec SIM-ECO-062
#[test]
fn a_description_states_no_figure() {
    for r in ResourceId::ALL {
        let d = content::resource_description(r);
        assert!(
            !d.chars().any(|c| c.is_ascii_digit()),
            "{r:?} states a figure: {d:?}"
        );
    }
}

/// The name stands above the description, and a flavour line can wear no coat,
/// so a description names no resource, its own included.
/// @spec SIM-ECO-062
#[test]
fn a_description_names_no_resource() {
    for r in ResourceId::ALL {
        let d = content::resource_description(r);
        for other in ResourceId::ALL {
            for word in spellings(&spaced(&format!("{other:?}"))) {
                assert!(
                    !names_whole(d, &word),
                    "{r:?}'s description names {other:?} as {word:?}: {d:?}"
                );
            }
        }
    }
}

/// A store is listed long before what makes or spends it is found, so a
/// description names nothing else from the content tables.
/// @spec SIM-ECO-062
#[test]
fn a_description_names_nothing_else_from_the_tables() {
    let mut taken: Vec<(String, String)> = Vec::new();
    for s in StructureId::ALL {
        taken.push((format!("{s:?}"), content::structure_def(s).name.to_string()));
    }
    for j in JobId::ALL {
        taken.push((format!("{j:?}"), content::job_def(j).name.to_string()));
    }
    for i in IdeaId::ALL {
        taken.push((format!("{i:?}"), content::idea_def(i).name.to_string()));
    }
    for c in CraftId::ALL {
        taken.push((format!("{c:?}"), content::craft_def(c).name.to_string()));
    }
    for r in RefinementId::ALL {
        taken.push((
            format!("{r:?}"),
            content::refinement_def(r).name.to_string(),
        ));
    }
    for r in RuleId::ALL {
        taken.push((format!("{r:?}"), content::rule_def(r).name.to_string()));
    }
    for r in RiteId::ALL {
        taken.push((format!("{r:?}"), content::rite_def(r).name.to_string()));
    }
    for a in ArchId::ALL {
        taken.push((format!("{a:?}"), content::arch_def(a).name.to_string()));
    }
    for c in ColonyId::ALL {
        taken.push((format!("{c:?}"), content::colony_def(c).name.to_string()));
    }
    for t in TraditionId::ALL {
        taken.push((format!("{t:?}"), spaced(&format!("{t:?}"))));
    }
    for a in AdventureId::ALL {
        taken.push((format!("{a:?}"), spaced(&format!("{a:?}"))));
    }
    for s in SiteId::ALL {
        taken.push((format!("{s:?}"), content::site_def(s).name.to_string()));
    }
    for l in DreamLoomId::ALL {
        taken.push((
            format!("{l:?}"),
            content::dream_loom_def(l).name.to_string(),
        ));
    }
    assert!(taken.len() > 200, "fixture: every table walked");
    for r in ResourceId::ALL {
        let d = content::resource_description(r);
        for (id, name) in &taken {
            assert!(
                !names_whole(d, name),
                "{r:?}'s description names {id} as {name:?}: {d:?}"
            );
        }
    }
}
