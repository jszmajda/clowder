//! The find-anything palette's result list: a pure function of state and the
//! typed query, re-derived at every look so the shell, the drawing, and the
//! reducer's jump can never disagree about what is on offer.
//!
//! The universe is exactly what `rows` already lists in every revealed
//! section — nothing here restates a content table, and nothing here can find
//! what the section lists could not. @spec TUI-NAV-063

use clowder_core::{AdventureId, ColonyId, ContentId, GameState, ResourceId, SiteId};

use super::rows::{revealed_sections, rows, RowKey};
use super::section::Section;

/// One thing the palette can jump to: the section it lives in, its first
/// row's key, and the name its rows display. @spec TUI-NAV-063
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteResult {
    pub section: Section,
    pub key: RowKey,
    pub name: String,
}

/// What a row is *about* — several rows can belong to one item (a craft's
/// make and tinkerer rows, a kitten's two rows, a colony's two rows), and
/// such an item is one result, not several. The tag a `RowKey` carries to
/// tell its own rows apart is exactly what this identity drops.
/// @spec TUI-NAV-063
#[derive(PartialEq)]
enum Item {
    Content(ContentId),
    Resource(ResourceId),
    Colony(ColonyId),
    Site(SiteId),
    Adventure(Option<AdventureId>),
    Kitten(usize),
    Fixed(&'static str),
}

fn item_of(key: &RowKey) -> Item {
    match key {
        RowKey::Content(id, _) => Item::Content(*id),
        RowKey::Resource(r) => Item::Resource(*r),
        RowKey::Colony(c, _) => Item::Colony(*c),
        RowKey::Site(s) => Item::Site(*s),
        RowKey::Adventure(a) => Item::Adventure(*a),
        RowKey::Kitten(ix, _) => Item::Kitten(*ix),
        RowKey::Fixed(tag) => Item::Fixed(tag),
    }
}

/// The palette's full, unfiltered universe: one result per item with a
/// selectable row in a revealed section, in tab-bar then list order — never a
/// group header, and never an item whose only rows are ungated or Hidden,
/// since those never reach `rows` at all. @spec TUI-NAV-063
fn universe(state: &GameState) -> Vec<PaletteResult> {
    let mut out = Vec::new();
    for section in revealed_sections(state) {
        let mut seen: Vec<Item> = Vec::new();
        for row in rows(state, section) {
            if !row.selectable() {
                continue;
            }
            let item = item_of(&row.key);
            if seen.contains(&item) {
                continue;
            }
            seen.push(item);
            out.push(PaletteResult {
                section,
                key: row.key.clone(),
                name: name_for(state, &row.key),
            });
        }
    }
    out
}

/// The results for `query`: every result when the query is empty or nothing
/// but spaces (no query aims at nothing), else the matches ranked by
/// closeness — equal, then prefix, then elsewhere — ties kept in tab-bar then
/// list order, case ignored throughout and spaces kept as typed.
/// @spec TUI-NAV-062, TUI-NAV-038
pub fn results(state: &GameState, query: &str) -> Vec<PaletteResult> {
    let all = universe(state);
    if query.chars().all(|c| c == ' ') {
        return all;
    }

    let q = query.to_lowercase();
    let mut hits: Vec<(u8, usize)> = all
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let name = r.name.to_lowercase();
            if name == q {
                Some((0, i))
            } else if name.starts_with(&q) {
                Some((1, i))
            } else if name.contains(&q) {
                Some((2, i))
            } else {
                None
            }
        })
        .collect();
    hits.sort();
    hits.into_iter().map(|(_, i)| all[i].clone()).collect()
}

/// A row's stable display name — the content name where the row is content,
/// or a readable label invented from the row's own fixed key otherwise.
/// Mirrors `render::row_label`: two consumers, both deliberately reading off
/// the row's own key rather than a shared cache, so neither can drift behind
/// a rename of the other.
fn name_for(state: &GameState, key: &RowKey) -> String {
    match key {
        RowKey::Content(id, _) => content_name(*id),
        RowKey::Resource(r) => humanize(&format!("{r:?}")),
        RowKey::Colony(c, _) => humanize(&format!("{c:?}")),
        RowKey::Site(s) => humanize(&format!("{s:?}")),
        RowKey::Adventure(Some(a)) => humanize(&format!("{a:?}")),
        RowKey::Adventure(None) => "No Adventure".to_string(),
        RowKey::Kitten(ix, _) => {
            let name = state
                .kittens
                .get(*ix)
                .map(|k| k.name.as_str())
                .unwrap_or("");
            if name.is_empty() {
                format!("Kitten {}", ix + 1)
            } else {
                name.to_string()
            }
        }
        RowKey::Fixed(tag) => humanize(tag),
    }
}

fn content_name(id: ContentId) -> String {
    use clowder_core::content;
    match id {
        ContentId::Structure(s) => content::structure_def(s).name.to_string(),
        ContentId::Idea(i) => content::idea_def(i).name.to_string(),
        ContentId::Craft(c) => content::craft_def(c).name.to_string(),
        ContentId::Job(j) => content::job_def(j).name.to_string(),
        ContentId::Rite(r) => content::rite_def(r).name.to_string(),
        ContentId::Arch(a) => content::arch_def(a).name.to_string(),
        ContentId::Refinement(r) => content::refinement_def(r).name.to_string(),
        ContentId::Rule(r) => content::rule_def(r).name.to_string(),
        ContentId::Tradition(t) => humanize(&format!("{t:?}")),
        ContentId::DreamLoom(l) => content::dream_loom_def(l).name.to_string(),
    }
}

/// Turn an identifier-shaped tag into a spaced label.
fn humanize(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len() + 4);
    for (i, c) in tag.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}
