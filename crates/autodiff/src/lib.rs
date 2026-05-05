#![no_std]

extern crate alloc;

pub mod numeric;

pub fn gradient_nll(_params: &[f32; 6], _zs: &[[f32; 2]]) -> [f32; 6] {
    todo!(
        "stub: see docs/plan.md Lab 2 — \
         lift params to DualVec<f32, U6>, \
         call kalman::cv_2d::neg_log_likelihood, \
         read 6 partials off .eps"
    )
}

/// Jacobian of the bearing-only observation model `h(x) = atan2(y - sy, x - sx)`
/// at `state`, computed via dual numbers. This is the load-bearing AD use:
/// the EKF needs a fresh Jacobian every update, so we cannot precompute.
pub fn bearing_jacobian(_state: &[f32; 4], _sensor: &[f32; 2]) -> [f32; 4] {
    todo!("stub: see docs/plan.md Lab 1c — DualVec<f32, U4> evaluation of h")
}
