//! With a queen present, a Brood entity should appear within roughly
//! one egg-interval. Verifies queen_tick is firing and spawning eggs.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::TileType;
use crate::sim::components::Brood;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "queen_lays_egg",
        description: "A queen alone in a chamber lays at least one egg within one egg-interval + 2s.",
        seed: 55,
        // QUEEN_EGG_INTERVAL_S = 8s; allow generous slack for the schedule.
        timeout_seconds: QUEEN_EGG_INTERVAL_S + 2.0,
        setup,
        predicate,
        failure_hint: "queen_tick isn't producing brood entities — reproduction broken",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let cx = COLONY_X;
    let cy = COLONY_Y + 5;
    s.fill_rect(cx - 4, cy, cx + 4, cy + 2, TileType::Chamber)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    s.spawn_queen_tagged(cx, cy + 1, 0);
}

fn predicate(world: &World) -> bool {
    // Any Brood entity in the world counts.
    world.iter_entities().any(|e| e.contains::<Brood>())
}
