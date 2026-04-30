//! Integration test entry point that pulls in the scenario harness +
//! every scenario module. `cargo test --test scenarios` runs them all.

#[path = "scenarios/mod.rs"]
mod scenarios;
