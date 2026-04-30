//! A worker carrying dirt starts in a side chamber connected to the
//! entrance only via a winding tunnel — the kind of layout that pure
//! direct-line steering chokes on. Verifies that step_return + auto-
//! climb together can navigate non-straight paths.

use crate::scenarios::Scenario;
use colony::config::*;
use colony::world::TileType;

#[test]
fn worker_climbs_out_of_winding_tunnel() {
    let mut s = Scenario::new(13);
    s.clear_creatures();

    // Layout (rows = depth below SURFACE_ROW):
    //   row+1: vertical shaft from entrance straight down
    //   row+8: shaft turns LEFT through a horizontal tunnel for 12 tiles
    //   then turns DOWN again for 4 tiles
    //   then turns LEFT for 8 more tiles
    //   ending in a small 4x3 chamber where the worker starts
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
     .reveal_all();

    let worker = s.spawn_worker(final_x - 1, final_y, Some(TileType::Soil));

    let elapsed = s.run_until(20.0, |world| {
        if let Some(p) = world.get::<colony::sim::components::Position>(worker) {
            (p.0.y as i32) <= SURFACE_ROW
        } else { false }
    });

    match elapsed {
        Ok(t)  => println!("escape_winding_tunnel: surfaced in {:.2}s", t),
        Err(t) => panic!("worker stuck in winding tunnel after {:.1}s — \
                          step_return doesn't navigate non-straight paths. \
                          Likely needs pheromone trail or BFS pathfinding.", t),
    }
}
