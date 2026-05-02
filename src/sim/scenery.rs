//! Above-ground scenery: a barn, a wandering dog, a riding lawn mower
//! that comes around now and again, scattered trees, drifting clouds.
//! The dog, trees, clouds and barn are pure flavour. The mower is
//! mechanical — it shaves above-ground dirt piles and kills any
//! creatures in its path.
//!
//! All balance knobs live in `config.rs` so adjustments are a single-
//! file change.

use bevy_ecs::prelude::*;
use glam::Vec2;
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::config::*;
use crate::world::{TileGrid, TileType};
use super::Time;
use super::EventLog;
use super::components::{Position, Velocity, Health, Food, Corpse, Brood};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DecorKind {
    Barn,
    Tree,
    Dog,
    Cloud,
    SunMoon,
    /// Riding lawn mower. Walks the surface like the dog (using
    /// `topmost_walkable_y`) but slower, so passes are infrequent.
    /// Lifecycle is driven by `MowerSchedule`: the mower runs
    /// `MOWER_LAPS_PER_VISIT` traversals and then despawns for
    /// `MOWER_COOLDOWN_SECONDS` of sim time. Whatever it rolls over
    /// above the natural ground is shaved by the blade, and any
    /// living creature inside `MOWER_KILL_RADIUS` of its centre is
    /// killed.
    Mower,
}

/// Per-column scratch on a Decoration. Used by the mower to track
/// which column it last cut so it shaves at most once per column
/// crossing instead of once per frame. Anything that doesn't track
/// position can leave this as -1.
#[derive(Component, Copy, Clone, Debug)]
pub struct Decoration {
    pub kind:     DecorKind,
    pub frame:    u8,
    pub anim_t:   f32,
    pub vx:       f32,    // dogs and mowers walk; clouds drift
    pub flip_x:   bool,
    /// Last grid column this entity acted on. The mower uses this so
    /// the blade fires once per column transition. -1 means "no column
    /// recorded yet."
    pub last_col: i16,
}

impl Decoration {
    pub fn new(kind: DecorKind) -> Self {
        Self { kind, frame: 0, anim_t: 0.0, vx: 0.0, flip_x: false, last_col: -1 }
    }
}

#[derive(Component, Copy, Clone, Debug)]
pub struct DecorPos {
    pub x: f32,
    pub y: f32,
}

// ─── mower lifecycle ────────────────────────────────────────────────

/// State machine for the mower's "appears for a few laps, leaves for
/// a while" cycle. Lives as a Resource so a single shared schedule
/// drives whatever mower entity happens to be in the world.
#[derive(Copy, Clone, Debug)]
pub enum MowerPhase {
    /// Mower is on the map. The wrapped value is laps remaining
    /// before it retires; reaching 0 triggers a despawn.
    Active(u32),
    /// Mower is gone. Wrapped value is sim seconds until it returns.
    Cooldown(f32),
}

#[derive(Resource, Copy, Clone, Debug)]
pub struct MowerSchedule {
    pub phase: MowerPhase,
}

impl Default for MowerSchedule {
    fn default() -> Self {
        Self { phase: MowerPhase::Active(MOWER_LAPS_PER_VISIT) }
    }
}

// ─── per-frame animation ────────────────────────────────────────────

pub fn animate_scenery(
    time: Res<Time>,
    mut grid: ResMut<TileGrid>,
    mut schedule: ResMut<MowerSchedule>,
    mut grass: ResMut<crate::world::GrassField>,
    mut q: Query<(&mut DecorPos, &mut Decoration)>,
) {
    let dt = time.dt;
    // Per-frame RNG seeded from the sim clock so the mower's coin
    // flips are deterministic with respect to total elapsed time but
    // varied across frames.
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0x4C61_776E_4D6F_7765 /* "LawnMowe" */);

    let world_w = grid.width;

    for (mut p, mut d) in q.iter_mut() {
        d.anim_t += dt;
        match d.kind {
            DecorKind::Dog => {
                p.x += d.vx * dt;
                if d.anim_t > 0.18 {
                    d.anim_t = 0.0;
                    d.frame = (d.frame + 1) & 3;
                }
                let foot_center_x = p.x + 1.5;
                let foot_y = topmost_walkable_y(&grid, foot_center_x as i32);
                p.y = (foot_y - 2) as f32;

                // Patrol bounds — see config.rs DOG_PATROL_*.
                let min_x = (COLONY_X + DOG_PATROL_WEST_OFFSET) as f32;
                let max_x = (COLONY_X + DOG_PATROL_EAST_OFFSET) as f32;
                let probe_x = foot_center_x + d.vx.signum() * 2.0;
                if probe_x < min_x || probe_x > max_x {
                    d.vx = -d.vx;
                    d.flip_x = !d.flip_x;
                }
            }
            DecorKind::Mower => {
                p.x += d.vx * dt;
                if d.anim_t > 0.30 {
                    d.anim_t = 0.0;
                    d.frame ^= 1;
                }
                let foot_center_x = p.x + 2.0;
                let foot_col = foot_center_x as i32;
                let foot_y = topmost_walkable_y(&grid, foot_col);
                p.y = (foot_y - 1) as f32;

                // Patrol bounds — see config.rs MOWER_PATROL_*.
                let min_x = (COLONY_X + MOWER_PATROL_WEST_OFFSET) as f32;
                let max_x = world_w as f32 - MOWER_PATROL_RIGHT_MARGIN;
                let probe_x = foot_center_x + d.vx.signum() * 2.5;
                if probe_x < min_x || probe_x > max_x {
                    d.vx = -d.vx;
                    d.flip_x = !d.flip_x;
                    // Each bound-bounce counts as one completed lap.
                    // The lifecycle system reads this and despawns
                    // the mower when laps_remaining reaches 0.
                    if let MowerPhase::Active(ref mut laps) = schedule.phase {
                        *laps = laps.saturating_sub(1);
                    }
                }

                // Shave once per column transition — multiple rolls
                // per crossing make the mower visibly more
                // destructive than the original "cut at most one
                // tile per pass" behaviour.
                if (d.last_col as i32) != foot_col {
                    d.last_col = foot_col as i16;
                    for _ in 0..MOWER_TILES_PER_COLUMN {
                        if rng.gen::<f32>() < MOWER_SHAVE_CHANCE {
                            shave_top_above_surface(&mut grid, foot_col);
                        }
                    }
                    // Mow the lawn — every column the deck rolls
                    // over gets its grass clipped to zero.
                    grass.mow(foot_col);
                }
            }
            DecorKind::Cloud => {
                p.x += d.vx * dt;
                if p.x > world_w as f32 + 8.0 { p.x = -8.0; }
                if p.x < -8.0                 { p.x = world_w as f32 + 8.0; }
            }
            _ => {}
        }
    }
}

// ─── lifecycle + ant-killing ────────────────────────────────────────

/// Drives the mower's appear / despawn / cooldown cycle and applies
/// its kill radius to ants and hostiles each frame. Runs after
/// `animate_scenery` so the lap counter is current.
pub fn mower_lifecycle(
    time: Res<Time>,
    mut schedule: ResMut<MowerSchedule>,
    mut log: ResMut<EventLog>,
    decors: Query<(Entity, &DecorPos, &Decoration)>,
    targets: Query<(Entity, &Position), (With<Health>, Without<Brood>)>,
    mut commands: Commands,
) {
    let dt = time.dt;
    if dt <= 0.0 { return; }

    // Find the live mower (if any) and its sprite-relative position.
    let mut mower: Option<(Entity, f32, f32)> = None;
    for (e, p, d) in decors.iter() {
        if matches!(d.kind, DecorKind::Mower) {
            mower = Some((e, p.x, p.y));
            break;
        }
    }

    match schedule.phase {
        MowerPhase::Active(laps_remaining) => {
            if laps_remaining == 0 {
                // Time to retire. Despawn whichever mower is in the
                // world, log it, switch to cooldown.
                if let Some((e, _, _)) = mower {
                    commands.entity(e).despawn();
                    log.push("The lawn mower heads back to the shed.",
                             [0.78, 0.74, 0.62, 1.0]);
                }
                schedule.phase = MowerPhase::Cooldown(MOWER_COOLDOWN_SECONDS);
                return;
            }

            // No mower yet (initial app boot, or scenario set up
            // without one): spawn one at the configured position.
            if mower.is_none() {
                let mx = (COLONY_X + MOWER_SPAWN_X_OFFSET) as f32;
                let my = (SURFACE_ROW - 2) as f32;
                commands.spawn((
                    DecorPos { x: mx, y: my },
                    Decoration { kind: DecorKind::Mower, frame: 0, anim_t: 0.0,
                                 vx: -MOWER_SPEED, flip_x: true, last_col: -1 },
                ));
                mower = Some((Entity::PLACEHOLDER, mx, my));
                log.push("A lawn mower fires up nearby!",
                         [0.96, 0.84, 0.30, 1.0]);
            }

            // Run-over kill pass: every entity with Health within
            // `MOWER_KILL_RADIUS` of the mower's centre dies and
            // drops a corpse the colony can scavenge.
            if let Some((_, mx, my)) = mower {
                let cx = mx + 2.0;   // mower sprite centre (4 tiles wide)
                let cy = my + 1.0;   // mower sprite centre (2 tiles tall)
                let r2 = MOWER_KILL_RADIUS * MOWER_KILL_RADIUS;
                let mut killed_any = false;
                for (e, p) in targets.iter() {
                    let dx = p.0.x - cx;
                    let dy = p.0.y - cy;
                    if dx*dx + dy*dy <= r2 {
                        commands.entity(e).despawn();
                        commands.spawn((
                            Position(p.0),
                            Velocity(Vec2::ZERO),
                            Food   { value: MOWER_KILL_FOOD_VALUE },
                            Corpse { decay: CORPSE_DECAY_S },
                        ));
                        killed_any = true;
                    }
                }
                if killed_any {
                    log.push("Crunch — the mower runs something down!",
                             [1.0, 0.42, 0.18, 1.0]);
                }
            }
        }
        MowerPhase::Cooldown(remaining) => {
            let new_remaining = remaining - dt;
            if new_remaining <= 0.0 {
                schedule.phase = MowerPhase::Active(MOWER_LAPS_PER_VISIT);
                log.push("Engine sound from the barn — a mower stirs.",
                         [0.96, 0.84, 0.30, 1.0]);
            } else {
                schedule.phase = MowerPhase::Cooldown(new_remaining);
            }
        }
    }
}

// ─── helpers ────────────────────────────────────────────────────────

/// Remove the topmost solid tile in column `x` that is above the
/// natural surface row. Does nothing if the column has no above-ground
/// pile, or if the topmost solid is at/below `SURFACE_ROW` (which is
/// the original grass — we never carve into untouched ground).
fn shave_top_above_surface(grid: &mut TileGrid, x: i32) {
    if x < 1 || x >= grid.width - 1 { return; }
    for y in 1..SURFACE_ROW {
        let t = grid.get(x, y);
        if t.solid() {
            grid.set(x, y, TileType::Air);
            grid.dirty = true;
            return;
        }
    }
}

/// Returns the y of the topmost air tile in column `x` whose neighbour
/// directly below is solid or grass — i.e. the surface a creature standing
/// at column `x` would walk on. Used by the dog and the mower so dirt
/// mounds become terrain they traverse rather than walls they bounce off.
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
    // Patrol box is configured in config.rs (DOG_PATROL_*).
    world.spawn((
        DecorPos { x: (COLONY_X - 50) as f32, y: (SURFACE_ROW - 3) as f32 },
        Decoration { kind: DecorKind::Dog, frame: 0, anim_t: 0.0, vx: DOG_SPEED,
                     flip_x: false, last_col: -1 },
    ));

    // The mower itself is *not* spawned here — `mower_lifecycle`
    // creates it on the first tick (or after each cooldown). This
    // way the schedule is the single source of truth for whether
    // a mower is in the world right now.

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
            Decoration { kind: DecorKind::Cloud, frame: 0, anim_t: 0.0, vx,
                         flip_x: false, last_col: -1 },
        ));
    }

    // Sun/moon — tracked by render layer using TimeOfDay; we keep one
    // entity tagged so the renderer knows where to draw it.
    world.spawn((
        DecorPos { x: 0.0, y: 0.0 }, // recomputed each frame from time of day
        Decoration::new(DecorKind::SunMoon),
    ));
}
