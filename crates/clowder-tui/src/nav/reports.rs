//! Queuing a bulk-time report's banner: the one seam that decides *whether* a
//! report the core just retained gets a banner at all, and where in the
//! journal's own ordering it sits. What the banner then says is
//! `render::widgets::overlays`'s to draw; this only ever asks the journal.
//! @spec TUI-NAV-027, TUI-NAV-050

use clowder_core::{Event, GameState, JournalItem, LoadedGame, OfflineReport};

use super::view::{PendingReport, ViewState};

/// Where `report` landed in `state.journal.kept` — the permanent list, never
/// the rolling window `recent` mixes in — judged from the journal alone,
/// never from a minimum span held here, which would be the core's own
/// retention rule (SAVE-015) reimplemented outside it. `kept` is append-only,
/// so an index into it stays valid however many ordinary occurrences land in
/// `recent` afterward — unlike a position in the two merged and re-sorted,
/// which an event stamped earlier than this report would shift out from
/// under it. A report reaches `kept` at the tick it was retained, so the one
/// this session just produced is the away entry stamped at the *current*
/// tick; an entry an earlier session already pinned, or one the core
/// declined to keep at all, answers `None`. @spec TUI-NAV-027
fn retained_entry_index(state: &GameState, report: &OfflineReport) -> Option<usize> {
    state
        .journal
        .kept
        .iter()
        .position(|entry| entry.tick == state.ticks && matches!(&entry.item, JournalItem::Away(r) if r == report))
}

/// Queue a banner for `report`, if the journal shows it was in fact retained,
/// at the front of the newest-first queue. `nap_years` is set only for the
/// nap route, which is the one place the core's own retained report cannot
/// say how long the sleep was (Decisions & Alternatives: how much time a
/// nap's banner says passed). @spec TUI-NAV-027, TUI-NAV-050, TUI-NAV-055
fn queue(view: &mut ViewState, state: &GameState, report: &OfflineReport, nap_years: Option<u32>) {
    let Some(entry) = retained_entry_index(state, report) else {
        return;
    };
    view.reports.insert(0, PendingReport { entry, dismissed: false });
    if let Some(years) = nap_years {
        view.nap_years.push((entry, years));
    }
}

/// Queue whatever report a load's catch-up retained, exactly where the core
/// kept it — called once, at start-up, with what the core's loader returned.
/// A load never runs through `update`, so this is its own entry point rather
/// than a case inside the reducer. @spec TUI-NAV-027
pub fn queue_load_report(view: &mut ViewState, loaded: &LoadedGame) {
    queue(view, &loaded.state, &loaded.report, None);
}

/// Queue a banner for whatever an applied command's own events say the core
/// just retained. Only an accepted nap returns a report this way today — a
/// load's report reaches the queue through `queue_load_report` instead, since
/// a load never runs through a command at all. @spec TUI-NAV-027, TUI-NAV-050
pub fn queue_command_reports(view: &mut ViewState, state: &GameState, events: &[Event]) {
    for event in events {
        if let Event::Napped { years, report } = event {
            queue(view, state, report, Some(*years));
        }
    }
}
