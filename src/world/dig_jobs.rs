//! Dig job slot table.
//!
//! The Python prototype lost work to leaked claims: an ant would claim a job,
//! fail to pathfind, abandon the action, and the queue would treat the claim
//! as still live forever. We sidestep that here by giving every slot a
//! generation counter and a TTL — a worker holds a `DigClaim { slot, gen }`,
//! and the queue rejects any operation whose generation no longer matches the
//! slot's current generation, or whose TTL has lapsed.

use bevy_ecs::prelude::*;
use rand::{SeedableRng, Rng, rngs::StdRng};
use crate::config::*;
use super::tiles::{TileGrid, TileType};

#[derive(Copy, Clone, Debug)]
pub struct DigClaim {
    pub slot: u32,
    pub gen:  u32,
}

#[derive(Copy, Clone, Debug)]
pub struct DigJobInfo {
    pub tx: i32,
    pub ty: i32,
    pub tile_type: TileType,
    pub progress:  f32,    // 0..1
    pub dig_seconds: f32,
}

#[derive(Copy, Clone, Debug)]
struct Slot {
    gen:        u32,
    occupied:   bool,
    info:       DigJobInfo,
    claimed:    bool,
    claim_ttl:  f32,
    completed:  bool,
}

impl Default for Slot {
    fn default() -> Self {
        Slot {
            gen: 0,
            occupied:  false,
            info: DigJobInfo {
                tx: 0, ty: 0,
                tile_type: TileType::Air,
                progress: 0.0, dig_seconds: 1.0,
            },
            claimed:   false,
            claim_ttl: 0.0,
            completed: false,
        }
    }
}

#[derive(Resource)]
pub struct DigJobs {
    slots: Vec<Slot>,
    rng:   StdRng,
    timer: f32,
}

/// Opaque snapshot used by the rewind buffer. Captures the full slot table +
/// timer so a rewind restores in-flight dig progress correctly.
#[derive(Clone)]
pub struct DigJobsSnapshot {
    slots: Vec<Slot>,
    timer: f32,
}

impl DigJobs {
    pub fn new(seed: u64) -> Self {
        Self {
            slots: vec![Slot::default(); EXPAND_MAX_QUEUE],
            rng: StdRng::seed_from_u64(seed.wrapping_add(999)),
            timer: EXPAND_INTERVAL * 0.5,
        }
    }

    pub fn snapshot(&self) -> DigJobsSnapshot {
        DigJobsSnapshot { slots: self.slots.clone(), timer: self.timer }
    }
    pub fn restore(&mut self, snap: &DigJobsSnapshot) {
        self.slots = snap.slots.clone();
        self.timer = snap.timer;
    }

    /// True if `claim` still refers to the slot it was minted from (same
    /// generation, slot still occupied and incomplete). Workers should call
    /// this whenever they're tempted to keep walking toward a target — an
    /// invalid claim means the slot's been recycled and the dig is dead.
    pub fn is_claim_valid(&self, claim: DigClaim) -> bool {
        self.slots.get(claim.slot as usize)
            .map_or(false, |s| s.occupied && !s.completed && s.gen == claim.gen)
    }

    pub fn unclaimed_count(&self) -> usize {
        self.slots.iter()
            .filter(|s| s.occupied && !s.claimed && !s.completed)
            .count()
    }

    pub fn occupied_count(&self) -> usize {
        self.slots.iter().filter(|s| s.occupied && !s.completed).count()
    }

    /// Iterate over all live job tile coords (for renderer markers).
    pub fn iter_jobs(&self) -> impl Iterator<Item = (i32, i32, f32, bool)> + '_ {
        self.slots.iter()
            .filter(|s| s.occupied && !s.completed)
            .map(|s| (s.info.tx, s.info.ty, s.info.progress, s.claimed))
    }

    /// Push a new job. Returns false if the queue is full.
    fn push(&mut self, tx: i32, ty: i32, tile_type: TileType) -> bool {
        // Skip duplicates
        for s in self.slots.iter() {
            if s.occupied && !s.completed && s.info.tx == tx && s.info.ty == ty {
                return false;
            }
        }
        for s in self.slots.iter_mut() {
            if !s.occupied || s.completed {
                s.gen = s.gen.wrapping_add(1);
                s.info = DigJobInfo {
                    tx, ty, tile_type,
                    progress: 0.0,
                    dig_seconds: tile_type.dig_seconds(),
                };
                s.occupied = true;
                s.claimed  = false;
                s.claim_ttl = 0.0;
                s.completed = false;
                return true;
            }
        }
        false
    }

    /// Claim the nearest unclaimed job. Returns the claim handle and the job
    /// info (so the worker doesn't have to look it up again).
    pub fn claim_nearest(&mut self, x: i32, y: i32) -> Option<(DigClaim, DigJobInfo)> {
        let mut best: Option<(usize, i64)> = None;
        for (i, s) in self.slots.iter().enumerate() {
            if !s.occupied || s.claimed || s.completed { continue; }
            let dx = (s.info.tx - x) as i64;
            let dy = (s.info.ty - y) as i64;
            let d2 = dx*dx + dy*dy;
            if best.map_or(true, |(_, b)| d2 < b) { best = Some((i, d2)); }
        }
        let (idx, _) = best?;
        let s = &mut self.slots[idx];
        s.claimed   = true;
        s.claim_ttl = DIG_CLAIM_TTL_S;
        Some((
            DigClaim { slot: idx as u32, gen: s.gen },
            s.info,
        ))
    }

    /// Add progress to a claimed job. The worker passes its claim handle and
    /// the queue verifies the generation matches. If it doesn't, the worker's
    /// claim has been recycled — it bails out gracefully.
    ///
    /// Returns Some(new_progress) on success; None if the claim is stale.
    pub fn tick_progress(&mut self, claim: DigClaim, dt: f32) -> Option<f32> {
        let s = self.slots.get_mut(claim.slot as usize)?;
        if !s.occupied || s.completed || s.gen != claim.gen { return None; }
        s.info.progress += dt / s.info.dig_seconds;
        s.claim_ttl = DIG_CLAIM_TTL_S;
        Some(s.info.progress)
    }

    /// Mark a job complete. Returns the tile coord to dig, or None if stale.
    pub fn complete(&mut self, claim: DigClaim) -> Option<(i32, i32)> {
        let s = self.slots.get_mut(claim.slot as usize)?;
        if !s.occupied || s.gen != claim.gen { return None; }
        s.completed = true;
        // Bump generation so any lingering claim is invalidated.
        s.gen = s.gen.wrapping_add(1);
        Some((s.info.tx, s.info.ty))
    }

    /// Voluntarily release a claim. No-op if stale.
    pub fn release(&mut self, claim: DigClaim) {
        if let Some(s) = self.slots.get_mut(claim.slot as usize) {
            if s.occupied && s.gen == claim.gen {
                s.claimed = false;
                s.claim_ttl = 0.0;
            }
        }
    }

    /// Tick the queue: TTL expiry on claims, slot recycling on completed jobs.
    pub fn tick(&mut self, dt: f32) {
        for s in self.slots.iter_mut() {
            if s.occupied && s.claimed {
                s.claim_ttl -= dt;
                if s.claim_ttl <= 0.0 {
                    // Auto-expire a stuck claim. Bump gen so the holder's
                    // claim is invalid; the slot becomes unclaimed again.
                    s.claimed = false;
                    s.gen = s.gen.wrapping_add(1);
                }
            }
            if s.completed {
                s.occupied = false;
            }
        }
    }
}

/// Colony director: scans the frontier (diggable tile adjacent to passable),
/// picks a handful weighted by depth, and pushes them to the queue. Runs once
/// per `EXPAND_INTERVAL`; cheap because the search is bounded by population
/// and the queue rejects duplicates.
pub fn director_update(
    mut grid: ResMut<TileGrid>,
    mut jobs: ResMut<DigJobs>,
    time: Res<crate::sim::Time>,
    pop:  Res<crate::sim::Population>,
) {
    let _ = &mut *grid; // grid not mutated, but we want exclusive access
    jobs.timer -= time.dt;
    jobs.tick(time.dt);
    if jobs.timer > 0.0 { return; }
    jobs.timer = EXPAND_INTERVAL;
    if jobs.occupied_count() >= EXPAND_MAX_QUEUE { return; }
    if pop.workers < 5 { return; }

    let candidates = frontier_candidates(&grid, &mut jobs.rng, pop.workers as i32);
    let mut added = 0;
    for (tx, ty) in candidates {
        if added >= EXPAND_JOBS_PER_PASS { break; }
        let t = grid.get(tx, ty);
        if !t.diggable() { continue; }
        if jobs.push(tx, ty, t) { added += 1; }
    }
}

fn frontier_candidates(grid: &TileGrid, rng: &mut StdRng, pop: i32) -> Vec<(i32, i32)> {
    let cx = COLONY_X;
    let cy = COLONY_Y;
    let target_depth = SURFACE_ROW
        + (5 + pop.min(60)).min(grid.height - SURFACE_ROW - 5);
    let depth_weight = (pop as f32 / 30.0).min(2.5);
    let max_dist     = 60 + pop * 2;

    // Sparse scan: walk every passable tile and check for diggable neighbours.
    // For our world sizes this finishes in a fraction of a millisecond.
    let mut out: Vec<((i32,i32,i32), f32)> = Vec::with_capacity(256);
    for y in (SURFACE_ROW + 1)..(grid.height - 1) {
        for x in 1..(grid.width - 1) {
            let i = grid.idx(x, y);
            let t = TileType::from_u8(grid.tiles[i]);
            if !t.passable() { continue; }
            // Check 4-neighbours for diggable
            for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                let nx = x + dx; let ny = y + dy;
                if !grid.in_bounds(nx, ny) { continue; }
                let nt = grid.get(nx, ny);
                if !nt.diggable() { continue; }
                let dist  = (nx - cx).abs() + (ny - cy).abs();
                if dist > max_dist { continue; }
                let depth_diff = (ny - target_depth).abs() as f32;
                let jitter     = rng.gen_range(0.0..3.0);
                let score      = dist as f32 * 0.4 + depth_diff * depth_weight + jitter;
                out.push(((nx, ny, dist), score));
            }
        }
    }
    out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    out.into_iter().take(64).map(|((x,y,_),_)| (x,y)).collect()
}
