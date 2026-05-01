//! A worker carrying dirt starts in a side chamber connected to the
//! entrance only via a winding tunnel — the kind of layout that pure
//! direct-line steering chokes on. Verifies that step_return + auto-
//! climb together can navigate non-straight paths. Pins the value of
//! the BFS-based ReturnFlowField.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_pos};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "escape_winding_tunnel",
        description: "A worker with dirt navigates two right-angle bends back to the surface.",
        seed: 13,
        timeout_seconds: 20.0,
        setup,
        predicate,
        failure_hint: "step_return doesn't navigate non-straight paths — likely a flow-field regression",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let shaft_x = COLONY_X;
    let bend1_y = COLONY_Y + 8;
    let mid_x   = COLONY_X - 12;
    let bend2_y = COLONY_Y + 12;
    let final_x = COLONY_X - 20;
    let final_y = COLONY_Y + 12;

    s.carve_vertical(shaft_x, COLONY_Y, bend1_y, 1)
     .carve_horizontal(mid_x,   shaft_x, bend1_y, 2)
     .carve_vertical(mid_x,    bend1_y, bend2_y, 1)
     .carve_horizontal(final_x, mid_x,   bend2_y, 2)
     .fill_rect(final_x - 3, final_y - 1, final_x + 1, final_y + 1, TileType::Chamber)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    s.spawn_worker(final_x - 1, final_y, Some(TileType::Soil));
}

fn predicate(world: &World) -> bool {
    if let Some(p) = subject_pos(world, 0) {
        (p.y as i32) <= SURFACE_ROW
    } else { false }
}
