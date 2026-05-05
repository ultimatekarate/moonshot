// Synthetic constant-velocity trajectory: position moves at vx=1, vy=0.5
// from origin, sampled every 0.1s for 10 seconds. Same trace the host-side
// equivalence test uses, so QEMU output can be compared directly against it.
pub const SAMPLES: [(f32, f32); 100] = {
    let mut s = [(0.0_f32, 0.0_f32); 100];
    let mut k = 0;
    while k < 100 {
        let t = k as f32 * 0.1;
        s[k] = (t, 0.5 * t);
        k += 1;
    }
    s
};
