//! Lifecycle stub: keeps Population resource current. Brood / corpse decay /
//! egg-laying lands in a follow-up.

use bevy_ecs::prelude::*;
use super::components::*;
use super::time::Population;

pub fn update_population(
    mut pop: ResMut<Population>,
    ants:    Query<(&Ant, Option<&WorkerBrain>, Option<&Cargo>)>,
    brood:   Query<&Brood>,
) {
    let mut workers  = 0;
    let mut queens   = 0;
    let mut soldiers = 0;
    let mut digging  = 0;
    let mut foraging = 0;
    let mut hauling  = 0;
    let mut fighting = 0;
    for (a, brain, cargo) in ants.iter() {
        match a.kind {
            AntKind::Worker => {
                workers += 1;
                if let Some(b) = brain {
                    if b.mode == WorkerMode::Dig { digging += 1; }
                    if b.mode == WorkerMode::DepositDebris { hauling += 1; }
                    if b.mode == WorkerMode::FightBack { fighting += 1; }
                    let has_cargo = cargo.map_or(false, |c| c.amount > 0);
                    if has_cargo || b.mode == WorkerMode::SeekFood { foraging += 1; }
                }
            }
            AntKind::Queen   => queens   += 1,
            AntKind::Soldier => soldiers += 1,
        }
    }
    pop.workers  = workers;
    pop.queens   = queens;
    pop.soldiers = soldiers;
    pop.brood    = brood.iter().count();
    pop.digging  = digging;
    pop.foraging = foraging;
    pop.hauling  = hauling;
    pop.fighting = fighting;
}
