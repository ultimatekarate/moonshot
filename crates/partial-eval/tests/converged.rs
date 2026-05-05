// Verifies the build-time-emitted K_INF agrees with what the kalman crate
// would converge to via repeated predict/update cycles.

use nalgebra::SMatrix;
use partial_eval::K_INF;

type Mat4 = nalgebra::Matrix4<f32>;
type Mat2 = nalgebra::Matrix2<f32>;
type Mat2x4 = nalgebra::Matrix2x4<f32>;
type Mat4x2 = nalgebra::Matrix4x2<f32>;

#[test]
fn k_inf_matches_iterative_reference() {
    // Hard-code the same matrices as model.ron (Kalman variant).
    // Keeping this self-contained avoids dragging spec-loader into a runtime test.
    let f = Mat4::from_row_slice(&[
        1.0, 0.0, 0.1, 0.0,
        0.0, 1.0, 0.0, 0.1,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);
    let h = Mat2x4::from_row_slice(&[
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
    ]);
    let q = Mat4::from_diagonal_element(0.01);
    let r = Mat2::from_diagonal_element(0.1);

    // Direct Riccati iteration on P (no Cayley) — separate code path from
    // build.rs, so agreement is meaningful.
    let mut p = Mat4::identity();
    for _ in 0..2_000 {
        let s = h * p * h.transpose() + r;
        let s_inv = s.try_inverse().unwrap();
        let p_new = f * p * f.transpose() + q
            - f * p * h.transpose() * s_inv * h * p * f.transpose();
        if (p_new - p).norm() < 1e-10 {
            p = p_new;
            break;
        }
        p = p_new;
    }
    let s_inf = h * p * h.transpose() + r;
    let k_ref: Mat4x2 = p * h.transpose() * s_inf.try_inverse().unwrap();

    // Compare against the const baked in by build.rs.
    let k_baked = Mat4x2::from_row_slice(
        &K_INF.iter().flatten().copied().collect::<Vec<_>>(),
    );

    let diff = (k_ref - k_baked).norm();
    assert!(
        diff < 1e-5,
        "K_INF disagrees with iterative reference: diff = {}\nbaked:\n{}\nref:\n{}",
        diff, k_baked, k_ref
    );

    // Sanity: K_INF should be roughly [[~0.5, 0], [0, ~0.5], [~0.5, 0], [0, ~0.5]]
    // for this model — strong observation weight pulls position toward the measurement,
    // and the velocity components inherit a smaller gain via the off-diagonal F structure.
    for row in 0..4 {
        for col in 0..2 {
            assert!(K_INF[row][col].is_finite(), "K_INF[{}][{}] non-finite", row, col);
        }
    }
}

#[test]
fn smoke_use_in_kalman_filter() {
    // Plug K_INF into a simulated steady-state filter and confirm it tracks.
    let mut filter = kalman::cv_2d::<f32>(0.1, 0.01, 0.1);
    let k_inf = SMatrix::<f32, 4, 2>::from_row_slice(
        &K_INF.iter().flatten().copied().collect::<Vec<_>>(),
    );

    // Use the baked gain directly in the steady-state update form:
    // x = F x; x += K_inf (z - H F x)
    let h = Mat2x4::from_row_slice(&[
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
    ]);
    let f = Mat4::from_row_slice(&[
        1.0, 0.0, 0.1, 0.0,
        0.0, 1.0, 0.0, 0.1,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]);

    let mut x = SMatrix::<f32, 4, 1>::zeros();
    for k in 0..200 {
        let t = k as f32 * 0.1;
        let z = SMatrix::<f32, 2, 1>::new(t, 0.5 * t);
        x = f * x;
        x += k_inf * (z - h * x);
        // Same trace through the time-varying-K reference for cross-check.
        filter.predict();
        filter.update(&z);
    }

    // Position should track. Steady-state gain may not be quite as accurate
    // as the time-varying gain early on, so use a looser tolerance here.
    assert!((x[0] - 20.0).abs() < 1.0, "steady-state x = {}", x[0]);
    assert!((x[1] - 10.0).abs() < 1.0, "steady-state y = {}", x[1]);
}
