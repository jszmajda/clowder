mod common;

use clowder_core::*;
use common::*;

// @spec SIM-CUR-001
#[test]
fn no_box_no_ideas() {
    let mut s = GameState::new(1);
    give(&mut s, ResourceId::Curiosity, 100.0); // hand-edited plenty changes nothing
    assert!(!s.idea_unlocked(IdeaId::PantryOrganization));
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
            .unwrap_err(),
        CommandError::Locked
    );
    grant(&mut s, StructureId::CardboardBox, 1);
    assert!(
        s.idea_unlocked(IdeaId::PantryOrganization),
        "the box opens the mind"
    );
}

// @spec SIM-CUR-002
#[test]
fn prerequisites_gate_the_ladder() {
    let mut s = GameState::new(2);
    grant(&mut s, StructureId::CardboardBox, 5);
    give(&mut s, ResourceId::Curiosity, 250.0);
    assert!(!s.idea_unlocked(IdeaId::GroomedToPurrfection));
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::GroomedToPurrfection))
            .unwrap_err(),
        CommandError::Locked
    );
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert!(s.idea_unlocked(IdeaId::GroomedToPurrfection));
    s.apply(Command::ResearchIdea(IdeaId::GroomedToPurrfection))
        .unwrap();
    assert!(s.ideas.contains(&IdeaId::GroomedToPurrfection));
}

// @spec SIM-CUR-003
#[test]
fn research_spends_in_full_once() {
    let mut s = GameState::new(3);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 20.0);
    let events = s
        .apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert!(events.contains(&Event::IdeaResearched {
        id: IdeaId::PantryOrganization
    }));
    assert!((s.stock(ResourceId::Curiosity) - 5.0).abs() < 1e-9);
    assert_eq!(
        s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
            .unwrap_err(),
        CommandError::AlreadyResearched
    );
    // Unlocked but unaffordable reads as InsufficientResources.
    let err = s
        .apply(Command::ResearchIdea(IdeaId::ScratchingArts))
        .unwrap_err();
    assert!(matches!(err, CommandError::InsufficientResources { .. }));
}

// @spec SIM-CUR-004
#[test]
fn ideas_are_forever() {
    let mut s = GameState::new(4);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 50.0);
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    run_days(&mut s, 400); // seasons turn, famines come and go
    assert!(s.ideas.contains(&IdeaId::PantryOrganization));
    // And they survive the save file.
    let json = save_string(&s, 1_000_000);
    let loaded = load_string(&json, 1_000_000).unwrap();
    assert!(loaded.state.ideas.contains(&IdeaId::PantryOrganization));
}

// @spec SIM-CUR-005
#[test]
fn the_idea_tree_matches_the_design_table() {
    // The founding nine keep their tabled costs and prerequisites; the rest
    // of the 57-idea banded tree is pinned by tests/economy_depth.rs.
    let costs: Vec<f64> = IdeaId::ALL[..9]
        .iter()
        .map(|i| content::idea_def(*i).cost)
        .collect();
    assert_eq!(
        costs,
        vec![15.0, 25.0, 40.0, 60.0, 80.0, 120.0, 150.0, 200.0, 250.0]
    );

    use IdeaId::*;
    let prereqs: Vec<&[IdeaId]> = IdeaId::ALL[..9]
        .iter()
        .map(|i| content::idea_def(*i).prereqs)
        .collect();
    let expected: Vec<&[IdeaId]> = vec![
        &[],
        &[],
        &[PantryOrganization],
        &[ScratchingArts],
        &[PantryOrganization],
        &[GroomedToPurrfection],
        &[TwigArchitecture],
        &[BiscuitMaking],
        &[SeasonalAwareness],
    ];
    assert_eq!(prereqs, expected);
    for i in IdeaId::ALL {
        assert!(!content::idea_def(i).name.is_empty());
        assert!(
            !content::idea_def(i).blurb.is_empty(),
            "whimsy with clarity: every idea explains itself"
        );
    }
}

// @spec SIM-CUR-006
#[test]
fn research_touches_only_curiosity_and_the_idea_set() {
    let mut s = GameState::new(5);
    grant(&mut s, StructureId::CardboardBox, 1);
    give(&mut s, ResourceId::Curiosity, 30.0);
    adopt(&mut s, 1);
    let before = s.clone();
    s.apply(Command::ResearchIdea(IdeaId::PantryOrganization))
        .unwrap();
    assert_eq!(s.ticks, before.ticks);
    assert_eq!(s.rng, before.rng);
    assert_eq!(s.structures, before.structures);
    assert_eq!(s.kittens, before.kittens);
    assert_eq!(
        s.stock(ResourceId::Catnip),
        before.stock(ResourceId::Catnip)
    );
    assert_eq!(s.stock(ResourceId::Twigs), before.stock(ResourceId::Twigs));
    assert!(
        (before.stock(ResourceId::Curiosity) - s.stock(ResourceId::Curiosity) - 15.0).abs() < 1e-9
    );
    assert_eq!(s.ideas.len(), before.ideas.len() + 1);
}

// ---------------------------------------------------------------------------
// Blurbs name resources by marks — SIM-CUR-013, SIM-CUR-014, SIM-CUR-015
// ---------------------------------------------------------------------------

use content::{blurb_parts, blurb_text, BlurbPart};

/// Every blurb in the content tables.
fn every_blurb() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    out.extend(IdeaId::ALL.iter().map(|i| content::idea_def(*i).blurb));
    out.extend(
        RefinementId::ALL
            .iter()
            .map(|r| content::refinement_def(*r).blurb),
    );
    out.extend(RuleId::ALL.iter().map(|r| content::rule_def(*r).blurb));
    out
}

/// An identifier spelled as words: a space before each capital after the
/// first.
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

/// A spelling with its last word made singular: a final "ies" read as "y",
/// otherwise a final "s" dropped.
fn singular(name: &str) -> String {
    if let Some(stem) = name.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = name.strip_suffix('s') {
        stem.to_string()
    } else {
        name.to_string()
    }
}

/// A name made plural: a final "y" read as "ies", otherwise an "s" added.
fn plural(name: &str) -> String {
    if let Some(stem) = name.strip_suffix('y') {
        format!("{stem}ies")
    } else {
        format!("{name}s")
    }
}

/// The spellings that mention a resource, lower-cased: its name as the stores
/// spell it, and that spelling with its last word made singular.
fn mention_forms(r: ResourceId) -> Vec<String> {
    let name = spaced(&format!("{r:?}"));
    vec![name.to_lowercase(), singular(&name).to_lowercase()]
}

fn resource_by_identifier(id: &str) -> Option<ResourceId> {
    ResourceId::ALL.into_iter().find(|r| format!("{r:?}") == id)
}

/// The names of the content items whose names may hold a resource's word
/// without mentioning it — every kind but resources and crafts — each as the
/// tables spell it, and made plural.
fn other_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    names.extend(
        StructureId::ALL
            .iter()
            .map(|s| content::structure_def(*s).name.to_string()),
    );
    names.extend(
        JobId::ALL
            .iter()
            .map(|j| content::job_def(*j).name.to_string()),
    );
    names.extend(
        IdeaId::ALL
            .iter()
            .map(|i| content::idea_def(*i).name.to_string()),
    );
    names.extend(
        RefinementId::ALL
            .iter()
            .map(|r| content::refinement_def(*r).name.to_string()),
    );
    names.extend(
        RuleId::ALL
            .iter()
            .map(|r| content::rule_def(*r).name.to_string()),
    );
    names.extend(
        RiteId::ALL
            .iter()
            .map(|r| content::rite_def(*r).name.to_string()),
    );
    names.extend(
        ArchId::ALL
            .iter()
            .map(|a| content::arch_def(*a).name.to_string()),
    );
    names.extend(TraditionId::ALL.iter().map(|t| spaced(&format!("{t:?}"))));
    names.extend(
        DreamLoomId::ALL
            .iter()
            .map(|l| content::dream_loom_def(*l).name.to_string()),
    );
    names.extend(
        ColonyId::ALL
            .iter()
            .map(|c| content::colony_def(*c).name.to_string()),
    );
    names.extend(
        SiteId::ALL
            .iter()
            .map(|s| content::site_def(*s).name.to_string()),
    );
    names.extend(AdventureId::ALL.iter().map(|a| spaced(&format!("{a:?}"))));
    let plurals: Vec<String> = names.iter().map(|n| plural(n)).collect();
    names.extend(plurals);
    names
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Where `needle` (lower-case) stands as whole words in `hay`, ignoring case:
/// the char index of each occurrence.
fn whole_word_hits(hay: &[char], needle: &str) -> Vec<usize> {
    let needle: Vec<char> = needle.chars().collect();
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    for start in 0..=hay.len() - needle.len() {
        let end = start + needle.len();
        let same = hay[start..end]
            .iter()
            .zip(&needle)
            .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()));
        let left = start == 0 || !is_word_char(hay[start - 1]);
        let right = end == hay.len() || !is_word_char(hay[end]);
        if same && left && right {
            out.push(start);
        }
    }
    out
}

/// Whether the char at `at` begins a sentence: the blurb's start, or directly
/// after a full stop, question mark, or exclamation mark and a space.
fn starts_sentence(text: &[char], at: usize) -> bool {
    at == 0 || (at >= 2 && text[at - 1] == ' ' && matches!(text[at - 2], '.' | '?' | '!'))
}

/// Every mention of a resource in a blurb is marked, save one inside another
/// content item's name or a capitalised name standing within a sentence.
// @spec SIM-CUR-013
#[test]
fn every_resource_a_blurb_mentions_is_marked() {
    let others = other_names();
    let mut marked = 0;
    for blurb in every_blurb() {
        // The blurb's words, and which of them its marks cover.
        let mut text: Vec<char> = Vec::new();
        let mut covered: Vec<bool> = Vec::new();
        for part in blurb_parts(blurb) {
            let (words, is_mark) = match part {
                BlurbPart::Text(t) => (t, false),
                BlurbPart::Resource(_, t) => {
                    marked += 1;
                    (t, true)
                }
            };
            for c in words.chars() {
                text.push(c);
                covered.push(is_mark);
            }
        }
        // The spans the tables' own names cover, in their own capitalisation.
        let mut in_name = vec![false; text.len()];
        for name in &others {
            let n: Vec<char> = name.chars().collect();
            if n.is_empty() || n.len() > text.len() {
                continue;
            }
            for start in 0..=text.len() - n.len() {
                let end = start + n.len();
                let left = start == 0 || !is_word_char(text[start - 1]);
                let right = end == text.len() || !is_word_char(text[end]);
                if text[start..end] == n[..] && left && right {
                    in_name[start..end].iter_mut().for_each(|b| *b = true);
                }
            }
        }
        for r in ResourceId::ALL {
            for form in mention_forms(r) {
                for at in whole_word_hits(&text, &form) {
                    let end = at + form.chars().count();
                    if covered[at..end].iter().all(|c| *c) {
                        continue;
                    }
                    let within_name = in_name[at..end].iter().all(|c| *c);
                    let proper = text[at].is_uppercase() && !starts_sentence(&text, at);
                    let shown: String = text.iter().collect();
                    assert!(
                        within_name || proper,
                        "{blurb:?} mentions {r:?} unmarked at {at}: {shown:?}"
                    );
                }
            }
        }
    }
    assert!(
        marked > 20,
        "fixture: the blurbs mark many resources ({marked})"
    );
}

/// A blurb's reading, consecutive plain runs joined, so a test pins what the
/// reading says rather than where the parser happens to split it.
fn reading(blurb: &str) -> Vec<(Option<ResourceId>, String)> {
    let mut out: Vec<(Option<ResourceId>, String)> = Vec::new();
    for part in blurb_parts(blurb) {
        match part {
            BlurbPart::Text(t) => match out.last_mut() {
                Some((None, run)) => run.push_str(t),
                _ => out.push((None, t.to_string())),
            },
            BlurbPart::Resource(r, t) => out.push((Some(r), t.to_string())),
        }
    }
    out.retain(|(r, t)| r.is_some() || !t.is_empty());
    out
}

// @spec SIM-CUR-014
#[test]
fn a_blurb_reads_as_text_and_resource_references() {
    let plain = |t: &str| (None, t.to_string());
    let res = |r: ResourceId, t: &str| (Some(r), t.to_string());

    assert_eq!(
        reading("Stack the {catnip|Catnip}, gently."),
        vec![
            plain("Stack the "),
            res(ResourceId::Catnip, "catnip"),
            plain(", gently.")
        ]
    );
    assert_eq!(
        blurb_text("Stack the {catnip|Catnip}, gently."),
        "Stack the catnip, gently."
    );

    assert_eq!(
        reading("{Twigs} cap +100."),
        vec![res(ResourceId::Twigs, "Twigs"), plain(" cap +100.")]
    );
    assert_eq!(
        reading("A {jar of purrs|JarsOfPurrs} each."),
        vec![
            plain("A "),
            res(ResourceId::JarsOfPurrs, "jar of purrs"),
            plain(" each.")
        ]
    );

    // The identifier is matched exactly; a mark naming no resource reads as
    // its text, the braces and the identifier gone.
    assert_eq!(
        reading("Some {fish|Fish} swim."),
        vec![plain("Some fish swim.")]
    );
    assert_eq!(
        reading("Some {nip|catnip} here."),
        vec![plain("Some nip here.")]
    );
    assert_eq!(blurb_text("Some {fish|Fish} swim."), "Some fish swim.");

    // A brace that opens or closes no mark reads as itself, and later marks
    // still read.
    assert_eq!(reading("open { brace"), vec![plain("open { brace")]);
    assert_eq!(
        reading("stray } then {Dew}"),
        vec![plain("stray } then "), res(ResourceId::Dew, "Dew")]
    );
    assert_eq!(blurb_text("open { brace"), "open { brace");
    assert_eq!(reading("No marks at all."), vec![plain("No marks at all.")]);
}

// @spec SIM-CUR-015
#[test]
fn every_mark_in_the_tables_names_a_resource_its_words_spell() {
    for blurb in every_blurb() {
        let mut rest = blurb;
        while let Some(open) = rest.find(['{', '}']) {
            assert!(
                rest[open..].starts_with('{'),
                "{blurb:?}: a '}}' closes no mark"
            );
            let after = &rest[open + 1..];
            let close = after
                .find('}')
                .unwrap_or_else(|| panic!("{blurb:?}: a '{{' is never closed"));
            let inside = &after[..close];
            assert!(
                !inside.contains('{'),
                "{blurb:?}: a mark opens inside a mark"
            );
            let (words, id) = inside.split_once('|').unwrap_or((inside, inside));
            let r = resource_by_identifier(id)
                .unwrap_or_else(|| panic!("{blurb:?}: {{{inside}}} names no resource"));
            assert!(
                mention_forms(r).contains(&words.to_lowercase()),
                "{blurb:?}: {{{inside}}} names {r:?}, which {words:?} does not spell"
            );
            rest = &after[close + 1..];
        }
    }
}
