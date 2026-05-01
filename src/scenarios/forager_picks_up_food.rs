//! A worker placed next to a food pellet on the surface should pick
//! it up — meaning the food entity gets despawned and the worker's
//! cargo.amount goes positive. Pins pickup_and_deposit.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_cargo};
use crate::config::*;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "forager_picks_up_food",
        description: "A worker on the surface walks to a food pellet and picks it up within 6s.",
        seed: 101,
        timeout_seconds: 6.0,
        setup,
        predicate,
        failure_hint: "pickup_and_deposit isn't claiming food onto adjacent workers",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Place the worker and food adjacent on the surface walkway. The
    // pickup logic claims any food within ±1 tile of an empty-cargo
    // worker — adjacency is enough.
    let wx = COLONY_X + 3;
    let fx = COLONY_X + 4;
    let y  = SURFACE_ROW - 1;

    s.spawn_worker(wx, y, None);
    s.spawn_food(fx, y);
}

fn predicate(world: &World) -> bool {
    match subject_cargo(world, 0) {
        Some(c) => c.amount > 0,
        None    => false,
    }
}
