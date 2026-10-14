//! The UX grammar: what a key means, where the cursor is, and what a row is.
//!
//! Owns no drawing (that is `render`) and no game rules (those are the core).
//! @spec TUI-NAV-001..057

pub mod action;
pub mod keys;
pub mod mnemonics;
pub mod palette;
pub mod pointer;
pub mod reports;
pub mod rows;
pub mod section;
pub mod update;
pub mod view;

pub use action::{Action, CountSemantics};
pub use pointer::{journal_key, pointer_event};
pub use reports::{queue_command_reports, queue_load_report};
pub use rows::{row_still_listed, Row, RowKey, RowKind};
pub use section::Section;
pub use update::{
    dismiss, expire_unlisted_arming, opening, relist_focus, reset_hidden_tabs, toggle_inspector,
    update, Outcome,
};
pub use view::{ArmedRow, CountPrompt, Overlay, PendingReport, SeenMark, ViewState};

/// A command rejection in words a player can act on.
///
/// The core reports rejections as typed data precisely so the shell can say
/// them plainly; printing the type's Debug form would leak internal spellings
/// and tell the player nothing they could do about it.
/// @spec TUI-NAV-014
pub fn plain_error(e: &clowder_core::CommandError) -> String {
    use clowder_core::CommandError as E;
    match e {
        E::UnknownId => "that isn't a thing here".into(),
        E::Locked => "not yet — something else comes first".into(),
        E::AlreadyResearched => "already known".into(),
        E::NoIdleKittens => "nobody's free to take that on".into(),
        E::NoSeats => "no room at the bench".into(),
        E::NoWorkersAssigned => "nobody's doing that yet".into(),
        E::StorageFull => "nowhere to put it — that store is full".into(),
        E::InsufficientResources { missing } => match missing.first() {
            Some((r, short)) => format!(
                "not enough {} — need {} more",
                format!("{r:?}").to_lowercase(),
                crate::render::fmt::compact(*short)
            ),
            None => "not enough to hand".into(),
        },
    }
}
