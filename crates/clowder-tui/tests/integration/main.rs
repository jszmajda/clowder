//! The integration-test suite for `clowder-tui`. Every file in this directory is a module of one
//! test binary, so the suite links once and all of its tests run in parallel. A new test file
//! belongs here as a module, not directly under `tests/`, where Cargo would build it as a binary
//! of its own. The exceptions are files that change process-wide state, which would reach every
//! test in a shared binary: `tests/theme.rs` sets environment variables and `tests/term.rs` sends
//! signals to its own process, so each stays a test target of its own.

mod achievements;
mod act_results;
mod adventures;
mod armed_summary;
mod bench;
mod chrome;
mod clock;
mod composed;
mod count_prompt;
mod fmt;
mod frame;
mod inspector;
mod inspector_reasons;
mod keys;
mod kitten_card;
mod layout;
mod lock;
mod loop_body;
mod loop_order;
mod neighbour_offers;
mod overlays;
mod palette;
mod pen_pals;
mod persist;
mod pointer;
mod readouts;
mod report_queue;
mod resize;
mod rows;
mod section_reveal;
mod session;
mod sidebar_links;
mod sidebar_scroll;
mod sources_hint;
mod store_card;
mod terminal_lost;
mod theme_frames;
mod ticker_lights;
mod transients;
mod update;
mod waking;
