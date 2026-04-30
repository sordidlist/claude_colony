//! Four-channel float32 pheromone grid. Decay runs over the whole slice each
//! frame; deposits clamp at PHERO_MAX. Reads are O(1) tile lookups.

use bevy_ecs::prelude::Resource;
use crate::config::*;

#[repr(usize)]
#[derive(Copy, Clone)]
pub enum PheromoneChannel {
    Food    = 0,
    Return  = 1,
    Explore = 2,
    Alarm   = 3,
}

#[derive(Resource)]
pub struct PheromoneGrid {
    pub width:  i32,
    pub height: i32,
    /// channel-major: chan * (w*h) + y * w + x
    pub data:   Vec<f32>,
}

impl PheromoneGrid {
    pub fn new(width: i32, height: i32) -> Self {
        let n = (width * height) as usize * PHEROMONE_CHANNELS;
        Self { width, height, data: vec![0.0; n] }
    }

    #[inline] fn channel_offset(&self, c: PheromoneChannel) -> usize {
        c as usize * (self.width * self.height) as usize
    }

    #[inline] pub fn deposit(&mut self, x: i32, y: i32, c: PheromoneChannel, amt: f32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height { return; }
        let off = self.channel_offset(c);
        let i   = off + (y * self.width + x) as usize;
        let v   = self.data[i] + amt;
        self.data[i] = v.min(PHERO_MAX);
    }

    #[inline] pub fn level(&self, x: i32, y: i32, c: PheromoneChannel) -> f32 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height { return 0.0; }
        let off = self.channel_offset(c);
        self.data[off + (y * self.width + x) as usize]
    }

    /// Return the (dx, dy) offset of the strongest neighbour for this channel,
    /// or None if no neighbour has > min_value.
    pub fn strongest_neighbour(&self, x: i32, y: i32, c: PheromoneChannel,
                               min_value: f32) -> Option<(i32, i32)>
    {
        let mut best = min_value;
        let mut out  = None;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 { continue; }
                let v = self.level(x + dx, y + dy, c);
                if v > best { best = v; out = Some((dx, dy)); }
            }
        }
        out
    }

    /// Multiplicative decay across all channels. dt in seconds.
    pub fn decay(&mut self, dt: f32) {
        let factor = (-PHERO_DECAY_PER_S * dt).exp();
        for v in self.data.iter_mut() {
            *v *= factor;
            if *v < 0.05 { *v = 0.0; }
        }
    }
}
