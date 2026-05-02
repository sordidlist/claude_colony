//! Debug inspector. Shows a popup describing a creature's full state
//! (health, cargo, AI mode, recent decisions) when the user holds Tab
//! and hovers the mouse over it. Works on workers, soldiers, queens,
//! spiders and rivals — anything carrying an `AiTrace` component.
//!
//! The picker uses a generous radius (~2 tiles in world space) so the
//! user doesn't have to land the cursor exactly on a 1-tile sprite,
//! and it ignores the camera zoom because the radius is in world
//! coordinates, not pixels.
//!
//! `draw_inspector` is the only entry point. Both binaries call it
//! every frame; it short-circuits when Tab isn't held.

use macroquad::prelude::*;
use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;
use crate::sim::components::*;
use crate::sim::scenery::Decoration;
use super::Camera;

/// Pixel padding inside the panel.
const PAD: f32 = 12.0;
/// Maximum number of trace lines shown (matches `AI_TRACE_CAPACITY`).
const TRACE_LINES: usize = 10;

/// Draw the inspector panel if Tab is held and the mouse is over a
/// supported creature. No-op otherwise. Call once per frame, after
/// the world has been drawn so the panel sits on top.
pub fn draw_inspector(world: &World, cam: &Camera) {
    if !is_key_down(KeyCode::Tab) { return; }

    let (msx, msy) = mouse_position();
    let (mwx, mwy) = cam.screen_to_world(msx, msy);

    let Some(target) = nearest_creature(world, mwx, mwy, 2.0) else {
        // Tab held but nothing to inspect — show a small hint.
        draw_hint(msx, msy);
        return;
    };

    // Highlight ring on the target so the user can see what they
    // selected, especially in a crowd.
    if let Some(p) = world.entity(target).get::<Position>() {
        let (sx, sy) = cam.world_to_screen(p.0.x, p.0.y);
        draw_circle_lines(sx, sy, cam.zoom * 0.9, 2.0,
            Color::new(0.96, 0.86, 0.30, 0.85));
        draw_circle_lines(sx, sy, cam.zoom * 1.1, 1.0,
            Color::new(0.96, 0.86, 0.30, 0.45));
    }

    draw_panel(world, target, msx, msy);
}

/// Find the entity with `AiTrace` (or, failing that, with `Brood` /
/// `Decoration` for the mower) closest to (wx, wy) within `radius`.
/// Returns None if nothing matches.
fn nearest_creature(world: &World, wx: f32, wy: f32, radius: f32) -> Option<Entity> {
    let r2 = radius * radius;
    let mut best: Option<(Entity, f32)> = None;
    for e in world.iter_entities() {
        // Only entities the inspector knows how to render are valid
        // pick targets. AiTrace covers ants, soldiers, queens,
        // spiders, rivals. Decoration::Mower is a special-case
        // surface entity worth picking too.
        let inspectable =
            e.contains::<AiTrace>()
            || e.get::<Decoration>().map_or(false, |d|
                matches!(d.kind, crate::sim::scenery::DecorKind::Mower));
        if !inspectable { continue; }
        let p = if let Some(p) = e.get::<Position>() {
            (p.0.x, p.0.y)
        } else if let Some(p) = e.get::<crate::sim::scenery::DecorPos>() {
            // Mower's anchor is its top-left; use sprite centre for
            // distance so the picker hits the body, not just the
            // upper-left corner pixel.
            (p.x + 2.0, p.y + 1.0)
        } else { continue; };
        let dx = p.0 - wx;
        let dy = p.1 - wy;
        let d2 = dx*dx + dy*dy;
        if d2 > r2 { continue; }
        if best.map_or(true, |(_, b)| d2 < b) {
            best = Some((e.id(), d2));
        }
    }
    best.map(|(e, _)| e)
}

fn draw_panel(world: &World, target: Entity, mouse_x: f32, mouse_y: f32) {
    let lines = build_lines(world, target);

    // Layout. Width fixed; height grows with line count.
    let w = 360.0;
    let line_h = 18.0;
    let header_h = 28.0;
    let h = header_h + line_h * lines.body.len() as f32 + PAD * 2.0;

    // Anchor near the cursor, but clamp so the panel never spills
    // off-screen.
    let anchor_dx = 18.0;
    let anchor_dy = 18.0;
    let mut x = mouse_x + anchor_dx;
    let mut y = mouse_y + anchor_dy;
    if x + w > screen_width()  { x = mouse_x - anchor_dx - w; }
    if y + h > screen_height() { y = mouse_y - anchor_dy - h; }
    x = x.max(4.0);
    y = y.max(4.0);

    let body   = Color::new(0.06, 0.05, 0.04, 0.92);
    let shadow = Color::new(0.0, 0.0, 0.0, 0.55);
    let border = Color::new(lines.accent[0], lines.accent[1], lines.accent[2], 1.0);
    let fg     = Color::new(0.96, 0.92, 0.78, 1.0);
    let dim    = Color::new(0.72, 0.68, 0.58, 1.0);

    draw_rectangle(x + 3.0, y + 3.0, w, h, shadow);
    draw_rectangle(x, y, w, h, body);
    draw_rectangle_lines(x, y, w, h, 2.0, border);

    // Title bar
    draw_rectangle(x, y, w, header_h, Color::new(border.r * 0.35,
                                                 border.g * 0.35,
                                                 border.b * 0.35, 1.0));
    draw_text(&lines.title, x + PAD, y + 20.0, 22.0, fg);

    let mut row_y = y + header_h + PAD + 12.0;
    for line in &lines.body {
        let col = if line.is_dim { dim } else { fg };
        draw_text(&line.text, x + PAD, row_y, 16.0, col);
        row_y += line_h;
    }
}

fn draw_hint(mouse_x: f32, mouse_y: f32) {
    let txt = "Tab: hover a creature to inspect";
    let tw = measure_text(txt, None, 14, 1.0).width;
    let pad = 6.0;
    let w = tw + pad * 2.0;
    let h = 22.0;
    let x = mouse_x + 18.0;
    let y = mouse_y + 18.0;
    draw_rectangle(x, y, w, h, Color::new(0.05, 0.05, 0.06, 0.85));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(0.62, 0.46, 0.22, 1.0));
    draw_text(txt, x + pad, y + 15.0, 14.0,
              Color::new(0.92, 0.86, 0.72, 1.0));
}

// ── line builder ────────────────────────────────────────────────────

struct InspectorLines {
    title:  String,
    accent: [f32; 3],
    body:   Vec<Line>,
}

struct Line {
    text:   String,
    is_dim: bool,
}

fn line(text: impl Into<String>) -> Line {
    Line { text: text.into(), is_dim: false }
}
fn dim(text: impl Into<String>) -> Line {
    Line { text: text.into(), is_dim: true }
}

fn build_lines(world: &World, target: Entity) -> InspectorLines {
    let e = world.entity(target);

    // Identify what kind of thing we're looking at, set the title
    // and accent colour from that, then pile on the kind-specific
    // detail lines.
    let (title, accent) = title_and_accent(&e, target);
    let mut body = Vec::with_capacity(16);

    // Common: position, velocity, health.
    if let Some(p) = e.get::<Position>() {
        body.push(line(format!(
            "Pos: ({:.1}, {:.1})   tile ({}, {})",
            p.0.x, p.0.y, p.0.x as i32, p.0.y as i32)));
    }
    if let Some(v) = e.get::<Velocity>() {
        body.push(line(format!("Vel: ({:+.2}, {:+.2})", v.0.x, v.0.y)));
    }
    if let Some(h) = e.get::<Health>() {
        body.push(line(format!(
            "HP:  {:>4.1} / {:>4.1}   {}",
            h.hp.max(0.0), h.max_hp, hp_bar(h.hp, h.max_hp))));
    }

    // Worker / soldier / queen specifics.
    if let Some(ant) = e.get::<Ant>() {
        match ant.kind {
            AntKind::Worker => {
                if let Some(c) = e.get::<Cargo>() {
                    body.push(line(format!("Cargo: {}", cargo_string(c))));
                }
                if let Some(b) = e.get::<WorkerBrain>() {
                    body.push(dim(""));
                    body.push(line(format!("AI mode: {:?}", b.mode)));
                    if matches!(b.mode, WorkerMode::Dig) {
                        if let Some((tx, ty)) = b.dig_target {
                            body.push(dim(format!("  target tile ({}, {})", tx, ty)));
                        }
                        if b.dig_claim.is_some() {
                            body.push(dim(format!("  phase: {:?}", b.dig_phase)));
                        } else {
                            body.push(dim("  (no live claim)"));
                        }
                    }
                    if matches!(b.mode, WorkerMode::DepositDebris) {
                        body.push(dim(format!(
                            "  haul {} dist {} (stuck {:.1}s)",
                            if b.haul_direction >= 0 { "→" } else { "←" },
                            b.haul_target_dist, b.haul_stuck_time)));
                    }
                    body.push(dim(format!("  next replan in {:.2}s", b.replan_in)));
                }
            }
            AntKind::Soldier => {
                if let Some(s) = e.get::<SoldierAi>() {
                    body.push(dim(""));
                    if s.patrol_target.is_none() {
                        body.push(line("AI mode: Chasing enemy"));
                    } else {
                        body.push(line("AI mode: Patrol"));
                        if let Some((tx, ty)) = s.patrol_target {
                            body.push(dim(format!("  target ({:.1}, {:.1})", tx, ty)));
                        }
                        body.push(dim(format!(
                            "  retarget in {:.2}s", s.patrol_timer)));
                    }
                }
            }
            AntKind::Queen => {
                if let Some(q) = e.get::<QueenState>() {
                    body.push(dim(""));
                    body.push(line(format!("Eggs laid: {}", q.eggs_laid)));
                    body.push(dim(format!(
                        "  next egg in {:.2}s",
                        crate::config::QUEEN_EGG_INTERVAL_S - q.egg_timer)));
                    body.push(dim(format!(
                        "Migrations: {} (next check in {:.0}s)",
                        q.migrations, q.migration_timer)));
                }
            }
        }
    }

    // Spider / rival specifics.
    if let Some(s) = e.get::<Spider>() {
        body.push(dim(""));
        body.push(line("Predator: Spider"));
        body.push(dim(format!("  heading reset in {:.2}s", s.heading_timer)));
    }
    if let Some(r) = e.get::<RivalAnt>() {
        body.push(dim(""));
        body.push(line("Hostile: Rival ant"));
        body.push(dim(format!("  heading reset in {:.2}s", r.heading_timer)));
    }

    // Attacker (if any) — handy to see weapon tuning while debugging.
    if let Some(a) = e.get::<Attacker>() {
        body.push(dim(format!(
            "  atk: dmg {:.1}, range {:.1}, cd {:.1}s (timer {:.2}s)",
            a.damage, a.range, a.cooldown, a.timer)));
    }

    // Mower has no AiTrace; skip the trace section if no AiTrace exists.
    if let Some(t) = e.get::<AiTrace>() {
        body.push(dim(""));
        body.push(line(format!("Recent decisions ({}):", t.entries.len())));
        let take = TRACE_LINES.min(t.entries.len());
        for entry in t.entries.iter().rev().take(take) {
            body.push(dim(format!("  t={:>6.2}  {}", entry.time, entry.text)));
        }
        if t.entries.is_empty() {
            body.push(dim("  (nothing yet)"));
        }
    } else if let Some(_d) = e.get::<Decoration>() {
        // Mower-only block — surfaces lifecycle state via the schedule.
        body.push(dim(""));
        body.push(line("Lawn mower"));
        let phase = world.resource::<crate::sim::scenery::MowerSchedule>().phase;
        match phase {
            crate::sim::scenery::MowerPhase::Active(laps) =>
                body.push(dim(format!("  laps remaining: {}", laps))),
            crate::sim::scenery::MowerPhase::Cooldown(s) =>
                body.push(dim(format!("  cooldown: {:.0}s", s))),
        }
    }

    InspectorLines { title, accent, body }
}

fn title_and_accent(e: &bevy_ecs::world::EntityRef, ent: Entity) -> (String, [f32; 3]) {
    if let Some(ant) = e.get::<Ant>() {
        return match ant.kind {
            AntKind::Worker  => (format!("Worker ant #{}",  ent.index()), [0.86, 0.62, 0.96]),
            AntKind::Soldier => (format!("Soldier ant #{}", ent.index()), [0.96, 0.78, 0.36]),
            AntKind::Queen   => (format!("Queen #{}",       ent.index()), [0.78, 0.46, 0.94]),
        };
    }
    if e.contains::<Spider>()   { return (format!("Spider #{}", ent.index()),    [0.66, 0.42, 0.92]); }
    if e.contains::<RivalAnt>() { return (format!("Rival ant #{}", ent.index()), [0.96, 0.36, 0.30]); }
    if let Some(_d) = e.get::<Decoration>() {
        return (format!("Mower #{}", ent.index()), [0.96, 0.84, 0.30]);
    }
    (format!("Entity #{}", ent.index()), [0.62, 0.62, 0.62])
}

fn cargo_string(c: &Cargo) -> String {
    match (c.debris, c.amount) {
        (Some(t), 0) => format!("{:?} pebble", t),
        (None, 0)    => "empty".to_string(),
        (None, n)    => format!("food x{}", n),
        (Some(t), n) => format!("{:?} pebble + food x{}", t, n),
    }
}

fn hp_bar(hp: f32, max_hp: f32) -> String {
    let frac = (hp / max_hp).clamp(0.0, 1.0);
    let cells = 12;
    let filled = (frac * cells as f32).round() as usize;
    let empty  = cells - filled;
    let mut s = String::with_capacity(cells + 2);
    s.push('[');
    for _ in 0..filled { s.push('#'); }
    for _ in 0..empty  { s.push('.'); }
    s.push(']');
    s
}
