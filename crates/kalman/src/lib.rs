#![no_std]

pub mod filter;

pub use filter::KalmanFilter;

pub type Cv2dF32 = filter::KalmanFilter<f32, 4, 2>;

/// 2D constant-velocity Kalman: state `[x, y, vx, vy]`, observation `[x, y]`.
/// Process noise `q` and observation noise `r` are scalar variances applied
/// uniformly to all state / observation components (toy simplification).
/// Initial state is zero; initial covariance is `I` (diffuse-ish prior).
pub fn cv_2d<T>(dt: T, q: T, r: T) -> filter::KalmanFilter<T, 4, 2>
where
    T: nalgebra::RealField + Copy,
{
    let zero = T::zero();
    let one = T::one();

    let f = nalgebra::SMatrix::<T, 4, 4>::from_row_slice(&[
        one,  zero, dt,   zero,
        zero, one,  zero, dt,
        zero, zero, one,  zero,
        zero, zero, zero, one,
    ]);
    let h = nalgebra::SMatrix::<T, 2, 4>::from_row_slice(&[
        one,  zero, zero, zero,
        zero, one,  zero, zero,
    ]);
    let q_mat = nalgebra::SMatrix::<T, 4, 4>::from_diagonal_element(q);
    let r_mat = nalgebra::SMatrix::<T, 2, 2>::from_diagonal_element(r);
    let x = nalgebra::SMatrix::<T, 4, 1>::zeros();
    let p = nalgebra::SMatrix::<T, 4, 4>::from_diagonal_element(one);

    filter::KalmanFilter { x, p, f, h, q: q_mat, r: r_mat }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SMatrix;

    /// Smoke test: 20 predict/update cycles on a synthetic linear trajectory
    /// don't blow the state to NaN.
    #[test]
    fn cv_2d_runs_without_nan() {
        let mut filter = cv_2d::<f32>(0.1, 0.01, 0.1);
        for k in 0..20 {
            let t = k as f32 * 0.1;
            let z = SMatrix::<f32, 2, 1>::new(t, 0.5 * t);
            filter.predict();
            filter.update(&z);
        }
        for x in filter.x.iter() {
            assert!(x.is_finite(), "state went non-finite");
        }
    }

    /// Tracks a known linear trajectory: position should converge toward
    /// the true state once observations dominate the prior.
    #[test]
    fn cv_2d_tracks_linear_trajectory() {
        let mut filter = cv_2d::<f32>(0.1, 0.001, 0.001);
        // True state: moving at vx=1, vy=0.5 from origin.
        for k in 0..200 {
            let t = k as f32 * 0.1;
            let z = SMatrix::<f32, 2, 1>::new(t, 0.5 * t);
            filter.predict();
            filter.update(&z);
        }
        // After 20 seconds of low-noise observations, position estimate
        // should be very close to the true trajectory at t = 20s.
        assert!((filter.x[0] - 20.0).abs() < 0.5, "x = {}", filter.x[0]);
        assert!((filter.x[1] - 10.0).abs() < 0.5, "y = {}", filter.x[1]);
    }
}
