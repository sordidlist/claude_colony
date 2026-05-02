//! A worker minding its own business — wandering, no cargo, no
//! current dig — should switch to FightBack mode within a frame or
//! two of a spider walking into its `WORKER_THREAT_RADIUS`. Pins the
//! direct-sight detection in `choose_mode`: if it regresses,
//! workers go back to ignoring spiders that haven't yet stained
//! enough alarm pheromone for the gradient path to fire.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::TileType;
use crate::scenarios::TestSubject;
use crate::sim::components::{WorkerBrain, WorkerMode};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "worker_engages_nearby_spider",
        description: "Worker switches to FightBack within ~1s of a spider entering its threat radius.",
        seed: 2024,
        // ANT_REPLAN_HZ is 1 Hz with up to ±0.3s jitter. Two seconds
        // gives the worker at least one replan tick to catch the
        // sight check.
        timeout_seconds: 2.0,
        setup,
        predicate,
        failure_hint: "choose_mode isn't picking up direct-sight hostiles within WORKER_THREAT_RADIUS",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    let cx = COLONY_X;
    let cy = COLONY_Y + 6;
    s.fill_rect(cx - 5, cy, cx + 5, cy + 1, TileType::Tunnel)
     .mark_dirty()
     .rebuild_flow_field();

    // Worker (test subject id 0) and spider placed within
    // WORKER_THREAT_RADIUS of each other. Worker is far enough away
    // that combat doesn't fire on frame 1 — the test is about
    // *detection*, not the eventual fight.
    s.spawn_worker_tagged(cx - 4, cy + 1, None, 0);
    s.spawn_spider_tagged(cx + 1, cy + 1, 1);
}

fn predicate(world: &World) -> bool {
    for e in world.iter_entities() {
        let Some(t) = e.get::<TestSubject>() else { continue; };
        if t.id != 0 { continue; }
        if let Some(b) = e.get::<WorkerBrain>() {
            if matches!(b.mode, WorkerMode::FightBack) { return true; }
        }
    }
    false
}
