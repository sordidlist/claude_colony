//! Once a worker is on the surface with a debris pebble, they should
//! drop it somewhere outside the colony entrance corridor within a
//! reasonable amount of sim time. Pins down the surface-side of the
//! haul cycle (independent of underground escape).

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_cargo};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "hauler_drops_outside",
        description: "A surface worker carrying dirt drops it outside the entrance within 15s.",
        seed: 21,
        timeout_seconds: 15.0,
        setup,
        predicate,
        failure_hint: "step_deposit_debris isn't completing the haul cycle on the surface",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    let start_x = COLONY_X + 1;
    let start_y = SURFACE_ROW - 1;
    s.spawn_worker(start_x, start_y, Some(TileType::Soil));
}

fn predicate(world: &World) -> bool {
    match subject_cargo(world, 0) {
        Some(c) => c.debris.is_none(),
        None    => false,
    }
}
