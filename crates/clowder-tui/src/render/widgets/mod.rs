//! The frame's parts. One module per region so each can be drawn, and tested,
//! on its own — `mod.rs` composes them and owns nothing but the composition.

pub mod chrome;
pub mod inspector;
pub mod overlays;
