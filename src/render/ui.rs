//! HUD: stats strip along the bottom edge, alert ticker in the top-left.
//!
//! Alerts age in wall-clock time (passed in by the caller) so messages stay
//! visible the same amount of real seconds whether the sim is paused, running,
//! or fast-forwarded. The stats strip uses `screen_height()` directly so it
//! sticks to the bottom even on resize.

use macroquad::prelude::*;
use colony::sim::{Population, EventLog, TimeOfDay};
use colony::world::DigJobs;

#[derive(Copy, Clone)]
pub struct TimeStatus<'a> {
    pub paused:    bool,
    pub rewinding: bool,
    pub ff_label:  &'a str,
    pub history_seconds: f32,
}

pub fn draw_bottom_stats(pop: &Population, jobs: &DigJobs, tod: &TimeOfDay,
                         time: TimeStatus, fps: f32) {
    let h = 32.0;
    let y = screen_height() - h;
    // SNES-style chiseled bevel — outer dark frame, inner light highlight on
    // top/left, dim on bottom/right, deep panel body in the middle.
    let body   = Color::new(0.10, 0.07, 0.04, 0.95);
    let outer  = Color::new(0.02, 0.01, 0.00, 1.0);
    let bevel_hi = Color::new(0.62, 0.46, 0.22, 1.0);
    let bevel_dk = Color::new(0.18, 0.12, 0.06, 1.0);
    let fg     = Color::new(0.92, 0.86, 0.72, 1.0);
    let hi     = Color::new(0.98, 0.82, 0.30, 1.0);
    let danger = Color::new(1.0, 0.36, 0.18, 1.0);

    let w = screen_width();
    // Outer 2-pixel dark frame
    draw_rectangle(0.0, y, w, h, outer);
    // Recessed body
    draw_rectangle(2.0, y + 2.0, w - 4.0, h - 4.0, body);
    // Top + left highlights (light source upper-left)
    draw_rectangle(2.0, y + 2.0, w - 4.0, 1.0, bevel_hi);
    draw_rectangle(2.0, y + 2.0, 1.0, h - 4.0, bevel_hi);
    // Bottom + right shadows
    draw_rectangle(2.0, y + h - 3.0, w - 4.0, 1.0, bevel_dk);
    draw_rectangle(w - 3.0, y + 2.0, 1.0, h - 4.0, bevel_dk);

    let ty = y + h * 0.5 + 6.0;
    let mut x = 16.0;

    let put = |label: &str, value: &str, x: &mut f32, value_color: Color| {
        draw_text(label, *x, ty, 18.0, fg);
        let lw = measure_text(label, None, 18, 1.0).width;
        draw_text(value, *x + lw + 2.0, ty, 18.0, value_color);
        let vw = measure_text(value, None, 18, 1.0).width;
        *x += lw + vw + 26.0;
    };

    put("Queen ",    &format!("{}", pop.queens),   &mut x,
        if pop.queens > 0 { hi } else { danger });
    put("Workers ",  &format!("{}", pop.workers),  &mut x, hi);
    put("Soldiers ", &format!("{}", pop.soldiers), &mut x, hi);
    put("Brood ",    &format!("{}", pop.brood),    &mut x, fg);
    put("Foraging ", &format!("{}", pop.foraging), &mut x, fg);
    put("Digging ",  &format!("{}", pop.digging),  &mut x, fg);
    put("Hauling ",  &format!("{}", pop.hauling),  &mut x, fg);
    if pop.fighting > 0 {
        put("Fighting ", &format!("{}", pop.fighting), &mut x, danger);
    }
    put("Jobs ",     &format!("{}/{} unc",
                              jobs.occupied_count(),
                              jobs.unclaimed_count()), &mut x, fg);
    put("Day ",      &format!("{} · {}", tod.day_number, tod.phase_name()),
                     &mut x, hi);
    put("Speed ",    time.ff_label, &mut x, hi);
    put("FPS ",      &format!("{:.0}", fps), &mut x, fg);

    if time.rewinding {
        draw_text(&format!("◄◄ REWIND  ({:.0}s buffered)",
                            time.history_seconds),
                  x, ty, 18.0, hi);
    } else if time.paused {
        draw_text("PAUSED", x, ty, 18.0, danger);
    }
}

/// Each event renders as its own banner — a small framed plate that fades in
/// from the side, holds, then fades out. The banners have independent
/// timelines (each tracks its own age), so they bloom and dissolve on their
/// own beats rather than sharing one console panel.
pub fn draw_alert_banners(log: &EventLog) {
    if log.events.is_empty() { return; }
    let x0       = 12.0;
    let y0       = 12.0;
    let banner_h = 28.0;
    let gap      = 4.0;
    let pad_x    = 12.0;

    // Newest at the top — most recent events grab the eye first.
    for (i, e) in log.events.iter().rev().enumerate() {
        // Per-banner alpha curve: 0.3s fade-in, full alpha during the body
        // of the lifetime, 1.0s fade-out at the tail. Each banner runs its
        // own clock from `age`.
        const FADE_IN:  f32 = 0.3;
        const FADE_OUT: f32 = 1.0;
        let remain = (e.ttl - e.age).max(0.0);
        let alpha = if e.age < FADE_IN {
            (e.age / FADE_IN).clamp(0.0, 1.0)
        } else if remain < FADE_OUT {
            (remain / FADE_OUT).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if alpha <= 0.0 { continue; }

        // Subtle slide-in from the left during fade-in for a bit of motion.
        let slide_x = if e.age < FADE_IN {
            -10.0 * (1.0 - e.age / FADE_IN)
        } else { 0.0 };

        let text_w = measure_text(&e.text, None, 18, 1.0).width;
        let banner_w = text_w + pad_x * 2.0;
        let by = y0 + i as f32 * (banner_h + gap);
        let bx = x0 + slide_x;

        let (r, g, b, _) = (e.color[0], e.color[1], e.color[2], e.color[3]);
        // Background: deep tint of the event's accent colour.
        let bg = Color::new(r * 0.12 + 0.04,
                            g * 0.12 + 0.03,
                            b * 0.12 + 0.05,
                            0.86 * alpha);
        // Border picks up the event colour at full saturation so different
        // event types are visually distinct at a glance.
        let border = Color::new(r * 0.85, g * 0.85, b * 0.85, alpha);
        let text_c = Color::new((r + 0.25).min(1.0),
                                (g + 0.25).min(1.0),
                                (b + 0.25).min(1.0),
                                alpha);
        // Soft drop shadow for separation from the world behind.
        draw_rectangle(bx + 2.0, by + 2.0, banner_w, banner_h,
                       Color::new(0.0, 0.0, 0.0, 0.35 * alpha));
        draw_rectangle(bx, by, banner_w, banner_h, bg);
        draw_rectangle_lines(bx, by, banner_w, banner_h, 2.0, border);
        draw_text(&e.text, bx + pad_x, by + 19.0, 18.0, text_c);
    }
}
