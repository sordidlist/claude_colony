//! Tile types + flat tile grid resource.

use bevy_ecs::prelude::Resource;
use crate::config::*;

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TileType {
    Air     = 0,
    Grass   = 1,
    Soil    = 2,
    Dirt1   = 3,
    Dirt2   = 4,
    Dirt3   = 5,
    Rock    = 6,
    Tunnel  = 7,
    Chamber = 8,
    Sand    = 9,
    Fungus  = 10,
    Mud     = 11,
}

impl TileType {
    #[inline] pub fn from_u8(v: u8) -> TileType {
        // Safe: we only ever store values in 0..=11
        debug_assert!(v <= 11);
        unsafe { std::mem::transmute(v) }
    }
    #[inline] pub fn passable(self) -> bool {
        matches!(self, Self::Air | Self::Grass | Self::Tunnel | Self::Chamber | Self::Fungus)
    }
    #[inline] pub fn diggable(self) -> bool {
        matches!(self, Self::Soil | Self::Dirt1 | Self::Dirt2 | Self::Dirt3 | Self::Sand | Self::Mud)
    }
    #[inline] pub fn solid(self) -> bool { !self.passable() }

    pub fn dig_seconds(self) -> f32 {
        match self {
            Self::Soil  => DIG_TIME_SOIL,
            Self::Sand  => DIG_TIME_SAND,
            Self::Dirt1 => DIG_TIME_DIRT1,
            Self::Dirt2 => DIG_TIME_DIRT2,
            Self::Dirt3 => DIG_TIME_DIRT3,
            Self::Mud   => 5.0,
            _           => f32::INFINITY,
        }
    }
}

#[derive(Resource)]
pub struct TileGrid {
    pub width:  i32,
    pub height: i32,
    pub tiles:    Vec<u8>,    // SoA — TileType as u8
    pub variants: Vec<u8>,    // 0..=3 visual variant per tile
    pub dirty:    bool,       // set when any tile changed; renderer rebakes texture
}

impl TileGrid {
    pub fn new(width: i32, height: i32) -> Self {
        let n = (width * height) as usize;
        Self {
            width,
            height,
            tiles:    vec![TileType::Air as u8; n],
            variants: vec![0; n],
            dirty:    true,
        }
    }

    #[inline] pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    #[inline] pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    #[inline] pub fn get(&self, x: i32, y: i32) -> TileType {
        if !self.in_bounds(x, y) { return TileType::Rock; }
        TileType::from_u8(self.tiles[self.idx(x, y)])
    }

    #[inline] pub fn set(&mut self, x: i32, y: i32, t: TileType) {
        if !self.in_bounds(x, y) { return; }
        let i = self.idx(x, y);
        self.tiles[i] = t as u8;
        self.dirty = true;
    }

    #[inline] pub fn passable(&self, x: i32, y: i32) -> bool {
        self.get(x, y).passable()
    }

    /// Convert a tile to TUNNEL. Called when a worker finishes a dig job.
    pub fn dig(&mut self, x: i32, y: i32) {
        if !self.in_bounds(x, y) { return; }
        self.set(x, y, TileType::Tunnel);
    }
}

pub fn dirt_for_depth(row: i32, surface: i32) -> TileType {
    let d = row - surface;
    if d <= 4       { TileType::Soil  }
    else if d <= 24 { TileType::Dirt1 }
    else if d <= 60 { TileType::Dirt2 }
    else            { TileType::Dirt3 }
}
