// The whole point of Lab 4: macro-expanded `Filter4x2` (steady-state K_INF
// baked in, straight-line update) should match the time-varying reference
// `kalman::cv_2d` to within tolerance after the filter reaches steady state.

use codegen_demo::Filter4x2;
use nalgebra::SMatrix;

#[test]
fn filter4x2_matches_kalman_reference_after_settling() {
    let mut macro_filter = Filter4x2::new();
    let mut ref_filter = kalman::cv_2d::<f32>(0.1, 0.01, 0.1);

    // 200 steps of synthetic constant-velocity trajectory.
    for k in 0..200 {
        let t = k as f32 * 0.1;
        let z_x = t;
        let z_y = 0.5 * t;
        macro_filter.update(z_x, z_y);
        ref_filter.predict();
        ref_filter.update(&SMatrix::<f32, 2, 1>::new(z_x, z_y));
    }

    // After settling, the steady-state and time-varying gains should give
    // very close estimates. Tolerance is per-component; position dominates
    // the trajectory so use slightly looser bound.
    let macro_x = macro_filter.x;
    let ref_x = [ref_filter.x[0], ref_filter.x[1], ref_filter.x[2], ref_filter.x[3]];

    for i in 0..4 {
        let diff = (macro_x[i] - ref_x[i]).abs();
        assert!(
            diff < 0.5,
            "component {}: macro = {}, ref = {}, diff = {}",
            i, macro_x[i], ref_x[i], diff
        );
    }

    // Both should be tracking near the true position at t = 20s.
    assert!((macro_x[0] - 20.0).abs() < 1.0, "macro x = {}", macro_x[0]);
    assert!((macro_x[1] - 10.0).abs() < 1.0, "macro y = {}", macro_x[1]);
}

#[test]
fn filter4x2_runs_without_nan() {
    let mut filter = Filter4x2::new();
    for k in 0..50 {
        let t = k as f32 * 0.1;
        filter.update(t, 0.5 * t);
    }
    for x in filter.x.iter() {
        assert!(x.is_finite(), "state went non-finite");
    }
}
