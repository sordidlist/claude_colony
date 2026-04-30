//! Event log resource — a small ring of recent alerts shown in the top-left
//! ticker. Each entry has a wall-clock TTL so messages stay visible the same
//! amount of real time regardless of the in-game speed multiplier.

use bevy_ecs::prelude::Resource;

#[derive(Clone)]
pub struct Event {
    pub text:  String,
    pub color: [f32; 4],
    pub age:   f32,   // wall-clock seconds since pushed
    pub ttl:   f32,
}

#[derive(Resource, Default)]
pub struct EventLog {
    pub events: Vec<Event>,
}

impl EventLog {
    pub fn push(&mut self, text: impl Into<String>, color: [f32; 4]) {
        self.push_with_ttl(text, color, 6.0);
    }

    pub fn push_with_ttl(&mut self, text: impl Into<String>, color: [f32; 4], ttl: f32) {
        // Keep the ring bounded — drop oldest if we'd exceed 8 visible.
        if self.events.len() >= 8 { self.events.remove(0); }
        self.events.push(Event { text: text.into(), color, age: 0.0, ttl });
    }

    /// Tick by *wall-clock* seconds, not sim dt. Caller passes
    /// `get_frame_time()` directly so speed-up/pause don't change visibility.
    pub fn age_wallclock(&mut self, wall_dt: f32) {
        for e in self.events.iter_mut() { e.age += wall_dt; }
        self.events.retain(|e| e.age < e.ttl);
    }
}
