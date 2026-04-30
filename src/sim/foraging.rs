//! Pickup + deposit logic. Worker walks onto a Food entity → pickup. Worker
//! reaches the colony entrance with cargo → deposit (sets food_stored counter
//! and clears cargo).

use bevy_ecs::prelude::*;
use crate::config::*;
use crate::world::{PheromoneGrid, PheromoneChannel};
use super::components::*;

#[derive(Resource, Default, Copy, Clone)]
pub struct ColonyStores {
    pub food_stored: u32,
}

pub fn pickup_and_deposit(
    mut ants:    Query<(&Position, &mut Cargo, &Ant)>,
    food:        Query<(Entity, &Position), With<Food>>,
    mut phero:   ResMut<PheromoneGrid>,
    mut stores:  ResMut<ColonyStores>,
    mut commands: Commands,
) {
    // Pickup: any worker without cargo standing on a food tile claims it.
    // Iterate ants once and food once; check overlap by tile.
    let foods: Vec<(Entity, i32, i32)> = food.iter()
        .map(|(e, p)| (e, p.0.x as i32, p.0.y as i32))
        .collect();
    if foods.is_empty() {
        // still process deposits below
    }
    let mut consumed: Vec<Entity> = Vec::new();
    for (apos, mut cargo, a) in ants.iter_mut() {
        if !matches!(a.kind, AntKind::Worker) { continue; }
        if cargo.amount > 0 || cargo.debris.is_some() { continue; }
        let atx = apos.0.x as i32;
        let aty = apos.0.y as i32;
        for &(fe, fx, fy) in &foods {
            if consumed.contains(&fe) { continue; }
            if (fx - atx).abs() <= 1 && (fy - aty).abs() <= 1 {
                cargo.amount = 1;
                consumed.push(fe);
                phero.deposit(atx, aty, PheromoneChannel::Food, 80.0);
                break;
            }
        }
    }
    for e in consumed {
        commands.entity(e).despawn();
    }

    // Deposit: any worker carrying food and inside a tight radius of the
    // colony entrance dumps it into the colony stores.
    for (apos, mut cargo, a) in ants.iter_mut() {
        if !matches!(a.kind, AntKind::Worker) { continue; }
        if cargo.amount == 0 { continue; }
        let dx = apos.0.x - (COLONY_X as f32 + 0.5);
        let dy = apos.0.y - (COLONY_Y as f32 + 0.5);
        if dx*dx + dy*dy <= 9.0 {
            stores.food_stored += cargo.amount as u32;
            cargo.amount = 0;
        }
    }
}
