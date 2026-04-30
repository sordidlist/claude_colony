//! Fog-of-war state. One byte per tile: 0 = unseen, 255 = revealed.
//!
//! Above-ground tiles (sky + surface row) are always considered explored —
//! the player should always be able to see what's happening on the surface.
//! Underground tiles only reveal when an ant has been near them.

use bevy_ecs::prelude::Resource;
use crate::config::*;

#[derive(Resource)]
pub struct ExploredGrid {
    pub width:  i32,
    pub height: i32,
    pub data:   Vec<u8>,
    pub dirty:  bool,
}

impl ExploredGrid {
    pub fn new(width: i32, height: i32) -> Self {
        let n = (width * height) as usize;
        let mut data = vec![0u8; n];
        // Above-ground band — the sky and grass strip — is always visible.
        for y in 0..=SURFACE_ROW {
            for x in 0..width {
                data[(y * width + x) as usize] = 255;
            }
        }
        // Pre-reveal a generous bubble around the colony entrance so the
        // starter shaft + chambers are visible at t=0 (the queen is already
        // there; the player should see her).
        let r = 18;
        for dy in -3..=r {
            for dx in -r..=r {
                let nx = COLONY_X + dx;
                let ny = COLONY_Y + dy;
                if nx < 0 || ny < 0 || nx >= width || ny >= height { continue; }
                if dx*dx + dy*dy <= r*r {
                    data[(ny * width + nx) as usize] = 255;
                }
            }
        }
        Self { width, height, data, dirty: true }
    }

    #[inline] pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    #[inline] pub fn is_explored(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width || y >= self.height { return false; }
        self.data[self.idx(x, y)] != 0
    }

    /// Mark a tile explored. Returns true if this was a new reveal.
    #[inline] pub fn reveal(&mut self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width || y >= self.height { return false; }
        let i = self.idx(x, y);
        if self.data[i] == 0 {
            self.data[i] = 255;
            self.dirty = true;
            true
        } else {
            false
        }
    }
}
