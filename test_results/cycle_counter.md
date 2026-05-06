# Cycle counter behavior in QEMU

**Date:** 2026-05-06
**Target:** `thumbv7m-none-eabi`, profile = release
**Runner:** `qemu-system-arm -cpu cortex-m3 -machine lm3s6965evb -nographic -semihosting-config enable=on,target=native -icount shift=0 -kernel <bin>`
**Cycle-counter setup:** `DCB::enable_trace()` + `DWT::enable_cycle_counter()` at boot, `DWT::cycle_count()` sampled before/after each `update()` inside `cortex_m::interrupt::free` with surrounding `compiler_fence(SeqCst)`.

## Result

Every per-sample cycle delta on this configuration is **0**. Sample output:

```
update # 71:    0 cycles  x=   7.100 y=   3.550 vx= 0.999 vy= 0.500
update # 72:    0 cycles  x=   7.200 y=   3.600 vx= 0.999 vy= 0.500
...
update # 99:    0 cycles  x=   9.900 y=   4.950 vx= 1.000 vy= 0.500
FLAT (macro): all 100 updates ran in exactly 0 cycles.
```

## Interpretation

The "FLAT" message is not a successful demonstration of cycle determinism. It is the sentinel value for **QEMU's `lm3s6965evb` model not implementing `DWT::CYCCNT`**. The register reads return 0 unconditionally, so `after.wrapping_sub(before) == 0` for every measurement, regardless of what the CPU actually did.

This is a known limitation of QEMU's older Stellaris-family machine models. Newer machines (`mps2-an385` for Cortex-M3, `mps2-an386` for Cortex-M4) emulate the DWT registers correctly. The original choice of `lm3s6965evb` predates the cycle-counting goal.

## What this proves and doesn't prove

- ✓ The DWT setup code in [crates/embedded/src/main.rs](../crates/embedded/src/main.rs) compiles, links, and runs without panicking. The peripheral access path through the `cortex-m` crate is intact.
- ✓ The trajectory is correct (positions track `(0.1·k, 0.05·k)`, velocities settle to `(1.0, 0.5)`), so the filter math is doing its job. Independent of the cycle-counter issue.
- ✗ **Cycle determinism is not demonstrated.** The flat zero is an artifact of the emulator, not a property of the binary. Any claim of the form "every input takes the same number of cycles" is unverified on the current setup.
- ✗ **Variance** of the soft-float arithmetic on a real Cortex-M3 is also unverified; we don't have a baseline of true cycle counts to point to.

## What would actually verify cycle determinism

1. **Switch QEMU machine to `mps2-an385`.** Same Cortex-M3 ISA, working DWT model. Requires a different `memory.x` (the AN385 has different flash/RAM addresses) and a different runner in [.cargo/config.toml](../.cargo/config.toml).
2. **Run on real Cortex-M3 silicon** (e.g., STM32F103 "Blue Pill"). DWT::CYCCNT is implemented in hardware. Adds the toolchain dependency on `probe-rs` and a physical board.
3. **Switch to a Cortex-M4F target** (e.g., `mps2-an386` or STM32G431). Hardware FPU with `FPSCR.FZ=1` would give constant-cycle floating-point operations directly. Different `memory.x`, different runner, different toolchain target (`thumbv7em-none-eabihf`).

Until one of those is done, the cycle-determinism property is a hypothesis, not an observation.

## Note on `-icount shift=0`

The `-icount shift=0` flag was added to make QEMU's instruction count deterministic across runs. It does that — but it doesn't make `lm3s6965evb` start emulating DWT. The two are independent. Removing `-icount` does not change the result above; the cycle counter still reports 0.
