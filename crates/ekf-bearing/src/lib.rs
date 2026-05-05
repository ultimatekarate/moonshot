#![no_std]

use nalgebra::SMatrix;

pub struct EkfBearing<T>
where
    T: nalgebra::RealField + Copy,
{
    pub x: SMatrix<T, 4, 1>,
    pub p: SMatrix<T, 4, 4>,
    pub f: SMatrix<T, 4, 4>,
    pub q: SMatrix<T, 4, 4>,
    pub r: T,
    pub sensor: SMatrix<T, 2, 1>,
}

impl<T> EkfBearing<T>
where
    T: nalgebra::RealField + Copy,
{
    pub fn predict(&mut self) {
        todo!("stub: see docs/plan.md Lab 1c")
    }

    /// Update on a scalar bearing observation. The observation Jacobian
    /// `H = ∂h/∂x` where `h(x) = atan2(y − sy, x − sx)` is computed at
    /// runtime via dual numbers — this is what makes the autodiff lab
    /// load-bearing (unlike Kalman, where it's optional for parameter learning).
    pub fn update(&mut self, _bearing: T) {
        todo!("stub: linearize h around current x, run EKF update")
    }
}
