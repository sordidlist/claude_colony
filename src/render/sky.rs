//! Sky and distant-hills layer. Painted into a single world-sized texture
//! once at startup with a Bayer-dithered vertical gradient and two layers of
//! silhouette hills cresting just above the surface band. Drawn each frame
//! with a horizontal parallax offset so the camera glides past the hills
//! more slowly than the foreground — the small SNES-era trick that turns a
//! flat sky into a sense of depth.

use macroquad::prelude::*;
use crate::config::*;

const PARALLAX: f32 = 0.55;   // 1.0 = lock to camera, 0.0 = static

pub struct SkyRenderer {
    texture: Texture2D,
    img_w:   f32,
    img_h:   f32,
}

impl SkyRenderer {
    pub fn new() -> Self {
        let img_w = (WORLD_WIDTH  as u16) * 8;
        let img_h = (SURFACE_ROW  as u16) * 8;
        let mut img = Image::gen_image_color(img_w, img_h,
            Color::new(0.0, 0.0, 0.0, 0.0));
        paint_sky(&mut img, img_w, img_h);
        let texture = Texture2D::from_image(&img);
        texture.set_filter(FilterMode::Nearest);
        Self { texture, img_w: img_w as f32, img_h: img_h as f32 }
    }

    pub fn draw(&self, cam: &super::Camera, tint: Color) {
        // Reduce camera influence horizontally for parallax. Sky still
        // stretches across the world — we just shift its anchor by less
        // than the camera, so it appears to scroll slower.
        let cam_x_offset = (cam.center.x - WORLD_WIDTH as f32 * 0.5)
                         * (1.0 - PARALLAX);
        let anchor_x = -cam_x_offset;
        let (sx, sy) = cam.world_to_screen(anchor_x, 0.0);
        let dest_w = WORLD_WIDTH as f32 * cam.zoom;
        let dest_h = SURFACE_ROW as f32 * cam.zoom;
        draw_texture_ex(&self.texture, sx, sy, tint, DrawTextureParams {
            dest_size: Some(Vec2::new(dest_w, dest_h)),
            ..Default::default()
        });
        let _ = self.img_w; let _ = self.img_h;
    }
}

fn paint_sky(img: &mut Image, w: u16, h: u16) {
    // Vertical colour stops, top → horizon. Five stops gives a soft band.
    let stops: &[(u8, u8, u8)] = &[
        ( 28,  68, 152),
        ( 60, 116, 196),
        (104, 168, 220),
        (164, 204, 232),
        (212, 224, 232),
    ];

    const BAYER: [[u8; 4]; 4] = [
        [ 0,  8,  2, 10],
        [12,  4, 14,  6],
        [ 3, 11,  1,  9],
        [15,  7, 13,  5],
    ];

    // Gradient pass with Bayer dithering between adjacent stops
    for y in 0..h {
        let t = y as f32 / (h as f32 - 1.0);
        let segs = stops.len() - 1;
        let pos  = t * segs as f32;
        let band = (pos as usize).min(segs - 1);
        let frac = pos - band as f32;
        let lo = stops[band];
        let hi = stops[band + 1];

        for x in 0..w {
            let bayer_v = BAYER[(y as usize) % 4][(x as usize) % 4] as f32;
            let pick_hi = frac * 16.0 > bayer_v;
            let (r, g, b) = if pick_hi { hi } else { lo };
            img.set_pixel(x as u32, y as u32,
                Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0));
        }
    }

    // Two layers of silhouette hills. The far layer is lighter and lower,
    // the near layer is darker, taller, and crests just below the horizon.
    let far_color  = (152, 168, 196);
    let near_color = ( 96, 112, 148);
    let crest      = (h as f32 * 0.62) as i32;

    // Far hills (lighter, smoother)
    for x in 0..w as i32 {
        let xf = x as f32;
        let amp = 6.0 * (xf * 0.012).sin()
                + 3.0 * (xf * 0.040).sin()
                + 4.0 * (xf * 0.003).cos();
        let top = (crest + 14 - amp as i32).max(0).min(h as i32 - 1);
        for y in top..(h as i32) {
            let (r, g, b) = far_color;
            img.set_pixel(x as u32, y as u32,
                Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0));
        }
    }

    // Near hills (darker, more dramatic)
    for x in 0..w as i32 {
        let xf = x as f32;
        let amp = 10.0 * (xf * 0.018).sin()
                +  5.0 * (xf * 0.060).sin()
                +  4.0 * (xf * 0.005).cos()
                +  3.0 * (xf * 0.110).sin();
        let top = (crest + 26 - amp as i32).max(0).min(h as i32 - 1);
        for y in top..(h as i32) {
            // Subtle gradient inside the hill body
            let depth = (y - top) as f32;
            let darken = 1.0 - (depth * 0.005).min(0.18);
            let (r, g, b) = near_color;
            let r = (r as f32 * darken) as u8;
            let g = (g as f32 * darken) as u8;
            let b = (b as f32 * darken) as u8;
            img.set_pixel(x as u32, y as u32,
                Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0));
        }
    }
}
