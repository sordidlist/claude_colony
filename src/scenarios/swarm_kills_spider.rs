//! A spider in a chamber surrounded by workers should die within a
//! reasonable time. Pins the combat balance: workers are individually
//! weaker than spiders (the user wants spiders to feel formidable),
//! but a numerical advantage backed by alarm-pheromone swarming should
//! consistently win. If this fails, the colony can't defend itself
//! against deep-tunnel predators.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_alive};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "swarm_kills_spider",
        description: "Six workers in a chamber kill a single spider within 20s of sim time.",
        seed: 91,
        timeout_seconds: 20.0,
        setup,
        predicate,
        failure_hint: "combat balance broken or workers don't engage on alarm",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    // 16x4 underground chamber a few rows below the surface.
    let cx = COLONY_X;
    let cy = COLONY_Y + 8;
    s.fill_rect(cx - 8, cy, cx + 8, cy + 3, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    // The spider is the test subject — predicate watches for its death.
    s.spawn_spider_tagged(cx, cy + 2, 0);

    // Six workers around the spider. Tagged with non-zero ids so the
    // predicate doesn't get confused; but the win condition only cares
    // about the spider (id 0) being gone.
    for (i, dx) in [-3i32, -2, -1, 1, 2, 3].iter().enumerate() {
        s.spawn_worker_tagged(cx + dx, cy + 2, None, (i + 1) as u8);
    }
}

fn predicate(world: &World) -> bool {
    !subject_alive(world, 0)
}
