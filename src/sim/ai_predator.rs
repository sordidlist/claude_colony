//! Predator FSM stub. Lands fully in a follow-up — placeholder system keeps
//! the schedule shape stable.

use bevy_ecs::prelude::*;

pub fn predator_ai(_q: Query<&crate::sim::components::Position>) {
    // intentionally empty — the predator entities aren't spawned yet
}
