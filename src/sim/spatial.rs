//! Uniform spatial hash. Rebuilt every frame in O(n). Queries return
//! candidate entity ids in a 3×3 cell neighbourhood — no full-N scans.

use bevy_ecs::prelude::*;
use crate::config::*;
use super::components::Position;

#[derive(Resource)]
pub struct SpatialGrid {
    pub cols: i32,
    pub rows: i32,
    /// One bucket per cell. We push entity ids in `rebuild` and clear at the
    /// start of the next frame; the Vec capacity is preserved across frames so
    /// allocations are amortised.
    buckets: Vec<Vec<Entity>>,
}

impl SpatialGrid {
    pub fn new() -> Self {
        let cols = (WORLD_WIDTH  + SPATIAL_CELL - 1) / SPATIAL_CELL;
        let rows = (WORLD_HEIGHT + SPATIAL_CELL - 1) / SPATIAL_CELL;
        let n = (cols * rows) as usize;
        Self { cols, rows, buckets: vec![Vec::new(); n] }
    }

    #[inline] fn cell_of(&self, tx: i32, ty: i32) -> Option<usize> {
        let cx = tx / SPATIAL_CELL;
        let cy = ty / SPATIAL_CELL;
        if cx < 0 || cy < 0 || cx >= self.cols || cy >= self.rows { return None; }
        Some((cy * self.cols + cx) as usize)
    }

    pub fn clear(&mut self) {
        for b in self.buckets.iter_mut() { b.clear(); }
    }

    pub fn insert(&mut self, e: Entity, tx: i32, ty: i32) {
        if let Some(i) = self.cell_of(tx, ty) {
            self.buckets[i].push(e);
        }
    }

    /// Visit every entity within `radius_cells` cells of (tx, ty).
    pub fn query<F: FnMut(Entity)>(&self, tx: i32, ty: i32, radius_cells: i32, mut f: F) {
        let cx = tx / SPATIAL_CELL;
        let cy = ty / SPATIAL_CELL;
        let r  = radius_cells.max(0);
        for dy in -r..=r {
            for dx in -r..=r {
                let nx = cx + dx; let ny = cy + dy;
                if nx < 0 || ny < 0 || nx >= self.cols || ny >= self.rows { continue; }
                for e in &self.buckets[(ny * self.cols + nx) as usize] {
                    f(*e);
                }
            }
        }
    }
}

/// System: clear & re-insert all positioned entities into the spatial grid.
pub fn rebuild_spatial(
    mut grid: ResMut<SpatialGrid>,
    q: Query<(Entity, &Position)>,
) {
    grid.clear();
    for (e, p) in q.iter() {
        grid.insert(e, p.0.x as i32, p.0.y as i32);
    }
}
