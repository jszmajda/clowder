//! The event journal: what the player has lived through, kept in the save.
//!
//! Retention is a bounded rolling window of recent entries plus every
//! milestone and away-report permanently. Growth is bounded without the
//! moments worth remembering ever being what gets dropped.
//! @spec SIM-ENG-027, SIM-ENG-029

use crate::events::Event;
use crate::save::OfflineReport;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// What a journal entry records. An away-report is not an `Event` — it is a
/// fold of an interval rather than an occurrence at an instant, which is why
/// it needs its own arm rather than a variant in the event vocabulary.
/// @spec SAVE-015
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JournalItem {
    Happened(Event),
    Away(OfflineReport),
}

/// One entry, stamped with the tick it occurred at so entries group by day.
/// @spec SIM-ENG-027
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub tick: u64,
    pub item: JournalItem,
}

/// The retained history. `recent` rolls; `kept` does not.
/// @spec SIM-ENG-027, SIM-ENG-029
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Journal {
    /// Bounded rolling window, oldest at the front.
    pub recent: VecDeque<JournalEntry>,
    /// Milestones and away-reports, never evicted.
    pub kept: Vec<JournalEntry>,
}

impl Journal {
    /// Append a live occurrence. Milestones land in `kept`; everything else
    /// joins the rolling window, evicting the oldest when it is full.
    /// Replay events never reach here — SIM-ENG-028 keeps them out.
    /// @spec SIM-ENG-027, SIM-ENG-028, SIM-ENG-029
    pub fn record(&mut self, _tick: u64, _event: &Event) {
        todo!("SIM-ENG-027/028/029: classify, append, evict oldest non-milestone")
    }

    /// Record a qualifying away-report as a permanent entry.
    /// @spec SAVE-015
    pub fn record_away(&mut self, _tick: u64, _report: &OfflineReport) {
        todo!("SAVE-015: retain a report above the minimum span, else nothing")
    }

    /// Every retained entry in occurrence order, rolling window and kept
    /// entries merged — what a reader scrolls.
    pub fn iter_chronological(&self) -> Vec<&JournalEntry> {
        todo!("SIM-ENG-027: merge kept and recent in occurrence order")
    }
}
