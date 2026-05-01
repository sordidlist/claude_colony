use macroquad::prelude::*;
use colony::app::App;
use colony::config::*;
use colony::render::{self, Atlas, Camera, TileMapRenderer, FogRenderer, SkyRenderer};

/// Cycles through fast-forward speeds and tracks pause/rewind. Owned by
/// `main.rs` so input mapping stays out of the sim layer.
struct TimeController {
    paused:   bool,
    ff_index: usize,
}
impl TimeController {
    fn new() -> Self { Self { paused: false, ff_index: 0 } }
    fn cycle_ff_up(&mut self) {
        self.ff_index = (self.ff_index + 1) % FF_LEVELS.len();
    }
    fn cycle_ff_down(&mut self) {
        self.ff_index = if self.ff_index == 0 {
            FF_LEVELS.len() - 1
        } else { self.ff_index - 1 };
    }
    fn passes(&self)   -> u32 { FF_LEVELS[self.ff_index] }
    fn label(&self)    -> &'static str {
        match FF_LEVELS[self.ff_index] {
            1   => "1×",
            2   => "2×",
            4   => "4×",
            10  => "10×",
            100 => "100×",
            _   => "?",
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title:  "Colony".to_string(),
        window_width:  SCREEN_WIDTH,
        window_height: SCREEN_HEIGHT,
        high_dpi:      false,
        fullscreen:    false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let seed = std::env::args()
        .skip(1)
        .find_map(|a| a.strip_prefix("--seed=").and_then(|s| s.parse().ok()))
        .unwrap_or(42u64);

    let mut app   = App::new(seed);
    let atlas     = Atlas::build();
    let mut camera = Camera::new();
    let mut tilemap = {
        let g = app.world.resource::<colony::world::TileGrid>();
        TileMapRenderer::new(&g, &atlas)
    };
    let mut fog = {
        let e = app.world.resource::<colony::world::ExploredGrid>();
        FogRenderer::new(&e)
    };
    let sky = SkyRenderer::new();
    let mut tc = TimeController::new();

    // FPS display smoothing — `get_fps()` returns the instantaneous value
    // every frame, which flickers four digits a second and is unreadable.
    // Sample twice a second and hold; the eye gets a stable number.
    let mut shown_fps: f32 = 0.0;
    let mut fps_clock: f32 = 0.0;
    const FPS_REFRESH: f32 = 0.5;

    // Seed the alert ticker so it's visible from the first frame.
    {
        let mut log = app.world.resource_mut::<colony::sim::EventLog>();
        log.push(format!("Colony founded — seed {}", seed),
                 [0.96, 0.80, 0.28, 1.0]);
    }

    loop {
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) { break; }
        if is_key_pressed(KeyCode::Space) { tc.paused = !tc.paused; }
        // Speed controls live on the bracket keys so they don't fight
        // arrow-key panning. `]` cycles to a faster speed, `[` to a slower
        // one. Hold Backspace to rewind through the history buffer.
        if is_key_pressed(KeyCode::RightBracket) { tc.cycle_ff_up(); }
        if is_key_pressed(KeyCode::LeftBracket)  { tc.cycle_ff_down(); }
        let rewinding = is_key_down(KeyCode::Backspace);

        let wall_dt = get_frame_time().min(0.05);
        camera.handle_input(wall_dt);

        if rewinding {
            // Pop one snapshot per frame — at 60 fps over 1 s/snapshot, that
            // means ~60s of history rewinds in 1s of real time.
            app.rewind_one_step();
        } else if !tc.paused {
            // Multi-pass fast-forward: cap iterations at 10, scale dt to
            // preserve total elapsed sim-time. 100× = 10 passes of 10× dt.
            let passes = tc.passes();
            let max_iter: u32 = 10;
            let actual_passes = passes.min(max_iter);
            let scaled_dt = wall_dt * (passes as f32) / (actual_passes as f32);
            for _ in 0..actual_passes {
                app.step(scaled_dt);
            }
        }
        // Age the alert ticker on wall-clock — messages stay up the same
        // amount of real time whether paused, normal-speed, or rewinding.
        {
            let mut log = app.world.resource_mut::<colony::sim::EventLog>();
            log.age_wallclock(wall_dt);
        }
        fps_clock += wall_dt;
        if fps_clock >= FPS_REFRESH {
            fps_clock -= FPS_REFRESH;
            shown_fps = get_fps() as f32;
        }

        // Refresh GPU textures if their backing grids changed.
        {
            let mut g = app.world.resource_mut::<colony::world::TileGrid>();
            tilemap.refresh_if_dirty(&mut g, &atlas);
        }
        {
            let mut e = app.world.resource_mut::<colony::world::ExploredGrid>();
            fog.refresh_if_dirty(&mut e);
        }

        // ── Render ──────────────────────────────────────────────────────
        let nf = app.world.resource::<colony::sim::TimeOfDay>().night_factor;
        let tint = render::day_tint(nf);
        clear_background(render::sky_color(nf));

        // Sky band: dithered gradient + parallax distant hills, drawn before
        // foreground scenery so trees and the barn punch through.
        sky.draw(&camera, tint);
        // Above-ground scenery sits behind the tile layer so trees / barn
        // overlap the grass at their feet, not float above it.
        render::scenery::draw_scenery(&mut app.world, &atlas, &camera, tint);
        tilemap.draw(&camera, tint);
        fog.draw(&camera);

        {
            let phero = app.world.resource::<colony::world::PheromoneGrid>();
            render::overlays::draw_pheromones(&phero, &camera);
        }
        {
            let jobs = app.world.resource::<colony::world::DigJobs>();
            render::overlays::draw_dig_markers(&jobs, &atlas, &camera);
        }

        render::sprites::draw_sprites(&mut app.world, &atlas, &camera, tint);

        {
            let log = app.world.resource::<colony::sim::EventLog>();
            render::ui::draw_alert_banners(&log);
        }
        {
            let pop  = *app.world.resource::<colony::sim::Population>();
            let jobs = app.world.resource::<colony::world::DigJobs>();
            let tod  = *app.world.resource::<colony::sim::TimeOfDay>();
            let history_seconds = {
                let h = app.world.resource::<colony::sim::history::History>();
                h.seconds_buffered()
            };
            let status = render::ui::TimeStatus {
                paused:    tc.paused,
                rewinding,
                ff_label:  tc.label(),
                history_seconds,
            };
            render::ui::draw_bottom_stats(&pop, &jobs, &tod, status,
                                                 shown_fps);
        }

        next_frame().await;
    }
}
