//! Soldier AI: patrols around the colony entrance and chases the nearest
//! hostile (Spider / RivalAnt) into melee range. Combat damage is handled
//! by the shared `combat_step` system.

use bevy_ecs::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::config::*;
use crate::world::TileGrid;
use super::components::*;
use super::Time;

const SOLDIER_SPEED: f32 = 3.6;

pub fn soldier_tick(
    time: Res<Time>,
    grid: Res<TileGrid>,
    enemies: Query<(Entity, &Position),
                   (Without<Ant>, Without<Brood>, Without<Food>,
                    Or<(With<Spider>, With<RivalAnt>)>)>,
    mut soldiers: Query<(&Position, &mut Velocity, &mut SoldierAi,
                         &mut VisualState, &Ant, &mut AiTrace)>,
) {
    if time.dt <= 0.0 { return; }
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0xC471_2A88_8810_F102);

    let entrance = (COLONY_X as f32 + 0.5, COLONY_Y as f32 + 0.5);

    for (pos, mut vel, mut ai, mut vis, a, mut trace) in soldiers.iter_mut() {
        if a.kind != AntKind::Soldier { continue; }
        // patrol_target == None means "currently chasing an enemy" in
        // this AI; track it to detect engage / disengage transitions.
        let was_chasing = ai.patrol_target.is_none();

        // Hunt: scan for the nearest enemy within sense radius.
        let sense2 = (SOLDIER_SENSE_RADIUS_T as f32).powi(2);
        let mut best: Option<(f32, f32, f32)> = None;
        for (_, ep) in enemies.iter() {
            let dx = ep.0.x - pos.0.x;
            let dy = ep.0.y - pos.0.y;
            let d2 = dx*dx + dy*dy;
            if d2 < sense2 && best.map_or(true, |(_, _, b)| d2 < b) {
                best = Some((ep.0.x, ep.0.y, d2));
            }
        }
        if let Some((ex, ey, _)) = best {
            let dx = ex - pos.0.x;
            let dy = ey - pos.0.y;
            let d  = (dx*dx + dy*dy).sqrt().max(0.01);
            vel.0.x = dx / d * SOLDIER_SPEED;
            vel.0.y = dy / d * SOLDIER_SPEED;
            ai.patrol_target = None;
        } else {
            // Patrol: pick a random target within patrol radius around the
            // entrance every few seconds, walk toward it.
            ai.patrol_timer -= time.dt;
            let dist_home2 = (pos.0.x - entrance.0).powi(2)
                           + (pos.0.y - entrance.1).powi(2);
            let needs_target = ai.patrol_target.is_none()
                || ai.patrol_timer <= 0.0
                || dist_home2 > (SOLDIER_PATROL_RADIUS * 1.5).powi(2);
            if needs_target {
                ai.patrol_timer = rng.gen_range(2.5..5.0);
                let r = rng.gen_range(4.0..SOLDIER_PATROL_RADIUS);
                let ang: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
                let tx = (entrance.0 + ang.cos() * r)
                    .clamp(2.0, grid.width as f32 - 3.0);
                let ty = (entrance.1 + ang.sin().abs() * r)
                    .clamp(SURFACE_ROW as f32 - 4.0, grid.height as f32 - 3.0);
                ai.patrol_target = Some((tx, ty));
            }
            let (tx, ty) = ai.patrol_target.unwrap_or(entrance);
            let dx = tx - pos.0.x;
            let dy = ty - pos.0.y;
            let d  = (dx*dx + dy*dy).sqrt();
            if d > 0.6 {
                vel.0.x = dx / d * SOLDIER_SPEED;
                vel.0.y = dy / d * SOLDIER_SPEED;
            } else {
                vel.0.x = 0.0;
                vel.0.y = 0.0;
            }
        }

        let is_chasing = ai.patrol_target.is_none();
        if is_chasing != was_chasing {
            trace.record(time.total,
                if is_chasing { "Engaging enemy" } else { "Resumed patrol" });
        }

        vis.anim_t += time.dt;
        if vis.anim_t > 0.12 {
            vis.anim_t = 0.0;
            vis.anim_frame = (vis.anim_frame + 1) % 4;
        }
        if vel.0.x >  0.2 { vis.facing =  1; }
        else if vel.0.x < -0.2 { vis.facing = -1; }
    }
}
