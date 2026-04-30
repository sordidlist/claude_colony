//! Once a worker is on the surface with a debris pebble, they should
//! drop it somewhere outside the colony entrance corridor within a
//! reasonable amount of sim time. This pins down the surface-side of
//! the haul cycle (independent of underground escape).

use crate::scenarios::Scenario;
use colony::config::*;
use colony::world::TileType;

#[test]
fn surface_worker_drops_dirt_outside() {
    let mut s = Scenario::new(21);
    s.clear_creatures();
    s.reveal_all();

    // Spawn worker right next to the entrance, on the air tile above
    // grass, already carrying soil cargo.
    let start_x = COLONY_X + 1;
    let start_y = SURFACE_ROW - 1;
    let worker = s.spawn_worker(start_x, start_y, Some(TileType::Soil));

    let elapsed = s.run_until(15.0, |world| {
        // Drop succeeds when the worker no longer has debris.
        if let Some(c) = world.get::<colony::sim::components::Cargo>(worker) {
            c.debris.is_none()
        } else { false }
    });

    let t = match elapsed {
        Ok(t)  => t,
        Err(t) => panic!("surface worker carried dirt for {:.1}s without dropping — \
                          step_deposit_debris isn't completing the haul cycle", t),
    };
    println!("surface_worker_drops_dirt_outside: dropped in {:.2}s", t);

    // And the drop must have actually placed a tile somewhere outside
    // the entrance corridor (otherwise the worker just lost cargo).
    let g = s.app.world.resource::<colony::world::TileGrid>();
    let mut found = false;
    for y in 1..SURFACE_ROW {
        for x in 1..(g.width - 1) {
            if g.get(x, y).solid() {
                let dx = (x - COLONY_X).abs();
                if dx >= 2 {
                    found = true;
                    break;
                }
            }
        }
        if found { break; }
    }
    assert!(found, "drop completed but no soil tile appeared above ground outside the entrance");
}
