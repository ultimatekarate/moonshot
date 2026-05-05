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
        todo!("stub: x = F x; P = F P Fᵀ + Q")
    }

    pub fn update(&mut self, _z: &SMatrix<T, M, 1>) {
        todo!("stub: Joseph-form covariance update")
    }

    pub fn neg_log_likelihood(&mut self, _zs: &[SMatrix<T, M, 1>]) -> T {
        todo!("stub: sum of innovation log-likelihoods")
    }
}
