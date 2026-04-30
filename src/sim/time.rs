use bevy_ecs::prelude::Resource;

#[derive(Resource, Default, Debug, Copy, Clone)]
pub struct Time {
    pub dt:    f32,   // seconds since previous frame
    pub total: f32,
}

#[derive(Resource, Default, Debug, Copy, Clone)]
pub struct Population {
    pub workers:  usize,
    pub queens:   usize,
    pub soldiers: usize,
    pub brood:    usize,
    pub digging:  usize,
    pub foraging: usize,
    pub hauling:  usize,
    pub fighting: usize,
}
