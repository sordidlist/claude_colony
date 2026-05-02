//! Live-balance debug panel. Renders only when Shift is held; shows
//! a small list of `BalanceTunables` fields with `[-] value [+]`
//! buttons. Clicking a button mutates the resource directly, so
//! the next frame's queen-tick / invader-spawn / mower-speed reads
//! the new value. The panel is intentionally minimal — no fancy
//! sliders, just discrete steps that round-trip through the
//! resource cleanly.

use macroquad::prelude::*;
use bevy_ecs::world::World;
use crate::sim::BalanceTunables;

/// Draw the panel and apply any button clicks. Call once per frame
/// after the world has been drawn so the panel sits on top.
pub fn draw_balance_panel(world: &mut World) {
    let shift_held = is_key_down(KeyCode::LeftShift)
                  || is_key_down(KeyCode::RightShift);
    if !shift_held {
        // Tiny hint in the bottom-left so the player can discover
        // the panel exists.
        let hint = "Hold Shift for live tuning";
        let pad = 8.0;
        let h = 18.0;
        let tw = measure_text(hint, None, 12, 1.0).width;
        let x = pad;
        let y = screen_height() - 36.0 - h - pad;
        draw_rectangle(x, y, tw + pad * 2.0, h,
                       Color::new(0.0, 0.0, 0.0, 0.55));
        draw_text(hint, x + pad, y + 13.0, 12.0,
                  Color::new(0.78, 0.74, 0.62, 1.0));
        return;
    }

    let click = is_mouse_button_pressed(MouseButton::Left);
    let (mx, my) = mouse_position();

    // Layout — top-left, below the alert ticker.
    let pad = 12.0;
    let row_h = 26.0;
    let panel_w = 380.0;

    // Build the row spec list — (label, getter, setter, step, fmt).
    // Each entry knows how to read/write its own field on the
    // tunables resource, so the button-handler loop is uniform.
    struct Row {
        label: &'static str,
        value: f32,
        step:  f32,
        unit:  &'static str,
        clamp: (f32, f32),
    }

    let cur = *world.resource::<BalanceTunables>();
    let rows = [
        Row { label: "Queen egg interval", value: cur.queen_egg_interval,
              step: 1.0, unit: "s",  clamp: (1.0,  60.0) },
        Row { label: "Invader interval",   value: cur.invader_interval,
              step: 5.0, unit: "s",  clamp: (10.0, 600.0) },
        Row { label: "Wave size mult",     value: cur.invader_wave_mult,
              step: 0.1, unit: "x",  clamp: (0.1, 10.0) },
        Row { label: "Mower speed",        value: cur.mower_speed,
              step: 0.25, unit: "t/s", clamp: (0.25, 8.0) },
    ];

    let header_h = 28.0;
    let panel_h = header_h + row_h * rows.len() as f32 + pad;
    let px = pad;
    let py = pad + 70.0;  // leave room for alert banners above

    draw_rectangle(px + 3.0, py + 3.0, panel_w, panel_h,
                   Color::new(0.0, 0.0, 0.0, 0.55));
    draw_rectangle(px, py, panel_w, panel_h,
                   Color::new(0.06, 0.05, 0.04, 0.92));
    draw_rectangle_lines(px, py, panel_w, panel_h, 2.0,
                         Color::new(0.96, 0.84, 0.30, 1.0));
    // Header strip
    draw_rectangle(px, py, panel_w, header_h,
                   Color::new(0.32, 0.26, 0.10, 1.0));
    draw_text("LIVE BALANCE  (Shift held)",
              px + 12.0, py + 19.0, 18.0,
              Color::new(0.96, 0.92, 0.78, 1.0));

    // Individual rows.
    let mut deltas = [0.0_f32; 4];
    for (i, r) in rows.iter().enumerate() {
        let row_y = py + header_h + i as f32 * row_h + 4.0;

        // Label
        draw_text(r.label, px + 12.0, row_y + 17.0, 16.0,
                  Color::new(0.92, 0.86, 0.72, 1.0));

        // [-]/[+] buttons + value box on the right
        let btn_w = 24.0;
        let btn_h = 20.0;
        let val_w = 64.0;
        let right = px + panel_w - 12.0;
        let plus_x  = right - btn_w;
        let val_x   = plus_x - val_w - 4.0;
        let minus_x = val_x - btn_w - 4.0;
        let yy      = row_y;

        if button(minus_x, yy, btn_w, btn_h, "-", mx, my, click) {
            deltas[i] = -r.step;
        }
        if button(plus_x,  yy, btn_w, btn_h, "+", mx, my, click) {
            deltas[i] = r.step;
        }

        // Value readout
        draw_rectangle(val_x, yy, val_w, btn_h,
                       Color::new(0.16, 0.13, 0.08, 1.0));
        draw_rectangle_lines(val_x, yy, val_w, btn_h, 1.0,
                             Color::new(0.62, 0.46, 0.22, 1.0));
        let txt = format_value(r.value, r.unit);
        let tw = measure_text(&txt, None, 16, 1.0).width;
        draw_text(&txt,
                  val_x + (val_w - tw) * 0.5,
                  yy + 15.0, 16.0,
                  Color::new(0.96, 0.92, 0.78, 1.0));
    }

    if deltas.iter().any(|d| *d != 0.0) {
        let mut t = world.resource_mut::<BalanceTunables>();
        t.queen_egg_interval = (t.queen_egg_interval + deltas[0])
            .clamp(rows[0].clamp.0, rows[0].clamp.1);
        t.invader_interval = (t.invader_interval + deltas[1])
            .clamp(rows[1].clamp.0, rows[1].clamp.1);
        t.invader_wave_mult = (t.invader_wave_mult + deltas[2])
            .clamp(rows[2].clamp.0, rows[2].clamp.1);
        t.mower_speed = (t.mower_speed + deltas[3])
            .clamp(rows[3].clamp.0, rows[3].clamp.1);
    }
}

fn format_value(v: f32, unit: &str) -> String {
    if unit == "x" {
        format!("{:.1}{}", v, unit)
    } else if v >= 10.0 {
        format!("{:.0}{}", v, unit)
    } else {
        format!("{:.1}{}", v, unit)
    }
}

/// Render a labelled rectangular button. Returns true on the frame
/// the user clicked it.
fn button(x: f32, y: f32, w: f32, h: f32, label: &str,
          mx: f32, my: f32, click: bool) -> bool {
    let hovered = mx >= x && mx < x + w && my >= y && my < y + h;
    let bg = if hovered {
        Color::new(0.46, 0.32, 0.10, 1.0)
    } else {
        Color::new(0.24, 0.18, 0.08, 1.0)
    };
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(x, y, w, h, 1.0,
                         Color::new(0.62, 0.46, 0.22, 1.0));
    let tw = measure_text(label, None, 18, 1.0).width;
    draw_text(label, x + (w - tw) * 0.5, y + 15.0, 18.0,
              Color::new(0.96, 0.92, 0.78, 1.0));
    hovered && click
}
