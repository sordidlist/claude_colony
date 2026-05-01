//! The mower's blade radius kills any creature it overlaps. Spawn a
//! worker right under the mower's centre and verify it's gone within
//! a few frames.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_alive};
use crate::config::*;
use crate::sim::scenery::{Decoration, DecorKind, DecorPos};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "mower_kills_workers",
        description: "An ant standing under the mower's deck is killed within a couple frames.",
        seed: 909,
        timeout_seconds: 1.0,
        setup,
        predicate,
        failure_hint: "mower_lifecycle isn't running over creatures inside MOWER_KILL_RADIUS",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Drop a mower at a flat surface column away from the entrance,
    // then put a worker directly under its centre.
    let mx = (COLONY_X + 30) as f32;
    let my = (SURFACE_ROW - 2) as f32;
    s.app.world.spawn((
        DecorPos { x: mx, y: my },
        Decoration { kind: DecorKind::Mower, frame: 0, anim_t: 0.0,
                     vx: 0.0, flip_x: false, last_col: -1 },
    ));

    // Worker is the test subject — predicate watches for its death.
    let worker_x = (mx + 2.0) as i32;       // mower sprite centre
    let worker_y = (my + 1.0) as i32;
    s.spawn_worker_tagged(worker_x, worker_y, None, 0);
}

fn predicate(world: &World) -> bool {
    !subject_alive(world, 0)
}
