//! Fog-of-war overlay. One pixel per tile in a tiny texture, drawn over the
//! tilemap as a single quad. Updated incrementally — only re-uploads to the
//! GPU when the explored mask changes.

use macroquad::prelude::*;
use colony::config::*;
use colony::world::ExploredGrid;
use super::Camera;

pub struct FogRenderer {
    image:   Image,
    texture: Texture2D,
}

impl FogRenderer {
    pub fn new(grid: &ExploredGrid) -> Self {
        let mut image = Image::gen_image_color(
            grid.width as u16, grid.height as u16, BLACK);
        bake(&mut image, grid);
        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Nearest);
        Self { image, texture }
    }

    pub fn refresh_if_dirty(&mut self, grid: &mut ExploredGrid) {
        if !grid.dirty { return; }
        bake(&mut self.image, grid);
        self.texture.update(&self.image);
        grid.dirty = false;
    }

    pub fn draw(&self, cam: &Camera) {
        let (sx, sy) = cam.world_to_screen(0.0, 0.0);
        let dest_w = WORLD_WIDTH  as f32 * cam.zoom;
        let dest_h = WORLD_HEIGHT as f32 * cam.zoom;
        draw_texture_ex(&self.texture, sx, sy, WHITE, DrawTextureParams {
            dest_size: Some(Vec2::new(dest_w, dest_h)),
            ..Default::default()
        });
    }
}

fn bake(image: &mut Image, grid: &ExploredGrid) {
    let opaque = Color::new(0.02, 0.02, 0.04, 1.0);
    let clear  = Color::new(0.0, 0.0, 0.0, 0.0);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let i = grid.idx(x, y);
            let c = if grid.data[i] == 0 { opaque } else { clear };
            image.set_pixel(x as u32, y as u32, c);
        }
    }
}
