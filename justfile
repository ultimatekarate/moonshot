default:
    @just --list

# Architecture check — runs first so layer drift fails fast.
basis:
    basis-cli check --spec basis.yaml .

# Lab 1 — reference Kalman, builds for both host and thumbv7m.
lab1:
    cargo test -p model
    cargo build -p model --target thumbv7m-none-eabi

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
verify:
    just basis
    cargo tree -p embedded | grep -vq nalgebra
    cargo run -p end-to-end -- --with-qemu-trace

# Track flash/RAM cost as we specialize more.
size:
    cargo size -p embedded --release --target thumbv7m-none-eabi

# Inspect the macro-expanded output of a specific crate.
expand crate:
    cargo expand -p {{crate}}
