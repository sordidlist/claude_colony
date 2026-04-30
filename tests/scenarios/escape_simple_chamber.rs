//! A worker carrying dirt is dropped into a small chamber connected to
//! the surface by a single straight vertical shaft. Verifies the worker
//! climbs out under its own AI within a generous time budget. This is
//! the easy case — if it fails, basic step_return + auto-climb is
//! broken.

use crate::scenarios::Scenario;
use colony::config::*;
use colony::world::TileType;

#[test]
fn worker_climbs_out_of_simple_chamber() {
    let mut s = Scenario::new(7);
    s.clear_creatures();

    // Carve a 6-wide × 4-tall chamber a short distance below the surface,
    // and a vertical shaft connecting it straight up to the entrance.
    let chamber_x0 = COLONY_X - 3;
    let chamber_x1 = COLONY_X + 3;
    let chamber_y0 = COLONY_Y + 6;
    let chamber_y1 = COLONY_Y + 9;
    s.fill_rect(chamber_x0, chamber_y0, chamber_x1, chamber_y1, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, chamber_y0, 1)
     .mark_dirty()
     .reveal_all();

    // Worker stands on the chamber floor, carrying a soil pebble.
    let worker = s.spawn_worker(COLONY_X, chamber_y1 - 1, Some(TileType::Soil));

    // Goal: worker reaches a tile on or above the surface row within 12s.
    let elapsed = s.run_until(12.0, |world| {
        if let Some(p) = world.get::<colony::sim::components::Position>(worker) {
            (p.0.y as i32) <= SURFACE_ROW
        } else { false }
    });

    match elapsed {
        Ok(t)  => println!("escape_simple_chamber: surfaced in {:.2}s", t),
        Err(t) => panic!("worker failed to climb out of a straight shaft within {:.1}s — \
                          step_return / auto-climb regression", t),
    }
}
