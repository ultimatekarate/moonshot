# Test results

Measurements taken on this codebase, with date and toolchain so anyone can reproduce or refute. Add new results as their own file; do not edit historical files in place — they're a record of what was true on a given day with a given build.

## Index

- [binary_size.md](binary_size.md) — Flash and RAM usage of the embedded binary, measured with `llvm-size`. Distinguishes ELF-on-disk size (debug info included) from flash image (what actually goes on the chip).
- [cycle_counter.md](cycle_counter.md) — Behavior of `DWT::CYCCNT` on QEMU's `lm3s6965evb` machine. Documents what the cycle-determinism demo currently shows and what it does not.
- [end_to_end.md](end_to_end.md) — Trajectory comparison between the reference Kalman, the macro-emitted steady-state filter, and the synthetic ground truth. Numbers from `cargo run -p end-to-end`.
- [basis_check.md](basis_check.md) — Architectural-governance check from `basis-cli`. Records that the layered design is intact and which layer-name aliases were needed.

## What's measured vs what's claimed

The README intentionally avoids stating performance numbers in prose. Numbers belong here, attached to the command that produced them and the date of measurement, so that drift between the README narrative and reality is impossible.

If you find a claim in the README that this directory does not back up, that's a bug — file an issue or strip the claim.
