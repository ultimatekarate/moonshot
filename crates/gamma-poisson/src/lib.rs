#![no_std]

/// Posterior over the Poisson rate λ. Gamma(α, β).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GammaPosterior {
    pub shape: f32,
    pub rate: f32,
}

impl GammaPosterior {
    pub const fn new(prior: &spec::GammaPoissonSpec) -> Self {
        Self { shape: prior.prior_shape, rate: prior.prior_rate }
    }

    /// Conjugate update on observing `k` counts in one tick.
    pub fn update(&mut self, _k: u32) {
        todo!("stub: see docs/plan.md Lab 1b — (α, β) → (α + k, β + 1)")
    }

    pub fn posterior_mean(&self) -> f32 {
        todo!("stub: α / β")
    }
}
