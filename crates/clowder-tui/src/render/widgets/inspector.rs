//! Truth, one keypress away — and none at all when the pane fits.
//!
//! The inspector is where whimsy pays for itself: the row shows a flavour name,
//! and this states the plain mechanical fact behind it. A player must never
//! lose a decision because a name was cute.
//! Layout is not decided here. TUI-NAV-047's dead-band and TUI-NAV-048's
//! three-column split are breakpoints, and live in `render::layout` beside the
//! render specs that state them a second time.
//! @spec TUI-NAV-017, TUI-NAV-018, TUI-NAV-029

use clowder_core::content;
use clowder_core::{ContentId, GameState, LockReason, ResourceId};
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::nav::rows::{rows, RowKey};
use crate::nav::{Section, ViewState};
use crate::render::fmt;

/// The section a frame draws when the view state has not chosen one yet —
/// matching `render::DEFAULT_SECTION`, since the inspector must describe the
/// same row the list would.
const DEFAULT_SECTION: Section = Section::Meadow;

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

/// A row's flavour name, in full — never shortened, unlike the list's column.
/// @spec TUI-NAV-018
fn full_name(state: &GameState, key: &RowKey) -> String {
    match key {
        RowKey::Content(id, _) => content_name(*id),
        RowKey::Resource(r) => humanize(&format!("{r:?}")),
        RowKey::Colony(c, _) => humanize(&format!("{c:?}")),
        RowKey::Site(s) => humanize(&format!("{s:?}")),
        RowKey::Adventure(Some(a)) => humanize(&format!("{a:?}")),
        RowKey::Adventure(None) => "No Adventure".to_string(),
        RowKey::Kitten(ix, _) => {
            let name = state.kittens.get(*ix).map(|k| k.name.as_str()).unwrap_or("");
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

fn resource_label(r: ResourceId) -> String {
    humanize(&format!("{r:?}"))
}

/// The plain mechanical statement behind a piece of content — what the row's
/// flavour name is borrowing against. Assembled from the def's own fields
/// rather than a template, so two items with different mechanics read
/// differently. @spec TUI-NAV-018
fn mechanics(id: ContentId) -> String {
    match id {
        ContentId::Structure(s) => {
            let def = content::structure_def(s);
            let mut parts: Vec<String> = Vec::new();
            if !def.productions.is_empty() {
                let items: Vec<String> = def
                    .productions
                    .iter()
                    .map(|(r, amt)| format!("{} {} per tick", fmt::exact(*amt), resource_label(*r)))
                    .collect();
                parts.push(format!("Produces {}.", items.join(", ")));
            }
            if def.housing > 0 {
                parts.push(format!("Houses {} kittens.", def.housing));
            }
            if !def.cap_bonuses.is_empty() {
                let items: Vec<String> = def
                    .cap_bonuses
                    .iter()
                    .map(|(r, amt)| format!("{} more {} capacity", fmt::exact(*amt), resource_label(*r)))
                    .collect();
                parts.push(format!("Raises storage: {}.", items.join(", ")));
            }
            if def.crowding_relief > 0.0 {
                parts.push("Eases crowding.".to_string());
            }
            if def.happiness_points != 0.0 {
                parts.push("Lifts the clowder's mood.".to_string());
            }
            if parts.is_empty() {
                parts.push("Sits in the meadow, quietly load-bearing.".to_string());
            }
            parts.join(" ")
        }
        ContentId::Job(j) => {
            let def = content::job_def(j);
            match def.yield_per_tick {
                Some((r, amt)) => format!(
                    "Assign a kitten to work here. Yields {} {} per tick per worker.",
                    fmt::exact(amt),
                    resource_label(r)
                ),
                None => "Assign a kitten to work here.".to_string(),
            }
        }
        ContentId::Idea(i) => {
            let def = content::idea_def(i);
            if def.blurb.is_empty() {
                "Unlocks new understanding.".to_string()
            } else {
                def.blurb.to_string()
            }
        }
        ContentId::Craft(c) => {
            let output = resource_label(content::craft_output(c));
            format!("Crafts {output} from raw materials.")
        }
        ContentId::Rite(_) => "A ritual observed with the clowder.".to_string(),
        ContentId::Arch(_) => "A decorative arch for the meadow.".to_string(),
        ContentId::Refinement(r) => {
            let def = content::refinement_def(r);
            if def.blurb.is_empty() {
                "Refines the clowder's craft.".to_string()
            } else {
                def.blurb.to_string()
            }
        }
        ContentId::Rule(r) => {
            let def = content::rule_def(r);
            if def.blurb.is_empty() {
                "A standing rule for the meadow.".to_string()
            } else {
                def.blurb.to_string()
            }
        }
        ContentId::Tradition(_) => "A tradition carried forward through Elder Wisdom.".to_string(),
        ContentId::DreamLoom(_) => "A dream woven for the clowder.".to_string(),
    }
}

/// How many the player owns of this content, when ownership is a count that
/// makes sense to show. @spec TUI-NAV-018
fn owned_count(state: &GameState, id: ContentId) -> Option<u32> {
    match id {
        ContentId::Structure(s) => Some(state.structures[s.ix()]),
        ContentId::Idea(i) => Some(state.ideas.contains(&i) as u32),
        ContentId::Rite(r) => Some(state.rite_counts[r.ix()]),
        ContentId::Arch(a) => Some(state.arches_bought.contains(&a) as u32),
        ContentId::Refinement(r) => Some(state.refinements.contains(&r) as u32),
        ContentId::Rule(r) => Some(state.rules.contains(&r) as u32),
        ContentId::Tradition(t) => Some(state.traditions.contains(&t) as u32),
        ContentId::DreamLoom(l) => Some(state.dream_looms.contains(&l) as u32),
        ContentId::Craft(_) | ContentId::Job(_) => None,
    }
}

/// The plain reason a lock reason is what it is — taken word-for-word from the
/// core's decision, never re-derived from stock. @spec TUI-NAV-018
fn lock_sentence(reason: &LockReason) -> String {
    match reason {
        LockReason::NotYetReached => "Not yet reached.".to_string(),
        LockReason::Prerequisite => "A prerequisite is not yet met.".to_string(),
        LockReason::AlreadyYours => "Already yours.".to_string(),
        LockReason::NoOneFree => "Nobody is free to take this on.".to_string(),
        LockReason::NoRoom => "There is no room for this.".to_string(),
        LockReason::ShortBy { missing } => {
            let items: Vec<String> = missing
                .iter()
                .map(|(r, amt)| format!("{} {}", fmt::exact(*amt), resource_label(*r)))
                .collect();
            format!("Short by {}.", items.join(", "))
        }
    }
}

/// The lines describing the focused item's cost, one per component: what it
/// needs and what the player has, in each's exact form, plus the
/// time-to-afford at the core's own net rate for that component.
/// @spec TUI-NAV-018
fn cost_lines(state: &GameState, id: ContentId) -> Vec<String> {
    let costs = state.cost_of_next(id, 1);
    costs
        .iter()
        .map(|(r, need)| {
            let have = state.stock(*r);
            let mut line = format!(
                "{}: need {}, have {}",
                resource_label(*r),
                fmt::exact(*need),
                fmt::exact(have)
            );
            if have < *need {
                let rate = state.net_rate(*r);
                if rate > 0.0 {
                    let seconds = (*need - have) / rate;
                    line.push_str(&format!(" — {} ticks to afford", fmt::exact(seconds)));
                } else {
                    line.push_str(" — not accruing");
                }
            }
            line
        })
        .collect()
}

/// The whole of the inspector's content for one row.
/// @spec TUI-NAV-018
fn draw_body(f: &mut Frame, area: Rect, state: &GameState, section: Section, key: &RowKey) {
    if rows(state, section).iter().all(|r| &r.key != key) {
        return;
    }

    let mut text = full_name(state, key);

    if let RowKey::Content(id, _) = key {
        text.push_str("\n\n");
        text.push_str(&mechanics(*id));

        if let Some(owned) = owned_count(state, *id) {
            text.push_str(&format!("\n\nOwned: {owned}."));
        }

        let costs = cost_lines(state, *id);
        if !costs.is_empty() {
            text.push_str("\n\nNext cost:\n");
            text.push_str(&costs.join("\n"));
        }

        if let Some(reason) = state.lock_reason(*id) {
            text.push_str("\n\n");
            text.push_str(&lock_sentence(&reason));
        }
    }

    let lines: Vec<Line<'static>> = text.lines().map(|l| Line::from(l.to_string())).collect();
    let para = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

/// Flavour name, plain description, exact next cost with have/need per
/// component, time-to-afford at the current net rate, owned count and effects,
/// and — for anything not actionable — the plain reason, taken from the core
/// rather than decided here. @spec TUI-NAV-018
pub fn draw(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let section = view.section.unwrap_or(DEFAULT_SECTION);
    let Some(key) = view.focus[section.index()].clone() else {
        return;
    };
    draw_body(f, area, state, section, &key);
}

/// Where the inspector stops being a swap over the list and becomes a pane of
/// its own. Mirrors `render::layout`'s own breakpoint exactly, because the
/// placement and the layout the frame draws are the same decision.
const PANE_BREAKPOINT: u16 = 100;

/// Whether the inspector shows as a persistent pane, swaps in over the list, or
/// is absent — a pure function of width and whether the player asked for it, so
/// it cannot disagree with the layout after a resize.
/// @spec TUI-NAV-017, TUI-NAV-029
pub fn placement(width: u16, wanted: bool) -> Placement {
    if width >= PANE_BREAKPOINT {
        Placement::Pane
    } else if wanted {
        Placement::Swapped
    } else {
        Placement::Absent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Pane,
    Swapped,
    Absent,
}
