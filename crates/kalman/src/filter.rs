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

}

/// Specialised to `M = 2` because `nalgebra::Matrix::determinant` needs
/// `Const<M>: ToTypenum` bounds that fight bare const generics; we use a
/// closed-form 2x2 det instead. Generalising lifts cleanly when needed.
impl<T, const N: usize> KalmanFilter<T, N, 2>
where
    T: nalgebra::RealField + Copy,
{
    /// Sum of per-step Gaussian innovation log-likelihoods over a sequence
    /// of observations. Drops the additive `0.5 · log((2π)^M)` constants
    /// since they don't affect gradients w.r.t. parameters. Mutates `self`
    /// (the filter advances state as it consumes observations).
    pub fn neg_log_likelihood(&mut self, zs: &[SMatrix<T, 2, 1>]) -> T {
        let half = T::one() / (T::one() + T::one());
        let mut nll = T::zero();
        for z in zs {
            self.predict();
            let y = z - self.h * self.x;
            let s = self.h * self.p * self.h.transpose() + self.r;
            let s_inv = s.try_inverse().expect("innovation covariance singular");
            // Closed-form 2x2 determinant.
            let det_s = s[(0, 0)] * s[(1, 1)] - s[(0, 1)] * s[(1, 0)];
            let log_det_s = det_s.ln();
            let quad = (y.transpose() * s_inv * y)[(0, 0)];
            nll = nll + half * (log_det_s + quad);
            self.update(z);
        }
        nll
    }
}
