//! Truth, one keypress away — and none at all when the pane fits.
//!
//! The inspector is where whimsy pays for itself: the row shows a flavour name,
//! and this states the plain mechanical fact behind it. A player must never
//! lose a decision because a name was cute. Every fact stated here is a core
//! query or a content definition's own field, never a rule worked out in the
//! shell: no total for copies owned, no yield after mood, no coverage inferred
//! from stock, no wait where the core reports no reason to wait, no flow
//! credited to a source by the shell's own reckoning.
//! Layout is not decided here. TUI-NAV-047's dead-band and TUI-NAV-048's
//! three-column split are breakpoints, and live in `render::layout` beside the
//! render specs that state them a second time.
//! @spec TUI-NAV-017, TUI-NAV-018, TUI-NAV-029, TUI-NAV-078, TUI-NAV-079,
//!       TUI-NAV-080, TUI-NAV-108, TUI-NAV-135

use clowder_core::content::{self, TICKS_PER_SECOND};
use clowder_core::{
    Arrival, Command, ContentId, Event, FlowSource, GameState, JournalItem, LockReason, ResourceId,
};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::nav::rows::{rows, Row, RowKey, RowKind};
use crate::nav::{CountPrompt, Section, ViewState};
use crate::render::{fmt, line_from_pieces, pieces_text, Piece};
use crate::theme::Role;

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

/// A line of the inspector: its pieces, each a run of plain text or a
/// resource's name wearing its coat.
type Pieces = Vec<Piece>;

fn plain(text: String) -> Pieces {
    vec![Piece::plain(text)]
}

/// The description's first line — the row's name in full, a stores row's
/// name in its coat. @spec TUI-NAV-018, TUI-THEME-019
fn name_line(state: &GameState, key: &RowKey) -> Pieces {
    let name = full_name(state, key);
    match key {
        RowKey::Resource(r) => {
            let chars = name.chars().count();
            vec![Piece::name(*r, name, chars)]
        }
        _ => plain(name),
    }
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
        RowKey::Bench => crate::nav::rows::BENCH_NAME.to_string(),
        RowKey::Energy => crate::nav::rows::ENERGY_NAME.to_string(),
        RowKey::Mess => crate::nav::rows::MESS_NAME.to_string(),
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

/// A resource named in a sentence of the description: its name as the stores
/// spell it, in its coat (TUI-THEME-022).
fn resource(r: ResourceId) -> Piece {
    let name = resource_label(r);
    let chars = name.chars().count();
    Piece::name(r, name, chars)
}

/// Items joined as a sentence joins them: two by ` and `, three or more by
/// `, ` with `, and ` before the last.
fn join_as_sentence(items: Vec<Pieces>) -> Pieces {
    let n = items.len();
    let mut out: Pieces = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push(Piece::plain(match (n, i == n - 1) {
                (2, _) => " and ",
                (_, true) => ", and ",
                _ => ", ",
            }));
        }
        out.extend(item);
    }
    out
}

/// What a colony has shown the clowder, as the core's offers query reports
/// it: nothing before a caravan has reached it; then its learned goods, the
/// rest named by a colony warm enough to tell of them — those it keeps for
/// closer friends marked — or counted by one that is not.
/// @spec TUI-NAV-162
fn offer_lines(state: &GameState, c: clowder_core::ColonyId) -> Vec<Pieces> {
    let offers = state.neighbour_offers(c);
    if !offers.reached {
        return Vec::new();
    }
    let learned: Vec<Pieces> = offers.learned.iter().map(|r| vec![resource(*r)]).collect();
    let line = |head: &str, items: Vec<Pieces>| -> Pieces {
        let mut out = vec![Piece::plain(head.to_string())];
        out.extend(join_as_sentence(items));
        out.push(Piece::plain("."));
        out
    };
    let mut out: Vec<Pieces> = Vec::new();
    let unseen = offers.unseen.len();
    if offers.tells_by_name {
        let told = !learned.is_empty();
        if told {
            out.push(line("Offers: ", learned));
        }
        if unseen > 0 {
            let named: Vec<Pieces> = offers
                .unseen
                .iter()
                .map(|(r, now)| {
                    let mut item = vec![resource(*r)];
                    if !now {
                        item.push(Piece::plain(" (for closer friends)"));
                    }
                    item
                })
                .collect();
            out.push(line(
                if told {
                    "Also mentions: "
                } else {
                    "Mentions: "
                },
                named,
            ));
        }
    } else if learned.is_empty() {
        if unseen > 0 {
            let noun = if unseen == 1 { "good" } else { "goods" };
            out.push(plain(format!(
                "Offers: {unseen} {noun} you haven't seen yet."
            )));
        }
    } else {
        let mut items = learned;
        if unseen > 0 {
            items.push(plain(format!("{unseen} more")));
        }
        out.push(line("Offers: ", items));
    }
    out
}

/// Sentences run together, a blank between each.
fn sentences(parts: Vec<Pieces>) -> Pieces {
    let mut out: Pieces = Vec::new();
    for part in parts {
        if !out.is_empty() {
            out.push(Piece::plain(" "));
        }
        out.extend(part);
    }
    out
}

/// A structure's statement: every field its definition carries apart from its
/// name, its price and that price's escalation, its reveal threshold, and its
/// gate, wherever the field departs from its baseline. Per-tick fields are
/// stated per game second; ratios as the percentage they add.
fn structure_statement(s: clowder_core::StructureId) -> Pieces {
    let d = content::structure_def(s);
    let mut parts: Vec<Pieces> = Vec::new();

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
    for (r, amt) in d.productions {
        let mut tail = " per second".to_string();
        if !qualifiers.is_empty() {
            tail.push_str(&format!(", {}", qualifiers.join(", ")));
        }
        tail.push('.');
        parts.push(vec![
            Piece::plain(format!("Produces {} ", per_second(*amt))),
            resource(*r),
            Piece::plain(tail),
        ]);
    }
    if d.productions.is_empty() && !qualifiers.is_empty() {
        parts.push(plain(format!("Is {}.", qualifiers.join(", "))));
    }

    if d.housing != 0 {
        parts.push(plain(format!("Houses {} kittens.", d.housing)));
    }
    if d.tinkerer_seats != 0 {
        parts.push(plain(format!(
            "Seats {} {}.",
            d.tinkerer_seats,
            tinkerer_noun(d.tinkerer_seats)
        )));
    }
    for (r, amt) in d.cap_bonuses {
        parts.push(vec![
            Piece::plain(format!("Adds {} storage for ", fmt::exact(*amt))),
            resource(*r),
            Piece::plain("."),
        ]);
    }
    if d.happiness_points != 0.0 {
        parts.push(plain(format!(
            "Lifts mood by {} points.",
            fmt::exact(d.happiness_points)
        )));
    }
    if d.crowding_relief != 0.0 {
        parts.push(plain(format!(
            "Eases crowding by {}%.",
            percent(d.crowding_relief)
        )));
    }
    for (r, amt) in d.resource_ratios {
        parts.push(vec![
            Piece::plain("Raises production of "),
            resource(*r),
            Piece::plain(format!(" by {}%.", percent(*amt))),
        ]);
    }
    if d.craft_ratio != 0.0 {
        parts.push(plain(format!(
            "Raises craft output by {}%.",
            percent(d.craft_ratio)
        )));
    }
    if d.luxury_demand != 0.0 {
        parts.push(plain(format!(
            "Shifts luxury demand by {}%.",
            percent(d.luxury_demand)
        )));
    }
    if d.jamboree_ratio != 0.0 {
        parts.push(plain(format!(
            "Raises jamboree mood by {}%.",
            percent(d.jamboree_ratio)
        )));
    }
    if d.jamboree_arrival != 0.0 {
        parts.push(plain(format!(
            "Raises jamboree arrivals by {}%.",
            percent(d.jamboree_arrival)
        )));
    }
    if d.global_ratio != 0.0 {
        parts.push(plain(format!(
            "Raises the global ratio by {}%.",
            percent(d.global_ratio)
        )));
    }
    if let Some(conv) = &d.converter {
        for (r, amt) in conv.inputs {
            parts.push(vec![
                Piece::plain(format!("Converts {} ", per_second(*amt))),
                resource(*r),
                Piece::plain(" per second."),
            ]);
        }
        for (r, amt) in conv.outputs {
            parts.push(vec![
                Piece::plain(format!("Yields {} ", per_second(*amt))),
                resource(*r),
                Piece::plain(" per second."),
            ]);
        }
    }
    // A negative mess field tidies; it is stated as what it tidies.
    // @spec TUI-NAV-177
    if d.mess > 0.0 {
        parts.push(plain(format!(
            "Makes {} mess per second.",
            per_second(d.mess)
        )));
    } else if d.mess < 0.0 {
        parts.push(plain(format!(
            "Tidies {} mess per second.",
            per_second(-d.mess)
        )));
    }
    if d.energy_prod != 0.0 {
        parts.push(plain(format!(
            "Makes {} energy.",
            fmt::exact(d.energy_prod)
        )));
    }
    if d.energy_cons != 0.0 {
        parts.push(plain(format!("Uses {} energy.", fmt::exact(d.energy_cons))));
    }
    if d.outdoor_ratio != 0.0 {
        parts.push(plain(format!(
            "Raises outdoor production by {}%.",
            percent(d.outdoor_ratio)
        )));
    }
    if d.long_sun_cap != 0.0 {
        parts.push(plain(format!(
            "Extends the long sun by {}.",
            fmt::exact(d.long_sun_cap)
        )));
    }

    sentences(parts)
}

/// A job's statement: the yield per worker per game second, in the resource
/// its definition names. A job with no yield (the Tinkerer) has none.
fn job_statement(j: clowder_core::JobId) -> Pieces {
    match content::job_def(j).yield_per_tick {
        Some((r, per_tick)) => vec![
            Piece::plain(format!("Yields {} ", per_second(per_tick))),
            resource(r),
            Piece::plain(" per worker per second."),
        ],
        None => Vec::new(),
    }
}

/// A craft's statement: the resource its recipe makes, and the tinkerer work
/// tier the recipe meets. Shared by the make row and the tinkerer count row —
/// both describe the same recipe.
fn craft_statement(c: clowder_core::CraftId) -> Pieces {
    let def = content::craft_def(c);
    let tail = if def.tier != 0 {
        format!(", at tinkerer tier {}.", def.tier)
    } else {
        ".".to_string()
    };
    vec![
        Piece::plain("Makes "),
        resource(content::craft_output(c)),
        Piece::plain(tail),
    ]
}

/// A blurb as the description states it: its words, each mark read as its
/// text, and each resource a mark names in that resource's coat across the
/// words as the blurb spells them. @spec TUI-NAV-018, TUI-THEME-022
fn blurb_statement(blurb: &str) -> Pieces {
    content::blurb_parts(blurb)
        .into_iter()
        .map(|part| match part {
            content::BlurbPart::Text(t) => Piece::plain(t),
            content::BlurbPart::Resource(r, t) => Piece::name(r, t, t.chars().count()),
        })
        .collect()
}

/// The plain mechanical statement behind a piece of content — what the row's
/// flavour name is borrowing against. For a kind whose definition carries a
/// blurb, the blurb *is* the statement and its effect fields are not restated.
/// A rite, an arch, a tradition, and a dream loom carry nothing past name,
/// price, and gate, so they have none. Every resource the statement names
/// wears its coat. @spec TUI-NAV-018, TUI-THEME-022
fn statement(id: ContentId) -> Pieces {
    match id {
        ContentId::Structure(s) => structure_statement(s),
        ContentId::Job(j) => job_statement(j),
        ContentId::Craft(c) => craft_statement(c),
        ContentId::Idea(i) => blurb_statement(content::idea_def(i).blurb),
        ContentId::Refinement(r) => blurb_statement(content::refinement_def(r).blurb),
        ContentId::Rule(r) => blurb_statement(content::rule_def(r).blurb),
        ContentId::Rite(r) => blurb_statement(content::rite_def(r).blurb),
        ContentId::Arch(a) => blurb_statement(content::arch_def(a).blurb),
        ContentId::Tradition(t) => blurb_statement(content::tradition_def(t).blurb),
        ContentId::DreamLoom(l) => blurb_statement(content::dream_loom_def(l).blurb),
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
    pieces_text(&lock_reason_pieces(reason))
}

/// The lock reason as the description draws it: each resource a shortfall
/// names in its coat. @spec TUI-NAV-079, TUI-THEME-022
fn lock_reason_pieces(reason: &LockReason) -> Pieces {
    match reason {
        LockReason::NotYetReached => plain("Not reached yet.".to_string()),
        LockReason::Prerequisite => plain("A prerequisite is still unmet.".to_string()),
        LockReason::AlreadyYours => plain("Already yours.".to_string()),
        LockReason::NoOneFree => plain("Nobody in the meadow is free to take this on.".to_string()),
        LockReason::NoRoom => plain("There is no room for this yet.".to_string()),
        LockReason::ShortBy { missing } => {
            if missing.is_empty() {
                return plain("The price is not yet met.".to_string());
            }
            let mut out = vec![Piece::plain("Short by ")];
            for (k, (r, amt)) in missing.iter().enumerate() {
                if k > 0 {
                    out.push(Piece::plain(", "));
                }
                out.push(Piece::plain(format!("{} ", fmt::exact(*amt))));
                out.push(resource(*r));
            }
            out.push(Piece::plain("."));
            out
        }
    }
}

// ---------------------------------------------------------------------------
// The next cost and its wait — TUI-NAV-078
// ---------------------------------------------------------------------------

/// A stretch of real time, in a plain unit — never rounded to sooner than the
/// rate gives.
fn wait_text(seconds: f64) -> String {
    const UNITS: [(f64, &str); 4] = [
        (86_400.0, "days"),
        (3_600.0, "hours"),
        (60.0, "minutes"),
        (1.0, "seconds"),
    ];
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
fn component_line(
    state: &GameState,
    r: ResourceId,
    need: f64,
    reason: &Option<LockReason>,
) -> Vec<Pieces> {
    let mut comes_from = None;
    let have = state.stock(r);
    let name = resource_label(r);
    let chars = name.chars().count();
    let mut line = format!(": need {}, have {}", fmt::exact(need), fmt::exact(have));

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
                comes_from = comes_from_line(state, r);
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
    // The name heads the line in its coat, short or covered alike; the line
    // is the inherited foreground whatever the shortfall. @spec TUI-THEME-019
    let mut out = vec![vec![Piece::name(r, name, chars), Piece::plain(line)]];
    out.extend(comes_from);
    out
}

/// Where a store can come from, beneath a short component whose store is not
/// growing: nothing at all where any source the core credits is raising it,
/// or where the core lists no source. The sources the player knows are named,
/// in the core's order; the rest are gestured at, once for each kind.
/// @spec TUI-NAV-163
fn comes_from_line(state: &GameState, r: ResourceId) -> Option<Pieces> {
    use clowder_core::Source;
    if state.flows(r).iter().any(|(_, rate)| *rate > 0.0) {
        return None;
    }
    let sources = state.sources_of(r);
    if sources.is_empty() {
        return None;
    }
    const PHRASES: [&str; 9] = [
        "a building you haven't found",
        "work no kitten knows yet",
        "a recipe you haven't learned",
        "trade with a neighbour",
        "something you can't do yet",
        "an arch upgrade you haven't found",
        "a refinement you haven't found",
        "a rite you haven't learned",
        "a loom you haven't found",
    ];
    let kind = |src: &Source| match src {
        Source::Structure(_) => 0,
        Source::Job(_) => 1,
        Source::Craft(_) => 2,
        Source::Colony(_) => 3,
        Source::Act(_) => 4,
        Source::Arch(_) => 5,
        Source::Refinement(_) => 6,
        Source::Rite(_) => 7,
        Source::Loom(_) => 8,
    };
    let mut items: Vec<Pieces> = sources
        .iter()
        .filter(|(_, known)| *known)
        .map(|(src, _)| {
            // A recipe is named for the store it makes, so its name is that
            // store's and wears its coat. @spec TUI-THEME-019
            if let Source::Craft(c) = src {
                let made = content::craft_output(*c);
                if content_name(ContentId::Craft(*c)) == resource_label(made) {
                    return vec![resource(made), Piece::plain(" (recipe)")];
                }
            }
            plain(match src {
                Source::Structure(b) => content_name(ContentId::Structure(*b)),
                Source::Job(j) => format!("{} (job)", content_name(ContentId::Job(*j))),
                Source::Craft(c) => format!("{} (recipe)", content_name(ContentId::Craft(*c))),
                Source::Colony(c) => format!("trade with {}", content::colony_def(*c).name),
                Source::Act(a) => humanize(&format!("{a:?}")),
                Source::Arch(a) => content_name(ContentId::Arch(*a)),
                Source::Refinement(f) => content_name(ContentId::Refinement(*f)),
                Source::Rite(t) => content_name(ContentId::Rite(*t)),
                Source::Loom(l) => content_name(ContentId::DreamLoom(*l)),
            })
        })
        .collect();
    let mut unknown: Vec<usize> = sources
        .iter()
        .filter(|(_, known)| !known)
        .map(|(src, _)| kind(src))
        .collect();
    unknown.sort_unstable();
    unknown.dedup();
    items.extend(unknown.into_iter().map(|k| plain(PHRASES[k].to_string())));
    let mut line = vec![Piece::plain("Comes from: ")];
    line.extend(join_as_sentence(items));
    line.push(Piece::plain("."));
    Some(line)
}

/// The next cost lines for a content item that takes one: the core's price for
/// one more, per component with need and have, marked by the core's lock
/// reason. @spec TUI-NAV-078
fn cost_lines(state: &GameState, id: ContentId, reason: &Option<LockReason>) -> Vec<Pieces> {
    let cost = state.cost_of_next(id, 1);
    cost.iter()
        .flat_map(|(r, need)| component_line(state, *r, *need, reason))
        .collect()
}

/// The next cost for a Pounce or Trade row: the one-unit price the core
/// reports, per component. The core reports no lock reason for these rows, so
/// need and have stand unmarked and no wait is stated beside them.
/// @spec TUI-NAV-073
fn unit_cost_lines(state: &GameState, unit: &[(ResourceId, f64)]) -> Vec<Pieces> {
    unit.iter()
        .map(|(r, need)| {
            let name = resource_label(*r);
            let chars = name.chars().count();
            vec![
                Piece::name(*r, name, chars),
                Piece::plain(format!(
                    ": need {}, have {}",
                    fmt::exact(*need),
                    fmt::exact(state.stock(*r))
                )),
            ]
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
fn content_lines(
    state: &GameState,
    row: &Row,
    id: ContentId,
    tag: &str,
    prompt: Option<&CountPrompt>,
) -> Vec<Pieces> {
    let mut out: Vec<Pieces> = Vec::new();

    let stmt = statement(id);
    if !pieces_text(&stmt).trim().is_empty() {
        out.push(stmt);
    }

    match id {
        ContentId::Structure(s) => out.push(plain(format!("Owned: {}.", state.structures[s.ix()]))),
        ContentId::Rite(r) => out.push(plain(format!("Held: {}.", state.rite_counts[r.ix()]))),
        _ => {}
    }

    if let ContentId::Job(j) = id {
        out.push(plain(format!(
            "Workers in the meadow: {}.",
            state.effective_workers(j)
        )));
    }

    if tag == "tinker" {
        if let ContentId::Craft(c) = id {
            out.push(plain(format!(
                "Tinkerers in the meadow: {}.",
                bound_tinkerers(state, c)
            )));
        }
        if let Some(p) = prompt {
            out.push(crate::render::count_prompt_preview_pieces(
                state, row, &p.digits,
            ));
        }
        // A tinkerer count row binds rather than makes: no next cost, and the
        // bind's own reason rather than the recipe's. @spec TUI-NAV-160
        if let ContentId::Craft(c) = id {
            if let Some(reason) = state.tinkerer_bind_reason(c) {
                out.push(bind_reason_pieces(&reason));
            }
        }
        return out;
    }

    let reason = state.lock_reason(id);

    // TUI-NAV-035: the prompt's count and preview substitute for the next
    // cost alone, exactly where the core reports one to substitute — the
    // lock reason beside it (TUI-NAV-079) is read whether or not a prompt is
    // held to this row.
    if let Some(p) = prompt {
        out.push(crate::render::count_prompt_preview_pieces(
            state, row, &p.digits,
        ));
    } else if reason != Some(LockReason::AlreadyYours) {
        out.extend(cost_lines(state, id, &reason));
    }
    if let Some(r) = &reason {
        out.push(lock_reason_pieces(r));
    }

    out
}

/// Why a bind would be refused, in the bind's own words: its no-one-free and
/// no-room mean a kitten holding the Tinkerer job and a seat, not a kitten
/// free for any job or room in general. @spec TUI-NAV-160
fn bind_reason_pieces(reason: &LockReason) -> Pieces {
    match reason {
        LockReason::NoOneFree => plain("No tinkerer is waiting for a seat.".to_string()),
        LockReason::NoRoom => plain("Every seat at the bench is taken.".to_string()),
        other => lock_reason_pieces(other),
    }
}

/// `tinkerer` for one, `tinkerers` for any other count.
fn tinkerer_noun(n: u32) -> &'static str {
    if n == 1 {
        "tinkerer"
    } else {
        "tinkerers"
    }
}

/// Two blank columns that wrapping never trims, so a line set in beneath a
/// heading keeps its indent.
const INDENT: &str = "\u{a0}\u{a0}";

/// The tinkerers' bench, read from the core's bench query and the kittens'
/// names and homes as state holds them: the seats; each revealed structure
/// that gives seats, with its seats per copy and its copies; who sits where;
/// and who waits. @spec TUI-NAV-158, TUI-NAV-159
fn bench_lines(state: &GameState) -> Vec<Pieces> {
    let bench = state.bench();
    let taken = bench.seated.len() as u32;
    let mut out = vec![plain(format!(
        "Seats: {} — {} taken, {} free.",
        bench.seats,
        taken,
        bench.seats.saturating_sub(taken)
    ))];
    for s in clowder_core::StructureId::ALL {
        let per = content::structure_def(s).tinkerer_seats;
        let id = ContentId::Structure(s);
        if per == 0 || state.availability(id) == clowder_core::Availability::Hidden {
            continue;
        }
        out.push(plain(format!(
            "Each {} seats {per} {}. Owned: {}.",
            content_name(id),
            tinkerer_noun(per),
            state.structures[s.ix()]
        )));
    }
    if !bench.seated.is_empty() {
        out.push(plain("At the bench:".to_string()));
        for (ix, c) in &bench.seated {
            let away = state.kittens[*ix].location != clowder_core::Location::Home;
            out.push(plain(format!(
                "{INDENT}{} — {}{}",
                full_name(state, &RowKey::Kitten(*ix, "")),
                content_name(ContentId::Craft(*c)),
                if away { " (visiting)" } else { "" }
            )));
        }
    }
    if !bench.waiting.is_empty() {
        let names: Vec<String> = bench
            .waiting
            .iter()
            .map(|ix| full_name(state, &RowKey::Kitten(*ix, "")))
            .collect();
        out.push(plain(format!("Waiting for a seat: {}.", names.join(", "))));
    }
    out
}

/// The tinkerers in the meadow bound to a recipe — a kitten away from the
/// meadow keeps its binding but is not counted while away.
fn bound_tinkerers(state: &GameState, c: clowder_core::CraftId) -> usize {
    state
        .kittens
        .iter()
        .filter(|k| k.tinkering == Some(c) && k.location == clowder_core::Location::Home)
        .count()
}

/// A store's rate in the inspector: the exact form of its size behind `+`
/// where it is above zero and `-` where it is below, with no sign where it
/// reads `0`. @spec TUI-RENDER-012
fn signed_rate(rate: f64) -> String {
    let size = fmt::exact(rate.abs());
    if size == "0" {
        size
    } else if rate > 0.0 {
        format!("+{size}")
    } else {
        format!("-{size}")
    }
}

/// The name a stores row gives a flow's source: an item as its own row names
/// it, the kittens themselves by the roster's word for them.
fn flow_source_name(src: FlowSource) -> String {
    match src {
        FlowSource::Structure(s) => content_name(ContentId::Structure(s)),
        FlowSource::Job(j) => content_name(ContentId::Job(j)),
        FlowSource::Craft(c) => content_name(ContentId::Craft(c)),
        FlowSource::Arch(a) => content_name(ContentId::Arch(a)),
        FlowSource::Kittens => "kittens".to_string(),
    }
}

/// A stores row: the resource's exact stock, the cap the core reports or that
/// it has none, and the core's net rate for it per game second; then what
/// moves it — the core's flows by source, those adding to the store under an
/// In heading and those drawing it down under an Out heading, each as its
/// source's name and its signed rate, in the core's order. A flow that reads
/// `0` has no line, and a heading stands only above a line.
/// @spec TUI-NAV-080, TUI-RENDER-012
/// @spec TUI-NAV-135, TUI-NAV-136, TUI-NAV-137, TUI-NAV-138, TUI-NAV-139
fn store_lines(state: &GameState, r: ResourceId) -> Vec<Pieces> {
    let cap = state.cap(r);
    let cap_text = if cap.is_finite() {
        fmt::exact(cap)
    } else {
        "none".to_string()
    };
    let mut lines = vec![plain(format!(
        "Stock: {}. Cap: {cap_text}. Net rate: {} per game second.",
        fmt::exact(state.stock(r)),
        signed_rate(state.net_rate(r)),
    ))];

    let mut ins = Vec::new();
    let mut outs = Vec::new();
    for (src, rate) in state.flows(r) {
        let text = signed_rate(rate);
        if text == "0" {
            continue;
        }
        let line = plain(format!("{} {text}", flow_source_name(src)));
        if rate > 0.0 {
            ins.push(line);
        } else {
            outs.push(line);
        }
    }
    if !ins.is_empty() || !outs.is_empty() {
        lines.push(Vec::new());
    }
    if !ins.is_empty() {
        lines.push(plain("In, per game second:".to_string()));
        lines.extend(ins);
    }
    if !outs.is_empty() {
        lines.push(plain("Out, per game second:".to_string()));
        lines.extend(outs);
    }
    lines
}

/// What the row's act brought home the last time it was taken: its newest
/// return in the core's journal — on the Pounce row a pounce's, on a Trade row
/// a trade's with the row's colony — added up with every other return of that
/// act at the same tick, since one press (`X` making one call per unit) takes
/// all its calls between two ticks. Read from the journal every frame, so
/// nothing is kept here; nothing is stated where the journal holds no such
/// act. The caller sets the lines beneath the `last time` divider.
/// @spec TUI-NAV-148, TUI-NAV-149, TUI-THEME-019
fn last_time_lines(state: &GameState, command: &Command) -> Vec<Pieces> {
    // One return of the row's act: squads or caravans sent, what it raised,
    // caravans napped.
    type Return = (u32, Vec<(ResourceId, f64)>, u32);
    let returned = |item: &JournalItem| -> Option<Return> {
        match (item, command) {
            (
                JournalItem::Happened(Event::PounceReturned {
                    squads,
                    feathers,
                    bells,
                    shinies,
                    unicorns,
                }),
                Command::Pounce(_),
            ) => Some((
                *squads,
                super::overlays::pounce_haul(*feathers, *bells, *shinies, *unicorns),
                0,
            )),
            (
                JournalItem::Happened(Event::TradeReturned {
                    colony,
                    caravans,
                    napped,
                    goods,
                }),
                Command::Trade(row_colony, _),
            ) if colony == row_colony => Some((*caravans, goods.clone(), *napped)),
            _ => None,
        }
    };
    let noun = match command {
        Command::Pounce(_) => "squad",
        _ => "caravan",
    };

    let entries = state.journal.iter_chronological();
    let Some(tick) = entries
        .iter()
        .rev()
        .find(|e| returned(&e.item).is_some())
        .map(|e| e.tick)
    else {
        return Vec::new();
    };
    let (mut sent, mut napped) = (0u32, 0u32);
    let mut raised = [0.0f64; ResourceId::ALL.len()];
    for (n, goods, slept) in entries
        .iter()
        .filter(|e| e.tick == tick)
        .filter_map(|e| returned(&e.item))
    {
        sent += n;
        napped += slept;
        for (r, amount) in goods {
            raised[r.ix()] += amount;
        }
    }
    let goods: Vec<(ResourceId, f64)> = ResourceId::ALL
        .into_iter()
        .map(|r| (r, raised[r.ix()]))
        .filter(|(_, amount)| *amount > 0.0)
        .collect();

    let plural = if sent == 1 { "" } else { "s" };
    let close = if goods.is_empty() { "." } else { " with:" };
    let mut out = vec![plain(format!("{sent} {noun}{plural} came home{close}"))];
    for (r, amount) in goods {
        out.push(vec![
            resource(r),
            Piece::plain(format!(" +{}", fmt::exact(amount))),
        ]);
    }
    if napped > 0 {
        out.push(plain(format!("{napped} of them napped.")));
    }
    out
}

/// The whole of the inspector's content for one row. While an open count
/// prompt is held to this row, its count and preview (TUI-NAV-067) are shown
/// alongside — the same row the prompt was opened on, by identity, so a
/// cursor that has since moved off it never previews on the wrong row.
/// @spec TUI-NAV-018, TUI-NAV-077, TUI-NAV-035, TUI-THEME-011, TUI-NAV-089,
///       TUI-NAV-091
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

    let mut lines: Vec<Pieces> = vec![name_line(state, key)];

    // The Elder's row states the mark whole beneath the name, whether or not
    // the list's row had room to draw it. @spec TUI-NAV-108
    if matches!(key, RowKey::Kitten(ix, _) if state.elder == Some(*ix)) {
        lines.push(plain(crate::render::ELDER_MARK.to_string()));
    }

    // An adventure's row says, beneath the name, which meadow the core holds
    // it for: active in this one first, then armed for the next.
    // @spec TUI-NAV-179
    if let RowKey::Adventure(Some(a)) = key {
        if state.active_adventure == Some(*a) {
            lines.push(plain("Under way in this meadow.".to_string()));
        }
        if state.armed_adventure == Some(*a) {
            lines.push(plain("Chosen for the next meadow.".to_string()));
        }
    }

    // A later row of a multi-row item states its own label whole, beneath the
    // item's name — never the item's name a second time. @spec TUI-NAV-091
    if let Some(label) = crate::nav::rows::later_row_label(key) {
        lines.push(plain(label.to_string()));
    }

    // The swapped-in inspector carries the same consequence summary the list
    // would show beneath the armed row, in the same attention colour it wears
    // there — never on its own line, only on what it expands to.
    // @spec TUI-NAV-077, TUI-THEME-011
    let mut attention_rows: Option<std::ops::Range<usize>> = None;
    let mut dim_rows: Vec<usize> = Vec::new();
    if let Some(armed) = armed.filter(|a| a.key == *key && crate::render::armed_is_live(a)) {
        let start = lines.len();
        lines.push(Vec::new());
        lines.extend(
            crate::render::consequence_summary(state, armed)
                .into_iter()
                .map(plain),
        );
        attention_rows = Some(start..lines.len());
    }

    match key {
        RowKey::Content(id, tag) => {
            lines.push(Vec::new());
            lines.extend(content_lines(state, &row, *id, tag, on_this_row));
        }
        RowKey::Resource(r) => {
            lines.push(Vec::new());
            lines.extend(store_lines(state, *r));
        }
        RowKey::Bench => {
            lines.push(Vec::new());
            lines.extend(bench_lines(state));
        }
        RowKey::Energy => {
            lines.push(Vec::new());
            lines.extend(energy_lines(state));
        }
        RowKey::Mess => {
            lines.push(Vec::new());
            lines.extend(mess_lines(state));
        }
        // An adventure's definition carries a blurb, and the blurb is its
        // statement. @spec TUI-NAV-018
        RowKey::Adventure(Some(a)) => {
            lines.push(Vec::new());
            lines.push(blurb_statement(content::adventure_def(*a).blurb));
        }
        _ => {
            if let Some(p) = on_this_row {
                lines.push(Vec::new());
                lines.push(crate::render::count_prompt_preview_pieces(
                    state, &row, &p.digits,
                ));
            } else if let RowKey::Colony(c, "penpal") = &row.key {
                // The next letter's price, then what the letters have done.
                // @spec TUI-NAV-171, TUI-NAV-172
                if let Some(report) = state.pen_pal(*c) {
                    lines.push(Vec::new());
                    lines.extend(unit_cost_lines(state, &[(ResourceId::Charm, report.price)]));
                    lines.push(Vec::new());
                    lines.extend(pen_pal_lines(*c, &report));
                }
            } else if let RowKind::Act { command, .. } = &row.kind {
                if let Some(unit) = state.unit_price(command) {
                    lines.push(Vec::new());
                    lines.extend(unit_cost_lines(state, &unit));
                }
            }
            if let RowKey::Colony(c, "trade") = &row.key {
                lines.extend(offer_lines(state, *c));
            }
            if let RowKind::Act { command, .. } = &row.kind {
                // The history is set apart from what the row is by a dim
                // divider drawn as a list's group header. @spec TUI-NAV-148
                let last = last_time_lines(state, command);
                if !last.is_empty() {
                    lines.push(Vec::new());
                    dim_rows.push(lines.len());
                    lines.push(plain(fmt::group_header_line(
                        "last time",
                        area.width as usize,
                    )));
                    lines.extend(last);
                }
            }
        }
    }

    // An attention line wears attention whole, every name in it included
    // (TUI-THEME-020); the divider wears `dim` whole, as a group header does;
    // every other line is the inherited foreground with each resource's name
    // in its coat (TUI-THEME-019).
    let out: Vec<Line<'static>> = lines
        .into_iter()
        .enumerate()
        .map(|(i, l)| match &attention_rows {
            Some(r) if r.contains(&i) => {
                Line::styled(pieces_text(&l), crate::render::style_for(Role::Attention))
            }
            _ if dim_rows.contains(&i) => {
                Line::styled(pieces_text(&l), crate::render::style_for(Role::Dim))
            }
            _ => line_from_pieces(&l, ratatui::style::Style::default(), true, usize::MAX),
        })
        .collect();
    let mut out = out;
    if let RowKey::Kitten(ix, _) = key {
        out.extend(about_lines(state, *ix, area.width as usize));
    }
    let para = Paragraph::new(out).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

/// What a neighbour's pen pals have done and what another letter would do,
/// from the core's Pen Pal query: the level, the standing in words with the
/// letters' part, what that standing does to a caravan, the next letter's
/// warmth, and the level at which something more opens. @spec TUI-NAV-172
fn pen_pal_lines(c: clowder_core::ColonyId, r: &clowder_core::PenPalReport) -> Vec<Pieces> {
    let name = content::colony_def(c).name;
    let pct = |v: f64| (v * 100.0).round() as i64;
    let mut out = vec![plain(format!("Pen pals: {}.", r.level))];
    let mut standing = if pct(r.standing) > 0 {
        format!("The {name} are warm to you: +{}%.", pct(r.standing))
    } else if pct(r.standing) < 0 {
        format!("The {name} are shy of you: \u{2212}{}%.", -pct(r.standing))
    } else {
        format!("The {name} are neutral towards you.")
    };
    if pct(r.warmth) > 0 {
        standing.push_str(&format!(" Your letters added +{}%.", pct(r.warmth)));
    }
    out.push(plain(standing));
    let odds = pct(r.standing.abs().min(1.0));
    out.push(plain(if pct(r.standing) > 0 {
        format!("About {odds}% of caravans come home with a bonus.")
    } else if pct(r.standing) < 0 {
        format!("About {odds}% of caravans find them napping.")
    } else {
        "Caravans trade at the usual odds.".to_string()
    }));
    out.push(plain(if pct(r.next_warmth) > 0 {
        format!("Another letter warms them by {}%.", pct(r.next_warmth))
    } else {
        "Letters can't warm them any further.".to_string()
    }));
    if let Some(level) = r.opens_at {
        out.push(plain(format!(
            "At pen pal level {level}, they'll trade something more."
        )));
    }
    out
}

/// The Energy readout: the totals, every structure's share under what it
/// makes and what it uses, and what the balance means — the core's answers,
/// stated. @spec TUI-NAV-169
fn energy_lines(state: &GameState) -> Vec<Pieces> {
    let (made, used) = (state.energy_production(), state.energy_consumption());
    let mut out = vec![
        plain(format!("Made: {}", fmt::exact(made))),
        plain(format!("Used: {}", fmt::exact(used))),
    ];
    let shares = state.energy_shares();
    for (heading, sign, positive) in [("Makes energy:", "+", true), ("Uses energy:", "-", false)] {
        let listed: Vec<&(clowder_core::StructureId, f64)> = shares
            .iter()
            .filter(|(_, v)| (*v > 0.0) == positive && *v != 0.0)
            .collect();
        if listed.is_empty() {
            continue;
        }
        out.push(plain(heading.to_string()));
        for (s, v) in listed {
            out.push(plain(format!(
                "{INDENT}{} \u{d7}{} {sign}{}",
                content::structure_def(*s).name,
                state.structures[s.ix()],
                fmt::exact(v.abs())
            )));
        }
    }
    out.push(if used > made {
        plain(format!(
            "Short of energy: global production, Elder Wisdom, and the Long Sun run at {}%.",
            (state.energy_deficit_ratio() * 100.0).round()
        ))
    } else if made > used {
        plain(format!(
            "Enough energy for everything, with {} to spare.",
            fmt::exact(made - used)
        ))
    } else {
        plain("Exactly enough energy for everything.".to_string())
    });
    out
}

/// The Mess readout: how much, which way it is going, what its level slows,
/// and where the next level begins. @spec TUI-NAV-170
fn mess_lines(state: &GameState) -> Vec<Pieces> {
    let r = state.mess_report();
    let direction = if r.rate > 0.0 {
        format!("rising by {}/s.", fmt::exact(r.rate))
    } else if r.rate < 0.0 {
        format!("falling by {}/s.", fmt::exact(-r.rate))
    } else {
        "holding steady.".to_string()
    };
    let mut out = vec![plain(format!(
        "Mess: {}, {direction}",
        fmt::exact(r.amount)
    ))];
    match r.level {
        None => out.push(plain("Not enough to slow anything down.".to_string())),
        Some(_) => {
            if r.catnip < 1.0 {
                out.push(plain(format!(
                    "Catnip grows at {}% of its pace.",
                    fmt::exact(r.catnip * 100.0)
                )));
            }
            if r.arrival < 1.0 {
                out.push(plain(format!(
                    "Kittens arrive at {}% of their pace.",
                    fmt::exact(r.arrival * 100.0)
                )));
            }
            if r.happiness > 0.0 {
                out.push(plain(format!(
                    "Happiness \u{2212}{}.",
                    fmt::exact(r.happiness)
                )));
            }
        }
    }
    let next = r.level.map_or(0, |l| l + 1);
    out.push(plain(format!(
        "Level {next} begins at {}.",
        fmt::exact(clowder_core::content::MESS_LEVEL_UNIT * 10f64.powi(next as i32))
    )));
    out
}

/// A kitten's `about` card, beneath everything else its row states: a dim
/// divider, its portrait, the facts the core reports about it, and its
/// favourites — every word the core's. A kitten the core holds no profile for
/// states its facts alone. @spec TUI-NAV-164, TUI-NAV-165, TUI-NAV-166
fn about_lines(state: &GameState, ix: usize, width: usize) -> Vec<Line<'static>> {
    let Some(facts) = state.kitten_facts(ix) else {
        return Vec::new();
    };
    let dim = crate::render::style_for(Role::Dim);
    let mut out = vec![
        Line::raw(""),
        Line::styled(fmt::group_header_line("about", width), dim),
    ];
    let profile = state.kittens[ix].profile.as_ref();
    if let Some(p) = profile {
        out.extend(crate::render::portrait::portrait_lines(p, width));
    }

    // The facts, each a mechanical line under the Journal's ▪.
    // @spec TUI-NAV-165
    let def = clowder_core::content::trait_def(facts.trait_id);
    let mut temperament = vec![Piece::plain(format!(
        "\u{25aa} Temperament: {}. ",
        def.name
    ))];
    temperament.extend(crate::render::marked_pieces(def.blurb));
    let best = match facts.best_at {
        Some(b) if b.bonus > 0.0 => format!(
            "\u{25aa} Best at: {} (+{}% from skill)",
            clowder_core::content::job_def(b.job).name,
            fmt::exact(b.bonus * 100.0)
        ),
        Some(b) => format!(
            "\u{25aa} Best at: {}",
            clowder_core::content::job_def(b.job).name
        ),
        None => "\u{25aa} Best at: nothing yet".to_string(),
    };
    let since = match facts.arrival {
        Arrival::Day { day, season, year } => {
            format!("\u{25aa} With the clowder since {season:?} Day {day} Year {year}")
        }
        Arrival::CarriedAlong => "\u{25aa} Came along from the old meadow".to_string(),
    };
    for pieces in [
        temperament,
        vec![Piece::plain(format!("\u{25aa} Rank {}", facts.rank))],
        vec![Piece::plain(best)],
        vec![Piece::plain(since)],
    ] {
        out.push(line_from_pieces(
            &pieces,
            Style::default(),
            true,
            usize::MAX,
        ));
    }

    // The favourites, each a flavour line under the Journal's ·, in dim.
    // @spec TUI-NAV-166
    for f in profile.map_or(&[][..], |p| &p.favourites[..]) {
        let label = clowder_core::content::favourite_kind_def(f.kind).label;
        out.push(Line::styled(format!("\u{b7} {label}: {}", f.entry), dim));
    }
    out
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
    let all = crate::nav::rows::drawn_rows(state, section, view);

    // The row the cursor is on is the row the list highlights, re-derived
    // every frame: when the remembered key has left the list, the list falls
    // back to its first row, and the inspector must follow it there rather
    // than describe a row that no longer exists. @spec TUI-NAV-018,
    // TUI-NAV-032
    // Never a header: where the remembered key has left the list, the list
    // falls back to its first selectable row. @spec TUI-NAV-032, TUI-NAV-099
    let Some(cursor) = crate::nav::rows::cursor_index(&all, view.focus[section.index()].as_ref())
    else {
        return;
    };
    let Some(row) = all.get(cursor) else {
        return;
    };
    let key = row.key.clone();
    draw_body(
        f,
        area,
        state,
        section,
        &key,
        view.armed.as_ref(),
        view.prompt.as_ref(),
    );
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
