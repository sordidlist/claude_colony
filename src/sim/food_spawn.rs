//! Surface food spawner. Periodically drops a `Food` entity on a random
//! grass tile within reach of the colony entrance and seeds a strong food
//! pheromone burst at that location so foragers can find it via the
//! existing `SeekFood` pheromone-following behaviour.

use bevy_ecs::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use glam::Vec2;
use crate::config::*;
use crate::world::{TileGrid, PheromoneGrid, PheromoneChannel, TileType};
use super::components::*;
use super::Time;

#[derive(Resource)]
pub struct SurfaceFoodSpawner {
    pub timer:    f32,
    pub interval: f32,
    rng:          StdRng,
}

impl SurfaceFoodSpawner {
    pub fn new(seed: u64) -> Self {
        Self {
            timer:    3.0,
            interval: FOOD_SPAWN_INTERVAL_S,
            rng:      StdRng::seed_from_u64(seed.wrapping_add(101)),
        }
    }
}

pub fn spawn_surface_food(
    mut spawner: ResMut<SurfaceFoodSpawner>,
    mut phero:   ResMut<PheromoneGrid>,
    grid:        Res<TileGrid>,
    time:        Res<Time>,
    existing:    Query<&Food>,
    mut commands: Commands,
) {
    spawner.timer -= time.dt;
    if spawner.timer > 0.0 { return; }
    spawner.timer = spawner.interval + spawner.rng.gen_range(-1.0..1.5);

    // Cap the simultaneous food count — without this, slow foraging plus
    // continuous spawning litters the grass with pellets.
    if existing.iter().count() >= FOOD_SPAWN_MAX { return; }

    for _ in 0..30 {
        let dx = spawner.rng.gen_range(-90..=90);
        let x = (COLONY_X + dx).clamp(2, grid.width - 3);
        let y = SURFACE_ROW;
        // Stand the pellet just above the grass row, on what's effectively
        // ground level visually.
        let stand_y = y - 1;
        if grid.get(x, y).passable()
            && grid.get(x, stand_y) == TileType::Air
        {
            commands.spawn((
                Position(Vec2::new(x as f32 + 0.5, stand_y as f32 + 0.5)),
                Velocity(Vec2::ZERO),
                Food { value: 1 },
            ));
            phero.deposit(x, stand_y, PheromoneChannel::Food, FOOD_PHEROMONE_BURST);
            return;
        }
    }
}
