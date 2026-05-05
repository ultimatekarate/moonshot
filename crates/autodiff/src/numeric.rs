use alloc::vec::Vec;
use nalgebra::SMatrix;

pub fn central_difference_gradient(params: &[f32; 6], zs: &[[f32; 2]], eps: f32) -> [f32; 6] {
    let zs_f32: Vec<SMatrix<f32, 2, 1>> = zs
        .iter()
        .map(|z| SMatrix::<f32, 2, 1>::new(z[0], z[1]))
        .collect();

    let mut grad = [0.0_f32; 6];
    for i in 0..6 {
        let mut p_plus = *params;
        let mut p_minus = *params;
        p_plus[i] += eps;
        p_minus[i] -= eps;
        let nll_plus = crate::nll(&p_plus, 0.1_f32, &zs_f32);
        let nll_minus = crate::nll(&p_minus, 0.1_f32, &zs_f32);
        grad[i] = (nll_plus - nll_minus) / (2.0 * eps);
    }
    grad
}
