use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR must be set by cargo"));
    let dest = out_dir.join("k_inf.rs");

    // Stub: emit a placeholder K_INF until the real Riccati solver lands.
    //
    // Real implementation runs in two preconditioning steps before the
    // bounded iteration begins:
    //
    //   1. State scaling. Apply a similarity transform with D = diag(state_scale):
    //          x'  = D⁻¹ x      F'  = D⁻¹ F D
    //          Q'  = D⁻¹ Q D⁻ᵀ  H'  = H D
    //      so that all state components have comparable dynamic range. (R is
    //      unchanged — it lives in measurement space.) This is well-conditioning
    //      the *problem* before any solver runs.
    //
    //   2. Cayley parametrization with reference α = cayley_alpha:
    //          Y_k = (P_k − αI)(P_k + αI)⁻¹      (Cayley map: PD cone → contractions)
    //          P_k = α(I + Y_k)(I − Y_k)⁻¹      (pullback)
    //
    //      The Riccati update on Y stays in the bounded set `‖Y‖ < 1` by
    //      construction — finite escape time is impossible in these coordinates.
    //      Inversion of `(I − Y)` is well-conditioned everywhere except the
    //      boundary. With (1) done, α = 1.0 is usually a fine starting reference.
    //
    // Boundary guard (replaces the old finite-escape guard): at each step,
    // compute `boundary_distance = 1 − σ_max(Y_k)`. If it drops below ε
    // (say 1e-6), return RiccatiResult::EscapedFiniteTime {at_iteration,
    // boundary_distance} and panic with a diagnostic. This is the
    // well-conditioned analogue of the divergent-norm check — the same
    // physical phenomenon (P → ∞) detected in coordinates where it is
    // actually finite and computable.
    //
    // Once converged, recover P_inf via the pullback and compute
    // K_inf = P_inf Hᵀ (H P_inf Hᵀ + R)⁻¹.
    //
    // Closed-loop stability check (the second build-time invariant): form
    //     F_cl = (I − K_inf H) F
    // and verify max|λ(F_cl)| < 1 via nalgebra's complex_eigenvalues().
    // A converged Riccati can still produce a marginally-stable gain that
    // drives slow runtime divergence; this catches it before bake. On
    // failure, return ClosedLoopStability::Unstable { spectral_radius }
    // and panic with the radius in the diagnostic.
    //
    // See docs/plan.md Lab 3 and the "Runtime invariants" section.
    let placeholder = "\
pub const K_INF: [[f32; 2]; 4] = [
    [0.0, 0.0],
    [0.0, 0.0],
    [0.0, 0.0],
    [0.0, 0.0],
];
pub const STEADY_STATE_AT: usize = 0;
";
    fs::write(&dest, placeholder).expect("write k_inf.rs");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../model.ron");
    println!("cargo:rerun-if-env-changed=MOONSHOT_MODEL");
}
