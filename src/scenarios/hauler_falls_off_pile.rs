//! Regression test for the "ants float in the sky after walking off a
//! tall pile" bug. step_deposit_debris snaps a hauler upward onto
//! the topmost walkable tile in its column so it can climb on top of
//! existing dirt mounds. When it then walks off the back side of the
//! pile, it must fall. A previous version of the AI was zeroing
//! vel.y every frame, which fought gravity hard enough that ants
//! floated at the pile's height instead of falling down to the lower
//! ground beyond.
//!
//! Setup: a 6-tile-tall pre-built dirt pile at column X with a single
//! flat surface to the right. Spawn a hauler on top of the pile with
//! cargo, walking right (away from the entrance). After a few seconds
//! the worker must have descended to the natural ground level.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_pos};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "hauler_falls_off_pile",
        description: "A hauler walking off the back of a tall pile actually falls to the ground.",
        seed: 314,
        timeout_seconds: 8.0,
        setup,
        predicate,
        failure_hint: "step_deposit_debris is fighting gravity — ants will float above piles",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Build a tall pile of soil 8 columns to the right of the entrance.
    // Use rows above SURFACE_ROW (smaller y = higher up). Pile occupies
    // a single column so the worker has clear flat ground beyond it.
    let pile_x = COLONY_X + 8;
    let pile_top_y = SURFACE_ROW - 6;
    for y in pile_top_y..SURFACE_ROW {
        let mut g = s.app.world.resource_mut::<crate::world::TileGrid>();
        g.set(pile_x, y, TileType::Soil);
    }
    s.mark_dirty();

    // Worker spawns on top of the pile with cargo, facing further right
    // (away from the entrance). step_deposit_debris will keep walking
    // it outward; when it crosses the right edge of the pile it must
    // fall to ground.
    let w = s.spawn_worker(pile_x, pile_top_y - 1, Some(TileType::Soil));

    // Push it past the snap so it has to actually move forward.
    if let Some(mut brain) = s.app.world.get_mut::<crate::sim::components::WorkerBrain>(w) {
        brain.haul_direction   = 1;
        brain.haul_target_dist = 12;
    }
}

fn predicate(world: &World) -> bool {
    let Some(p) = subject_pos(world, 0) else { return false; };
    // Must have moved to the right of the pile centre AND landed back
    // at the natural ground level (within 1 tile of SURFACE_ROW - 1).
    let on_ground = ((p.y as i32) - (SURFACE_ROW - 1)).abs() <= 1;
    let past_pile = p.x >= (COLONY_X + 9) as f32;
    on_ground && past_pile
}
