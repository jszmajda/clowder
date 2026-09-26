//! Truth, one keypress away — and none at all when the pane fits.
//!
//! The inspector is where whimsy pays for itself: the row shows a flavour name,
//! and this states the plain mechanical fact behind it. A player must never
//! lose a decision because a name was cute. Every fact stated here is a core
//! query or a content definition's own field, never a rule worked out in the
//! shell: no total for copies owned, no yield after mood, no coverage inferred
//! from stock, no wait where the core reports no reason to wait.
//! Layout is not decided here. TUI-NAV-047's dead-band and TUI-NAV-048's
//! three-column split are breakpoints, and live in `render::layout` beside the
//! render specs that state them a second time.
//! @spec TUI-NAV-017, TUI-NAV-018, TUI-NAV-029, TUI-NAV-078, TUI-NAV-079,
//!       TUI-NAV-080

use clowder_core::content::{self, TICKS_PER_SECOND};
use clowder_core::{ContentId, GameState, LockReason, ResourceId};
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::nav::rows::{rows, Row, RowKey, RowKind};
use crate::nav::{CountPrompt, Section, ViewState};
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
        RowKey::Resource(r) => resource_label(*r),
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

/// A value per tick, stated per game second (TICKS_PER_SECOND ticks per game
/// second), in the inspector's exact form.
fn per_second(per_tick: f64) -> String {
    fmt::exact(per_tick * TICKS_PER_SECOND as f64)
}

/// A ratio, stated as the percentage it adds.
fn percent(ratio: f64) -> String {
    fmt::exact(ratio * 100.0)
}

// ---------------------------------------------------------------------------
// The plain mechanical statement — TUI-NAV-018
// ---------------------------------------------------------------------------

/// A structure's statement: every field its definition carries apart from its
/// name, its price and that price's escalation, its reveal threshold, and its
/// gate, wherever the field departs from its baseline. Per-tick fields are
/// stated per game second; ratios as the percentage they add.
fn structure_statement(s: clowder_core::StructureId) -> String {
    let d = content::structure_def(s);
    let mut parts: Vec<String> = Vec::new();

    for (r, amt) in d.productions {
        let mut clause =
            format!("Produces {} {} per second", per_second(*amt), resource_label(*r));
        let mut qualifiers: Vec<&str> = Vec::new();
        if d.seasonal {
            qualifiers.push("seasonal");
        }
        if d.surplus_scaled {
            qualifiers.push("scaled by the energy surplus");
        }
        if d.outdoor {
            qualifiers.push("routed through the outdoor channel");
        }
        if !qualifiers.is_empty() {
            clause.push_str(&format!(", {}", qualifiers.join(", ")));
        }
        clause.push('.');
        parts.push(clause);
    }
    if d.productions.is_empty() && (d.seasonal || d.surplus_scaled || d.outdoor) {
        let mut qualifiers: Vec<&str> = Vec::new();
        if d.seasonal {
            qualifiers.push("seasonal");
        }
        if d.surplus_scaled {
            qualifiers.push("scaled by the energy surplus");
        }
        if d.outdoor {
            qualifiers.push("routed through the outdoor channel");
        }
        parts.push(format!("Is {}.", qualifiers.join(", ")));
    }

    if d.housing != 0 {
        parts.push(format!("Houses {} kittens.", d.housing));
    }
    for (r, amt) in d.cap_bonuses {
        parts.push(format!("Adds {} storage for {}.", fmt::exact(*amt), resource_label(*r)));
    }
    if d.happiness_points != 0.0 {
        parts.push(format!("Lifts mood by {} points.", fmt::exact(d.happiness_points)));
    }
    if d.crowding_relief != 0.0 {
        parts.push(format!("Eases crowding by {}%.", percent(d.crowding_relief)));
    }
    for (r, amt) in d.resource_ratios {
        parts.push(format!("Raises production of {} by {}%.", resource_label(*r), percent(*amt)));
    }
    if d.craft_ratio != 0.0 {
        parts.push(format!("Raises craft output by {}%.", percent(d.craft_ratio)));
    }
    if d.luxury_demand != 0.0 {
        parts.push(format!("Shifts luxury demand by {}%.", percent(d.luxury_demand)));
    }
    if d.jamboree_ratio != 0.0 {
        parts.push(format!("Raises jamboree mood by {}%.", percent(d.jamboree_ratio)));
    }
    if d.jamboree_arrival != 0.0 {
        parts.push(format!("Raises jamboree arrivals by {}%.", percent(d.jamboree_arrival)));
    }
    if d.global_ratio != 0.0 {
        parts.push(format!("Raises the global ratio by {}%.", percent(d.global_ratio)));
    }
    if let Some(conv) = &d.converter {
        for (r, amt) in conv.inputs {
            parts.push(format!("Converts {} {} per second.", per_second(*amt), resource_label(*r)));
        }
        for (r, amt) in conv.outputs {
            parts.push(format!("Yields {} {} per second.", per_second(*amt), resource_label(*r)));
        }
    }
    if d.mess != 0.0 {
        parts.push(format!("Makes {} mess per second.", per_second(d.mess)));
    }
    if d.energy_prod != 0.0 {
        parts.push(format!("Makes {} energy.", fmt::exact(d.energy_prod)));
    }
    if d.energy_cons != 0.0 {
        parts.push(format!("Uses {} energy.", fmt::exact(d.energy_cons)));
    }
    if d.outdoor_ratio != 0.0 {
        parts.push(format!("Raises outdoor production by {}%.", percent(d.outdoor_ratio)));
    }
    if d.long_sun_cap != 0.0 {
        parts.push(format!("Extends the long sun by {}.", fmt::exact(d.long_sun_cap)));
    }

    parts.join(" ")
}

/// A job's statement: the yield per worker per game second, in the resource
/// its definition names. A job with no yield (the Tinkerer) has none.
fn job_statement(j: clowder_core::JobId) -> String {
    match content::job_def(j).yield_per_tick {
        Some((r, per_tick)) => {
            format!("Yields {} {} per worker per second.", per_second(per_tick), resource_label(r))
        }
        None => String::new(),
    }
}

/// A craft's statement: the resource its recipe makes, and the tinkerer work
/// tier the recipe meets. Shared by the make row and the tinkerer count row —
/// both describe the same recipe.
fn craft_statement(c: clowder_core::CraftId) -> String {
    let def = content::craft_def(c);
    let output = resource_label(content::craft_output(c));
    if def.tier != 0 {
        format!("Makes {output}, at tinkerer tier {}.", def.tier)
    } else {
        format!("Makes {output}.")
    }
}

/// The plain mechanical statement behind a piece of content — what the row's
/// flavour name is borrowing against. For a kind whose definition carries a
/// blurb, the blurb *is* the statement and its effect fields are not restated.
/// A rite, an arch, a tradition, and a dream loom carry nothing past name,
/// price, and gate, so they have none. @spec TUI-NAV-018
fn statement(id: ContentId) -> String {
    match id {
        ContentId::Structure(s) => structure_statement(s),
        ContentId::Job(j) => job_statement(j),
        ContentId::Craft(c) => craft_statement(c),
        ContentId::Idea(i) => content::idea_def(i).blurb.to_string(),
        ContentId::Refinement(r) => content::refinement_def(r).blurb.to_string(),
        ContentId::Rule(r) => content::rule_def(r).blurb.to_string(),
        ContentId::Rite(_) | ContentId::Arch(_) | ContentId::Tradition(_) | ContentId::DreamLoom(_) => {
            String::new()
        }
    }
}

// ---------------------------------------------------------------------------
// The lock reason, in plain words — TUI-NAV-079
// ---------------------------------------------------------------------------

/// The core's lock reason for an item, stated in plain words. Each of the
/// core's six reasons has its own wording, so two reasons never read alike; a
/// shortfall naming no component (a price outside the resource vocabulary)
/// says only that the price is unmet, and names no amount.
/// @spec TUI-NAV-079
pub fn lock_reason_text(reason: &LockReason) -> String {
    match reason {
        LockReason::NotYetReached => "Not reached yet.".to_string(),
        LockReason::Prerequisite => "A prerequisite is still unmet.".to_string(),
        LockReason::AlreadyYours => "Already yours.".to_string(),
        LockReason::NoOneFree => "Nobody in the meadow is free to take this on.".to_string(),
        LockReason::NoRoom => "There is no room for this yet.".to_string(),
        LockReason::ShortBy { missing } => {
            if missing.is_empty() {
                "The price is not yet met.".to_string()
            } else {
                let items: Vec<String> = missing
                    .iter()
                    .map(|(r, amt)| format!("{} {}", fmt::exact(*amt), resource_label(*r)))
                    .collect();
                format!("Short by {}.", items.join(", "))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The next cost and its wait — TUI-NAV-078
// ---------------------------------------------------------------------------

/// A stretch of real time, in a plain unit — never rounded to sooner than the
/// rate gives.
fn wait_text(seconds: f64) -> String {
    const UNITS: [(f64, &str); 4] =
        [(86_400.0, "days"), (3_600.0, "hours"), (60.0, "minutes"), (1.0, "seconds")];
    for (size, name) in UNITS {
        if seconds >= size {
            let n = (seconds / size).ceil();
            return format!("{} {name}", fmt::exact(n));
        }
    }
    format!("{} seconds", fmt::exact(seconds.max(0.0).ceil().max(1.0)))
}

/// One component of a next cost: what it needs and what the player has, in
/// each's exact form, marked only as the core's lock reason reports it — short
/// by the amount a shortfall names, covered where a shortfall leaves it out or
/// the core reports no reason, unmarked where another reason holds the item
/// back. @spec TUI-NAV-078
fn component_line(state: &GameState, r: ResourceId, need: f64, reason: &Option<LockReason>) -> String {
    let have = state.stock(r);
    let mut line =
        format!("{}: need {}, have {}", resource_label(r), fmt::exact(need), fmt::exact(have));

    let short_amount = match reason {
        Some(LockReason::ShortBy { missing }) => {
            missing.iter().find(|(mr, _)| *mr == r).map(|(_, amt)| *amt)
        }
        _ => None,
    };
    let other_reason = matches!(reason, Some(x) if !matches!(x, LockReason::ShortBy { .. }));

    if other_reason {
        // Unmarked: another reason holds the item back, so stock marks
        // nothing.
    } else if let Some(amount) = short_amount {
        line.push_str(&format!(", short by {}", fmt::exact(amount)));
        let cap = state.cap(r);
        if need > cap {
            line.push_str(" — the stores cannot hold that much yet");
        } else {
            let rate = state.net_rate(r);
            if rate <= 0.0 {
                line.push_str(" — the stock is not growing");
            } else {
                let seconds = amount / (rate * state.speed);
                line.push_str(&format!(" — wait {}", wait_text(seconds)));
            }
        }
    } else {
        // No shortfall names this component: covered, whether the core
        // reports no reason at all or a shortfall that leaves it out.
        line.push_str(", covered");
    }
    line
}

/// The next cost lines for a content item that takes one: the core's price for
/// one more, per component with need and have, marked by the core's lock
/// reason. @spec TUI-NAV-078
fn cost_lines(state: &GameState, id: ContentId, reason: &Option<LockReason>) -> Vec<String> {
    let cost = state.cost_of_next(id, 1);
    cost.iter().map(|(r, need)| component_line(state, *r, *need, reason)).collect()
}

/// The next cost for a Pounce or Trade row: the one-unit price the core
/// reports, per component. The core reports no lock reason for these rows, so
/// need and have stand unmarked and no wait is stated beside them.
/// @spec TUI-NAV-073
fn unit_cost_lines(state: &GameState, unit: &[(ResourceId, f64)]) -> Vec<String> {
    unit.iter()
        .map(|(r, need)| {
            format!(
                "{}: need {}, have {}",
                resource_label(*r),
                fmt::exact(*need),
                fmt::exact(state.stock(*r))
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Assembling a row's lines
// ---------------------------------------------------------------------------

/// The lines the inspector states about a piece of content, beyond its name.
/// While a count prompt is held to this row, its count and preview
/// (TUI-NAV-067) take the place of the next cost alone — the same
/// substitution the list makes in its own cost slot, leaving the lock reason
/// stated beside it exactly where the core reports one, prompt or none.
/// @spec TUI-NAV-018, TUI-NAV-078, TUI-NAV-079, TUI-NAV-035
fn content_lines(state: &GameState, row: &Row, id: ContentId, tag: &str, prompt: Option<&CountPrompt>) -> Vec<String> {
    let mut out = Vec::new();

    let stmt = statement(id);
    if !stmt.trim().is_empty() {
        out.push(stmt);
    }

    match id {
        ContentId::Structure(s) => out.push(format!("Owned: {}.", state.structures[s.ix()])),
        ContentId::Rite(r) => out.push(format!("Held: {}.", state.rite_counts[r.ix()])),
        _ => {}
    }

    if let ContentId::Job(j) = id {
        out.push(format!("Workers in the meadow: {}.", state.effective_workers(j)));
    }

    if tag == "tinker" {
        if let ContentId::Craft(c) = id {
            out.push(format!("Tinkerers in the meadow: {}.", bound_tinkerers(state, c)));
        }
        if let Some(p) = prompt {
            out.push(crate::render::count_prompt_preview(state, row, &p.digits));
        }
        // A tinkerer count row binds rather than makes: no next cost, no
        // reason — the core reports neither for it.
        return out;
    }

    let reason = state.lock_reason(id);

    // TUI-NAV-035: the prompt's count and preview substitute for the next
    // cost alone, exactly where the core reports one to substitute — the
    // lock reason beside it (TUI-NAV-079) is read whether or not a prompt is
    // held to this row.
    if let Some(p) = prompt {
        out.push(crate::render::count_prompt_preview(state, row, &p.digits));
    } else if reason != Some(LockReason::AlreadyYours) {
        out.extend(cost_lines(state, id, &reason));
    }
    if let Some(r) = &reason {
        out.push(lock_reason_text(r));
    }

    out
}

/// The tinkerers in the meadow bound to a recipe — a kitten away from the
/// meadow keeps its binding but is not counted while away.
fn bound_tinkerers(state: &GameState, c: clowder_core::CraftId) -> usize {
    state
        .kittens
        .iter()
        .filter(|k| {
            k.tinkering == Some(c) && k.location == clowder_core::Location::Home
        })
        .count()
}

/// A stores row: the resource's exact stock, the cap the core reports or that
/// it has none, and the core's net rate for it per game second.
/// @spec TUI-NAV-080
fn store_lines(state: &GameState, r: ResourceId) -> Vec<String> {
    let cap = state.cap(r);
    let cap_text = if cap.is_finite() { fmt::exact(cap) } else { "none".to_string() };
    vec![format!(
        "Stock: {}. Cap: {cap_text}. Net rate: {} per game second.",
        fmt::exact(state.stock(r)),
        fmt::exact(state.net_rate(r))
    )]
}

/// The whole of the inspector's content for one row. While an open count
/// prompt is held to this row, its count and preview (TUI-NAV-067) are shown
/// alongside — the same row the prompt was opened on, by identity, so a
/// cursor that has since moved off it never previews on the wrong row.
/// @spec TUI-NAV-018, TUI-NAV-077, TUI-NAV-035
fn draw_body(
    f: &mut Frame,
    area: Rect,
    state: &GameState,
    section: Section,
    key: &RowKey,
    armed: Option<&crate::nav::ArmedRow>,
    prompt: Option<&CountPrompt>,
) {
    let Some(row) = rows(state, section).into_iter().find(|r| &r.key == key) else {
        return;
    };
    let on_this_row = prompt.filter(|p| p.row.as_ref() == Some(key));

    let mut lines: Vec<String> = vec![full_name(state, key)];

    // The swapped-in inspector carries the same consequence summary the list
    // would show beneath the armed row. @spec TUI-NAV-077
    if let Some(armed) = armed.filter(|a| a.key == *key && crate::render::armed_is_live(a)) {
        lines.push(String::new());
        lines.extend(crate::render::consequence_summary(state, armed));
    }

    match key {
        RowKey::Content(id, tag) => {
            lines.push(String::new());
            lines.extend(content_lines(state, &row, *id, tag, on_this_row));
        }
        RowKey::Resource(r) => {
            lines.push(String::new());
            lines.extend(store_lines(state, *r));
        }
        _ => {
            if let Some(p) = on_this_row {
                lines.push(String::new());
                lines.push(crate::render::count_prompt_preview(state, &row, &p.digits));
            } else if let RowKind::Act { command, .. } = &row.kind {
                if let Some(unit) = state.unit_price(command) {
                    lines.push(String::new());
                    lines.extend(unit_cost_lines(state, &unit));
                }
            }
        }
    }

    let text = lines.join("\n");
    let out: Vec<Line<'static>> = text.lines().map(|l| Line::from(l.to_string())).collect();
    let para = Paragraph::new(out).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

/// Flavour name, plain mechanical statement, exact next cost with have/need
/// per component and its wait, owned count, and — for anything not
/// actionable — the plain reason, taken from the core rather than decided
/// here. On a Pounce or Trade row (not content, but still priced), the next
/// cost is the core's one-unit price instead. On a stores row, the exact
/// stock, cap, and net rate.
/// @spec TUI-NAV-018, TUI-NAV-073, TUI-NAV-078, TUI-NAV-079, TUI-NAV-080
pub fn draw(f: &mut Frame, area: Rect, state: &GameState, view: &ViewState) {
    let section = view.section.unwrap_or(DEFAULT_SECTION);
    let all = rows(state, section);

    // The row the cursor is on is the row the list highlights, re-derived
    // every frame: when the remembered key has left the list, the list falls
    // back to its first row, and the inspector must follow it there rather
    // than describe a row that no longer exists. @spec TUI-NAV-018,
    // TUI-NAV-032
    let cursor = view.focus[section.index()]
        .as_ref()
        .and_then(|k| all.iter().position(|r| &r.key == k))
        .unwrap_or(0);
    let Some(row) = all.get(cursor) else {
        return;
    };
    let key = row.key.clone();
    draw_body(f, area, state, section, &key, view.armed.as_ref(), view.prompt.as_ref());
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
