default:
    @just --list

# Architecture check — runs first so layer drift fails fast.
basis:
    basis-cli check --spec basis.yaml .

# Lab 1 — reference Kalman, builds for both host and thumbv7m.
lab1:
    cargo test -p kalman
    cargo build -p kalman --target thumbv7m-none-eabi

# Lab 2 — dual-number gradient matches numerical gradient.
lab2:
    cargo test -p autodiff

# Lab 3 — Riccati gain becomes a literal const at compile time.
lab3:
    cargo test -p partial-eval
    cargo expand -p partial-eval | grep K_INF

# Lab 4 — proc-macro emits straight-line specialized update.
lab4:
    cargo test -p codegen-demo
    cargo expand -p codegen-demo

# Lab 5 — boot the embedded binary in QEMU.
qemu:
    cargo xtask qemu --capture

# End-to-end: architecture intact, no nalgebra in MCU dep tree, all 3 implementations agree.
# `-e no-proc-macro` filters out the codegen proc-macro's host-side use of nalgebra
# (which doesn't reach the binary). The negated grep would be silently broken otherwise:
# `grep -vq foo` exits 0 whenever any line doesn't match, which is almost always.
verify:
    just basis
    ! cargo tree -p embedded --target thumbv7m-none-eabi -e no-proc-macro | grep -q nalgebra
    cargo run -p end-to-end -- --with-qemu-trace

# Track flash/RAM cost as we specialize more.
size:
    cargo size -p embedded --release --target thumbv7m-none-eabi

# Inspect the macro-expanded output of a specific crate.
expand crate:
    cargo expand -p {{crate}}
