//! After enough sim time, the off-screen invader spawner should
//! deposit at least one rival ant or spider near the world edges at
//! the surface row. Pins the periodic-arrival mechanic that
//! replaces the old underground spider seeding.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::sim::components::{Spider, RivalAnt, Position};

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "invaders_arrive_from_offscreen",
        description: "A rival or spider spawns at a world edge after the first invader interval.",
        seed: 123_456,
        // First spawn fires at INVADER_FIRST_SPAWN_S; allow a few
        // seconds of slack on top so jitter doesn't false-fail.
        timeout_seconds: INVADER_FIRST_SPAWN_S + 30.0,
        setup,
        predicate,
        failure_hint: "InvaderSpawner / spawn_invaders system isn't producing edge arrivals",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();
}

fn predicate(world: &World) -> bool {
    // Look for any Spider or RivalAnt sitting on or near the surface
    // row at one of the world edges (within 6 tiles of x=0 or
    // x=WORLD_WIDTH-1). If we find one, the spawner fired.
    for e in world.iter_entities() {
        if !(e.contains::<Spider>() || e.contains::<RivalAnt>()) { continue; }
        let Some(p) = e.get::<Position>() else { continue; };
        let near_surface = (p.0.y as i32) <= SURFACE_ROW + 1;
        let near_edge    = (p.0.x as i32) < 8
                        || (p.0.x as i32) > WORLD_WIDTH - 8;
        if near_surface && near_edge { return true; }
    }
    false
}
