//! Pins the queen migration behaviour: when the colony has been dug
//! deeper than where she started, she must relocate to the new
//! deepest reachable chamber within roughly one
//! `QUEEN_MIGRATION_INTERVAL_S` window.
//!
//! Setup: spawn a queen at a shallow chamber. After setup, hand-carve
//! a new, much deeper chamber connected via a vertical shaft so the
//! flood-fill from the entrance reaches it. Run the sim. The queen's
//! position should drop by at least `QUEEN_MIGRATION_MIN_DEPTH_GAIN`
//! and her `migrations` counter should tick up.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::TileType;
use crate::sim::components::{QueenState, Position, Ant, AntKind};

const SHALLOW_Y: i32 = COLONY_Y + 4;   // founding chamber row
const DEEP_Y:    i32 = COLONY_Y + 30;  // dug-out deeper chamber

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "queen_migrates_deeper",
        description: "Queen relocates to a new, deeper chamber once one is reachable.",
        seed: 1313,
        // Migration is checked every QUEEN_MIGRATION_INTERVAL_S
        // (30s sim default). Allow two intervals plus slack so a
        // missed first window doesn't false-fail.
        timeout_seconds: QUEEN_MIGRATION_INTERVAL_S * 2.0 + 10.0,
        setup,
        predicate,
        failure_hint: "queen_migration isn't relocating the queen — check find_queen_spot wiring",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Carve the shallow founding chamber + a connecting shaft, then a
    // deeper chamber further down. Both are reachable from the
    // entrance via the shaft, so the flood-fill in `find_queen_spot`
    // will see the deeper one as the new optimum.
    s.fill_rect(COLONY_X - 2, SHALLOW_Y, COLONY_X + 2, SHALLOW_Y + 1,
                TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, DEEP_Y, 1)
     .fill_rect(COLONY_X - 3, DEEP_Y, COLONY_X + 3, DEEP_Y + 2,
                TileType::Chamber)
     .mark_dirty()
     .rebuild_flow_field();

    // Hand-place the queen in the shallow chamber so the test starts
    // in a known state. Also set her migration timer to 0 so the
    // first migration check fires immediately on the next tick — the
    // test isn't measuring the wait-for-first-check delay.
    let qe = s.spawn_queen_tagged(COLONY_X, SHALLOW_Y, 0);
    if let Some(mut q) = s.app.world.get_mut::<QueenState>(qe) {
        q.migration_timer = 0.0;
    }
}

fn predicate(world: &World) -> bool {
    // Find the test-subject queen and check that:
    //   1. she's deeper than her shallow starting row, AND
    //   2. her migrations counter is at least 1.
    for e in world.iter_entities() {
        let Some(ant) = e.get::<Ant>() else { continue; };
        if !matches!(ant.kind, AntKind::Queen) { continue; }
        let Some(p) = e.get::<Position>() else { continue; };
        let Some(q) = e.get::<QueenState>() else { continue; };
        let depth_gain = (p.0.y as i32) - SHALLOW_Y;
        if q.migrations >= 1
            && depth_gain >= QUEEN_MIGRATION_MIN_DEPTH_GAIN {
            return true;
        }
    }
    false
}
