//! Regression test for the rewind-zombifies-spiders bug.
//!
//! `restore_snapshot` used to re-spawn ants and hostiles without their
//! `Attacker` (and soldiers without `SoldierAi`). Combat queries
//! require `Attacker`, so after even one rewind every fighter dropped
//! out of combat — workers couldn't damage spiders, spiders couldn't
//! damage anything either, and the world filled up with unkillable
//! creatures wandering aimlessly. This scenario:
//!
//! 1. Sets up the same chamber as `swarm_kills_spider`.
//! 2. Lets the sim run long enough that History captures a snapshot
//!    *before* the spider is dead.
//! 3. Calls `rewind_one_step()` to apply that snapshot.
//! 4. Continues the sim. The spider must still die within the same
//!    timeout the swarm scenario uses, plus the rewound seconds.
//!
//! If history loses the Attacker on either side, combat freezes and
//! this test will time out — exactly the symptom the user was seeing.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_alive};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "rewind_doesnt_break_combat",
        description: "Workers still kill a spider after a mid-fight rewind round-trip.",
        seed: 91,
        // The 25s budget covers up to a couple seconds of pre-rewind
        // combat plus a fresh post-rewind swarm cycle.
        timeout_seconds: 25.0,
        setup,
        predicate,
        failure_hint: "history snapshot/restore is dropping Attacker — rewind freezes all combat",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let cx = COLONY_X;
    let cy = COLONY_Y + 8;
    s.fill_rect(cx - 8, cy, cx + 8, cy + 3, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    s.spawn_spider_tagged(cx, cy + 2, 0);
    for (i, dx) in [-3i32, -2, -1, 1, 2, 3].iter().enumerate() {
        s.spawn_worker_tagged(cx + dx, cy + 2, None, (i + 1) as u8);
    }

    // Pump the sim long enough that History captures at least one
    // snapshot (SNAPSHOT_INTERVAL_S = 1.0s). Then rewind once. After
    // this returns, the test predicate keeps stepping the sim until
    // the spider dies (or times out).
    let dt = 1.0 / 60.0;
    for _ in 0..((SNAPSHOT_INTERVAL_S / dt).ceil() as usize + 30) {
        s.app.step(dt);
    }
    let _ = s.app.rewind_one_step();
}

fn predicate(world: &World) -> bool {
    !subject_alive(world, 0)
}
