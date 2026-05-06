# End-to-end trajectory equivalence

**Date:** 2026-05-06
**Command:** `cargo run -p end-to-end -- --with-qemu-trace`
**Source:** [crates/end-to-end/src/main.rs](../crates/end-to-end/src/main.rs)

## What's compared

Three trajectories on the same 100-sample synthetic trace `z[k] = (0.1·k, 0.05·k)`:

1. **Reference** — `kalman::cv_2d::<f32>` running on the host with runtime `nalgebra`. Time-varying Kalman gain (P updates every step from `P_0 = I`).
2. **Macro** — `codegen_demo::Filter4x2` running on the host with `f32`. Steady-state K_inf baked in by the proc-macro at expansion time.
3. **QEMU** — same macro filter, cross-compiled for `thumbv7m-none-eabi`, run under QEMU's `lm3s6965evb`, output captured to `target/qemu-trace.bin`.
4. **Truth** — noise-free analytical trajectory `[0.1·k, 0.05·k, 1.0, 0.5]` for k in 0..100.

## Result

```
After 20-sample transient:
  reference (runtime-K) vs truth:
     x: max-abs-diff =   0.011585
     y: max-abs-diff =   0.005792
    vx: max-abs-diff =   0.040622
    vy: max-abs-diff =   0.020311
  macro (steady-state K_inf) vs truth:
     x: max-abs-diff =   0.050142
     y: max-abs-diff =   0.025071
    vx: max-abs-diff =   0.175896
    vy: max-abs-diff =   0.087948
  reference vs macro (informational — not asserted):
     x: max-abs-diff =   0.038557
     y: max-abs-diff =   0.019279
    vx: max-abs-diff =   0.135274
    vy: max-abs-diff =   0.067637

QEMU macro vs host macro (same code, no transient skip):
     x: max-abs-diff =   0.000000
     y: max-abs-diff =   0.000000
    vx: max-abs-diff =   0.000000
    vy: max-abs-diff =   0.000000

OK: truth-tracking and (if checked) QEMU equivalence within bounds.
```

## Interpretation

**Both filters track ground truth.** The reference (time-varying K) lands closer to truth because P adapts; the macro filter (steady-state K_inf from step 0) is slightly further off but still within `0.5` on every component, well under the noise scale of `R = 0.1`. Both are valid filters for this trace.

**Reference and macro do not match each other bit-for-bit.** The 0.04 position diff and 0.14 velocity diff between the two is the genuine difference between time-varying K and steady-state K, not a bug. Reported as informational; not asserted as a tolerance check.

**Host macro and QEMU macro agree to zero in all four components.** Same source, same `f32` arithmetic; the cross-compilation does not introduce any observable rounding difference on this trace. This is the bit-equivalence claim the harness is designed to test.

## What this proves and doesn't prove

- ✓ The proc-macro emits a *correct* filter — its trajectory tracks ground truth to within noise scale.
- ✓ The macro-emitted code is bit-identical between host execution and QEMU execution. Host f32 and Cortex-M3 soft-float `f32` produce the same numbers on this trace.
- ✓ The architectural pipeline composes end-to-end: `model.ron` → `build.rs` Cayley-Riccati → proc-macro Cayley-Riccati → emitted constants → no_std binary → QEMU → measurable trajectory.
- ✗ **Bit-identity does not generalize automatically.** The synthetic trace exercises a small region of the operand space. A trace with denormals, NaN-producing inputs, or extreme magnitudes might surface differences between host and target soft-float. We tested one trajectory.

## Tolerances chosen

- `TRUTH_TOLERANCE = 0.5` — generous, well above the noise scale `sqrt(R) ≈ 0.3`. Filters that diverge from the truth would still flag.
- `QEMU_TOLERANCE = 1e-4` — tight, because same code on two platforms should differ only by f32 reordering. Currently observes `0.0`.
- `TRANSIENT = 20` — first 20 samples skipped from the truth-tracking comparison to allow the time-varying-K reference to converge to its steady-state behavior.

## Reproducing this

```bash
cargo xtask qemu --capture                 # writes target/qemu-trace.bin
cargo run -p end-to-end -- --with-qemu-trace
```
