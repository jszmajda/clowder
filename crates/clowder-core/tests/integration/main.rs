//! The integration-test suite for `clowder-core`. Every file in this directory is a module of
//! one test binary, so the suite links once and all of its tests run in parallel. A new test file
//! belongs here as a module, not directly under `tests/`, where Cargo would build it as a binary
//! of its own.

mod common;

mod achievements;
mod blurb_truth;
mod blurbs;
mod bulktime;
mod curiosity;
mod draw_schedule;
mod economy;
mod economy_depth;
mod engine;
mod frontier;
mod idea_unlocks;
mod journal;
mod kittens;
mod legacy;
mod neighbour_offers;
mod persistence;
mod profiles;
mod queries_bench;
mod queries_flows;
mod queries_gating;
mod queries_rates;
mod queries_timers;
mod revealed_resources;
mod society;
mod sources;
mod store_descriptions;
mod store_links;
mod structure_fields;
mod trade_returns;
mod unit_price;
mod wonder;
