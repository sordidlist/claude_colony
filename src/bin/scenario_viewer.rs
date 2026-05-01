//! Visual scenario runner.
//!
//! Headless tests run as fast as the CPU allows (a 60-frame sim second
//! takes microseconds). This binary drives the same scenarios through
//! the live renderer at wall-clock 1× so you can *see* what the test
//! is actually checking — useful when debugging an AI change or
//! tuning numbers.
//!
//! Controls:
//!   Space            pause / unpause
//!   ] / [            cycle fast-forward speed (1×, 2×, 4×, 10×, 100×)
//!   Backspace (hold) rewind through buffered history
//!   R                reset the scenario to its initial state
//!   N                advance to the next scenario in the registry
//!   P                go back one scenario
//!   Tab              toggle the scenario list overlay
//!   WASD / arrows    pan the camera
//!   Mouse wheel      zoom
//!   Esc / Q          quit
//!
//! Usage:
//!   cargo run --release --bin scenario_viewer
//!     → opens with the first scenario in the registry
//!   cargo run --release --bin scenario_viewer -- --scenario=hauler_falls_off_pile
//!   cargo run --release --bin scenario_viewer -- --list
//!     → prints scenario names and exits
//!
//! Each scenario shows an overlay with its name, description, the
//! pass/fail predicate's current state, and how much sim time has
//! elapsed.

use macroquad::prelude::*;
use bevy_ecs::world::World;
use colony::config::*;
use colony::render::{self, Atlas, Camera, TileMapRenderer, FogRenderer, SkyRenderer};
use colony::scenarios::{self, Scenario, ScenarioDef};

struct TimeController {
    paused:   bool,
    ff_index: usize,
}
impl TimeController {
    fn new() -> Self { Self { paused: false, ff_index: 0 } }
    fn cycle_ff_up(&mut self)   { self.ff_index = (self.ff_index + 1) % FF_LEVELS.len(); }
    fn cycle_ff_down(&mut self) {
        self.ff_index = if self.ff_index == 0 { FF_LEVELS.len() - 1 }
                        else { self.ff_index - 1 };
    }
    fn passes(&self) -> u32 { FF_LEVELS[self.ff_index] }
    fn label(&self)  -> &'static str {
        match FF_LEVELS[self.ff_index] {
            1 => "1×", 2 => "2×", 4 => "4×", 10 => "10×", 100 => "100×", _ => "?",
        }
    }
}

/// Tracks one running scenario plus its result state.
struct RunningScenario {
    def:       ScenarioDef,
    scenario:  Scenario,
    sim_time:  f32,
    state:     RunState,
}
#[derive(Copy, Clone)]
enum RunState { Running, Passed(/*sim_time:*/ f32), TimedOut }

impl RunningScenario {
    fn new(def: ScenarioDef) -> Self {
        Self {
            def,
            scenario: def.build(),
            sim_time: 0.0,
            state: RunState::Running,
        }
    }
    fn step(&mut self, dt: f32) {
        if dt <= 0.0 { return; }
        self.scenario.app.step(dt);
        self.sim_time += dt;
        if matches!(self.state, RunState::Running) {
            if (self.def.predicate)(&self.scenario.app.world) {
                self.state = RunState::Passed(self.sim_time);
            } else if self.sim_time >= self.def.timeout_seconds {
                self.state = RunState::TimedOut;
            }
        }
    }
}

fn parse_args() -> (Option<String>, bool) {
    let mut name = None;
    let mut list = false;
    for a in std::env::args().skip(1) {
        if let Some(v) = a.strip_prefix("--scenario=") {
            name = Some(v.to_string());
        } else if a == "--list" {
            list = true;
        }
    }
    (name, list)
}

fn window_conf() -> Conf {
    Conf {
        window_title:  "Colony — scenario viewer".into(),
        window_width:  SCREEN_WIDTH,
        window_height: SCREEN_HEIGHT,
        high_dpi: false, fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let (req, list_only) = parse_args();
    let registry = scenarios::registry();

    if list_only {
        for d in &registry {
            println!("  {:32}  {}", d.name, d.description);
        }
        return;
    }

    if registry.is_empty() {
        eprintln!("no scenarios in registry");
        return;
    }

    let mut idx = match req {
        Some(name) => registry.iter().position(|d| d.name == name).unwrap_or_else(|| {
            eprintln!("scenario '{}' not found; opening first", name);
            0
        }),
        None => 0,
    };

    let atlas      = Atlas::build();
    let mut camera = Camera::new();
    let mut tc     = TimeController::new();

    let mut running = RunningScenario::new(registry[idx]);
    let mut tilemap = {
        let g = running.scenario.app.world.resource::<colony::world::TileGrid>();
        TileMapRenderer::new(g, &atlas)
    };
    let mut fog = {
        let e = running.scenario.app.world.resource::<colony::world::ExploredGrid>();
        FogRenderer::new(e)
    };
    let sky = SkyRenderer::new();

    // Frame the camera on the first test subject in the scenario, or
    // fall back to the colony entrance.
    let (fx, fy) = first_subject_pos(&running.scenario.app.world)
        .unwrap_or((COLONY_X as f32, (COLONY_Y + 6) as f32));
    camera.center = Vec2::new(fx, fy);

    let mut show_list = false;
    let mut shown_fps: f32 = 0.0;
    let mut fps_clock: f32 = 0.0;
    const FPS_REFRESH: f32 = 0.5;

    loop {
        // ── Input ─────────────────────────────────────────────────────
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) { break; }
        if is_key_pressed(KeyCode::Space)        { tc.paused = !tc.paused; }
        if is_key_pressed(KeyCode::RightBracket) { tc.cycle_ff_up(); }
        if is_key_pressed(KeyCode::LeftBracket)  { tc.cycle_ff_down(); }
        if is_key_pressed(KeyCode::Tab)          { show_list = !show_list; }

        let mut scenario_changed = false;
        if is_key_pressed(KeyCode::R) {
            running = RunningScenario::new(registry[idx]);
            scenario_changed = true;
        }
        if is_key_pressed(KeyCode::N) {
            idx = (idx + 1) % registry.len();
            running = RunningScenario::new(registry[idx]);
            scenario_changed = true;
        }
        if is_key_pressed(KeyCode::P) {
            idx = if idx == 0 { registry.len() - 1 } else { idx - 1 };
            running = RunningScenario::new(registry[idx]);
            scenario_changed = true;
        }
        if scenario_changed {
            tilemap = {
                let g = running.scenario.app.world.resource::<colony::world::TileGrid>();
                TileMapRenderer::new(g, &atlas)
            };
            fog = {
                let e = running.scenario.app.world.resource::<colony::world::ExploredGrid>();
                FogRenderer::new(e)
            };
            let (fx, fy) = first_subject_pos(&running.scenario.app.world)
                .unwrap_or((COLONY_X as f32, (COLONY_Y + 6) as f32));
            camera.center = Vec2::new(fx, fy);
        }

        let rewinding = is_key_down(KeyCode::Backspace);
        let wall_dt   = get_frame_time().min(0.05);
        camera.handle_input(wall_dt);

        // ── Sim step ──────────────────────────────────────────────────
        if rewinding {
            running.scenario.app.rewind_one_step();
        } else if !tc.paused {
            // Same multi-pass model as main.rs: cap iterations at 10,
            // scale dt to preserve total elapsed sim-time. 100× = 10
            // passes of 10×-dt, etc.
            let passes = tc.passes();
            let max_iter: u32 = 10;
            let actual_passes = passes.min(max_iter);
            let scaled_dt = wall_dt * (passes as f32) / (actual_passes as f32);
            for _ in 0..actual_passes {
                running.step(scaled_dt);
            }
        }

        // Age the alert ticker on wall-clock so banners read at the
        // same speed regardless of sim rate.
        running.scenario.app.world
               .resource_mut::<colony::sim::EventLog>()
               .age_wallclock(wall_dt);

        fps_clock += wall_dt;
        if fps_clock >= FPS_REFRESH {
            fps_clock -= FPS_REFRESH;
            shown_fps = get_fps() as f32;
        }

        // Refresh GPU textures for any tile/fog changes.
        {
            let mut g = running.scenario.app.world
                              .resource_mut::<colony::world::TileGrid>();
            tilemap.refresh_if_dirty(&mut g, &atlas);
        }
        {
            let mut e = running.scenario.app.world
                              .resource_mut::<colony::world::ExploredGrid>();
            fog.refresh_if_dirty(&mut e);
        }

        // ── Render ─────────────────────────────────────────────────────
        let nf = running.scenario.app.world
                        .resource::<colony::sim::TimeOfDay>().night_factor;
        let tint = render::day_tint(nf);
        clear_background(render::sky_color(nf));
        sky.draw(&camera, tint);
        render::scenery::draw_scenery(&mut running.scenario.app.world,
                                      &atlas, &camera, tint);
        tilemap.draw(&camera, tint);
        fog.draw(&camera);
        {
            let phero = running.scenario.app.world
                              .resource::<colony::world::PheromoneGrid>();
            render::overlays::draw_pheromones(phero, &camera);
        }
        {
            let jobs = running.scenario.app.world
                              .resource::<colony::world::DigJobs>();
            render::overlays::draw_dig_markers(jobs, &atlas, &camera);
        }
        render::sprites::draw_sprites(&mut running.scenario.app.world,
                                      &atlas, &camera, tint);
        {
            let log = running.scenario.app.world
                             .resource::<colony::sim::EventLog>();
            render::ui::draw_alert_banners(log);
        }
        // Bottom stats — bind one shared &World so every resource
        // borrow lives off the same root. DigJobs isn't `Clone`, so
        // we keep a borrow until after `draw_bottom_stats` returns.
        {
            let world = &running.scenario.app.world;
            let pop  = *world.resource::<colony::sim::Population>();
            let jobs = world.resource::<colony::world::DigJobs>();
            let tod  = *world.resource::<colony::sim::TimeOfDay>();
            let history_seconds = world
                .resource::<colony::sim::history::History>()
                .seconds_buffered();
            let status = render::ui::TimeStatus {
                paused:    tc.paused,
                rewinding,
                ff_label:  tc.label(),
                history_seconds,
            };
            render::ui::draw_bottom_stats(&pop, jobs, &tod, status, shown_fps);
        }

        draw_scenario_overlay(&running, idx, registry.len());
        if show_list { draw_scenario_list(&registry, idx); }

        next_frame().await;
    }
}

/// Find the first `TestSubject`'s position so we can centre the
/// camera on whatever the scenario cares about. Returns plain
/// `(f32, f32)` to dodge the macroquad-vs-glam Vec2 type collision.
fn first_subject_pos(world: &World) -> Option<(f32, f32)> {
    use colony::scenarios::TestSubject;
    use colony::sim::components::Position;
    for e in world.iter_entities() {
        if e.get::<TestSubject>().is_some() {
            if let Some(p) = e.get::<Position>() {
                return Some((p.0.x, p.0.y));
            }
        }
    }
    None
}

/// Top-right banner: scenario name, description, pass/fail state,
/// elapsed sim time, and a hotkey hint.
fn draw_scenario_overlay(r: &RunningScenario, idx: usize, total: usize) {
    let pad = 10.0;
    let w   = 540.0;
    let h   = 96.0;
    let x   = screen_width() - w - 12.0;
    let y   = 12.0;

    let body = Color::new(0.06, 0.05, 0.04, 0.88);
    let dim  = Color::new(0.0, 0.0, 0.0, 0.55);
    draw_rectangle(x + 3.0, y + 3.0, w, h, dim);
    draw_rectangle(x, y, w, h, body);

    let (state_label, state_col) = match r.state {
        RunState::Running   => ("RUNNING", Color::new(0.96, 0.80, 0.28, 1.0)),
        RunState::Passed(_) => ("PASSED",  Color::new(0.42, 0.92, 0.46, 1.0)),
        RunState::TimedOut  => ("TIMEOUT", Color::new(1.0,  0.36, 0.24, 1.0)),
    };
    draw_rectangle_lines(x, y, w, h, 2.0, state_col);

    let fg       = Color::new(0.96, 0.92, 0.78, 1.0);
    let dim_text = Color::new(0.78, 0.74, 0.62, 1.0);
    let title = format!("[{}/{}] {}", idx + 1, total, r.def.name);
    draw_text(&title, x + pad, y + 22.0, 22.0, fg);

    draw_text(r.def.description, x + pad, y + 42.0, 16.0, dim_text);

    let status = match r.state {
        RunState::Running => format!(
            "{}   sim {:.2}s / {:.0}s",
            state_label, r.sim_time, r.def.timeout_seconds),
        RunState::Passed(t) => format!(
            "{}   reached predicate at sim {:.2}s",
            state_label, t),
        RunState::TimedOut => format!(
            "{}   timed out — {}",
            state_label, r.def.failure_hint),
    };
    draw_text(&status, x + pad, y + 64.0, 18.0, state_col);

    let hint = "[N]ext   [P]rev   [R]eset   [Tab] list   [Space] pause   ] [ speed";
    draw_text(hint, x + pad, y + 86.0, 13.0, dim_text);
}

/// Tab-toggled side panel with the full scenario list.
fn draw_scenario_list(registry: &[ScenarioDef], idx: usize) {
    let w = 420.0;
    let row_h = 22.0;
    let h = (registry.len() as f32) * row_h + 24.0;
    let x = 12.0;
    let y = 60.0;

    draw_rectangle(x + 3.0, y + 3.0, w, h, Color::new(0.0,0.0,0.0,0.55));
    draw_rectangle(x, y, w, h, Color::new(0.06,0.05,0.04,0.92));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.62,0.46,0.22,1.0));

    let fg     = Color::new(0.96, 0.92, 0.78, 1.0);
    let dim    = Color::new(0.62, 0.58, 0.48, 1.0);
    let active = Color::new(0.46, 0.98, 0.50, 1.0);

    draw_text("scenarios (N / P to switch)", x + 10.0, y + 18.0, 14.0, dim);
    for (i, d) in registry.iter().enumerate() {
        let row_y = y + 36.0 + i as f32 * row_h;
        let col = if i == idx { active } else { fg };
        let prefix = if i == idx { "▶ " } else { "  " };
        draw_text(&format!("{}{}", prefix, d.name), x + 12.0, row_y, 16.0, col);
    }
}
