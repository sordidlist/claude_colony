//! The lawn should slowly grow shaggier over sim time. After a few
//! `GRASS_GROW_INTERVAL_S` ticks every column's `GrassField.length`
//! must have advanced beyond its initial value. Pins the
//! `world::grass::grow_grass` system in the schedule and the
//! `GrassField` resource construction.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::GrassField;
use crate::sim::scenery::{MowerSchedule, MowerPhase};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "grass_grows_over_time",
        description: "Grass length advances on every column after a few growth ticks.",
        seed: 5050,
        // Need at least 2 growth ticks (10s) to see length climb past
        // the initial value of 1.
        timeout_seconds: GRASS_GROW_INTERVAL_S * 3.0 + 1.0,
        setup,
        predicate,
        failure_hint: "grow_grass system isn't ticking — lawn stays flat forever",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();
    // Park the mower for the duration so it doesn't shave the lawn
    // while we're trying to confirm grass growth across every column.
    s.app.world.resource_mut::<MowerSchedule>().phase =
        MowerPhase::Cooldown(9_999.0);
}

fn predicate(world: &World) -> bool {
    let grass = world.resource::<GrassField>();
    grass.length.iter().all(|&l| l >= 3)
}
