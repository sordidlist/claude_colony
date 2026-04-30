//! Predator (spider) and rival-colony (red ant) AI.
//! Both use the existing movement system, so gravity + tile collision come
//! for free; this module just sets a wandering velocity and updates sprite
//! orientation. Combat lands in a follow-up.

use bevy_ecs::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::config::*;
use crate::world::TileGrid;
use super::components::*;
use super::Time;

const SPIDER_SPEED: f32 = 3.2;
const RIVAL_SPEED:  f32 = 4.0;

pub fn spider_tick(
    time: Res<Time>,
    grid: Res<TileGrid>,
    mut q: Query<(&Position, &mut Velocity, &mut Spider, &mut VisualState)>,
) {
    if time.dt <= 0.0 { return; }
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0xA37D_891B_2467_5C13);
    let entrance_x = COLONY_X as f32;
    for (pos, mut vel, mut s, mut vis) in q.iter_mut() {
        // Spiders accumulate at the entrance because random wander +
        // the entrance shaft being the only-passable upward path through
        // the dirt makes the shaft a funnel. Fix it from two sides:

        // (a) Hard "keep deep" floor. If a spider drifts above the deep
        // tunnel zone, force it back down with both vertical velocity
        // *and* outward horizontal velocity so it leaves the entrance
        // column instead of cycling up and down the shaft.
        if pos.0.y < (SURFACE_ROW + 18) as f32 {
            let outward = if pos.0.x < entrance_x { -1.0 } else { 1.0 };
            vel.0.x = outward * SPIDER_SPEED;
            vel.0.y = SPIDER_SPEED * 1.5;
            s.heading_timer = 2.0;
            continue;
        }

        // (b) Even when nominally deep, the entrance-shaft column
        // itself is off-limits. If a spider is climbing the shaft,
        // shove it sideways so it leaves the funnel.
        if (pos.0.x - entrance_x).abs() < 5.0
            && pos.0.y < (SURFACE_ROW + 25) as f32
        {
            let outward = if pos.0.x < entrance_x { -1.0 } else { 1.0 };
            vel.0.x = outward * SPIDER_SPEED * 1.5;
            vel.0.y = SPIDER_SPEED * 0.5;
            s.heading_timer = 1.5;
            continue;
        }

        s.heading_timer -= time.dt;
        if s.heading_timer <= 0.0 {
            s.heading_timer = rng.gen_range(1.2..2.6);
            if rng.gen::<f32>() < 0.10 {
                vel.0.x = 0.0; vel.0.y = 0.0;
            } else {
                let angle = rng.gen_range(0.0_f32..std::f32::consts::TAU);
                vel.0.x = angle.cos() * SPIDER_SPEED;
                vel.0.y = angle.sin() * SPIDER_SPEED;
            }
        }
        let probe_x = pos.0.x + vel.0.x.signum() * 0.6;
        let probe_y = pos.0.y + vel.0.y.signum() * 0.6;
        if !grid.passable(probe_x as i32, probe_y as i32) {
            s.heading_timer = 0.0;
        }
        vis.anim_t += time.dt;
        if vis.anim_t > 0.22 {
            vis.anim_t = 0.0;
            vis.anim_frame ^= 1;
        }
        if vel.0.x >  0.2 { vis.facing =  1; }
        else if vel.0.x < -0.2 { vis.facing = -1; }
    }
}

pub fn rival_tick(
    time: Res<Time>,
    grid: Res<TileGrid>,
    mut q: Query<(&Position, &mut Velocity, &mut RivalAnt, &mut VisualState)>,
) {
    if time.dt <= 0.0 { return; }
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0x53F8_AA12_99C7_3142);
    for (pos, mut vel, mut r, mut vis) in q.iter_mut() {
        r.heading_timer -= time.dt;
        if r.heading_timer <= 0.0 {
            r.heading_timer = rng.gen_range(1.0..3.0);
            // Drift toward colony entrance most of the time — rivals are
            // the colony's natural threat so they should *trend* toward it.
            let dx_to_colony = (COLONY_X as f32 + 0.5 - pos.0.x).signum();
            let bias = if rng.gen::<f32>() < 0.7 { dx_to_colony } else {
                if rng.gen::<bool>() { 1.0 } else { -1.0 }
            };
            vel.0.x = bias * RIVAL_SPEED;
            vel.0.y = rng.gen_range(-1.0..1.0);
        }
        let probe_x = pos.0.x + vel.0.x.signum() * 0.6;
        let probe_y = pos.0.y + vel.0.y.signum() * 0.6;
        if !grid.passable(probe_x as i32, probe_y as i32) {
            r.heading_timer = 0.0;
        }
        vis.anim_t += time.dt;
        if vis.anim_t > 0.10 {
            vis.anim_t = 0.0;
            vis.anim_frame = (vis.anim_frame + 1) % 4;
        }
        if vel.0.x >  0.2 { vis.facing =  1; }
        else if vel.0.x < -0.2 { vis.facing = -1; }
    }
}
