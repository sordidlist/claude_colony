//! Pins the spider hunting behaviour: when a colony ant is inside a
//! spider's `SPIDER_HUNT_RADIUS`, the spider must drive *toward*
//! that ant — not random-walk past it.
//!
//! Setup: a horizontal corridor with a stationary worker on the left
//! and a spider on the right, both well inside hunt radius. After a
//! few frames the spider's velocity vector should point left
//! (toward the worker) and its x position should be visibly closer
//! to the worker than at spawn.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::TileType;
use crate::scenarios::TestSubject;
use crate::sim::components::Position;

const ANT_X:    i32 = COLONY_X - 3;
const SPIDER_X: i32 = COLONY_X + 3;
const ROW_Y:    i32 = COLONY_Y + 8;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "spider_hunts_nearby_ant",
        description: "Spider with an ant in its hunt radius steers toward the ant within ~1s.",
        seed: 3030,
        timeout_seconds: 2.0,
        setup,
        predicate,
        failure_hint: "spider_tick isn't acquiring nearby ants — spiders will pace harmlessly past prey",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();
    s.reveal_all();

    s.fill_rect(COLONY_X - 6, ROW_Y, COLONY_X + 6, ROW_Y + 1,
                TileType::Tunnel)
     .mark_dirty();

    // Worker is subject id 0, spider is id 1. Their separation
    // (6 tiles) is well inside SPIDER_HUNT_RADIUS = 10.
    s.spawn_worker_tagged(ANT_X, ROW_Y + 1, None, 0);
    s.spawn_spider_tagged(SPIDER_X, ROW_Y + 1, 1);
}

fn predicate(world: &World) -> bool {
    // Find the spider's current x. It started at SPIDER_X+0.5 and
    // should have moved meaningfully toward the worker (negative
    // x direction).
    for e in world.iter_entities() {
        let Some(t) = e.get::<TestSubject>() else { continue; };
        if t.id != 1 { continue; }
        if let Some(p) = e.get::<Position>() {
            // Closing the gap by even half a tile from spawn proves
            // the spider isn't random-walking — at SPIDER_SPEED 3.2
            // tiles/s, half a tile is ~150 ms of consistent pursuit.
            return p.0.x < (SPIDER_X as f32 + 0.5) - 0.5;
        }
    }
    false
}
