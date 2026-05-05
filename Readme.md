# Moonshot

Pushing as much of a Bayesian inference pipeline into the Rust compiler as possible, then deploying to a 32KB-class microcontroller. Exploratory research — the goal is to find out what's possible, where the friction is, and whether the pipeline composes. It seems like it should work.

## Research question

This is the motivating question:

> How do we use Rust's const evaluation to verify a prior, specialize a sampler via partial evaluation, and deploy a bit-accurate, verified posterior solver to a 32KB RAM microcontroller?

## Original outline, and how it changed

I have no idea what I'm doing. This is **research**. I'm not supposed to know what I'm doing. I had an initial idea that I thought might work; but I ran into some snags. 

| Stage | First idea | Better(?) idea | Why it changed |
| --- | --- | --- | --- |
| Type-level dimensions | `nalgebra` | `nalgebra` const-generic `SMatrix<T, R, C>` | unchanged — turnkey |
| Compile-time AD | `rust-enzyme` | `num-dual` (forward mode) over generic `Scalar` | Enzyme needs a custom-built nightly rustc; this is already a house of cards, no sense in ripping one in half before I start building. |
| Partial evaluation | `weval` | `build.rs` (Riccati solver) + `const fn` + proc-macros | weval partial-evaluates WASM interpreters; doesn't apply to native MCU codegen |
| Specialized loops | `syn` / `quote` | same | unchanged |
| Embedded execution | `embassy` | `cortex-m-rt` + `embassy` (added once bare-metal boots) | unchanged |

Plus an architectural-governance layer: [`basis.yaml`](basis.yaml) declares layer/purity/boundary/newtypes/exhaustive-matching rules. `basis-cli check` blocks any commit that crosses them. Drift between the strict `no_std` core, the build-time IO, the proc-macro-time IO, and the embedded shell would otherwise quietly destroy the experiment.

I can hear the protests now: "Riccati equations?! They have finite escape time! Isn't that a footgun?" Yes, but I have a plan. Is it a good plan? Again, no idea. **Research**.

## The toy problem

A 2D constant-velocity Kalman filter. State `[x, y, vx, vy]`, observation `[x, y]`. Closed-form posterior, fits 32KB easily, has both static parts (steady-state gain) and dynamic parts (innovation update) — ideal for partial-evaluation experiments.

## Layout

```
moonshot/
├── basis.yaml                  # architectural governance, all five axes
├── model.ron                   # single source of truth for F, H, Q, R, dt
├── crates/
│   ├── spec/                   # dictionary: ModelSpec, newtypes, completeness enums
│   ├── spec-loader/            # IO bridge: file → ModelSpec (build.rs + proc-macro both use this)
│   ├── model/                  # Lab 1: KalmanFilter<T, N, M>, generic in T
│   ├── autodiff/               # Lab 2: forward-mode AD via num-dual
│   ├── partial-eval/           # Lab 3: build.rs solves Riccati, embeds K_inf as const
│   ├── codegen/                # Lab 4: kalman_filter! proc-macro
│   ├── codegen-demo/           # consumer of the macro
│   ├── embedded/               # Lab 5: no_std bin, runs in QEMU lm3s6965evb
│   └── end-to-end/             # host harness: reference vs macro vs QEMU trace
├── xtask/                      # cargo xtask qemu --capture, verify
└── docs/plan.md                # full design
```

## Running

```bash
just basis     # architecture check (basis-cli)
just lab1      # reference Kalman tests + thumbv7m-none-eabi build
just lab2      # dual-number gradient matches numerical (within 1e-3)
just lab3      # K_INF lands as a literal const
just lab4      # macro emits straight-line specialized update
just qemu      # boot the embedded binary in QEMU
just verify    # all of the above + dep-tree check + 3-way equivalence
```

The repo is currently scaffolded — every crate compiles structurally with `todo!()` bodies and pointers back to the relevant lab. See [`docs/plan.md`](docs/plan.md) for the per-lab specs, integration glue, open risks, and verification plan.
