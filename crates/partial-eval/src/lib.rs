#![no_std]

include!(concat!(env!("OUT_DIR"), "/k_inf.rs"));

pub const fn dt_squared(dt: f32) -> f32 {
    dt * dt
}
