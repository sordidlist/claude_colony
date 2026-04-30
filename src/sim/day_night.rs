//! Day/night cycle. Holds the time-of-day state, advances it from sim dt,
//! and pushes ticker events at dawn / dusk crossings and on each new day.

use bevy_ecs::prelude::*;
use std::f32::consts::PI;
use crate::config::DAY_LENGTH_SECONDS;
use super::{Time, EventLog};

#[derive(Resource, Copy, Clone, Debug)]
pub struct TimeOfDay {
    pub seconds:    f32,    // 0..DAY_LENGTH_SECONDS, wraps each day
    pub day_number: u32,
    pub night_factor: f32,  // 0 = noon, 1 = midnight (smooth)
    prev_night_factor: f32,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        // Start at "early morning" so the first frame isn't midnight.
        let seconds = DAY_LENGTH_SECONDS * 0.20;
        let mut t = Self {
            seconds,
            day_number: 1,
            night_factor: 0.0,
            prev_night_factor: 0.0,
        };
        t.recompute_factor();
        t.prev_night_factor = t.night_factor;
        t
    }
}

impl TimeOfDay {
    fn recompute_factor(&mut self) {
        let phase = self.seconds / DAY_LENGTH_SECONDS * 2.0 * PI;
        // cos: peaks at midnight (seconds=0), troughs at noon. We want
        // night_factor=1 at midnight, =0 at noon.
        self.night_factor = (1.0 + phase.cos()) * 0.5;
    }

    pub fn phase_name(&self) -> &'static str {
        // Eight named phases around the cycle.
        let f = (self.seconds / DAY_LENGTH_SECONDS * 8.0) as i32 & 7;
        ["Midnight", "Late Night", "Dawn", "Morning",
         "Noon",     "Afternoon",  "Dusk", "Evening"][f as usize]
    }
}

pub fn advance_day_night(
    time: Res<Time>,
    mut tod: ResMut<TimeOfDay>,
    mut log: ResMut<EventLog>,
) {
    if time.dt <= 0.0 { return; }
    let prev_seconds = tod.seconds;
    tod.seconds += time.dt;
    if tod.seconds >= DAY_LENGTH_SECONDS {
        tod.seconds -= DAY_LENGTH_SECONDS;
        tod.day_number += 1;
        log.push(format!("Day {} begins", tod.day_number),
                 [0.96, 0.84, 0.36, 1.0]);
    }
    tod.prev_night_factor = tod.night_factor;
    tod.recompute_factor();

    // Crossings: prev<0.5 → now>=0.5 is dusk; prev>=0.5 → now<0.5 is dawn.
    let prev_was_night = tod.prev_night_factor >= 0.5;
    let now_is_night   = tod.night_factor      >= 0.5;
    if !prev_was_night && now_is_night {
        log.push("Night falls…", [0.36, 0.46, 0.86, 1.0]);
    } else if prev_was_night && !now_is_night {
        log.push("Dawn breaks", [0.96, 0.78, 0.32, 1.0]);
    }
    let _ = prev_seconds;
}
