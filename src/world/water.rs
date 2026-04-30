//! Vectorised water cellular-automaton stub. Real CA logic lands in a follow-up;
//! for now the grid exists so dependent systems compile and overlays render.

use bevy_ecs::prelude::Resource;

#[derive(Resource)]
pub struct WaterGrid {
    pub width:  i32,
    pub height: i32,
    pub level:  Vec<f32>,
    pub step_accum: f32,
}

impl WaterGrid {
    pub fn new(width: i32, height: i32) -> Self {
        let n = (width * height) as usize;
        Self { width, height, level: vec![0.0; n], step_accum: 0.0 }
    }

    #[inline] pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    #[inline] pub fn get(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height { return 0.0; }
        self.level[self.idx(x, y)]
    }
}
