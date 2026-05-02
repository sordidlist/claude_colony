//! A worker that's busy doing something else (in DepositDebris with
//! a cargo pebble) and is *not* due for a 1Hz replan must still
//! react to a spider that walks into its threat radius — within a
//! single frame. Pins the per-frame hostile reflex in `worker_ai`
//! that overrides the longer replan tick.
//!
//! Without that reflex, a busy hauler can pass within attack range
//! of a spider and ignore it for over a second; in live gameplay
//! that's how spiders accumulate untouched in the colony.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef, TestSubject};
use crate::config::*;
use crate::world::TileType;
use crate::sim::components::{WorkerBrain, WorkerMode};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "worker_per_frame_hostile_reflex",
        description: "A hauler with cargo flips to FightBack the same frame a spider enters its threat radius.",
        seed: 8127,
        // The reflex fires once `replan_in > 0` and we're not in
        // FightBack yet. Within a tenth of a sim-second the worker
        // should switch.
        timeout_seconds: 0.5,
        setup,
        predicate,
        failure_hint: "per-frame hostile reflex isn't firing — busy workers ignore spiders for a full replan",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    let cy = COLONY_Y + 6;
    s.fill_rect(COLONY_X - 6, cy, COLONY_X + 6, cy + 1, TileType::Tunnel)
     .mark_dirty();

    // Worker starts mid-cycle (carrying cargo, mode locked to
    // DepositDebris, replan timer pushed out to ensure the choose_mode
    // path can't fire). The reflex has to do the work.
    let w = s.spawn_worker_tagged(COLONY_X - 4, cy + 1, Some(TileType::Soil), 0);
    if let Some(mut b) = s.app.world.get_mut::<WorkerBrain>(w) {
        b.mode = WorkerMode::DepositDebris;
        b.replan_in = 0.95; // ~one full replan window away from re-evaluating
    }

    // Spider stationed within WORKER_THREAT_RADIUS — well inside
    // the ~8-tile sight check.
    s.spawn_spider_tagged(COLONY_X + 1, cy + 1, 1);
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
