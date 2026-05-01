//! A Brood entity with a short timer should mature into a worker
//! within (BROOD_MATURE_S + small slack) seconds. Pins the
//! mature_brood pipeline.
//!
//! We don't tag the brood as a TestSubject because the brood entity
//! gets despawned at hatch — instead we just count workers in the
//! world before and after.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::TileType;
use crate::sim::components::{Ant, AntKind};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "brood_hatches_to_worker",
        description: "A short-timer brood entity matures into a worker within the maturation window.",
        seed: 88,
        // mature_in is set to 1.0 below; allow a couple seconds for
        // the schedule to actually fire.
        timeout_seconds: 4.0,
        setup,
        predicate,
        failure_hint: "mature_brood isn't hatching brood into workers — reproduction pipeline broken",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let cx = COLONY_X;
    let cy = COLONY_Y + 5;
    s.fill_rect(cx - 3, cy, cx + 3, cy + 2, TileType::Chamber)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    // 1-second timer keeps the test fast. Use the worker variant.
    s.spawn_brood(cx, cy + 1, 1.0, false);
}

fn predicate(world: &World) -> bool {
    // Workers exist iff at least one Ant.kind == Worker.
    world.iter_entities().any(|e| {
        e.get::<Ant>().map(|a| matches!(a.kind, AntKind::Worker)).unwrap_or(false)
    })
}
