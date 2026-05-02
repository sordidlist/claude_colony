//! Pins the force-drop escape hatch on `step_deposit_debris`: a
//! hauler that has tried `MAX_HAUL_ATTEMPTS` direction-flips
//! without depositing must drop the pebble (or clear cargo) and
//! return to normal mode. Without this, the live game develops
//! mature dirt mounds that gridlock the entire workforce above
//! ground — they oscillate forever between mound shoulders and
//! never go back underground to encounter spiders.
//!
//! Setup: place the worker on a narrow surface ledge wedged
//! between solid soil walls so neither direction has a valid drop
//! site, mark its cargo, and crank `haul_attempts` close to the
//! cap so the test resolves quickly.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, builder::subject_cargo};
use crate::config::*;
use crate::world::{TileGrid, TileType};
use crate::scenarios::TestSubject;
use crate::sim::components::WorkerBrain;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "hauler_unsticks_after_attempts",
        description: "A hauler trapped between mounds clears its cargo within a few seconds.",
        seed: 4711,
        // 4 flip attempts × 1.5s/flip ≈ 6s once we start at attempt 0;
        // give 8s of slack.
        timeout_seconds: 8.0,
        setup,
        predicate,
        failure_hint: "haul_attempts force-drop didn't fire — workforce will gridlock above ground",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    // Wall the surface immediately to either side of a single
    // walkable tile so the hauler can't make horizontal progress
    // and can't find an Air drop column. Force-drop has to fire.
    let trap_x = COLONY_X + 8;
    let trap_y = SURFACE_ROW - 1;
    {
        let mut g = s.app.world.resource_mut::<TileGrid>();
        // The worker's own air tile.
        g.set(trap_x, trap_y, TileType::Air);
        // Soil walls one step away in both directions, capped above
        // so auto-climb can't escape upward.
        for dx in [-1i32, 1] {
            for dy in -1..=1 {
                g.set(trap_x + dx, trap_y + dy, TileType::Soil);
            }
        }
        // Block the cell above the worker too so they can't auto-climb.
        g.set(trap_x, trap_y - 1, TileType::Soil);
        g.dirty = true;
    }

    let w = s.spawn_worker_tagged(trap_x, trap_y, Some(TileType::Soil), 0);
    if let Some(mut b) = s.app.world.get_mut::<WorkerBrain>(w) {
        // Pre-arm the failure path so we don't have to sit through
        // the full 6 s of stuck-time accumulation. We start one
        // attempt before the cap; the very first stuck-flip cycle
        // tips it over and triggers the force-drop.
        b.haul_attempts = MAX_HAUL_ATTEMPTS.saturating_sub(1);
        b.haul_direction = 1;
        b.haul_target_dist = 12;
    }
}

fn predicate(world: &World) -> bool {
    // Force-drop sets cargo.debris = None. That's our success signal.
    match subject_cargo(world, 0) {
        Some(c) => c.debris.is_none(),
        None    => false,
    }
}

#[allow(dead_code)]
fn _ensure_subject_used() {
    let _: Option<TestSubject> = None;
}
