//! The mower's wheel should clip the grass length to zero in every
//! column it rolls through. Pins the grass-mowing hook in
//! `animate_scenery`'s mower branch.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::GrassField;
use crate::sim::scenery::{Decoration, DecorKind, DecorPos};

const ROW_Y:        i32 = SURFACE_ROW - 2;
const SPAWN_X:      i32 = COLONY_X + 30;
const PROBE_X:      i32 = COLONY_X + 25;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "mower_shortens_grass",
        description: "A mower rolling through a tall-grass column clips it back to zero.",
        seed: 707,
        timeout_seconds: 10.0,
        setup,
        predicate,
        failure_hint: "mower isn't calling GrassField::mow on its foot column",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Pre-grow the lawn to its maximum so we can tell mowing
    // happened — the predicate checks that PROBE_X went from
    // GRASS_LENGTH_MAX down to 0.
    {
        let mut grass = s.app.world.resource_mut::<GrassField>();
        for cell in grass.length.iter_mut() {
            *cell = GRASS_LENGTH_MAX;
        }
    }

    // Spawn the mower a few tiles east of the probe column, moving
    // west so it'll cross PROBE_X within a couple seconds.
    s.app.world.spawn((
        DecorPos { x: SPAWN_X as f32, y: ROW_Y as f32 },
        Decoration { kind: DecorKind::Mower, frame: 0, anim_t: 0.0,
                     vx: -MOWER_SPEED, flip_x: true, last_col: -1 },
    ));
}

fn predicate(world: &World) -> bool {
    let grass = world.resource::<GrassField>();
    grass.at(PROBE_X) == 0
}
