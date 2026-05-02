//! Queen behaviour: lays eggs on a timer, and periodically migrates to
//! the deepest reachable chamber as workers extend the colony.
//! "Deepest reachable" is computed as a 4-connected flood fill from
//! the entrance, so a procgen pocket workers can't actually walk to
//! never tempts the queen into stranding herself.

use bevy_ecs::prelude::*;
use glam::Vec2;
use std::collections::VecDeque;
use crate::config::*;
use crate::world::TileGrid;
use super::components::*;
use super::{Time, EventLog};

/// 4-connected flood-fill from the colony entrance, returning the
/// deepest reachable standable tile (centred at .5 offsets so a
/// `Position` can use it directly). Public so `app.rs` can call it
/// at startup and the migration system can call it periodically.
pub fn find_queen_spot(grid: &TileGrid) -> (f32, f32) {
    let w = grid.width;
    let h = grid.height;
    let mut visited = vec![false; (w * h) as usize];
    let mut queue   = VecDeque::new();

    if grid.in_bounds(COLONY_X, COLONY_Y) && grid.passable(COLONY_X, COLONY_Y) {
        visited[grid.idx(COLONY_X, COLONY_Y)] = true;
        queue.push_back((COLONY_X, COLONY_Y));
    }

    // Prefer the deepest tile, then (tie-breaking) the one closest to
    // the entrance's vertical axis — keeps the queen in a central
    // chamber rather than at the end of a side branch.
    let mut best: Option<(i32, i32)> = None;
    while let Some((x, y)) = queue.pop_front() {
        let standable = y > COLONY_Y && grid.get(x, y + 1).solid();
        if standable {
            let better = match best {
                None => true,
                Some((bx, by)) => {
                    if y != by { y > by }
                    else { (x - COLONY_X).abs() < (bx - COLONY_X).abs() }
                }
            };
            if better { best = Some((x, y)); }
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = x + dx; let ny = y + dy;
            if !grid.in_bounds(nx, ny) { continue; }
            let i = grid.idx(nx, ny);
            if visited[i] || !grid.passable(nx, ny) { continue; }
            visited[i] = true;
            queue.push_back((nx, ny));
        }
    }

    if let Some((x, y)) = best {
        (x as f32 + 0.5, y as f32 + 0.5)
    } else {
        (COLONY_X as f32 + 0.5, COLONY_Y as f32 + 0.5)
    }
}

pub fn queen_tick(
    time: Res<Time>,
    balance: Res<crate::sim::BalanceTunables>,
    mut log: ResMut<EventLog>,
    mut queens: Query<(&Position, &mut QueenState, &Ant, &mut AiTrace)>,
    mut commands: Commands,
) {
    if time.dt <= 0.0 { return; }
    let interval = balance.queen_egg_interval.max(0.1);
    for (pos, mut state, ant, mut trace) in queens.iter_mut() {
        if ant.kind != AntKind::Queen { continue; }
        state.egg_timer += time.dt;
        while state.egg_timer >= interval {
            state.egg_timer -= interval;
            state.eggs_laid += 1;
            // Every 7th egg becomes a soldier — keeps the colony defended
            // without overrunning the worker caste.
            let will_be_soldier = state.eggs_laid % 7 == 0;
            commands.spawn((
                Position(pos.0),
                Velocity(Vec2::ZERO),
                Brood { timer: BROOD_MATURE_S, will_be_soldier },
            ));
            trace.record(time.total,
                format!("Egg #{} laid ({})", state.eggs_laid,
                    if will_be_soldier { "soldier" } else { "worker" }));
            if state.eggs_laid % 5 == 1 {
                log.push(format!("Queen laid an egg ({} total)", state.eggs_laid),
                         [0.78, 0.46, 0.94, 1.0]);
            }
        }
    }
    let _ = Vec2::ZERO;
}

/// Periodically check whether the colony has been dug deep enough that
/// a meaningfully safer chamber now exists, and relocate the queen if
/// so. The check is rate-limited to `QUEEN_MIGRATION_INTERVAL_S` so
/// we're not running a flood-fill every frame, and the move only
/// happens when the new spot is at least
/// `QUEEN_MIGRATION_MIN_DEPTH_GAIN` rows deeper than her current
/// position — without that threshold she'd hop between two adjacent
/// chambers any time the BFS tie-broke differently.
pub fn queen_migration(
    time: Res<Time>,
    grid: Res<TileGrid>,
    mut log: ResMut<EventLog>,
    mut queens: Query<(&mut Position, &mut QueenState, &mut AiTrace, &Ant)>,
) {
    if time.dt <= 0.0 { return; }
    for (mut pos, mut state, mut trace, ant) in queens.iter_mut() {
        if ant.kind != AntKind::Queen { continue; }

        // Tick down the inter-check timer.
        if state.migration_timer > 0.0 {
            state.migration_timer -= time.dt;
            if state.migration_timer > 0.0 { continue; }
        }
        state.migration_timer = QUEEN_MIGRATION_INTERVAL_S;

        let (nx, ny) = find_queen_spot(&grid);
        let cur_y    = pos.0.y as i32;
        let new_y    = ny as i32;
        if new_y - cur_y >= QUEEN_MIGRATION_MIN_DEPTH_GAIN {
            let (cur_x, cur_y_orig) = (pos.0.x, pos.0.y);
            pos.0.x = nx;
            pos.0.y = ny;
            state.migrations = state.migrations.saturating_add(1);
            trace.record(
                time.total,
                format!("Migrated to deeper chamber ({:.0},{:.0}) → ({:.0},{:.0})",
                        cur_x, cur_y_orig, nx, ny));
            log.push(
                format!("The queen moves to a deeper chamber (migration #{})",
                        state.migrations),
                [0.78, 0.46, 0.94, 1.0]);
        }
    }
}
