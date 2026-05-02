//! The dig director must not queue a second surface hole closer
//! than `MIN_SURFACE_HOLE_SPACING` to an existing one. Pins the
//! grass-row spacing rule in `surface_width_ok` so the lawn ends
//! up with discrete hill-with-tunnel openings rather than a
//! single chasm.
//!
//! Setup: world boots with the procgen colony shaft (one entrance
//! at COLONY_X). We hand-carve a vertical spike of tunnel reaching
//! up to row `SURFACE_ROW + 1` at a column 4 tiles offset from
//! the entrance — close enough that opening grass at that column
//! would violate the spacing rule. We then run the sim long enough
//! that the dig director would normally pick that grass tile as a
//! frontier candidate, and assert it never gets dug.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::{TileGrid, TileType};

const PROBE_X: i32 = COLONY_X + 4;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "surface_holes_keep_spacing",
        description: "A grass tile within MIN_SURFACE_HOLE_SPACING of an existing hole stays grass.",
        seed: 31415,
        timeout_seconds: EXPAND_INTERVAL * 14.0,
        setup,
        predicate,
        failure_hint: "surface_width_ok grass-row spacing rule isn't holding — the lawn will get pock-marked",
    }
}

fn setup(s: &mut Scenario) {
    s.reveal_all();
    // Carve a thin shaft of Tunnel from row SURFACE_ROW+1 down to a
    // chamber at row SURFACE_ROW+5, at PROBE_X. Row SURFACE_ROW
    // (the grass row) is left intact at PROBE_X — we want to see
    // whether the dig director will breach it.
    let mut g = s.app.world.resource_mut::<TileGrid>();
    for y in (SURFACE_ROW + 1)..=(SURFACE_ROW + 5) {
        g.set(PROBE_X, y, TileType::Tunnel);
    }
    // A small chamber under it so the tunnel terminates somewhere
    // sensible (otherwise it dead-ends with no passable adjacent).
    for dy in 0..=2 {
        for dx in -2..=2 {
            g.set(PROBE_X + dx, SURFACE_ROW + 5 + dy, TileType::Tunnel);
        }
    }
    g.dirty = true;
    drop(g);
    s.rebuild_flow_field();
}

fn predicate(world: &World) -> bool {
    let total = world.resource::<crate::sim::Time>().total;
    if total < EXPAND_INTERVAL * 12.0 { return false; }

    let g = world.resource::<TileGrid>();
    // The probe column should still be Grass at the surface — the
    // director shouldn't have queued (or workers shouldn't have
    // dug) a hole here, because PROBE_X is only 4 tiles from the
    // procgen entrance at COLONY_X (well inside
    // MIN_SURFACE_HOLE_SPACING of 14).
    let tile_at_probe = g.get(PROBE_X, SURFACE_ROW);
    if tile_at_probe != TileType::Grass {
        panic!(
            "surface_holes_keep_spacing: grass at ({}, {}) was breached \
             into {:?} despite being only {} tiles from the entrance \
             hole at {} (spacing rule = {}).",
            PROBE_X, SURFACE_ROW, tile_at_probe,
            (PROBE_X - COLONY_X).abs(), COLONY_X, MIN_SURFACE_HOLE_SPACING,
        );
    }
    true
}
