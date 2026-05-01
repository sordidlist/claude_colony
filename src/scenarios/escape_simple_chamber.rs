//! A worker carrying dirt is dropped into a small chamber connected to
//! the surface by a single straight vertical shaft. Verifies the worker
//! climbs out under its own AI within a generous time budget. The easy
//! case — if it fails, basic step_return + auto-climb is broken.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_pos};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "escape_simple_chamber",
        description: "A worker with dirt climbs out of a small chamber via one straight shaft.",
        seed: 7,
        timeout_seconds: 12.0,
        setup,
        predicate,
        failure_hint: "step_return / auto-climb regression — couldn't ascend a straight shaft",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let chamber_x0 = COLONY_X - 3;
    let chamber_x1 = COLONY_X + 3;
    let chamber_y0 = COLONY_Y + 6;
    let chamber_y1 = COLONY_Y + 9;
    s.fill_rect(chamber_x0, chamber_y0, chamber_x1, chamber_y1, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, chamber_y0, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    s.spawn_worker(COLONY_X, chamber_y1 - 1, Some(TileType::Soil));
}

fn predicate(world: &World) -> bool {
    if let Some(p) = subject_pos(world, 0) {
        (p.y as i32) <= SURFACE_ROW
    } else { false }
}
