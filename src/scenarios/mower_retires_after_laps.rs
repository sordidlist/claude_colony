//! Pins the mower lifecycle: after the configured number of patrol-
//! bound traversals, the mower must despawn and the schedule must
//! enter cooldown. We accelerate the test by overwriting the
//! `MowerSchedule` to a single lap and parking the mower one tile
//! shy of its east bound, so the first bounce decrements laps to 0
//! and the lifecycle system retires it.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::sim::scenery::{Decoration, DecorKind, DecorPos,
                          MowerSchedule, MowerPhase};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "mower_retires_after_laps",
        description: "Mower retires + enters cooldown once its laps_remaining hits zero.",
        seed: 4242,
        timeout_seconds: 6.0,
        setup,
        predicate,
        failure_hint: "mower_lifecycle isn't transitioning Active(0) → Cooldown — lifecycle wiring broken",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Park the mower 1 tile shy of its east bound, moving east.
    // First sub-frame of motion crosses the bound and triggers the
    // lap decrement.
    let east_bound = WORLD_WIDTH as f32 - MOWER_PATROL_RIGHT_MARGIN;
    let spawn_x = east_bound - 1.0;
    s.app.world.spawn((
        DecorPos { x: spawn_x, y: (SURFACE_ROW - 2) as f32 },
        Decoration { kind: DecorKind::Mower, frame: 0, anim_t: 0.0,
                     vx: MOWER_SPEED, flip_x: false, last_col: -1 },
    ));

    // Force a 1-lap budget so the test resolves in seconds, not the
    // multi-minute patrol the production cadence implies.
    s.app.world.resource_mut::<MowerSchedule>().phase = MowerPhase::Active(1);
}

fn predicate(world: &World) -> bool {
    matches!(
        world.resource::<MowerSchedule>().phase,
        MowerPhase::Cooldown(_)
    )
}
