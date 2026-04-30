//! Pan/zoom camera. World space is in tile units; screen space is pixels.

use macroquad::prelude::*;
use colony::config::*;

pub struct Camera {
    pub center: Vec2,   // world tile coords
    pub zoom:   f32,    // pixels per tile
}

impl Camera {
    pub fn new() -> Self {
        Self {
            center: Vec2::new(COLONY_X as f32, (COLONY_Y + 10) as f32),
            zoom:   TILE_SIZE,
        }
    }

    pub fn handle_input(&mut self, dt: f32) {
        let speed = 60.0 / self.zoom;
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left)  { self.center.x -= speed * dt * 60.0; }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) { self.center.x += speed * dt * 60.0; }
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up)    { self.center.y -= speed * dt * 60.0; }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down)  { self.center.y += speed * dt * 60.0; }

        let (_, scroll_y) = mouse_wheel();
        if scroll_y > 0.0 { self.zoom = (self.zoom * 1.15).min(48.0); }
        if scroll_y < 0.0 { self.zoom = (self.zoom / 1.15).max(2.0); }

        // Clamp center to world
        self.center.x = self.center.x.clamp(0.0, WORLD_WIDTH  as f32);
        self.center.y = self.center.y.clamp(0.0, WORLD_HEIGHT as f32);
    }

    /// Convert a world tile coord (with .5 offsets baked in for sprites) to a
    /// screen pixel position. Returns the top-left of an 8×zoom sprite cell.
    #[inline] pub fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        let sx = screen_width()  * 0.5 + (wx - self.center.x) * self.zoom;
        let sy = screen_height() * 0.5 + (wy - self.center.y) * self.zoom;
        (sx, sy)
    }

    pub fn visible_tile_rect(&self) -> (i32, i32, i32, i32) {
        let half_w = screen_width()  * 0.5 / self.zoom;
        let half_h = screen_height() * 0.5 / self.zoom;
        let x0 = (self.center.x - half_w).floor() as i32 - 1;
        let y0 = (self.center.y - half_h).floor() as i32 - 1;
        let x1 = (self.center.x + half_w).ceil()  as i32 + 1;
        let y1 = (self.center.y + half_h).ceil()  as i32 + 1;
        (x0, y0, x1, y1)
    }
}
