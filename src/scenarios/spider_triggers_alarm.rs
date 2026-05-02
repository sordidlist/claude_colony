//! A spider in the world should stamp the alarm pheromone channel
//! around itself every frame, broadcasting "danger here" to any
//! worker in pheromone-sensing range. Pins the
//! `hostile_alarm_emission` system: if it stops running or the
//! emission radius regresses, distant workers will never get the
//! alert and the colony stops responding to invasions.

use bevy_ecs::prelude::*;
use crate::scenarios::{Scenario, ScenarioDef};
use crate::config::*;
use crate::world::{TileType, PheromoneGrid, PheromoneChannel};

const SPIDER_X: i32 = COLONY_X + 4;
const SPIDER_Y: i32 = COLONY_Y + 8;

pub fn def() -> ScenarioDef {
    ScenarioDef {
        name: "spider_triggers_alarm",
        description: "Alarm pheromone around a spider rises above the trigger level within a frame.",
        seed: 1701,
        timeout_seconds: 2.0,
        setup,
        predicate,
        failure_hint: "hostile_alarm_emission isn't depositing — workers will never react to nearby spiders",
    }
}

fn setup(s: &mut Scenario) {
    s.clear_creatures();

    // Carve a chamber for the spider so it can't fall through unrelated
    // procgen geometry — the test cares only about pheromone, not motion.
    s.fill_rect(SPIDER_X - 2, SPIDER_Y - 1, SPIDER_X + 2, SPIDER_Y + 1,
                TileType::Tunnel)
     .mark_dirty()
     .reveal_all();

    // Spider is the test subject; nothing else needs to exist.
    s.spawn_spider_tagged(SPIDER_X, SPIDER_Y, 0);
}

fn predicate(world: &World) -> bool {
    let phero = world.resource::<PheromoneGrid>();
    // The emission stamps a `(2*ALARM_EMISSION_HALF_WIDTH+1)` square
    // around the spider every frame. After even one tick, the level
    // at the spider's tile and at the corner of the radius should be
    // above the worker's trigger level.
    let here   = phero.level(SPIDER_X, SPIDER_Y, PheromoneChannel::Alarm);
    let corner = phero.level(SPIDER_X + ALARM_EMISSION_HALF_WIDTH,
                             SPIDER_Y + ALARM_EMISSION_HALF_WIDTH,
                             PheromoneChannel::Alarm);
    here > ALARM_TRIGGER_LEVEL && corner > ALARM_TRIGGER_LEVEL
}
