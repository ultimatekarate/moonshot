#![no_std]

pub mod filter;

pub use filter::KalmanFilter;

pub type Cv2dF32 = filter::KalmanFilter<f32, 4, 2>;

pub fn cv_2d<T>(_dt: T, _q: T, _r: T) -> filter::KalmanFilter<T, 4, 2>
where
    T: nalgebra::RealField + Copy,
{
    todo!("stub: see docs/plan.md Lab 1 — construct F, H, Q, R for constant-velocity 2D model")
}
