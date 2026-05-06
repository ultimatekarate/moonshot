//! Cayley-parametrised solver for the discrete algebraic Riccati equation.
//!
//! Lives in the `kernel` layer (strict purity: no IO, no async). Host-only —
//! called at cargo build time from `partial-eval/build.rs` and at proc-macro
//! expansion time from `crates/codegen`. Stays out of the embedded binary
//! because its callers either emit a `const` file (build.rs) or emit literal
//! tokens (proc-macro); the runtime side of `partial-eval` only `include!()`s
//! the generated const file and never imports from here.
//!
//! The iteration runs in Cayley-transformed coordinates `Y = (P − αI)(P + αI)⁻¹`,
//! which lives in the bounded contraction set `‖Y‖ < 1` whenever `P` is PD.
//! Finite escape to infinity is therefore impossible by construction; the
//! conditioning failure that maps to `P → ∞` shows up as `σ_max(Y) → 1` and
//! is caught by the boundary-distance check before garbage propagates into
//! `K_INF`.

use spec::KalmanSpec;

type Mat4 = nalgebra::Matrix4<f32>;
type Mat2 = nalgebra::Matrix2<f32>;
type Mat2x4 = nalgebra::Matrix2x4<f32>;
type Mat4x2 = nalgebra::Matrix4x2<f32>;

const MAX_ITER: usize = 1_000;
const CONV_TOL: f32 = 1e-9;
const BOUNDARY_EPS: f32 = 1e-6;

/// The output of a successful Cayley-Riccati solve. `k_inf` is in physical
/// (un-scaled) coordinates so callers can drop it directly into emitted code
/// that operates on raw observations.
#[derive(Debug, Clone, Copy)]
pub struct SolvedKalman {
    pub k_inf: [[f32; 2]; 4],
    pub iters: usize,
    pub spectral_radius: f32,
}

#[derive(Debug)]
pub enum RiccatiError {
    SingularStateScale,
    SingularInnovationCov { during: &'static str },
    SingularPlusAlpha,
    SingularOneMinusY,
    BoundaryApproach { at_iteration: usize, boundary_distance: f32 },
    DidNotConverge { max_iter: usize },
    UnstableClosedLoop { spectral_radius: f32 },
}

impl core::fmt::Display for RiccatiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SingularStateScale => {
                write!(f, "state_scale must be all non-zero")
            }
            Self::SingularInnovationCov { during } => {
                write!(f, "innovation covariance singular during {}", during)
            }
            Self::SingularPlusAlpha => {
                write!(f, "(P + αI) singular — should not happen for PD P")
            }
            Self::SingularOneMinusY => {
                write!(f, "(I − Y) singular — Y at boundary, should be caught earlier")
            }
            Self::BoundaryApproach { at_iteration, boundary_distance } => write!(
                f,
                "Cayley iterate approached boundary at iter {}: 1 − σ_max(Y) = {} (must stay > {}). \
                 Likely (F, Q^½) not stabilizable, or numerical conditioning failure.",
                at_iteration, boundary_distance, BOUNDARY_EPS
            ),
            Self::DidNotConverge { max_iter } => {
                write!(f, "Cayley-Riccati did not converge in {} iterations", max_iter)
            }
            Self::UnstableClosedLoop { spectral_radius } => write!(
                f,
                "closed-loop unstable: spectral_radius = {} (must be < 1)",
                spectral_radius
            ),
        }
    }
}

impl std::error::Error for RiccatiError {}

/// Solve the discrete algebraic Riccati equation for a Kalman spec, returning
/// the steady-state gain in physical coordinates after verifying that the
/// closed loop `(I − K_inf H) F` has spectral radius < 1.
pub fn solve(spec: &KalmanSpec) -> Result<SolvedKalman, RiccatiError> {
    let f = mat4_from_rows(&spec.f.0);
    let h = mat2x4_from_rows(&spec.h.0);
    let q = mat4_from_rows(&spec.q.0);
    let r = mat2_from_rows(&spec.r.0);

    // State scaling: x' = D⁻¹ x  →  F' = D⁻¹ F D, Q' = D⁻¹ Q D⁻ᵀ, H' = H D.
    // R is unchanged (lives in measurement space).
    let d = Mat4::from_diagonal(&nalgebra::Vector4::new(
        spec.state_scale[0],
        spec.state_scale[1],
        spec.state_scale[2],
        spec.state_scale[3],
    ));
    let d_inv = d.try_inverse().ok_or(RiccatiError::SingularStateScale)?;

    let f_s = d_inv * f * d;
    let q_s = d_inv * q * d_inv.transpose();
    let h_s = h * d;

    let (p_inf_s, iters) = cayley_riccati(&f_s, &h_s, &q_s, &r, spec.cayley_alpha)?;

    // Recover K_inf in scaled coords, un-scale to physical: K_phys = D · K_scaled.
    let s_inf = h_s * p_inf_s * h_s.transpose() + r;
    let s_inv = s_inf
        .try_inverse()
        .ok_or(RiccatiError::SingularInnovationCov { during: "K_inf recovery" })?;
    let k_inf_s = p_inf_s * h_s.transpose() * s_inv;
    let k_inf_mat: Mat4x2 = d * k_inf_s;

    // Closed-loop stability check on the un-scaled system.
    let i_n = Mat4::identity();
    let f_cl = (i_n - k_inf_mat * h) * f;
    let eigs = f_cl.complex_eigenvalues();
    let spectral_radius: f32 = eigs.iter().map(|e| e.norm()).fold(0.0_f32, f32::max);
    if !(spectral_radius < 1.0) {
        return Err(RiccatiError::UnstableClosedLoop { spectral_radius });
    }

    let mut k_inf = [[0.0_f32; 2]; 4];
    for row in 0..4 {
        for col in 0..2 {
            k_inf[row][col] = k_inf_mat[(row, col)];
        }
    }

    Ok(SolvedKalman { k_inf, iters, spectral_radius })
}

fn cayley_riccati(
    f: &Mat4,
    h: &Mat2x4,
    q: &Mat4,
    r: &Mat2,
    alpha: f32,
) -> Result<(Mat4, usize), RiccatiError> {
    let alpha_i = Mat4::identity() * alpha;
    // Y_0 = 0 corresponds to P_0 = αI.
    let mut y = Mat4::zeros();

    for k in 0..MAX_ITER {
        // Pullback: P = α(I + Y)(I − Y)⁻¹.
        let i_n = Mat4::identity();
        let one_minus_y_inv = (i_n - y)
            .try_inverse()
            .ok_or(RiccatiError::SingularOneMinusY)?;
        let p = (i_n + y) * one_minus_y_inv * alpha;

        // One step of the Riccati recursion on P.
        let s = h * p * h.transpose() + r;
        let s_inv = s
            .try_inverse()
            .ok_or(RiccatiError::SingularInnovationCov { during: "iteration" })?;
        let p_new = f * p * f.transpose() + q
            - f * p * h.transpose() * s_inv * h * p * f.transpose();

        // Cayley map back: Y_new = (P_new − αI)(P_new + αI)⁻¹.
        let p_plus = (p_new + alpha_i)
            .try_inverse()
            .ok_or(RiccatiError::SingularPlusAlpha)?;
        let y_new = (p_new - alpha_i) * p_plus;

        // Boundary check: 1 − σ_max(Y_new) approaches zero as P → ∞.
        let sigma_max = y_new.svd(false, false).singular_values[0];
        let boundary_distance = 1.0 - sigma_max;
        if boundary_distance < BOUNDARY_EPS {
            return Err(RiccatiError::BoundaryApproach {
                at_iteration: k,
                boundary_distance,
            });
        }

        if (y_new - y).norm() < CONV_TOL {
            return Ok((p_new, k + 1));
        }

        y = y_new;
    }

    Err(RiccatiError::DidNotConverge { max_iter: MAX_ITER })
}

fn mat4_from_rows(rows: &[[f32; 4]; 4]) -> Mat4 {
    Mat4::from_row_slice(&rows.iter().flatten().copied().collect::<Vec<_>>())
}

fn mat2_from_rows(rows: &[[f32; 2]; 2]) -> Mat2 {
    Mat2::from_row_slice(&rows.iter().flatten().copied().collect::<Vec<_>>())
}

fn mat2x4_from_rows(rows: &[[f32; 4]; 2]) -> Mat2x4 {
    Mat2x4::from_row_slice(&rows.iter().flatten().copied().collect::<Vec<_>>())
}
