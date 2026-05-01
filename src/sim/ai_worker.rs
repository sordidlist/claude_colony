//! Worker AI: utility-style mode selection on a 1 Hz replan, then per-frame
//! steering. Reads pheromones + dig queue; writes Velocity, WorkerBrain, and
//! pheromone deposits.
//!
//! Mode priorities (highest first):
//!   - has cargo  → ReturnHome
//!   - food/pheromone in sense radius → SeekFood
//!   - dig job available (slot table) → Dig (probabilistic, ~55%)
//!   - else → Wander

use bevy_ecs::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::config::*;
use crate::world::{TileGrid, TileType, PheromoneGrid, PheromoneChannel, DigJobs, ReturnFlowField};
use super::components::*;
use super::Time;
use super::EventLog;


pub fn worker_ai(
    time:        Res<Time>,
    grid:        Res<TileGrid>,
    field:       Res<ReturnFlowField>,
    mut phero:   ResMut<PheromoneGrid>,
    mut jobs:    ResMut<DigJobs>,
    mut q:       Query<(Entity, &mut Position, &mut Velocity, &mut WorkerBrain,
                        &mut VisualState, &mut Cargo, &Ant)>,
) {
    let dt = time.dt;
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0x9E37_79B9_7F4A_7C15
    );

    for (e, mut pos, mut vel, mut brain, mut vis, mut cargo, ant) in q.iter_mut() {
        if ant.kind != AntKind::Worker { continue; }

        brain.replan_in -= dt;
        if brain.replan_in <= 0.0 {
            brain.replan_in = 1.0 / ANT_REPLAN_HZ + rng.gen_range(-0.3..0.3);
            choose_mode(e, &pos, &mut brain, &mut cargo, &grid, &phero, &mut jobs, &mut rng);
        }

        match brain.mode {
            WorkerMode::Wander       => step_wander(&mut pos, &mut vel, &grid, &mut rng),
            WorkerMode::SeekFood     => step_seek_food(&mut pos, &mut vel, &phero, &grid, &mut rng),
            WorkerMode::ReturnHome   => step_return(&mut pos, &mut vel, &field),
            WorkerMode::Dig          => {
                step_dig(e, &mut pos, &mut vel, &mut brain, &mut vis, &mut cargo,
                         &grid, &mut jobs, dt);
            }
            WorkerMode::DepositDebris => {
                step_deposit_debris(&mut pos, &mut vel, &mut brain, &mut cargo,
                                    &grid, &field, &mut rng);
            }
            WorkerMode::FightBack => {
                step_fight_back(&mut pos, &mut vel, &phero, &mut rng);
            }
        }

        // Deposit a pheromone trail proportional to current activity.
        let tx = pos.0.x as i32; let ty = pos.0.y as i32;
        match brain.mode {
            WorkerMode::ReturnHome if cargo.amount > 0 => {
                phero.deposit(tx, ty, PheromoneChannel::Food, 60.0 * dt);
            }
            WorkerMode::SeekFood => {
                phero.deposit(tx, ty, PheromoneChannel::Explore, 20.0 * dt);
            }
            _ => {}
        }

        // Light walking animation
        vis.anim_t += dt;
        if vis.anim_t > 0.12 {
            vis.anim_t = 0.0;
            vis.anim_frame = (vis.anim_frame + 1) % 4;
        }
        vis.digging = brain.mode == WorkerMode::Dig
                   && brain.dig_phase == DigPhase::Mining;

        // Update sprite orientation from movement direction. Threshold is
        // generous so a stationary ant keeps the last facing instead of
        // flipping back and forth on tiny noise.
        if vel.0.x >  0.3 { vis.facing =  1; }
        else if vel.0.x < -0.3 { vis.facing = -1; }
    }
}

fn choose_mode(
    _e:    Entity,
    pos:   &Position,
    brain: &mut WorkerBrain,
    cargo: &mut Cargo,
    _grid: &TileGrid,
    phero: &PheromoneGrid,
    jobs:  &mut DigJobs,
    rng:   &mut StdRng,
) {
    let tx = pos.0.x as i32;
    let ty = pos.0.y as i32;

    // Highest-priority overrides first. These can interrupt a dig.
    // Top priority: alarm pheromone in the local area means a fight is
    // happening nearby — drop everything and rush in. *Including*
    // haulers and food carriers: under attack, the colony wants every
    // available worker, not just the unencumbered ones. Workers in
    // Dig mode also abandon their claim. The dropped cargo is the
    // cost of the swarm response.
    let alarm_here = phero.level(tx, ty, PheromoneChannel::Alarm);
    let alarm_neighbour = phero
        .strongest_neighbour(tx, ty, PheromoneChannel::Alarm, ALARM_TRIGGER_LEVEL)
        .is_some();
    if alarm_here > ALARM_TRIGGER_LEVEL || alarm_neighbour {
        release_dig(brain, jobs);
        cargo.debris = None;     // drop dirt to fight
        cargo.amount = 0;        // drop food to fight
        brain.haul_direction = 0;
        brain.haul_target_dist = 0;
        brain.mode = WorkerMode::FightBack;
        return;
    }

    if cargo.debris.is_some() {
        release_dig(brain, jobs);
        brain.haul_direction = 0;
        brain.mode = WorkerMode::DepositDebris;
        return;
    }
    if cargo.amount > 0 {
        release_dig(brain, jobs);
        brain.mode = WorkerMode::ReturnHome;
        return;
    }
    // Food in the immediate area? Switch to forage even before considering dig.
    if phero.level(tx, ty, PheromoneChannel::Food) > 4.0
        || phero.strongest_neighbour(tx, ty, PheromoneChannel::Food, 4.0).is_some()
    {
        release_dig(brain, jobs);
        brain.mode = WorkerMode::SeekFood;
        return;
    }

    // Stay committed to an in-progress dig — but ONLY if the claim still
    // points to a live slot. The slot table recycles claims after a TTL
    // (and on completion), and a worker holding a recycled handle is
    // walking forever toward a dig site that no longer exists. Without
    // this validity check the worker would re-enter `step_dig` every
    // frame, never call `tick_progress` (because it never reaches the
    // target), and never detect the stale claim — meanwhile the unclaimed
    // jobs nobody picks up pile up in the queue and tunnel growth stalls.
    if brain.mode == WorkerMode::Dig {
        if let Some(claim) = brain.dig_claim {
            if jobs.is_claim_valid(claim) {
                return; // genuinely committed to a live job
            }
            // Claim was recycled out from under us — drop it and re-pick.
            brain.dig_claim   = None;
            brain.dig_target  = None;
        }
    }

    // Already handled food via the high-priority block above.

    // Try a dig job aggressively — with no food in the world there isn't
    // much else productive to do, and the user reads "lots of ants idle"
    // as the simulator being broken.
    if rng.gen::<f32>() < 0.85 {
        if let Some((claim, info)) = jobs.claim_nearest(tx, ty) {
            brain.dig_claim  = Some(claim);
            brain.dig_target = Some((info.tx, info.ty));
            brain.dig_phase  = DigPhase::Approach;
            brain.mode       = WorkerMode::Dig;
            return;
        }
    }

    brain.mode = WorkerMode::Wander;
}

fn release_dig(brain: &mut WorkerBrain, jobs: &mut DigJobs) {
    if let Some(c) = brain.dig_claim.take() { jobs.release(c); }
    brain.dig_target = None;
    brain.dig_phase  = DigPhase::Approach;
}

fn step_wander(pos: &mut Position, vel: &mut Velocity, grid: &TileGrid, rng: &mut StdRng) {
    // Occasional brief pauses give the colony less marching-in-lockstep
    // feel — ants in real colonies don't all walk continuously, they stop
    // to look around, groom, antennate one another. This is a cheap proxy.
    if rng.gen::<f32>() < 0.012 {
        vel.0.x = 0.0; vel.0.y = 0.0;
        return;
    }
    let speed = ANT_SPEED * rng.gen_range(0.7..1.1);
    for _ in 0..6 {
        let nx = vel.0.x + rng.gen_range(-1.5..1.5);
        let ny = vel.0.y + rng.gen_range(-1.5..1.5);
        let mag = (nx*nx + ny*ny).sqrt().max(0.01);
        let dx = nx / mag * speed;
        let dy = ny / mag * speed;
        let probe_x = pos.0.x + dx.signum() * 0.6;
        let probe_y = pos.0.y + dy.signum() * 0.6;
        if grid.passable(probe_x as i32, probe_y as i32)
            && surface_adjacent(grid, probe_x as i32, probe_y as i32)
        {
            vel.0.x = dx; vel.0.y = dy;
            return;
        }
    }
    vel.0.x = rng.gen_range(-speed..speed);
    vel.0.y = rng.gen_range(-speed..speed);
}

#[inline]
fn surface_adjacent(grid: &TileGrid, tx: i32, ty: i32) -> bool {
    grid.get(tx + 1, ty).solid()
        || grid.get(tx - 1, ty).solid()
        || grid.get(tx, ty + 1).solid()
        || grid.get(tx, ty - 1).solid()
}

fn step_seek_food(
    pos: &mut Position, vel: &mut Velocity,
    phero: &PheromoneGrid, _grid: &TileGrid, rng: &mut StdRng,
) {
    let tx = pos.0.x as i32; let ty = pos.0.y as i32;
    if let Some((dx, dy)) = phero.strongest_neighbour(tx, ty, PheromoneChannel::Food, 1.0) {
        let mag = ((dx*dx + dy*dy) as f32).sqrt().max(0.01);
        vel.0.x = dx as f32 / mag * ANT_SPEED;
        vel.0.y = dy as f32 / mag * ANT_SPEED;
    } else {
        // Lost the trail — drift
        vel.0.x = rng.gen_range(-ANT_SPEED..ANT_SPEED);
        vel.0.y = rng.gen_range(-ANT_SPEED..ANT_SPEED);
    }
}

/// Charge in the direction of the strongest alarm pheromone — that points
/// toward the active fight. Combat damage itself is handled by the
/// shared `combat_step` system, so all this needs to do is steer.
fn step_fight_back(
    pos: &mut Position, vel: &mut Velocity,
    phero: &PheromoneGrid, rng: &mut StdRng,
) {
    let tx = pos.0.x as i32;
    let ty = pos.0.y as i32;
    let here = phero.level(tx, ty, PheromoneChannel::Alarm);
    if let Some((dx, dy)) = phero.strongest_neighbour(tx, ty,
        PheromoneChannel::Alarm, here.max(1.0))
    {
        let mag = ((dx*dx + dy*dy) as f32).sqrt().max(0.01);
        vel.0.x = dx as f32 / mag * ANT_SPEED;
        vel.0.y = dy as f32 / mag * ANT_SPEED;
    } else if here > 1.0 {
        // We're at the alarm peak — the hostile is right here. Stand
        // ground so combat_step can land hits, instead of random-walking
        // away from the very target we're meant to engage.
        vel.0.x = 0.0;
        vel.0.y = 0.0;
    } else {
        // No alarm anywhere — wander until something re-triggers.
        vel.0.x = rng.gen_range(-ANT_SPEED..ANT_SPEED);
        vel.0.y = rng.gen_range(-ANT_SPEED..ANT_SPEED);
    }
}

fn step_return(pos: &mut Position, vel: &mut Velocity, field: &ReturnFlowField) {
    let tx = pos.0.x as i32;
    let ty = pos.0.y as i32;
    let (dx, dy) = field.step(tx, ty);

    // Flow-field hit — take the precomputed shortest-path step toward
    // the entrance. Robust to arbitrarily winding tunnels because the
    // BFS routes through whatever passable network actually exists.
    if dx != 0 || dy != 0 {
        let mag = ((dx * dx + dy * dy) as f32).sqrt().max(0.01);
        vel.0.x = dx as f32 / mag * ANT_SPEED;
        vel.0.y = dy as f32 / mag * ANT_SPEED;
        return;
    }

    // Fallback for tiles outside the field (unreachable from the
    // entrance, or the entrance tile itself). Direct steering with an
    // upward bias underground — the field gets rebuilt every couple of
    // seconds, so this only matters for an ant standing on a freshly-
    // dug tile in the gap before the next rebuild.
    let target_dx = COLONY_X as f32 + 0.5 - pos.0.x;
    let target_dy = COLONY_Y as f32 + 0.5 - pos.0.y;
    let (vx, vy) = if pos.0.y > SURFACE_ROW as f32 + 1.0 {
        let hx = if target_dx.abs() > 0.5 { target_dx.signum() * 0.5 } else { 0.0 };
        let mag = (hx * hx + 1.0).sqrt();
        (hx / mag * ANT_SPEED, -1.0 / mag * ANT_SPEED)
    } else {
        let d = (target_dx * target_dx + target_dy * target_dy).sqrt().max(0.01);
        (target_dx / d * ANT_SPEED, target_dy / d * ANT_SPEED)
    };
    vel.0.x = vx;
    vel.0.y = vy;
}

fn step_dig(
    _e: Entity,
    pos:   &mut Position,
    vel:   &mut Velocity,
    brain: &mut WorkerBrain,
    vis:   &mut VisualState,
    cargo: &mut Cargo,
    grid:  &TileGrid,
    jobs:  &mut DigJobs,
    dt:    f32,
) {
    let Some(claim)  = brain.dig_claim   else { brain.mode = WorkerMode::Wander; return; };
    // Belt-and-suspenders: even mid-frame a worker may discover its claim
    // was recycled (e.g., another worker just completed a near-duplicate
    // job). Bail immediately so the worker can replan to something live
    // instead of walking toward a dead target.
    if !jobs.is_claim_valid(claim) {
        brain.dig_claim = None;
        brain.dig_target = None;
        brain.mode = WorkerMode::Wander;
        return;
    }
    let Some(target) = brain.dig_target  else { brain.mode = WorkerMode::Wander; return; };
    let (tx, ty) = target;
    let dx = tx as f32 + 0.5 - pos.0.x;
    let dy = ty as f32 + 0.5 - pos.0.y;
    let dist = (dx*dx + dy*dy).sqrt();

    if dist > 1.6 {
        // Approach: head toward an adjacent passable tile of the dig target
        if let Some((ax, ay)) = adjacent_passable(grid, tx, ty) {
            let adx = ax as f32 + 0.5 - pos.0.x;
            let ady = ay as f32 + 0.5 - pos.0.y;
            let ad  = (adx*adx + ady*ady).sqrt().max(0.01);
            vel.0.x = adx / ad * ANT_SPEED;
            vel.0.y = ady / ad * ANT_SPEED;
        } else {
            // No reachable approach — release and bail. With the slot
            // table the release is structurally clean: even if we don't
            // call it, the claim TTL will recycle.
            jobs.release(claim);
            brain.dig_claim = None;
            brain.dig_target = None;
            brain.mode = WorkerMode::Wander;
        }
        brain.dig_phase = DigPhase::Approach;
        vis.digging = false;
    } else {
        brain.dig_phase = DigPhase::Mining;
        vel.0.x = 0.0; vel.0.y = 0.0;
        match jobs.tick_progress(claim, dt) {
            Some(p) if p >= 1.0 => {
                if let Some((cx, cy)) = jobs.complete(claim) {
                    // Capture the tile type BEFORE the staged dig flushes —
                    // the worker is going to haul this material out as a
                    // pebble and dump it on the surface.
                    let dug_t = grid.get(cx, cy);
                    PENDING_TILE_OPS.with(|c| {
                        c.borrow_mut().push((cx, cy, TileType::Tunnel))
                    });
                    PENDING_DIG_EVENTS.with(|c| c.borrow_mut().push((cx, cy)));
                    cargo.debris = Some(dug_t);
                    brain.dig_claim = None;
                    brain.dig_target = None;
                    brain.mode = WorkerMode::DepositDebris;
                    brain.haul_direction = 0;
                    return;
                }
                brain.dig_claim = None;
                brain.dig_target = None;
                brain.mode = WorkerMode::Wander;
            }
            None => {
                brain.dig_claim = None;
                brain.dig_target = None;
                brain.mode = WorkerMode::Wander;
            }
            _ => {}
        }
    }
}

/// Hauler logic. Now that `dirt_physics::settle_above_ground` handles the
/// shape of the mound, ants don't need to plan placement — they just drop
/// dirt outside the colony and let physics organise it.
///
///   1. Underground → walk to the entrance (`step_return`).
///   2. Once on the surface, pick a direction and a random walk distance.
///   3. Once past the chosen distance, drop the pebble at the tile just
///      in front of us. Sand physics will fall and slide it into place.
///   4. Otherwise keep walking outward; the snap-up below lets us ride
///      over shallow piles, and the auto-climb in `movement.rs` covers
///      the rest.
fn step_deposit_debris(
    pos:   &mut Position,
    vel:   &mut Velocity,
    brain: &mut WorkerBrain,
    cargo: &mut Cargo,
    grid:  &TileGrid,
    field: &ReturnFlowField,
    rng:   &mut StdRng,
) {
    if cargo.debris.is_none() {
        brain.mode = WorkerMode::Wander;
        brain.haul_direction   = 0;
        brain.haul_target_dist = 0;
        brain.haul_stuck_time  = 0.0;
        return;
    }
    let Some(t) = cargo.debris else { return; };

    // (1) Underground? Climb back to the entrance via the flow field.
    if (pos.0.y as i32) > SURFACE_ROW {
        step_return(pos, vel, field);
        brain.haul_direction   = 0;
        brain.haul_target_dist = 0;
        brain.haul_stuck_time  = 0.0;
        let _ = grid;
        return;
    }

    // (2) Pick a direction + walk distance the first time we surface.
    if brain.haul_direction == 0 {
        let from_dx = pos.0.x - COLONY_X as f32;
        let prefer = if from_dx >= 0.0 { 1 } else { -1 };
        brain.haul_direction = if rng.gen::<f32>() < 0.3 { -prefer } else { prefer };
        brain.haul_target_dist = rng.gen_range(4i8..15);
        brain.haul_last_x = pos.0.x as i16;
        brain.haul_stuck_time = 0.0;
    }

    // Surface-snap up to the column's walkable top — also fires when
    // the ant is on the grass row itself (pos.y ~ 28.5), which is what
    // happens right as they emerge from the entrance shaft. Without
    // this, ants on grass never reach the air-above-grass band where
    // the drop tile sits, and they pile up at the entrance with cargo.
    let tx_now = pos.0.x as i32;
    if tx_now > 0 && tx_now < grid.width - 1 {
        let target_y = worker_surface_y(grid, tx_now) as f32 + 0.5;
        if target_y < pos.0.y {
            pos.0.y = target_y;
        }
    }

    let dir_initial = brain.haul_direction as i32;
    let dx_from_entrance = (pos.0.x as i32 - COLONY_X).abs();

    // (3) Stuck detection. If horizontal progress has stalled for a
    // couple of seconds, flip the haul direction — the ant has hit
    // something solid (a pile shoulder, a tree trunk, the world edge)
    // and needs to try the other side. This is what makes haulers
    // *endeavour* to deposit outside instead of giving up: they keep
    // pushing toward open ground until they find a drop spot.
    let cur_x_i = pos.0.x as i16;
    if (cur_x_i - brain.haul_last_x).abs() < 1 {
        brain.haul_stuck_time += 1.0 / 60.0;
        if brain.haul_stuck_time > 1.5 {
            brain.haul_direction = -brain.haul_direction;
            brain.haul_stuck_time = 0.0;
        }
    } else {
        brain.haul_stuck_time = 0.0;
        brain.haul_last_x = cur_x_i;
    }
    let dir = brain.haul_direction as i32;

    // (4) Far enough from the entrance? Drop the pebble in the air tile
    // immediately ahead at the ant's current y. The drop column itself
    // is also gated to be ≥ 3 tiles from the entrance — without this
    // the entrance corridor sealed itself off as drops accumulated next
    // to it and settling slid more soil into the same area.
    if dx_from_entrance >= brain.haul_target_dist as i32 {
        let nx = pos.0.x as i32 + dir;
        let ny = pos.0.y as i32;
        let safe_distance = (nx - COLONY_X).abs() >= 3;
        if safe_distance
            && ny >= 1 && ny < SURFACE_ROW
            && grid.get(nx, ny) == TileType::Air
        {
            PENDING_TILE_OPS.with(|c| c.borrow_mut().push((nx, ny, t)));
            PENDING_DROP_EVENTS.with(|c| *c.borrow_mut() += 1);
            cargo.debris = None;
            brain.haul_direction   = 0;
            brain.haul_target_dist = 0;
            brain.haul_stuck_time  = 0.0;
            brain.mode = WorkerMode::Wander;
            return;
        }
    }

    // (5) Walk outward. *Don't* reset vel.y here — when an ant walks
    // off the far side of a pile (where the next column's surface is
    // lower), it needs gravity to accumulate over multiple sub-steps so
    // it actually falls down to the lower surface. The previous code
    // zeroed vel.y every frame, so gravity never built up, and ants
    // ended up floating in the air at whatever altitude they last had
    // a snap target.
    vel.0.x = dir as f32 * ANT_SPEED;
    let _ = dir_initial;
}

/// The y of the topmost air tile in column `x` whose tile-below is solid
/// or grass — i.e. the surface a creature would walk on at that column.
fn worker_surface_y(grid: &TileGrid, x: i32) -> i32 {
    for y in 0..(grid.height - 1) {
        if grid.get(x, y) == TileType::Air {
            let below = grid.get(x, y + 1);
            if below.solid() || matches!(below, TileType::Grass) {
                return y;
            }
        }
    }
    SURFACE_ROW - 1
}


fn adjacent_passable(grid: &TileGrid, tx: i32, ty: i32) -> Option<(i32, i32)> {
    for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
        let nx = tx + dx; let ny = ty + dy;
        if grid.passable(nx, ny) { return Some((nx, ny)); }
    }
    None
}

// Cross-system staging for tile mutations. worker_ai holds `Res<TileGrid>`
// (read-only) so the dig completion / debris drop can't mutate inline; a
// follow-up exclusive-write system flushes the buffer.
thread_local! {
    static PENDING_TILE_OPS: std::cell::RefCell<Vec<(i32, i32, TileType)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static PENDING_DIG_EVENTS: std::cell::RefCell<Vec<(i32, i32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static PENDING_DROP_EVENTS: std::cell::RefCell<u32> =
        const { std::cell::RefCell::new(0) };
}

#[derive(Resource, Default)]
pub struct DigStats {
    pub total: u32,
    last_milestone: u32,
    pub pebbles_dropped: u32,
    last_drop_milestone: u32,
}

pub fn flush_pending_tile_ops(
    mut grid:  ResMut<TileGrid>,
    mut log:   ResMut<EventLog>,
    mut stats: ResMut<DigStats>,
) {
    PENDING_TILE_OPS.with(|c| {
        let mut v = c.borrow_mut();
        for (x, y, t) in v.drain(..) {
            // Tunnel ops use the existing `dig()` helper for variant clearing;
            // pebble drops go straight in via `set()`.
            if matches!(t, TileType::Tunnel) {
                grid.dig(x, y);
            } else {
                grid.set(x, y, t);
            }
        }
    });
    PENDING_DIG_EVENTS.with(|c| {
        let mut v = c.borrow_mut();
        let added = v.len() as u32;
        v.clear();
        if added == 0 { return; }
        stats.total += added;
        let bucket = stats.total / 25;
        if bucket > stats.last_milestone {
            stats.last_milestone = bucket;
            log.push(format!("Tunnels expanded — {} tiles dug", stats.total),
                     [0.78, 0.62, 0.30, 1.0]);
        }
    });
    // Pebble-drop counter — surfaces "yes, the dirt is being moved" by way
    // of a periodic alert and a population stat the bottom strip displays.
    PENDING_DROP_EVENTS.with(|c| {
        let added = std::mem::replace(&mut *c.borrow_mut(), 0);
        if added == 0 { return; }
        stats.pebbles_dropped += added;
        let bucket = stats.pebbles_dropped / 25;
        if bucket > stats.last_drop_milestone {
            stats.last_drop_milestone = bucket;
            log.push(format!("Dirt piled outside — {} loads hauled",
                             stats.pebbles_dropped),
                     [0.86, 0.66, 0.34, 1.0]);
        }
    });
}
