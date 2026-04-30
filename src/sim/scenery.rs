//! Above-ground scenery: a barn, a wandering dog, scattered trees, drifting
//! clouds. Pure flavour — no interaction with the colony, but the surface
//! shouldn't be a barren stripe of grass.

use bevy_ecs::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::config::*;
use crate::world::TileGrid;
use super::Time;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DecorKind {
    Barn,
    Tree,
    Dog,
    Cloud,
    SunMoon,
}

#[derive(Component, Copy, Clone, Debug)]
pub struct Decoration {
    pub kind:   DecorKind,
    pub frame:  u8,
    pub anim_t: f32,
    pub vx:     f32,    // dogs walk; clouds drift
    pub flip_x: bool,
}

impl Decoration {
    pub fn new(kind: DecorKind) -> Self {
        Self { kind, frame: 0, anim_t: 0.0, vx: 0.0, flip_x: false }
    }
}

#[derive(Component, Copy, Clone, Debug)]
pub struct DecorPos {
    pub x: f32,
    pub y: f32,
}

pub fn animate_scenery(
    time: Res<Time>,
    grid: Res<TileGrid>,
    mut q: Query<(&mut DecorPos, &mut Decoration)>,
) {
    let dt = time.dt;
    for (mut p, mut d) in q.iter_mut() {
        d.anim_t += dt;
        match d.kind {
            DecorKind::Dog => {
                // The dog walks along whatever the topmost walkable surface
                // is at its current column — so when ants pile dirt into a
                // mound, the dog rises naturally up over it instead of
                // bouncing off a soil wall and getting trapped between two
                // piles flanking the entrance.
                p.x += d.vx * dt;
                if d.anim_t > 0.18 {
                    d.anim_t = 0.0;
                    d.frame = (d.frame + 1) & 3;
                }
                let foot_center_x = p.x + 1.5;
                let foot_y = topmost_walkable_y(&grid, foot_center_x as i32);
                p.y = (foot_y - 2) as f32;

                // Turn at world edges (the only obstacle the dog can't
                // climb over). Dirt mounds are no longer a turn-around
                // condition; the y adjustment handles them.
                let probe_x = foot_center_x + d.vx.signum() * 2.0;
                if probe_x < 3.0 || probe_x > grid.width as f32 - 3.0 {
                    d.vx = -d.vx;
                    d.flip_x = !d.flip_x;
                }
            }
            DecorKind::Cloud => {
                p.x += d.vx * dt;
                if p.x > grid.width as f32 + 8.0  { p.x = -8.0; }
                if p.x < -8.0                     { p.x = grid.width as f32 + 8.0; }
            }
            _ => {}
        }
    }
}

/// Returns the y of the topmost air tile in column `x` whose neighbour
/// directly below is solid or grass — i.e. the surface a creature standing
/// at column `x` would walk on. Used by the dog so dirt mounds become
/// terrain it can traverse rather than a wall it bounces off.
fn topmost_walkable_y(grid: &TileGrid, x: i32) -> i32 {
    let h = grid.height;
    for y in 0..(h - 1) {
        if grid.get(x, y) == crate::world::TileType::Air {
            let below = grid.get(x, y + 1);
            if below.solid() || matches!(below, crate::world::TileType::Grass) {
                return y;
            }
        }
    }
    SURFACE_ROW - 1
}

pub fn spawn_initial_scenery(world: &mut World, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(13));

    // Barn — 18×12 tiles, planted left of the entrance with its base on the
    // grass row. DecorPos here is the sprite's top-left tile.
    world.spawn((
        DecorPos { x: (COLONY_X - 64) as f32, y: (SURFACE_ROW - 12) as f32 },
        Decoration::new(DecorKind::Barn),
    ));

    // A loyal dog patrolling near the barn — 3×3 tiles, base on grass.
    world.spawn((
        DecorPos { x: (COLONY_X - 26) as f32, y: (SURFACE_ROW - 3) as f32 },
        Decoration { kind: DecorKind::Dog, frame: 0, anim_t: 0.0, vx: 2.4, flip_x: false },
    ));

    // A handful of trees scattered along the surface. Trees are 3×6 tiles
    // — base on the grass row means top-left at SURFACE_ROW - 6.
    for _ in 0..8 {
        let dx = rng.gen_range(-100..100);
        let x = (COLONY_X + dx).clamp(2, WORLD_WIDTH - 4) as f32;
        // Skip too close to barn / colony entrance.
        if (x - (COLONY_X - 50) as f32).abs() < 22.0 { continue; }
        if (x - COLONY_X as f32).abs() < 8.0 { continue; }
        world.spawn((
            DecorPos { x, y: (SURFACE_ROW - 6) as f32 },
            Decoration::new(DecorKind::Tree),
        ));
    }

    // Drifting clouds — different speeds and starting positions.
    for _ in 0..6 {
        let x  = rng.gen_range(0..WORLD_WIDTH) as f32;
        let y  = rng.gen_range(2..SURFACE_ROW - 8) as f32;
        let vx = rng.gen_range(0.4..1.2) * if rng.gen::<bool>() { 1.0 } else { -1.0 };
        world.spawn((
            DecorPos { x, y },
            Decoration { kind: DecorKind::Cloud, frame: 0, anim_t: 0.0, vx, flip_x: false },
        ));
    }

    // Sun/moon — tracked by render layer using TimeOfDay; we keep one
    // entity tagged so the renderer knows where to draw it.
    world.spawn((
        DecorPos { x: 0.0, y: 0.0 }, // recomputed each frame from time of day
        Decoration::new(DecorKind::SunMoon),
    ));
}
