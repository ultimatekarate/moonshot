#![no_std]

extern crate alloc;

pub mod numeric;

pub fn gradient_nll(_params: &[f32; 6], _zs: &[[f32; 2]]) -> [f32; 6] {
    todo!(
        "stub: see docs/plan.md Lab 2 — \
         lift params to DualVec<f32, U6>, \
         call model::cv_2d::neg_log_likelihood, \
         read 6 partials off .eps"
    )
}
