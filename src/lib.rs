//! Library crate that holds the game's simulation, rendering, and
//! test scaffolding. Two binaries depend on this lib:
//!
//!   * `colony`           — the playable game (`src/main.rs`).
//!   * `scenario_viewer`  — runs scenario tests at 1× wall-clock so
//!                          their setups can be inspected visually
//!                          (`src/bin/scenario_viewer.rs`).
//!
//! Module dependency direction:
//!
//! ```text
//!   render → sim → world
//!                 ↑
//!             config (leaf)
//!   scenarios → app → (sim, world)
//! ```
//!
//! Nothing in `world/` or `sim/` imports a renderer type, so the
//! simulation drives headless tests without touching the GPU.

pub mod config;
pub mod world;
pub mod sim;
pub mod app;
pub mod render;
pub mod scenarios;
