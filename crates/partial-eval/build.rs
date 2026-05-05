use spec::{KalmanSpec, ModelSpec};
use spec_loader::load;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

type Mat4 = nalgebra::Matrix4<f32>;
type Mat2 = nalgebra::Matrix2<f32>;
type Mat2x4 = nalgebra::Matrix2x4<f32>;
type Mat4x2 = nalgebra::Matrix4x2<f32>;

const MAX_ITER: usize = 1_000;
const CONV_TOL: f32 = 1e-9;
const BOUNDARY_EPS: f32 = 1e-6;

fn main() {
    let model_path = env::var_os("MOONSHOT_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../../model.ron"));

    let spec = load(&model_path).expect("load model.ron");

    let code = match spec {
        ModelSpec::Kalman(k) => emit_kalman(&k),
        ModelSpec::GammaPoisson(_) => panic!(
            "partial-eval/build.rs currently only handles ModelSpec::Kalman; \
             GammaPoisson dispatch is not yet implemented"
        ),
        ModelSpec::EkfBearing(_) => panic!(
            "partial-eval/build.rs currently only handles ModelSpec::Kalman; \
             EkfBearing dispatch is not yet implemented"
        ),
    };

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set by cargo"));
    fs::write(out_dir.join("k_inf.rs"), code).expect("write k_inf.rs");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", model_path.display());
    println!("cargo:rerun-if-env-changed=MOONSHOT_MODEL");
}

/// Solve the discrete algebraic Riccati equation for a Kalman spec, recover
/// `K_inf`, verify closed-loop stability, and emit the literal const.
fn emit_kalman(spec: &KalmanSpec) -> String {
    let f = mat4_from_rows(&spec.f.0);
    let h = mat2x4_from_rows(&spec.h.0);
    let q = mat4_from_rows(&spec.q.0);
    let r = mat2_from_rows(&spec.r.0);

    // State scaling: x' = D⁻¹ x  →  F' = D⁻¹ F D, Q' = D⁻¹ Q D⁻ᵀ, H' = H D
    let d = Mat4::from_diagonal(&nalgebra::Vector4::new(
        spec.state_scale[0],
        spec.state_scale[1],
        spec.state_scale[2],
        spec.state_scale[3],
    ));
    let d_inv = d.try_inverse().expect("state_scale must be all non-zero");

    let f_s = d_inv * f * d;
    let q_s = d_inv * q * d_inv.transpose();
    let h_s = h * d;
    // R is unchanged (lives in measurement space).

    let (p_inf_s, iters) = cayley_riccati(&f_s, &h_s, &q_s, &r, spec.cayley_alpha);

    // Recover K_inf in scaled coordinates, then un-scale: K_phys = D · K_scaled.
    let s_inf = h_s * p_inf_s * h_s.transpose() + r;
    let s_inv = s_inf
        .try_inverse()
        .expect("S_inf singular at convergence — should not happen for PD covariances");
    let k_inf_s = p_inf_s * h_s.transpose() * s_inv;
    let k_inf = d * k_inf_s;

    // Closed-loop stability check: spectral radius of (I − K_inf H) F < 1.
    let i_n = Mat4::identity();
    let f_cl = (i_n - k_inf * h) * f;
    let eigs = f_cl.complex_eigenvalues();
    let spectral_radius: f32 = eigs.iter().map(|e| e.norm()).fold(0.0_f32, f32::max);
    if !(spectral_radius < 1.0) {
        panic!(
            "closed-loop unstable: spectral_radius = {} (must be < 1). \
             Refusing to bake an unstable gain into the binary.",
            spectral_radius
        );
    }

    emit_const(&k_inf, iters, spectral_radius)
}

/// Cayley-parametrised Riccati iteration. Returns the converged `P_inf` and
/// the number of iterations consumed.
///
/// The iterate `Y_k = (P_k − αI)(P_k + αI)⁻¹` lives in the bounded set
/// `‖Y‖ < 1` whenever `P` is PD, so the iteration cannot escape to infinity.
/// Boundary-distance check guards against `‖Y‖ → 1` (the image of `P → ∞`).
fn cayley_riccati(
    f: &Mat4,
    h: &Mat2x4,
    q: &Mat4,
    r: &Mat2,
    alpha: f32,
) -> (Mat4, usize) {
    let alpha_i = Mat4::identity() * alpha;
    // Y_0 = 0 corresponds to P_0 = αI.
    let mut y = Mat4::zeros();

    for k in 0..MAX_ITER {
        // Pullback: P = α(I + Y)(I − Y)⁻¹.
        let i_n = Mat4::identity();
        let one_minus_y_inv = (i_n - y)
            .try_inverse()
            .expect("(I − Y) singular — Y at boundary, should be caught earlier");
        let p = (i_n + y) * one_minus_y_inv * alpha;

        // One step of the Riccati recursion on P.
        let s = h * p * h.transpose() + r;
        let s_inv = s
            .try_inverse()
            .expect("innovation covariance singular during iteration");
        let p_new = f * p * f.transpose() + q
            - f * p * h.transpose() * s_inv * h * p * f.transpose();

        // Cayley map back: Y_new = (P_new − αI)(P_new + αI)⁻¹.
        let p_plus = (p_new + alpha_i)
            .try_inverse()
            .expect("(P + αI) singular — should not happen for PD P");
        let y_new = (p_new - alpha_i) * p_plus;

        // Boundary check: 1 − σ_max(Y_new). Approaches zero as P → ∞.
        let sigma_max = y_new.svd(false, false).singular_values[0];
        let boundary_distance = 1.0 - sigma_max;
        if boundary_distance < BOUNDARY_EPS {
            panic!(
                "Cayley iterate approached boundary at iter {}: \
                 1 − σ_max(Y) = {} (must stay > {}). \
                 Likely (F, Q^½) not stabilizable, or numerical conditioning failure.",
                k, boundary_distance, BOUNDARY_EPS
            );
        }

        // Convergence check.
        let resid = (y_new - y).norm();
        if resid < CONV_TOL {
            return (p_new, k + 1);
        }

        y = y_new;
    }

    panic!(
        "Cayley-Riccati did not converge in {} iterations. \
         Last residual: ‖Y_{{k+1}} − Y_k‖_F at cap.",
        MAX_ITER
    );
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

fn emit_const(k_inf: &Mat4x2, iters: usize, spectral_radius: f32) -> String {
    let mut out = String::new();
    writeln!(out, "// Generated by partial-eval/build.rs — do not edit.").unwrap();
    writeln!(
        out,
        "// Closed-loop spectral radius: {} (< 1 ⇒ stable)",
        spectral_radius
    )
    .unwrap();
    writeln!(out, "pub const K_INF: [[f32; 2]; 4] = [").unwrap();
    for row in 0..4 {
        writeln!(
            out,
            "    [{:?}_f32, {:?}_f32],",
            k_inf[(row, 0)],
            k_inf[(row, 1)]
        )
        .unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out, "pub const STEADY_STATE_AT: usize = {};", iters).unwrap();
    out
}
