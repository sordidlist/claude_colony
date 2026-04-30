//! Queen behaviour: stationary, lays eggs on a timer that hatch directly into
//! workers (brood pipeline lands later). Pushes a ticker event every few eggs
//! so the player notices the colony growing on its own.

use bevy_ecs::prelude::*;
use glam::Vec2;
use crate::config::*;
use super::components::*;
use super::{Time, EventLog};

pub fn queen_tick(
    time: Res<Time>,
    mut log: ResMut<EventLog>,
    mut queens: Query<(&Position, &mut QueenState, &Ant)>,
    mut commands: Commands,
) {
    if time.dt <= 0.0 { return; }
    for (pos, mut state, ant) in queens.iter_mut() {
        if ant.kind != AntKind::Queen { continue; }
        state.egg_timer += time.dt;
        while state.egg_timer >= QUEEN_EGG_INTERVAL_S {
            state.egg_timer -= QUEEN_EGG_INTERVAL_S;
            state.eggs_laid += 1;
            // Every 7th egg becomes a soldier — keeps the colony defended
            // without overrunning the worker caste.
            let will_be_soldier = state.eggs_laid % 7 == 0;
            commands.spawn((
                Position(pos.0),
                Velocity(Vec2::ZERO),
                Brood { timer: BROOD_MATURE_S, will_be_soldier },
            ));
            if state.eggs_laid % 5 == 1 {
                log.push(format!("Queen laid an egg ({} total)", state.eggs_laid),
                         [0.78, 0.46, 0.94, 1.0]);
            }
        }
    }
    // velocity import unused here now — keep the import alive cheaply.
    let _ = Vec2::ZERO;
}
