//! The UX grammar: what a key means, where the cursor is, and what a row is.
//!
//! Owns no drawing (that is `render`) and no game rules (those are the core).
//! @spec TUI-NAV-001..057

pub mod action;
pub mod keys;
pub mod rows;
pub mod section;
pub mod view;

pub use action::{Action, CountSemantics};
pub use rows::{Row, RowKey, RowKind};
pub use section::Section;
pub use view::{ArmedRow, CountPrompt, InspectorMode, Overlay, ViewState};
