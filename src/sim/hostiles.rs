//! Predator (spider) and rival-colony (red ant) AI.
//! Both use the existing movement system, so gravity + tile collision come
//! for free; this module just sets a wandering velocity and updates sprite
//! orientation. Combat lands in a follow-up.

use bevy_ecs::prelude::*;
use glam::Vec2;
use rand::{Rng, SeedableRng, rngs::StdRng};
use crate::config::*;
use crate::world::{TileGrid, PheromoneGrid, PheromoneChannel};
use super::components::*;
use super::{Time, EventLog};

const SPIDER_SPEED: f32 = 3.2;
const RIVAL_SPEED:  f32 = 4.0;

/// Hostiles continuously emit alarm pheromone at their tile so nearby
/// colony workers detect them and swarm via the existing FightBack
/// behaviour. Spiders are formidable 1-on-1 (worker damage 1.5, spider
/// damage 3.0; spider hp 22 vs worker hp 14) but a coordinated swarm
/// of workers wins. This is the natural resolution to "spiders cluster
/// at the entrance and never get killed" — workers just fight them.
pub fn hostile_alarm_emission(
    mut phero: ResMut<PheromoneGrid>,
    spiders:   Query<&Position, With<Spider>>,
    rivals:    Query<&Position, With<RivalAnt>>,
) {
    // Spread the alarm into a 3×3 area around each hostile so workers
    // 1-2 tiles away are pulled in too, not just ones standing on the
    // spider's exact tile. This is the propagation that turns a 1v1
    // skirmish into the swarm response.
    let r = ALARM_EMISSION_HALF_WIDTH;
    for pos in spiders.iter() {
        let tx = pos.0.x as i32;
        let ty = pos.0.y as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                phero.deposit(tx + dx, ty + dy,
                              PheromoneChannel::Alarm, 50.0);
            }
        }
    }
    for pos in rivals.iter() {
        let tx = pos.0.x as i32;
        let ty = pos.0.y as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                phero.deposit(tx + dx, ty + dy,
                              PheromoneChannel::Alarm, 40.0);
            }
        }
    }
}

pub fn spider_tick(
    time: Res<Time>,
    grid: Res<TileGrid>,
    // Colony ants are what spiders hunt — `With<Ant>` covers
    // workers, soldiers, and the queen, all of whom are valid prey.
    // Hostiles like rival ants don't have `Ant` so they're excluded.
    prey: Query<&Position, (With<Ant>, Without<Spider>)>,
    mut q: Query<(&Position, &mut Velocity, &mut Spider, &mut VisualState,
                  &mut AiTrace)>,
) {
    if time.dt <= 0.0 { return; }
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0xA37D_891B_2467_5C13);

    // Snapshot prey positions once per frame. Cheap with our
    // population size; reused per spider below.
    let prey_pos: Vec<Vec2> = prey.iter().map(|p| p.0).collect();
    let hunt_r2 = SPIDER_HUNT_RADIUS * SPIDER_HUNT_RADIUS;

    for (pos, mut vel, mut s, mut vis, mut trace) in q.iter_mut() {
        // Post-kill retreat — overrides the random-walk loop. The
        // spider drives away from the colony entrance at a boosted
        // speed for `SPIDER_RETREAT_AFTER_KILL_S` sim seconds, then
        // resumes hunting.
        if s.retreat_timer > 0.0 {
            s.retreat_timer -= time.dt;
            let dx = pos.0.x - COLONY_X as f32;
            let dy = pos.0.y - COLONY_Y as f32 + 6.0; // bias slightly downward
            let mag = (dx*dx + dy*dy).sqrt().max(0.01);
            vel.0.x = dx / mag * SPIDER_SPEED * SPIDER_RETREAT_SPEED_MULT;
            vel.0.y = dy / mag * SPIDER_SPEED * SPIDER_RETREAT_SPEED_MULT;
            // Animate facing + frames same as the normal branch.
            vis.anim_t += time.dt;
            if vis.anim_t > 0.16 {
                vis.anim_t = 0.0;
                vis.anim_frame ^= 1;
            }
            if vel.0.x >  0.2 { vis.facing =  1; }
            else if vel.0.x < -0.2 { vis.facing = -1; }
            if s.retreat_timer <= 0.0 {
                trace.record(time.total, "Returning to hunt");
            }
            continue;
        }

        // Active hunting — find the nearest colony ant inside the
        // hunt radius and drive straight at it. Overrides the random
        // heading. This is what stops spiders from "flooding the
        // colony but doing nothing once there": once they get close
        // to ants, they actively chase them.
        let mut nearest: Option<(Vec2, f32)> = None;
        for tp in &prey_pos {
            let dx = tp.x - pos.0.x;
            let dy = tp.y - pos.0.y;
            let d2 = dx*dx + dy*dy;
            if d2 < hunt_r2 && nearest.map_or(true, |(_, b)| d2 < b) {
                nearest = Some((*tp, d2));
            }
        }
        if let Some((tp, d2)) = nearest {
            let dx = tp.x - pos.0.x;
            let dy = tp.y - pos.0.y;
            let mag = (dx*dx + dy*dy).sqrt().max(0.01);
            vel.0.x = dx / mag * SPIDER_SPEED;
            vel.0.y = dy / mag * SPIDER_SPEED;
            // Re-arm the random-heading clock so we don't snap back
            // to wandering as soon as we're between targets.
            s.heading_timer = 0.5;
            // Only trace on a fresh acquisition (very-close targets
            // would otherwise spam the trace each frame).
            if d2 > 4.0 {
                trace.record(time.total, "Hunting");
            }
            vis.anim_t += time.dt;
            if vis.anim_t > 0.16 {
                vis.anim_t = 0.0;
                vis.anim_frame ^= 1;
            }
            if vel.0.x >  0.2 { vis.facing =  1; }
            else if vel.0.x < -0.2 { vis.facing = -1; }
            continue;
        }

        s.heading_timer -= time.dt;
        if s.heading_timer <= 0.0 {
            s.heading_timer = rng.gen_range(1.2..2.6);
            if rng.gen::<f32>() < 0.10 {
                vel.0.x = 0.0; vel.0.y = 0.0;
                trace.record(time.total, "Pause");
            } else {
                let angle = rng.gen_range(0.0_f32..std::f32::consts::TAU);
                vel.0.x = angle.cos() * SPIDER_SPEED;
                vel.0.y = angle.sin() * SPIDER_SPEED;
                trace.record(time.total,
                    format!("New heading {}", compass(vel.0.x, vel.0.y)));
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
    mut q: Query<(&Position, &mut Velocity, &mut RivalAnt, &mut VisualState,
                  &mut AiTrace)>,
) {
    if time.dt <= 0.0 { return; }
    let mut rng = StdRng::seed_from_u64(
        (time.total * 1000.0) as u64 ^ 0x53F8_AA12_99C7_3142);
    for (pos, mut vel, mut r, mut vis, mut trace) in q.iter_mut() {
        r.heading_timer -= time.dt;
        if r.heading_timer <= 0.0 {
            r.heading_timer = rng.gen_range(1.0..3.0);
            // Drift toward colony entrance most of the time — rivals are
            // the colony's natural threat so they should *trend* toward it.
            let dx_to_colony = (COLONY_X as f32 + 0.5 - pos.0.x).signum();
            let toward = rng.gen::<f32>() < 0.7;
            let bias = if toward { dx_to_colony } else {
                if rng.gen::<bool>() { 1.0 } else { -1.0 }
            };
            vel.0.x = bias * RIVAL_SPEED;
            vel.0.y = rng.gen_range(-1.0..1.0);
            trace.record(time.total,
                format!("New heading {} ({})",
                    compass(vel.0.x, vel.0.y),
                    if toward { "advance" } else { "drift" }));
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

/// Eight-point compass label for an x/y velocity vector. Used by the
/// inspector trace so a heading change reads "New heading NE" instead
/// of dumping raw floats.
fn compass(vx: f32, vy: f32) -> &'static str {
    if vx.abs() < 0.05 && vy.abs() < 0.05 { return "·"; }
    let ang = vy.atan2(vx);
    let pi = std::f32::consts::PI;
    let octants = [
        ( 0.0,        "E"),
        ( pi * 0.25,  "SE"),
        ( pi * 0.5,   "S"),
        ( pi * 0.75,  "SW"),
        ( pi,         "W"),
        (-pi * 0.75,  "NW"),
        (-pi * 0.5,   "N"),
        (-pi * 0.25,  "NE"),
    ];
    let mut best = "E";
    let mut best_d = f32::MAX;
    for (a, lbl) in &octants {
        let mut d = (ang - a).abs();
        if d > pi { d = 2.0 * pi - d; }
        if d < best_d { best_d = d; best = lbl; }
    }
    best
}

// ─── invader spawner ────────────────────────────────────────────────

/// Tracks the off-screen hostile arrival schedule. Each tick of the
/// timer triggers a *wave* — a small group of invaders chosen by
/// `pick_wave` from the current `wave_count` and the live colony
/// population. Waves escalate from 1-2 rival workers to mixed
/// soldier/spider groups as the colony matures, then continue to
/// scale with population so a 1000-ant nest sees real threats and
/// not just the same trickle as a 10-ant founding.
#[derive(Resource)]
pub struct InvaderSpawner {
    pub timer: f32,
    pub wave_count: u32,
    rng: StdRng,
}

impl InvaderSpawner {
    pub fn new(seed: u64) -> Self {
        Self {
            timer:      INVADER_FIRST_SPAWN_S,
            wave_count: 0,
            rng:        StdRng::seed_from_u64(seed.wrapping_add(0xCAFE_F00D)),
        }
    }
}

/// Composition of one invader wave.
#[derive(Copy, Clone, Debug, Default)]
struct WaveComp {
    workers:  u8,
    soldiers: u8,
    spiders:  u8,
}

/// Pick a wave's composition based on the wave index (escalation
/// curve) and the current colony population (scale with strength).
/// The progression is deliberate:
///
///   wave 1     — 1-2 rival workers, no soldiers, no spiders.
///   wave 2     — 2-3 rival workers, occasional soldier.
///   wave 3     — 2-3 workers + 1 soldier; first spider possible.
///   wave 4     — first definite spider, plus mixed rivals.
///   wave 5+    — scaled by colony pop (stronger waves for bigger
///                colonies), interesting compositions.
fn pick_wave(wave_n: u32, colony_pop: u32, mult: f32, rng: &mut StdRng) -> WaveComp {
    let scale = |x: f32| ((x * mult).max(0.0).round() as u32).min(255) as u8;
    match wave_n {
        1 => WaveComp {
            workers:  scale(rng.gen_range(1.0..=2.0)),
            soldiers: 0,
            spiders:  0,
        },
        2 => WaveComp {
            workers:  scale(rng.gen_range(2.0..=3.0)),
            soldiers: if rng.gen::<f32>() < 0.4 { scale(1.0) } else { 0 },
            spiders:  0,
        },
        3 => WaveComp {
            workers:  scale(rng.gen_range(2.0..=3.0)),
            soldiers: scale(1.0),
            spiders:  if rng.gen::<f32>() < 0.5 { 1 } else { 0 },
        },
        4 => WaveComp {
            workers:  scale(rng.gen_range(2.0..=4.0)),
            soldiers: scale(rng.gen_range(1.0..=2.0)),
            spiders:  1,
        },
        _ => {
            // Scale further by colony population — bigger nest, bigger
            // raid. `pop_scale` ramps from 1.0 at 50 workers to 4.0 at
            // 800+, so a small colony still gets manageable waves
            // while a thousand-ant nest sees real threats.
            let pop_scale = ((colony_pop as f32 - 50.0) / 200.0)
                                .clamp(0.0, 4.0) + 1.0;
            WaveComp {
                workers:  scale(rng.gen_range(3.0..=6.0) * pop_scale),
                soldiers: scale(rng.gen_range(1.0..=3.0) * pop_scale),
                spiders:  rng.gen_range(1..=3),
            }
        }
    }
}

pub fn spawn_invaders(
    time: Res<Time>,
    balance: Res<super::balance::BalanceTunables>,
    pop: Res<super::Population>,
    mut log: ResMut<EventLog>,
    mut spawner: ResMut<InvaderSpawner>,
    mut commands: Commands,
) {
    if time.dt <= 0.0 { return; }
    spawner.timer -= time.dt;
    if spawner.timer > 0.0 { return; }

    let interval = balance.invader_interval.max(5.0);
    spawner.timer = interval
        + spawner.rng.gen_range(-INVADER_SPAWN_JITTER_S..INVADER_SPAWN_JITTER_S)
            .min(interval * 0.5);
    spawner.wave_count = spawner.wave_count.saturating_add(1);

    let from_left = spawner.rng.gen::<bool>();
    let edge_x = if from_left { 2 } else { WORLD_WIDTH - 3 };
    let direction = if from_left { "west" } else { "east" };

    let mult = balance.invader_wave_mult.max(0.1);
    let comp = pick_wave(spawner.wave_count, pop.workers as u32, mult,
                         &mut spawner.rng);

    // Stagger spawn x by 1 tile per invader so they don't all land
    // on top of each other and look like a single creature.
    let mut idx: i32 = 0;
    let pos_for = |i: i32| {
        let dx = if from_left { i } else { -i };
        Vec2::new((edge_x + dx) as f32 + 0.5,
                  (SURFACE_ROW - 1) as f32 + 0.5)
    };

    for _ in 0..comp.workers {
        commands.spawn(rival_bundle(pos_for(idx), RivalKind::Worker));
        idx += 1;
    }
    for _ in 0..comp.soldiers {
        commands.spawn(rival_bundle(pos_for(idx), RivalKind::Soldier));
        idx += 1;
    }
    for _ in 0..comp.spiders {
        commands.spawn(spider_bundle(pos_for(idx)));
        idx += 1;
    }

    let total = comp.workers as u32 + comp.soldiers as u32 + comp.spiders as u32;
    let banner_color = if comp.spiders > 0 {
        [0.66, 0.42, 0.92, 1.0]
    } else if comp.soldiers > 0 {
        [0.96, 0.50, 0.30, 1.0]
    } else {
        [0.96, 0.36, 0.30, 1.0]
    };
    log.push(
        format!(
            "Wave {} from the {}: {} rival worker{}{}{}",
            spawner.wave_count, direction,
            comp.workers,
            if comp.workers == 1 { "" } else { "s" },
            if comp.soldiers > 0 {
                format!(", {} soldier{}",
                        comp.soldiers,
                        if comp.soldiers == 1 { "" } else { "s" })
            } else { String::new() },
            if comp.spiders > 0 {
                format!(", {} spider{}",
                        comp.spiders,
                        if comp.spiders == 1 { "" } else { "s" })
            } else { String::new() },
        ),
        banner_color,
    );
    let _ = total;
}

fn rival_bundle(pos: Vec2, kind: RivalKind) -> impl Bundle {
    // Soldier rivals are tougher and hit harder than worker rivals
    // — a soldier should be the "save your soldiers for this one"
    // tier of threat, not just a worker reskin.
    let (hp, dmg, range, cd) = match kind {
        RivalKind::Worker  => (8.0,  2.0, 1.3, 0.7),
        RivalKind::Soldier => (16.0, 3.2, 1.4, 0.8),
    };
    (
        Position(pos),
        Velocity(Vec2::ZERO),
        Health { hp, max_hp: hp },
        FactionTag(Faction::Rival),
        RivalAnt { heading_timer: 0.0, kind },
        Attacker::new(dmg, range, cd),
        VisualState::default(),
        AiTrace::default(),
    )
}

fn spider_bundle(pos: Vec2) -> impl Bundle {
    (
        Position(pos),
        Velocity(Vec2::ZERO),
        Health { hp: SPIDER_HP, max_hp: SPIDER_HP },
        FactionTag(Faction::Predator),
        Spider::default(),
        Attacker::new(SPIDER_ATTACK_DAMAGE, SPIDER_ATTACK_RANGE,
                      SPIDER_ATTACK_COOLDOWN),
        VisualState::default(),
        AiTrace::default(),
    )
}
