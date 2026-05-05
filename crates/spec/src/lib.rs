#![no_std]

extern crate alloc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Timestep(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogLikelihood(pub f32);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StateDim(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MeasDim(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Frobenius(pub f32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub f: [[f32; 4]; 4],
    pub h: [[f32; 2]; 4],
    pub q: [[f32; 4]; 4],
    pub r: [[f32; 2]; 2],
    pub dt: Timestep,
    /// Diagonal preconditioner for the state. Used as a similarity transform
    /// `x' = D⁻¹ x`, `F' = D⁻¹ F D`, etc. so that all state components have
    /// comparable dynamic range before the Riccati solver runs. `[1.0; 4]`
    /// means "no scaling".
    pub state_scale: [f32; 4],
    /// Scalar reference for the Cayley map: `Y = (P − αI)(P + αI)⁻¹`. Best
    /// conditioning when α sits near the geometric middle of `P`'s spectrum.
    /// If `state_scale` has done its job, `α = 1.0` is usually fine.
    pub cayley_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RiccatiResult {
    Converged(usize),
    /// The iteration drifted past tolerance without approaching the boundary.
    Diverged,
    /// Iteration cap hit before any of the other terminal conditions.
    MaxIterReached,
    /// The Cayley-transformed iterate `Y` approached the boundary of the
    /// contraction set (`σ_max(Y) → 1`), which is the image of `P → ∞`
    /// under the Cayley pullback. With the bounded parametrization, the
    /// iterate cannot escape to infinity — but it can drift to the boundary
    /// where the pullback `P = (I + Y)(I − Y)⁻¹` becomes singular. Caught
    /// here so the build fails loudly rather than baking garbage into K_INF.
    EscapedFiniteTime { at_iteration: usize, boundary_distance: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GradientCheck {
    WithinTolerance(f32),
    Mismatch { dual: f32, numeric: f32 },
}

/// Build-time check that the steady-state Kalman gain produces a stable
/// closed loop. `F_cl = (I − K_inf H) F`; spectral radius must be < 1.
/// Without this, a converged-but-marginal gain can drive slow divergence
/// at runtime even with bounded inputs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClosedLoopStability {
    Stable { spectral_radius: f32 },
    Unstable { spectral_radius: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TestEquivalence {
    Bitwise,
    Tolerant(f32),
    Failed,
}
