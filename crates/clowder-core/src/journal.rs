//! The event journal: what the player has lived through, kept in the save.
//!
//! Retention is a bounded rolling window of recent entries plus every
//! milestone and away-report permanently. Growth is bounded without the
//! moments worth remembering ever being what gets dropped.
//! @spec SIM-ENG-027, SIM-ENG-029

use crate::events::{Event, EventClass};
use crate::save::{OfflineReport, MIN_REPORTABLE_ABSENCE_SECONDS};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// The rolling window's capacity. Bounded so a months-long game's save stays
/// a fixed size regardless of how much has happened — milestones are what
/// carry the permanent record. @spec SIM-ENG-027, SIM-ENG-029
const RECENT_CAPACITY: usize = 1_000;

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
    /// Append a live occurrence. Milestones land in `kept`, permanently;
    /// everything else joins the rolling window, evicting the oldest entry
    /// when it is full. Replay events never reach here — SIM-ENG-028 keeps
    /// them out by routing replay through its own non-journaling tick.
    /// @spec SIM-ENG-027, SIM-ENG-028, SIM-ENG-029
    pub fn record(&mut self, tick: u64, event: &Event) {
        match event {
            // Speed is the pace you watch the meadow at, not something that
            // happens in it. Keeping it out is also what preserves the
            // invariant that two meadows differing only in speed are
            // otherwise identical. @spec SIM-ENG-024, SIM-ENG-028
            Event::SpeedChanged { .. } => return,
            // A nap's report was already retained as its away entry; journaling
            // the event too would record one sleep in two places.
            // @spec SIM-DREAM-008
            Event::Napped { .. } => return,
            _ => {}
        }
        let entry = JournalEntry {
            tick,
            item: JournalItem::Happened(event.clone()),
        };
        if event.class() == EventClass::Milestone {
            self.kept.push(entry);
        } else {
            if self.recent.len() >= RECENT_CAPACITY {
                self.recent.pop_front();
            }
            self.recent.push_back(entry);
        }
    }

    /// Record a qualifying away-report as a permanent entry. Reports shorter
    /// than the reportable minimum leave no trace — nothing worth
    /// remembering happened in a blink. @spec SAVE-015
    pub fn record_away(&mut self, tick: u64, report: &OfflineReport) {
        if report.elapsed_seconds < MIN_REPORTABLE_ABSENCE_SECONDS {
            return;
        }
        self.kept.push(JournalEntry {
            tick,
            item: JournalItem::Away(report.clone()),
        });
    }

    /// Every retained entry in occurrence order, rolling window and kept
    /// entries merged — what a reader scrolls. @spec SIM-ENG-027
    pub fn iter_chronological(&self) -> Vec<&JournalEntry> {
        let mut merged: Vec<&JournalEntry> = self.kept.iter().chain(self.recent.iter()).collect();
        merged.sort_by_key(|e| e.tick);
        merged
    }
}
