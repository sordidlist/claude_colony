//! Return-to-entrance flow field.
//!
//! BFS once from the colony entrance through every passable tile and
//! store, at each tile, the single tile-step direction toward the
//! entrance along the shortest-passable path. Workers heading home then
//! just look up their tile and take that step — no per-ant pathfinding,
//! no direct-line steering, robust to arbitrarily winding tunnels.
//!
//! Cost: one BFS over ≤ width × height ≈ 64 K cells when the world
//! changes; per-ant lookup is a single index. Rebuilding every couple of
//! seconds keeps the field current as the colony digs new tunnels.

use bevy_ecs::prelude::*;
use std::collections::VecDeque;
use crate::config::*;
use super::tiles::TileGrid;
use crate::sim::Time;

#[derive(Resource)]
pub struct ReturnFlowField {
    pub width:  i32,
    pub height: i32,
    /// `directions[idx(x, y)] = (dx, dy)` — the tile-step from (x, y)
    /// that takes you one step toward the entrance. (0, 0) means "you
    /// are at the entrance, or this tile isn't reachable from it."
    directions:    Vec<(i8, i8)>,
    visited:       Vec<bool>,
    rebuild_accum: f32,
}

impl ReturnFlowField {
    pub fn new(width: i32, height: i32) -> Self {
        let n = (width * height) as usize;
        Self {
            width, height,
            directions:    vec![(0, 0); n],
            visited:       vec![false; n],
            rebuild_accum: 0.0,
        }
    }

    #[inline] fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// Step direction at (x, y) toward the entrance. (0, 0) for tiles
    /// outside the field, unreachable, or at the entrance itself.
    pub fn step(&self, x: i32, y: i32) -> (i32, i32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return (0, 0);
        }
        let (dx, dy) = self.directions[self.idx(x, y)];
        (dx as i32, dy as i32)
    }

    pub fn rebuild(&mut self, grid: &TileGrid) {
        for d in self.directions.iter_mut() { *d = (0, 0); }
        for v in self.visited.iter_mut()    { *v = false; }

        let entrance = (COLONY_X, COLONY_Y);
        if entrance.0 < 0 || entrance.0 >= self.width
            || entrance.1 < 0 || entrance.1 >= self.height
            || !grid.passable(entrance.0, entrance.1)
        {
            return;
        }
        let entrance_idx = self.idx(entrance.0, entrance.1);
        self.visited[entrance_idx] = true;

        let mut queue: VecDeque<(i32, i32)> = VecDeque::with_capacity(256);
        queue.push_back(entrance);

        while let Some((cx, cy)) = queue.pop_front() {
            for (dx, dy) in [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 || nx >= self.width || ny >= self.height {
                    continue;
                }
                if !grid.passable(nx, ny) { continue; }
                let i = self.idx(nx, ny);
                if self.visited[i] { continue; }
                self.visited[i] = true;
                // Direction back toward parent: (cx-nx, cy-ny) = (-dx, -dy)
                self.directions[i] = (-dx as i8, -dy as i8);
                queue.push_back((nx, ny));
            }
        }
    }
}

const REBUILD_INTERVAL_S: f32 = 2.0;

pub fn maintain_flow_field(
    mut field: ResMut<ReturnFlowField>,
    grid:      Res<TileGrid>,
    time:      Res<Time>,
) {
    field.rebuild_accum += time.dt;
    if field.rebuild_accum < REBUILD_INTERVAL_S { return; }
    field.rebuild_accum = 0.0;
    field.rebuild(&grid);
}
