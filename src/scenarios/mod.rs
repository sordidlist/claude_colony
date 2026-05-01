//! Scenario harness: deterministic, hand-built mini-worlds with a goal
//! the simulation must reach. The same definitions feed both
//! `cargo test --test scenarios` (headless, fast as the CPU goes) and
//! the `scenario_viewer` binary (live, animated, at 1× wall-clock).
//!
//! Each scenario is a `ScenarioDef` exporting a `setup` closure that
//! carves the world and spawns actors, plus a `predicate` that the
//! harness polls each frame to detect success. Failures are diagnosed
//! by the `failure_hint` text — a one-line pointer at the system that
//! would be implicated if the goal isn't reached in time.
//!
//! To add a new scenario: copy one of the existing modules below,
//! return a `ScenarioDef` from `def()`, then list it in `registry()`.

use bevy_ecs::prelude::*;

mod builder;
pub use builder::{Scenario, TestSubject};

/// Self-contained description of a behaviour test.
///
/// `setup` and `predicate` are plain function pointers (not closures)
/// so the registry can be built without heap allocation and so the
/// definitions are usable from `const`-style call sites.
#[derive(Copy, Clone)]
pub struct ScenarioDef {
    pub name:             &'static str,
    pub description:      &'static str,
    pub seed:             u64,
    pub timeout_seconds:  f32,
    pub setup:            fn(&mut Scenario),
    pub predicate:        fn(&World) -> bool,
    pub failure_hint:     &'static str,
}

impl ScenarioDef {
    /// Build a fully-set-up scenario ready to either run headlessly or
    /// hand off to a renderer.
    pub fn build(&self) -> Scenario {
        let mut s = Scenario::new(self.seed);
        (self.setup)(&mut s);
        s
    }

    /// Headless run: step at fixed 1/60 dt until predicate returns true
    /// or `timeout_seconds` elapses. Returns `Ok(elapsed)` on success,
    /// `Err(elapsed)` on timeout.
    pub fn run_headless(&self) -> Result<f32, f32> {
        let mut s = self.build();
        s.run_until(self.timeout_seconds, self.predicate)
    }
}

// ── Scenario modules ───────────────────────────────────────────────
pub mod escape_simple_chamber;
pub mod escape_winding_tunnel;
pub mod hauler_drops_outside;
pub mod hauler_falls_off_pile;
pub mod swarm_kills_spider;
pub mod soldier_kills_lone_spider;
pub mod queen_lays_egg;
pub mod brood_hatches_to_worker;
pub mod forager_picks_up_food;
pub mod dirt_settles_into_slope;
pub mod rewind_doesnt_break_combat;
pub mod mower_shaves_piles;
pub mod mower_retires_after_laps;
pub mod mower_kills_workers;

/// All scenarios known to the test runner and viewer.
pub fn registry() -> Vec<ScenarioDef> {
    vec![
        escape_simple_chamber::def(),
        escape_winding_tunnel::def(),
        hauler_drops_outside::def(),
        hauler_falls_off_pile::def(),
        swarm_kills_spider::def(),
        soldier_kills_lone_spider::def(),
        queen_lays_egg::def(),
        brood_hatches_to_worker::def(),
        forager_picks_up_food::def(),
        dirt_settles_into_slope::def(),
        rewind_doesnt_break_combat::def(),
        mower_shaves_piles::def(),
        mower_retires_after_laps::def(),
        mower_kills_workers::def(),
    ]
}

/// Look up a scenario by exact name.
pub fn find(name: &str) -> Option<ScenarioDef> {
    registry().into_iter().find(|d| d.name == name)
}
