//! Library face of the simulator. Lets `tests/` and future bench harnesses
//! drive the App without going through the macroquad-wrapped binary.
//!
//! The renderer (`render/`) lives in `main.rs` only, so it isn't reachable
//! from here — keeps the test build off the GPU/windowing path.

pub mod config;
pub mod world;
pub mod sim;
pub mod app;
