# Moonshot

Pushing as much of a Bayesian inference pipeline into the Rust compiler as possible, then deploying to a Cortex-M target. Exploratory research — the goal is to find out what's possible, where the friction is, and whether the pipeline composes. The pipeline composes; the runtime properties are a separate question, measured per-target. See [test_results/](test_results/) for what's been measured and what hasn't.

## Research question

This is the motivating question:

> How do we use Rust's const evaluation to verify a prior, specialize a sampler via partial evaluation, and deploy a bit-accurate, verified posterior solver to a small embedded target?

## Original outline, and how it changed

This is **research**. I'm not supposed to know what I'm doing. I had an initial idea that I thought might work; but I ran into some snags.

| Stage | First idea | Better(?) idea | Why it changed |
| --- | --- | --- | --- |
| Type-level dimensions | `nalgebra` | `nalgebra` const-generic `SMatrix<T, R, C>` | unchanged — turnkey |
| Compile-time AD | `rust-enzyme` | `num-dual` (forward mode) over generic `Scalar` | Enzyme needs a custom-built nightly rustc; this is already a house of cards, no sense in ripping one in half before I start building. |
| Partial evaluation | `weval` | `build.rs` (Riccati solver) + `const fn` + proc-macros | weval partial-evaluates WASM interpreters; doesn't apply to native MCU codegen |
| Specialized loops | `syn` / `quote` | same | unchanged |
| Embedded execution | `embassy` | `cortex-m-rt` + `embassy` (added once bare-metal boots) | unchanged |

Plus an architectural-governance layer: [`basis.yaml`](basis.yaml) declares layer/purity/boundary/newtypes/exhaustive-matching rules. `basis-cli check` blocks any commit that crosses them. Drift between the strict `no_std` core, the build-time IO, the proc-macro-time IO, and the embedded shell would otherwise quietly destroy the experiment.

I can hear the protests now: "Riccati equations?! They have finite escape time! Isn't that a footgun?" Yes, but I have a plan. Is it a good plan? Again, no idea. **Research**.

## The toy problems (three of them)

One toy problem would let the architecture quietly over-fit to it. Three force the choices that survive across all three to be the real architectural commitments; everything else gets quarantined into the model's own crate.

| Model | Real-world stand-in | What it stresses |
| --- | --- | --- |
| **2D Kalman** (`crates/kalman`) | Drone IMU/GPS fusion, Li-ion SOC, AHRS | Closed-form gain, Riccati, eigenvalue stability |
| **Gamma-Poisson** (`crates/gamma-poisson`) | Predictive maintenance, packet-loss / queue-rate monitoring | The architecture *without* nalgebra, AD, or Riccati |
| **EKF-bearing** (`crates/ekf-bearing`) | Passive sonar, anti-drone DF, vision-based localization | AD genuinely load-bearing for the Jacobian, no steady state |

[`crates/spec`](crates/spec) holds a sum-typed `ModelSpec` over the three; every consumer (build.rs, proc-macros, host harness) dispatches.

## Layout

```
moonshot/
├── basis.yaml                  # architectural governance, all five axes
├── model.ron                   # Kalman variant of ModelSpec
├── gamma-poisson.ron           # GammaPoisson variant
├── ekf-bearing.ron             # EkfBearing variant
├── crates/
│   ├── spec/                   # dictionary: enum ModelSpec + per-variant structs
│   ├── spec-loader/            # IO bridge: file → ModelSpec
│   ├── kalman/                 # Lab 1a: KalmanFilter<T, N, M>, generic in T
│   ├── gamma-poisson/          # Lab 1b: conjugate update, no matrices, no AD
│   ├── ekf-bearing/            # Lab 1c: non-linear h, runtime Jacobian via AD
│   ├── autodiff/               # Lab 2: forward-mode AD via num-dual
│   ├── partial-eval/           # Lab 3: build.rs dispatcher (Riccati for Kalman branch)
│   ├── codegen/                # Lab 4: bayesian_filter! proc-macro (per-model emission)
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
just lab2      # dual-number gradient matches numerical
just lab3      # K_INF lands as a literal const
just lab4      # macro emits straight-line specialized update
just qemu      # boot the embedded binary in QEMU
just verify    # all of the above + dep-tree check + 3-way equivalence
```
