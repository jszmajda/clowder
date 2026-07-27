//! Drawing. A frame is a pure function of (snapshot, view state, size):
//! nothing cached between frames, no game value mirrored shell-side, and no
//! decision depending on what was drawn last. That is what makes layout
//! testable against an in-memory buffer at an exact size, which in turn is
//! what makes the size-dependent navigation specs verifiable rather than
//! hoped-for. @spec TUI-RENDER-001

pub mod fmt;
pub mod layout;

use clowder_core::GameState;
use ratatui::Frame;

use crate::nav::ViewState;

pub use layout::{Body, Layout};

/// Draw one frame, returning the layout it used.
///
/// The layout must escape: the breakpoints' dead-band is a function of the
/// *previous* frame's layout, so a frame that computed one and dropped it
/// would leave the next frame unable to tell which side of a boundary it was
/// already on. @spec TUI-RENDER-001, TUI-RENDER-002, TUI-RENDER-008
pub fn frame(
    _f: &mut Frame,
    _state: &GameState,
    _view: &ViewState,
    _prev: Option<&Layout>,
) -> Layout {
    todo!("TUI-RENDER-001/002: compute layout, draw each region, return it")
}
