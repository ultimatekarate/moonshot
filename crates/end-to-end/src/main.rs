fn main() {
    // Stub: see docs/plan.md "End-to-end glue".
    // Real implementation dispatches per model class:
    //   Kalman:        (a) kalman::cv_2d  (b) codegen_demo::Filter4x2  (c) QEMU trace
    //   GammaPoisson:  (a) gamma_poisson  (b) codegen_demo macro output (c) QEMU trace
    //   EkfBearing:    (a) ekf_bearing    (b) codegen_demo macro output (c) QEMU trace
    // Tolerance is per-component (no bit-identical claim); exit nonzero on violation.
    println!("end-to-end stub — not yet implemented");
}
