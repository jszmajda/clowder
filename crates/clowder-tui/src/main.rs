//! clowder — the terminal shell.
//!
//! Everything between the terminal and the deterministic core. No game rules
//! live here: player input becomes a `clowder_core::Command`, and every frame
//! is a render of a `GameState` snapshot plus the core's derived queries.

mod nav;
mod render;
mod runtime;
mod theme;

fn main() -> std::process::ExitCode {
    runtime::run()
}
