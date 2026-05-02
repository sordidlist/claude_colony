//! Combat balance pin (other side): four workers should reliably take
//! down a single spider, even though one or two of them may die in
//! the process. The buffed spider profile is intentionally strong
//! enough that two or three ants is *not* enough — four is the
//! threshold the colony's swarm response is sized for.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_alive};
use crate::config::*;
use crate::world::TileType;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "four_workers_kill_spider",
        description: "Four workers swarm a single spider and kill it (some may fall in the fight).",
        seed: 207,
        timeout_seconds: 25.0,
        setup,
        predicate,
        failure_hint: "four-worker swarm can't kill a spider — combat balance regression",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    let cx = COLONY_X;
    let cy = COLONY_Y + 8;
    s.fill_rect(cx - 6, cy, cx + 6, cy + 3, TileType::Tunnel)
     .carve_vertical(COLONY_X, COLONY_Y, cy, 1)
     .mark_dirty()
     .rebuild_flow_field()
     .reveal_all();

    // Spider is subject id 0 — predicate watches for its death.
    s.spawn_spider_tagged(cx, cy + 2, 0);
    // Four workers placed close enough that all can reach the spider
    // simultaneously. With four ants in melee, the swarm DPS exceeds
    // the spider's per-ant kill rate and the colony wins (with
    // probable casualties).
    for (i, dx) in [-2i32, -1, 1, 2].iter().enumerate() {
        s.spawn_worker_tagged(cx + dx, cy + 2, None, (i + 1) as u8);
    }
}

fn predicate(world: &World) -> bool {
    !subject_alive(world, 0)
}
