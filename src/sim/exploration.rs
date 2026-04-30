//! Fog-of-war update system. Each ant reveals a small square around itself
//! every frame. Cheap: a 5×5 stamp per ant, plain index writes.

use bevy_ecs::prelude::*;
use crate::world::ExploredGrid;
use super::components::{Position, Ant};

const REVEAL_RADIUS: i32 = 3;

pub fn update_exploration(
    mut explored: ResMut<ExploredGrid>,
    q: Query<&Position, With<Ant>>,
) {
    let r2 = REVEAL_RADIUS * REVEAL_RADIUS;
    for p in q.iter() {
        let tx = p.0.x as i32;
        let ty = p.0.y as i32;
        for dy in -REVEAL_RADIUS..=REVEAL_RADIUS {
            for dx in -REVEAL_RADIUS..=REVEAL_RADIUS {
                if dx*dx + dy*dy <= r2 {
                    explored.reveal(tx + dx, ty + dy);
                }
            }
        }
    }
}
