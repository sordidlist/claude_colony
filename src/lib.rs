//! Library face of the simulator. Lets `tests/`, the scenario viewer
//! binary, and future bench harnesses drive the App without going through
//! the main game binary.
//!
//! `render/` is now exposed as a library module so multiple binaries
//! (the main game and the scenario viewer) can share it. Tests don't
//! `use` it, so the GPU/windowing path stays out of the test path even
//! though macroquad is a top-level dependency.

pub mod config;
pub mod world;
pub mod sim;
pub mod app;
pub mod render;
pub mod scenarios;
