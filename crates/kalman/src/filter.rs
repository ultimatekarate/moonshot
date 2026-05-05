use nalgebra::SMatrix;

pub struct KalmanFilter<T, const N: usize, const M: usize>
where
    T: nalgebra::RealField + Copy,
{
    pub x: SMatrix<T, N, 1>,
    pub p: SMatrix<T, N, N>,
    pub f: SMatrix<T, N, N>,
    pub h: SMatrix<T, M, N>,
    pub q: SMatrix<T, N, N>,
    pub r: SMatrix<T, M, M>,
}

impl<T, const N: usize, const M: usize> KalmanFilter<T, N, M>
where
    T: nalgebra::RealField + Copy,
{
    pub fn predict(&mut self) {
        self.x = self.f * self.x;
        self.p = self.f * self.p * self.f.transpose() + self.q;
    }

    /// Standard (non-Joseph) covariance update. Joseph form is more numerically
    /// stable but is deferred — this is the bare-bones reference, and `(I − KH)P`
    /// is good enough at f32 precision for the toy. Closed-loop stability check
    /// at build time will catch the cases where it isn't.
    pub fn update(&mut self, z: &SMatrix<T, M, 1>) {
        let y = z - self.h * self.x;
        let s = self.h * self.p * self.h.transpose() + self.r;
        let s_inv = s.try_inverse().expect("innovation covariance singular");
        let k = self.p * self.h.transpose() * s_inv;
        self.x += k * y;
        let i = SMatrix::<T, N, N>::identity();
        self.p = (i - k * self.h) * self.p;
    }

    pub fn neg_log_likelihood(&mut self, _zs: &[SMatrix<T, M, 1>]) -> T {
        todo!("Lab 2 will need this; defer until autodiff is implemented")
    }
}
