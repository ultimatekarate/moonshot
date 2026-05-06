# Binary size and runtime timing — baseline measurements

| Field | Value |
| --- | --- |
| Date | 2026-05-06 |
| Toolchain | stable-x86_64-pc-windows-msvc, rustc 1.95.0 |
| Target | `thumbv7m-none-eabi` (lm3s6965evb in QEMU) |
| Profile | release: `opt-level=3, lto="fat", codegen-units=1, panic="abort"` |

## 1. Binary size

Measured via `llvm-size -A` on the linked ELF.

### Macro-only build (`Filter4x2` from the `bayesian_filter!` proc-macro)

```
section              size        addr
.vector_table        1024           0
.text               23,112        1024
.rodata              2,828       24,136
.bss                    12          —
Total flash         27,177
Static RAM              12 bytes
```

### Naive-baseline build (`--features naive-baseline`; runtime `kalman::cv_2d` runs alongside `Filter4x2`)

```
section              size        addr
.vector_table        1024           0
.text               28,572        1024
.rodata              3,060       29,600
.bss                    12          —
Total flash         32,869
Static RAM              12 bytes
```

### Delta

| Section | Macro-only | Macro + naive | Δ | Δ% |
| --- | ---: | ---: | ---: | ---: |
| `.text` | 23,112 | 28,572 | +5,460 | +24% |
| `.rodata` | 2,828 | 3,060 | +232 | +8% |
| **Total flash** | **27,177** | **32,869** | **+5,692** | **+21%** |
| Static RAM | 12 | 12 | 0 | 0% |

Adding the runtime nalgebra-based filter pathway costs **~5.5 KB of flash with full LTO**.

## 2. Honest interpretation of the size delta

The earlier framing in `docs/plan.md` ("order-of-magnitude smaller than runtime equivalent") **does not hold** apples-to-apples against lean Rust + LTO. It holds against C++/Eigen with allocator + exceptions + RTTI + vtables, where the same problem easily costs 50+ KB of overhead before any application code. Against a Rust runtime equivalent, the compile-time machinery saves ~5 KB and ~21%.

**Why the gap is smaller than the plan claimed:**

- Both builds use `panic = "abort"`, `no_std`, no allocator. The defensive infrastructure C++ would have is *already* absent from the runtime baseline.
- LTO + `codegen-units = 1` flattens nalgebra's generic dispatch entirely.
- nalgebra's 2×2 `try_inverse` is closed-form, not iterative — small.

**What the macro version actually avoids:**

- Runtime divisions in `try_inverse` (data-dependent timing on soft-float — see §3)
- Runtime `transpose` and matrix-multiply trait dispatch
- ~5 KB of `.text` for the above

The "20 KB+ saved" claim was against the wrong baseline. Update `docs/plan.md` accordingly.

## 3. Runtime timing

User-reported empirical result: cycle count per `Filter4x2::update()` is **bounded but not invariant**.

### Why bounded, not invariant

The instruction path through `update()` is fixed at compile time — straight-line `fmul`/`fadd` calls with literal coefficients, no data-dependent branches. Cycle count nevertheless varies because Cortex-M3 has no FPU; f32 ops go through compiler-rt's soft-float library:

- `__aeabi_fmul`, `__aeabi_fadd`, etc. have fast paths (normal numbers, ~10–20 cycles) and slow paths (subnormals, exact zeros, NaN/Inf — 30–100+ cycles).
- The instruction *sequence* is invariant; the *cycle cost* of each instruction is operand-dependent.

So the path is fixed; the per-instruction cost isn't.

### Why "bounded" is the right claim for real-time

Real-time scheduling theory requires:

- WCET is computable (an upper bound exists).
- WCET ≤ deadline.

It does *not* require BCET = WCET. The macro version satisfies both — its WCET is determined by the worst-case soft-float operand pattern × the fixed instruction path, and that bound is stable across calls. This is the property the literature calls *schedulable*. "Invariant" is strictly stronger and harder to defend; "bounded" is the load-bearing claim and is robust to a single counter-example.

### Recalibrated language for the docs

Earlier framing in `docs/plan.md` runtime-invariants and "what lifting buys at runtime" sections:

> Deterministic execution time per call. Same number of cycles, same for every input.

Honest framing:

> Bounded WCET per call, with a fixed instruction path. Cycle variation comes from soft-float operand handling, not from divergent code paths. The naive runtime version layers data-dependent control flow (inside `try_inverse` etc.) on top of the same soft-float jitter, so the macro version's WCET / BCET ratio is tighter and its WCET is computable from the spec alone.

## 4. Reproduction

```bash
# Macro-only build (default)
cargo build -p embedded --target thumbv7m-none-eabi --release
llvm-size -A target/thumbv7m-none-eabi/release/embedded

# Naive-baseline build
cargo build -p embedded --target thumbv7m-none-eabi --release --features naive-baseline
llvm-size -A target/thumbv7m-none-eabi/release/embedded

# Run in QEMU and observe cycle counts
PATH="/c/Program Files/qemu:$PATH" \
  cargo run -p embedded --target thumbv7m-none-eabi --release [--features naive-baseline]
```

`llvm-size` ships with the `llvm-tools` rustup component:

```bash
rustup component add llvm-tools
# Path: ~/.rustup/toolchains/<channel>/lib/rustlib/<host>/bin/llvm-size{,.exe}
```

## 5. Open: characterising WCET adversarially

The cycle-count harness currently runs against synthetic constant-velocity inputs (all-normal-number operands). The reported `min`/`max`/`spread` is the spread *over a benign trace*, not a defensible WCET.

To establish an empirical WCET bound, re-run with adversarial inputs:

- Subnormal operands (very small denormalised f32 values)
- Exact zeros (`0.0`, `-0.0`)
- `f32::INFINITY`, `f32::NEG_INFINITY`
- `f32::NAN` (signalling and quiet)
- Mixed signs near rounding boundaries

The maximum cycle count observed across all of these *is* the empirical WCET — the instruction path is fixed regardless of operand, so no input can take longer than the worst soft-float path through that fixed sequence. This is the measurement that backs the "schedulable" claim.

## 6. Static RAM headroom

12 bytes of `.bss` for the macro-only build. Stack usage not measured but bounded by the fixed locals in `main()` and `update()` (a few dozen bytes at most — no recursion, no allocation).

For comparison: a "32 KB-RAM microcontroller" target — the project's stated lower bound — uses 12 + small-stack ≈ <1 KB. **Static memory headroom: >97% of the budget unused.** The actual constraint on this class of chip is flash, not RAM, and at 27 KB flash we fit comfortably on devices with 64–128 KB flash and pinch on the smallest 32 KB-flash variants (e.g. STM32F030F4) without further trimming.
