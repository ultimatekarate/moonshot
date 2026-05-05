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

/// Fixed-shape matrix wrapper. `Matrix<R, C>` and `Matrix<C, R>` are distinct
/// types — confusing rows and columns becomes a compile error rather than a
/// silent shape bug at deserialisation time. R is rows, C is cols (same
/// convention as `nalgebra::SMatrix<T, R, C>` and standard math notation).
/// Storage is row-major: outer index is the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix<const R: usize, const C: usize>(pub [[f32; C]; R]);

// serde's derive doesn't propagate const generics into the bounds it needs on
// the inner `[[f32; C]; R]`. The where-clauses delegate the bound to the
// concrete instantiation site, where serde's array impls do exist.
impl<const R: usize, const C: usize> serde::Serialize for Matrix<R, C>
where
    [[f32; C]; R]: serde::Serialize,
{
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de, const R: usize, const C: usize> serde::Deserialize<'de> for Matrix<R, C>
where
    [[f32; C]; R]: serde::Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Matrix(serde::Deserialize::deserialize(d)?))
    }
}

/// Sum type over the three toy problems. Forces every consumer
/// (build.rs, proc-macros, host harness) to dispatch — preventing
/// architecture from over-fitting to any one problem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelSpec {
    Kalman(KalmanSpec),
    GammaPoisson(GammaPoissonSpec),
    EkfBearing(EkfBearingSpec),
}

/// 2D constant-velocity Kalman: linear-Gaussian filtering for sensor fusion.
/// State `[x, y, vx, vy]`, observation `[x, y]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KalmanSpec {
    pub f: Matrix<4, 4>,
    pub h: Matrix<2, 4>,
    pub q: Matrix<4, 4>,
    pub r: Matrix<2, 2>,
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

/// Gamma-Poisson conjugate update: on-device event-rate estimation.
/// Posterior over the rate `λ` given a stream of count observations.
/// `Gamma(α, β) → Gamma(α + Σk_i, β + n)` after `n` ticks of total counts `Σk_i`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GammaPoissonSpec {
    /// Prior shape `α₀` for the rate.
    pub prior_shape: f32,
    /// Prior rate `β₀` for the rate (units: per tick).
    pub prior_rate: f32,
    /// Tick interval — the time over which counts are aggregated.
    pub tick_interval: Timestep,
}

/// EKF for bearing-only 2D tracking: non-linear observation, AD genuinely
/// load-bearing for the Jacobian. Same 4D state as the linear Kalman.
/// Observation is a scalar bearing `θ = atan2(y, x)`; `H` is computed
/// at runtime via dual numbers, not embedded as a literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EkfBearingSpec {
    pub f: Matrix<4, 4>,
    pub q: Matrix<4, 4>,
    /// Scalar bearing measurement noise (radians²).
    pub r: f32,
    pub dt: Timestep,
    pub state_scale: [f32; 4],
    /// Sensor position in world frame. Bearing is measured *to* the target
    /// *from* this point.
    pub sensor_position: [f32; 2],
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
    /// Trajectories agree within the per-component tolerance. The `f32`
    /// reports the worst observed max-abs-diff so we can track drift.
    Tolerant(f32),
    Failed,
}
