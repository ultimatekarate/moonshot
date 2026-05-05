#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use nalgebra::SMatrix;
use num_dual::Dual32;

pub mod numeric;

/// Build a `cv_2d`-shaped filter where the 6 parameters
/// `[q_xx, q_yy, q_vxvx, q_vyvy, r_xx, r_yy]` are the diagonal entries of Q
/// and R. Generic over `T` so the same code drives both the f32 reference
/// (for central differences) and the `Dual32` AD path.
fn build_filter<T>(params: &[T; 6], dt: T) -> kalman::KalmanFilter<T, 4, 2>
where
    T: nalgebra::RealField + Copy,
{
    let zero = T::zero();
    let one = T::one();

    let f = SMatrix::<T, 4, 4>::from_row_slice(&[
        one,  zero, dt,   zero,
        zero, one,  zero, dt,
        zero, zero, one,  zero,
        zero, zero, zero, one,
    ]);
    let h = SMatrix::<T, 2, 4>::from_row_slice(&[
        one,  zero, zero, zero,
        zero, one,  zero, zero,
    ]);
    let mut q_mat = SMatrix::<T, 4, 4>::zeros();
    q_mat[(0, 0)] = params[0];
    q_mat[(1, 1)] = params[1];
    q_mat[(2, 2)] = params[2];
    q_mat[(3, 3)] = params[3];
    let mut r_mat = SMatrix::<T, 2, 2>::zeros();
    r_mat[(0, 0)] = params[4];
    r_mat[(1, 1)] = params[5];
    let x = SMatrix::<T, 4, 1>::zeros();
    let p = SMatrix::<T, 4, 4>::from_diagonal_element(one);
    kalman::KalmanFilter { x, p, f, h, q: q_mat, r: r_mat }
}

/// Negative log-likelihood at `params` for the observation sequence `zs`.
/// Generic in `T` so f32 and Dual32 share the same body.
pub fn nll<T>(params: &[T; 6], dt: T, zs: &[SMatrix<T, 2, 1>]) -> T
where
    T: nalgebra::RealField + Copy,
{
    let mut filter = build_filter(params, dt);
    filter.neg_log_likelihood(zs)
}

/// Forward-mode AD via num-dual. One forward pass per parameter (6 total
/// for our toy) — at this scale the cost is trivial and the implementation
/// is dramatically simpler than vector-mode dual numbers.
pub fn gradient_nll(params: &[f32; 6], zs: &[[f32; 2]]) -> [f32; 6] {
    let dt_const = Dual32::from(0.1_f32);
    let zs_dual: Vec<SMatrix<Dual32, 2, 1>> = zs
        .iter()
        .map(|z| SMatrix::<Dual32, 2, 1>::new(Dual32::from(z[0]), Dual32::from(z[1])))
        .collect();

    let mut grad = [0.0_f32; 6];
    for i in 0..6 {
        let dual_params: [Dual32; 6] = core::array::from_fn(|j| {
            let mut d = Dual32::from(params[j]);
            if j == i {
                d.eps = 1.0;
            }
            d
        });
        let result = nll(&dual_params, dt_const, &zs_dual);
        grad[i] = result.eps;
    }
    grad
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn synthetic_trace() -> Vec<[f32; 2]> {
        (0..50).map(|k| {
            let t = k as f32 * 0.1;
            [t, 0.5 * t]
        }).collect()
    }

    /// The whole point of Lab 2: dual-number gradients agree with central
    /// differences. If this passes, the AD path is real and Lab 2 is done.
    #[test]
    fn dual_grad_matches_numeric() {
        let params = [0.01_f32, 0.01, 0.01, 0.01, 0.1, 0.1];
        let zs = synthetic_trace();

        let g_dual = gradient_nll(&params, &zs);
        let g_numeric = numeric::central_difference_gradient(&params, &zs, 1e-3);

        for i in 0..6 {
            let diff = (g_dual[i] - g_numeric[i]).abs();
            let scale = g_dual[i].abs().max(g_numeric[i].abs()).max(1.0);
            assert!(
                diff / scale < 1e-2,
                "param {}: dual = {}, numeric = {}, rel_diff = {}",
                i, g_dual[i], g_numeric[i], diff / scale,
            );
        }
    }
}
