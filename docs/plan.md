# Moonshot — Compile-time Bayesian inference, end-to-end

## Context

You want to explore how far Bayesian inference can be lifted into the Rust compiler — using nalgebra for type-level dimensions, automatic differentiation for gradients, build-time partial evaluation, proc-macros for specialized loops, and embassy for embedded deployment. The goal is **not** to ship a useful product; it's to find out what's possible, where the friction is, and whether the pipeline composes. There is no published Rust pipeline that ties all five pieces together for an MCU target, so this is partially greenfield.

Three findings from research that shape the design:

1. **`weval` doesn't fit a native MCU target.** It partial-evaluates WASM interpreters. The faithful translation of your "embed pre-computed answers in the binary" intent on native is `build.rs` (offline solving) + `const fn` (compile-time folding) + proc-macros (specialized code emission). The plan uses these instead.
2. **Enzyme is too fragile for this stack.** It requires a custom-built nightly rustc (`llvm.enzyme = true`) and the official channel only carries it intermittently. With 6 parameters in our toy model, forward-mode AD via `num-dual` is functionally equivalent at trivial cost and keeps the entire workspace on stable Rust. The AD lab uses dual numbers; same source code, parameterised over scalar type.
3. **The remaining pipeline still requires custom glue.** nalgebra is a runtime library, proc-macros work on tokens, embassy targets bare metal, build.rs reads files. The integration is the research artifact — knit together via a shared `model.ron` + a small `crates/spec` dictionary that everyone consumes.

**Choices anchored from your answers**: three model classes (so the architecture can't over-fit to one), QEMU first (`lm3s6965evb`, Cortex-M3), labs as a scaffold with a thin end-to-end seam.

## Three model classes

The architecture must serve all three; choices that only fit one get quarantined into that one's crate. The trio is picked to maximise architectural pressure:

1. **2D Kalman filter** (`crates/kalman`) — linear-Gaussian sensor fusion. State `[x, y, vx, vy]`, observation `[x, y]`. Closed-form steady-state gain via Riccati. *Real-world stand-in for*: drone IMU/GPS fusion, Li-ion state-of-charge, AHRS, automotive radar/camera fusion. **Exercises**: nalgebra const dims, build-time numerical solver, eigenvalue stability check, Cayley parametrization.
2. **Gamma-Poisson conjugate update** (`crates/gamma-poisson`) — on-device event-rate estimation. Posterior `Gamma(α + Σk, β + n)` over the Poisson rate after `n` ticks. *Real-world stand-in for*: predictive maintenance, packet-loss tracking, queue monitoring, interrupt-rate anomaly detection. **Exercises**: the architecture *without* nalgebra, AD, or Riccati. If basis governance, the proc-macro story, and the embedded shell still hold for a model that has no matrices, those parts are genuinely architecture.
3. **EKF for bearing-only 2D tracking** (`crates/ekf-bearing`) — non-linear observation `θ = atan2(y - sy, x - sx)`. *Real-world stand-in for*: passive sonar, anti-drone direction-finding, marine wildlife acoustic tracking, vision-based robot localisation. **Exercises**: AD genuinely load-bearing (Jacobian computed every update via dual numbers, not precomputed), time-varying gain (no Riccati steady state), and a famously degenerate geometry that stresses the build-time machinery.

`ModelSpec` is a sum type over the three; every consumer (build.rs, proc-macros, host harness) dispatches. Adding a fourth model means adding a variant — basis exhaustive-matching forces every consumer to declare its handling, no silent fall-through.

**Pragmatic implementation order**: Kalman first (most existing scaffolding), Gamma-Poisson second (smallest), EKF-bearing last (depends on a working autodiff lab).

## Runtime invariants proven at build time

The actual deliverable. Every piece of the compile-time pipeline exists to lift one of these properties from "hopefully" to "guaranteed." Faster builds are a fine bonus, but lifting an invariant is what justifies new build-time machinery.

**Pipeline-wide (apply to all three models):**

3. **Straight-line update.** `update()` has no allocation, no panic, no recursion, and bounded execution time. *Lab 4 proc-macro emission.*
4. **No nalgebra in the embedded binary.** Macro output is raw arithmetic; `cargo tree -p embedded` check enforces. *Lab 4 + verification.*
5. **Host ↔ target equivalence.** The same input trace produces matching output within per-component tolerance across host f32 and soft-float Cortex-M3. (Bit-identical equivalence is explicitly *not* in scope.) *end-to-end harness.*
6. **Architectural layering preserved.** Strict-purity layers don't accidentally pull in IO, async, or wider deps. *basis.yaml + basis-cli check.*
7. **Prior validity.** All model priors are well-formed at build time (Q PD, R PD, Gamma shape/rate positive, etc.) — caught in `partial-eval/build.rs` before any solver runs. *applies to all three.*

**Kalman-family only (Kalman + EKF-bearing):**

1. **Riccati convergence.** `K_INF` is the converged fixed point of the Cayley-bounded iteration, not a non-converged or escaped iterate. The Cayley parametrization makes finite escape impossible by construction; the boundary-distance check catches the conditioning failure that maps to `P → ∞`. *Lab 3 build.rs.* **Kalman only** — the EKF has no steady-state.
2. **Closed-loop stability.** Eigenvalues of `(I − K_INF H) F` lie strictly inside the unit disk. A converged Riccati can still produce a marginally-stable gain that drives slow runtime divergence even with bounded inputs. *Lab 3 build.rs.* **Kalman only** for the same reason.
8. **Compile-time dimension correctness.** Matrix shape mismatches are compile errors, not runtime panics. *nalgebra const generics.* **Kalman + EKF only** (Gamma-Poisson has no matrices).

**Gamma-Poisson only:**

9. **Posterior is always a valid Gamma.** `α > 0`, `β > 0` invariant after every update — straightforwardly true because the update increments only, but worth naming.

Decision rule for new build-time work: it lifts one of these to guaranteed. Decision rule for an architectural choice: it serves a pipeline-wide invariant, OR it's quarantined into the model-specific crate where it belongs (Cayley + Riccati live in `crates/kalman` or `crates/partial-eval`'s Kalman branch, *not* in the proc-macro or the spec).

### What lifting buys at runtime

The seven invariants are the *means*; this is the *end*. Each lifted invariant removes one category of runtime check and one category of runtime failure. Cumulatively, the runtime stops being a *defensive program* and becomes a *proof carrier* — code that can run because it had a proof of correctness; the proof itself doesn't ship, only the code that depends on the proof being valid.

| Lifted | Runtime cost removed |
| --- | --- |
| Riccati convergence | NaN propagation from a bad initial gain; startup `K.is_finite()` checks |
| Closed-loop stability | Divergence watchdog (`if ‖x‖ > threshold then reset`); slow-failure recovery |
| Straight-line update | Allocator; panic handler; try/catch/recover; early-exit branches |
| No nalgebra in binary | Generic dispatch tables; monomorphization explosion; flash bloat |
| Architectural layering | Surprise IO in hot paths; async runtime in unexpected places |
| Dimension correctness | Shape-mismatch panics; runtime dimension checks; vtable dispatch |

**Concrete embedded payoff is order-of-magnitude.** A defensively-coded Kalman in C++ easily eats 20KB+ before any application code (allocator, exceptions, RTTI, vtables, panic infra, assertions, error codes). The proof-carrying version is the actual arithmetic: 1–3KB. On a 32KB MCU the remaining 29KB is yours.

**The marquee gain is timing determinism, not size.** With every defensive branch removed, `update()` takes a constant number of cycles — same for every input, every time. That's the property that lets the filter co-schedule with hard-real-time control loops on the same chip. Without it you can't promise the next IMU sample won't be missed; with it the schedule is mathematics, not hope.

**Bonus: auditability.** A 50-line straight-line f32 kernel can be read end-to-end by a reviewer who can convince themselves it does what it claims. A defensively-coded equivalent is thousands of lines across alloc/panic/dispatch/recover layers — nobody reads it, nobody can.

### Implications: what this enables

The rung above auditability. Removing defensive overhead reclaims memory you can spend on math — and we all love spending memory on math. Methods previously "too expensive at this scale" become tractable. The compile-time machinery doesn't just make existing methods safer; it changes which class of mathematics you can run on a given chip.

| Method | Why it was out | What it gives you |
| --- | --- | --- |
| Sliding-window MAP / factor graphs | O(window × state) RAM | Modern robotics standard; far more accurate than EKF on non-linear observations |
| Iterated EKF | Re-linearize multiple times per update; needs scratch | Eliminates EKF's linearization-point sensitivity |
| UKF (sigma points) | 2N+1 state copies; ~5× EKF storage | Captures non-linearity without Jacobians; more robust near the conditioning boundary |
| Particle filters | 100 particles × state ≈ 1.6KB minimum | Handles multimodal posteriors that Kalman cannot represent at all |
| Square-root information filters | Larger state representation per step | Dramatically better numerical stability — was routinely skipped for memory |
| Multiple-hypothesis / GMM tracking | N × per-model storage | Opens model-uncertainty tracking |
| Online Bayesian model selection | K filters in parallel + posterior weights | Currently sci-fi on small MCUs; freed budget makes it tractable |

This lands hardest on the EKF-bearing lab, where standard EKF is fragile in exactly the ways iterated EKF and sliding-window MAP are not. The choice "EKF because it fits" stops being forced.

**Caveats.** Memory isn't the only constraint. UKF trades storage for compute (N+1 propagations per step). Particle filters need RNG infrastructure of their own. Factor graphs need sparse linear solvers. Not a free buffet — but the *option* to choose these methods exists where it didn't before.

## Repo layout

```
moonshot/
├── Cargo.toml                 # virtual workspace
├── rust-toolchain.toml        # pinned stable channel + thumbv7m target
├── basis.yaml                 # architectural governance (see Basis section)
├── model.ron                  # Kalman variant of ModelSpec
├── gamma-poisson.ron          # GammaPoisson variant of ModelSpec
├── ekf-bearing.ron            # EkfBearing variant of ModelSpec
├── justfile                   # labs, qemu, verify, expand, basis
├── .cargo/config.toml         # qemu runner for thumbv7m
├── crates/
│   ├── spec/                  # dictionary: enum ModelSpec + per-variant structs
│   ├── spec-loader/           # IO bridge: file/path → ModelSpec
│   ├── kalman/                # Lab 1a: linear-Gaussian filter (was: model)
│   ├── gamma-poisson/         # Lab 1b: conjugate update — the "no-matrix" test
│   ├── ekf-bearing/           # Lab 1c: non-linear, AD-load-bearing
│   ├── autodiff/              # Lab 2: forward-mode AD via num-dual
│   ├── partial-eval/          # Lab 3: build.rs dispatcher (Riccati for Kalman branch)
│   ├── codegen/               # Lab 4: bayesian_filter! proc-macro (per-model dispatch)
│   ├── codegen-demo/          # consumer of the macro
│   ├── embedded/              # Lab 5: embassy bin, thumbv7m, QEMU target
│   └── end-to-end/            # host harness: reference vs macro vs QEMU trace, per model
└── xtask/                     # cargo xtask qemu --capture, verify, basis
```

Twelve crates. Two extra over the original sketch are the additional model labs (`gamma-poisson`, `ekf-bearing`); the rest pull their weight as before. Proc-macros must live in their own crate, embedded needs `no_std`/`no_main`, the comparison harness needs `std`, and `spec` is split from `spec-loader` so the inert types stay in the strict-purity dictionary layer while the file-reading lives in a relaxed-purity layer.

## Toolchain & setup

- `rust-toolchain.toml`: `channel = "stable"`, `components = ["rustfmt", "clippy"]`, `targets = ["thumbv7m-none-eabi"]`. Whole workspace is stable Rust — no nightly anywhere. (Enzyme dropped in favour of `num-dual`; see Lab 2.)
- Optional `cargo install` recommendations for the contributor: `cargo-expand`, `cargo-binutils` (for `cargo size`), `basis-cli`, and the `qemu-system-arm` system package.
- **QEMU**: `qemu-system-arm -cpu cortex-m3 -machine lm3s6965evb -nographic -semihosting-config enable=on,target=native -kernel <bin>`. Wired into `.cargo/config.toml` as the runner for `thumbv7m-none-eabi`.

## Basis governance (`basis.yaml`)

This project's whole point is *compile-time correctness*. Architectural drift between strict-purity `no_std` crates, build-time IO, proc-macro-time IO, and embedded entry points is the failure mode that quietly destroys the experiment. `basis.yaml` makes the layering enforceable: `basis-cli check` blocks any commit that crosses a boundary, leaks IO into a pure layer, or violates a newtype/enum rule.

### External layers (third-party packages grouped by concern)

| Layer | Packages |
| --- | --- |
| `serialization` | `serde`, `serde_derive`, `ron` |
| `linalg` | `nalgebra`, `num-dual`, `num-traits` |
| `proc-macro-utils` | `syn`, `quote`, `proc-macro2` |
| `embassy` | `embassy-executor`, `embassy-time`, `cortex-m`, `cortex-m-rt`, `cortex-m-semihosting`, `panic-semihosting` |
| `cli` | `clap` (xtask + end-to-end only) |
| `rust-internal` | `crate, super, self, std, core, alloc` |
| `all-external` | `*` (wildcard escape valve for shell layer) |

### Internal layers

| Layer | Crates / paths | Purity | Depends on (internal) |
| --- | --- | --- | --- |
| **spec** (dictionary) | `crates/spec` | strict, IO/network/async forbidden | — |
| **kernel** (laboratory) | `crates/kalman`, `crates/gamma-poisson`, `crates/ekf-bearing`, `crates/codegen-demo`, `crates/partial-eval/src` | strict, IO/network/async forbidden | spec |
| **autodiff** (laboratory) | `crates/autodiff` | strict, IO/network/async forbidden | spec, kernel |
| **codegen** (tooling) | `crates/codegen`, `crates/spec-loader`, `crates/partial-eval/build.rs` | relaxed, IO allowed | spec |
| **shell** (hands) | `crates/embedded`, `crates/end-to-end`, `xtask` | relaxed, IO + async allowed | spec, kernel, codegen-demo |

### Boundaries (Placement axis — explicit deny rules)

- `spec → linalg`: deny — data types must stay zero-deps so they cross-compile everywhere.
- `kernel → embassy`: deny — pure math must not pull in async runtime.
- `kernel → proc-macro-utils`: deny — runtime kernel must not depend on macro infrastructure.
- `autodiff → embassy`: deny — gradient math is host-side only.
- `codegen → embassy`: deny — proc-macros run on the host.

### Newtypes (Values axis)

Defined in `crates/spec/src/lib.rs`:
- `Timestep` wraps `f32` — prevents mixing `dt` with arbitrary times.
- `LogLikelihood` wraps `f32` — prevents adding log-probs to probabilities.
- `StateDim` wraps `usize`, `MeasDim` wraps `usize` — reinforces the const-generic dimension story at the spec level.
- `Frobenius` wraps `f32` — Riccati convergence tolerance.

### Exhaustive matching (Completeness axis)

- `RiccatiResult { Converged(usize), Diverged, MaxIterReached }` — `partial-eval/build.rs` must handle every case explicitly.
- `GradientCheck { WithinTolerance(f32), Mismatch { dual: f32, numeric: f32 } }` — `crates/autodiff` test harness must distinguish pass from instructive failure.
- `TestEquivalence { Bitwise, Tolerant(f32), Failed }` — `crates/end-to-end` comparison harness.

### Purity (forbidden in strict)

`file_io`, `network_io`, `stdout`, `stderr`, `env_vars`, `system_clock`, `dynamic_execution`, `subprocess`.

### Granularity

`max_lines: 600` — research repo, smaller files than basis's own 800-line cap.

### Known limit

Basis is a string-matching tool. It catches source-import violations but not Cargo dep-graph violations. The constraint "the macro output in `crates/embedded` must not transitively pull in nalgebra" is enforced at the *Cargo* level (codegen output emits no `use nalgebra::` and `crates/embedded/Cargo.toml` doesn't list it as a dep). `just verify` includes a `cargo tree -p embedded | grep -v nalgebra` check as a backstop.

## Lab specs

### Lab 0a — `crates/spec` (dictionary layer)

Inert types only. Strict purity. `no_std`-compatible.

- `src/lib.rs`: `#![no_std]`, exports `ModelSpec { f: [[f32; 4]; 4], h: [[f32; 2]; 4], q: [[f32; 4]; 4], r: [[f32; 2]; 2], dt: Timestep }` — plain serde-derived struct, plus the `Timestep`/`LogLikelihood`/`StateDim`/`MeasDim`/`Frobenius` newtypes (Values axis), and the `RiccatiResult` / `BackendStatus` / `TestEquivalence` enums (Completeness axis).
- Deps: `serde = { default-features = false, features = ["derive", "alloc"] }`. No nalgebra, no std, no IO.
- **Done when**: `cargo build -p spec --target thumbv7m-none-eabi` passes (proves no_std), and `cargo test -p spec` round-trips a sample spec.

### Lab 0b — `crates/spec-loader` (loader layer, relaxed purity)

The IO bridge that both `partial-eval/build.rs` and the `bayesian_filter!` proc-macro depend on. Loads any `ModelSpec` variant — consumers dispatch on what they get back.

- `src/lib.rs`: `pub fn load(path: &Path) -> Result<spec::ModelSpec, Error>`. Reads file, parses RON, returns owned `ModelSpec` from `crates/spec`.
- Deps: `spec` (path), `ron`, `serde`. Allowed file IO.
- **Done when**: `cargo test -p spec-loader` parses workspace `model.ron`.

### Lab 1a — `crates/kalman` (the "well-behaved continuous" reference)

Linear-Gaussian Kalman, generic over scalar `T` so Lab 2 can re-use the same code with `T = DualVec<f32, U6>`.

- `src/filter.rs`: `pub struct KalmanFilter<T, const N: usize, const M: usize> where T: nalgebra::RealField + Copy` over `nalgebra::SMatrix<T, _, _>`. Methods `predict`, `update` (Joseph form), `neg_log_likelihood` — all generic in `T`.
- `src/lib.rs`: `pub fn cv_2d<T>(dt: T, q: T, r: T) -> KalmanFilter<T, 4, 2>`. Convenience alias `pub type Cv2dF32 = KalmanFilter<f32, 4, 2>;`.
- **Done when**: `cargo test -p kalman` passes AND `cargo build -p kalman --target thumbv7m-none-eabi` passes.
- **Pitfalls**: nalgebra default features pull in `std` — disable. `f32::powi` is std-only. `T: RealField + Copy` is the bound that lets both `f32` and `num_dual::DualVec` work.

### Lab 1b — `crates/gamma-poisson` (the "no-matrix" stress test)

Conjugate update on the Poisson rate. The whole filter fits in two `f32`s. Exists to prove the architecture isn't quietly Kalman-shaped.

- `src/lib.rs`: `pub struct GammaPosterior { shape: f32, rate: f32 }` with `update(&mut self, k: u32)` doing `(α, β) → (α + k, β + 1)`. No nalgebra dep. `#![no_std]`.
- `tests/conjugate.rs`: prior + 100 simulated counts → posterior matches analytical formula exactly (this one *can* be bit-identical because it's integer arithmetic on the sufficient statistic).
- **Done when**: tests pass; `cargo tree -p gamma-poisson` shows no nalgebra, no num-dual, no anything beyond `spec` and `core`.
- **Pitfalls**: tempting to overengineer (add dispersion checks, posterior predictive, etc.) — resist. The point is minimality.

### Lab 1c — `crates/ekf-bearing` (the "AD-actually-needed" test)

EKF on `[x, y, vx, vy]` with scalar bearing observation `h(x) = atan2(y − sy, x − sx)`. The Jacobian `H = ∂h/∂x` is computed every update via `crates/autodiff::bearing_jacobian` — there's no precomputable steady-state because `H` depends on the current state estimate.

- `src/lib.rs`: `pub struct EkfBearing<T>` over `SMatrix<T, _, _>`, `predict` and `update(bearing: T)` methods.
- `tests/observability.rs`: simulate a stationary sensor and a constant-velocity target; verify the EKF stays consistent (covariance grows along the unobservable direction, shrinks along the observable one).
- **Done when**: tests pass; `crates/autodiff::bearing_jacobian` returns a non-stub Jacobian and the EKF uses it.
- **Pitfalls**: bearing-only is unobservable from a stationary sensor — the test must use sensor motion or two sensors, otherwise `P` grows unboundedly and the test is misleading. The atan2 derivative has a singularity at the sensor location; document the assumption that the target is never exactly there.

### Lab 2 — `crates/autodiff`

Forward-mode AD via `num-dual`. Same `model::nll` source, different scalar type, gradients fall out.

- `Cargo.toml`: deps `model` (path), `num-dual = { version = "0.10", default-features = false, features = ["alloc"] }`, `nalgebra` (workspace, no default features). `#![no_std]` — yes, `num-dual` works no_std with `alloc`.
- `src/lib.rs`: `pub fn gradient_nll(params: &[f32; 6], zs: &[[f32; 2]]) -> [f32; 6]` — internally constructs `DualVec<f32, U6>` parameters, calls `model::cv_2d::<DualVec<f32, U6>>(...).neg_log_likelihood(zs)`, reads off the 6 partials from the `.eps` field.
- `src/numeric.rs`: central-difference reference.
- `tests/grad_matches.rs`: assert `(d_dual - d_numeric).norm() < 1e-3`.
- **Done when**: `cargo test -p autodiff` passes the gradient match. Lab is no longer feature-gated — it Just Works on stable.
- **Pitfalls**: `num_dual::DualVec<f32, U6>` must satisfy `nalgebra::RealField + Copy`. Should work out of the box (num-dual implements these traits), but if not, the workaround is hand-rolled `Dual<f32> { real, dual }` with `impl RealField` — small enough to write by hand. Convert observation array `&[[f32; 2]]` to `&[[T; 2]]` via `.map(|z| z.map(T::from_f32))` inside `nll` (or take `&[[T; 2]]` directly and convert at the boundary).

### Lab 3 — `crates/partial-eval`

`build.rs` solves the discrete algebraic Riccati equation offline and emits a `const`. **Iteration runs in Cayley-transformed coordinates**, not on `P` directly.

- `build.rs`: read `$MOONSHOT_MODEL` (default workspace `model.ron`), iterate the Riccati recursion in Cayley coordinates `Y = (P − I)(P + I)⁻¹` (which lives in the bounded contraction set `‖Y‖ < 1`) until `‖Y_{k+1} − Y_k‖_F < 1e-9` (cap 1000 iterations), pull back `P_inf = (I + Y_inf)(I − Y_inf)⁻¹`, compute `K_inf = P_inf Hᵀ (H P_inf Hᵀ + R)⁻¹`, write `$OUT_DIR/k_inf.rs` containing `pub const K_INF: [[f32; 2]; 4] = [[..., ...], ...];` and `pub const STEADY_STATE_AT: usize = N;`. Emit `cargo:rerun-if-changed=...model.ron`.
- `src/lib.rs`: `#![no_std] include!(concat!(env!("OUT_DIR"), "/k_inf.rs"));` plus a `pub const fn dt_squared(dt: f32) -> f32 { dt * dt }` to demonstrate the const-fn angle.
- `tests/converged.rs`: run 50 explicit steps via `model::cv_2d`, assert gain converges to `K_INF` within 1e-5.
- **Done when**: `cargo expand -p partial-eval` shows a literal f32 array; convergence test passes.
- **Constant `dt` is load-bearing.** `K_INF` is precomputed from `(F, Q, R)`, and `dt` enters `F`. Baking `K_INF` as a `const` therefore requires `dt` to be a `const` too. Accepted on purpose: embedded sensors are hardware-clocked at a fixed rate, so this matches reality. Multi-rate operation (e.g., IMU + GPS) would emit one specialised `K_INF_*` per supported rate and dispatch at runtime — left as a follow-on.
- **Closed-loop stability check** (runtime invariant #2). After Riccati converges and `K_inf` is recovered, form `F_cl = (I − K_inf H) F`, compute its complex eigenvalues via `nalgebra::SMatrix::complex_eigenvalues()`, and verify `max|λ| < 1`. Emit `ClosedLoopStability::Stable { spectral_radius }` on success; on failure emit `Unstable { spectral_radius }` and panic loudly so the build fails before a divergent gain gets baked in.
- **Why Cayley?** The naive Riccati iteration on `P` admits finite escape time — the iterate can blow up in 2–3 steps when `(F, Q^½)` is not stabilizable, and a `‖P_{k+1} − P_k‖_F` tolerance check won't catch it (the residual is huge but the loop "succeeded"). The Cayley transform maps the cone `{P > 0}` to the open unit ball of symmetric contractions; iteration in those coordinates is bounded by construction, so finite escape is impossible. Inversions of `(I − Y)`-type denominators are well-conditioned everywhere except the boundary, and the boundary itself is a well-defined detectable surface (the image of `P → ∞`).
- **Two preconditioners** carried by `ModelSpec`, applied in order before the bounded iteration:
  1. `state_scale: [f32; 4]` — diagonal similarity transform `D` that rescales the state so all components have comparable dynamic range. Transforms the entire problem: `F' = D⁻¹ F D`, `Q' = D⁻¹ Q D⁻ᵀ`, `H' = H D`. Default `[1.0; 4]` is no-op; tune once we measure the conditioning of the iterate.
  2. `cayley_alpha: f32` — scalar reference for the Cayley map `Y = (P − αI)(P + αI)⁻¹`. Best-conditioned when α is in the geometric middle of `P`'s spectrum. With state scaling done, `α = 1.0` is typically fine.

  At the end, recovered `K_inf` is in the scaled coordinate system; un-scale via `K_inf_phys = D K_inf_scaled` before emitting the literal so the proc-macro and embedded code can operate directly on physical observations.
- **Pitfalls**:
  - **Boundary approach** (the Cayley analogue of finite escape). Track `boundary_distance = 1 − σ_max(Y_k)`. If it falls below ε (≈1e-6), return `RiccatiResult::EscapedFiniteTime { at_iteration, boundary_distance }` and panic loudly. This is the well-conditioned restatement of the same physical phenomenon — `P` would have escaped — caught in coordinates where it stays finite.
  - Singular `Q` still possible; panic loudly there too.
  - `build.rs` deps must not require `no_std` features of `model` — keep build.rs deps to `nalgebra` + `spec-loader` only.

### Lab 4 — `crates/codegen` + `crates/codegen-demo`

Proc-macro `bayesian_filter!{ spec = "...", name = Filter4x2 }` consumes the model spec, dispatches on the `ModelSpec` variant, and emits a model-specific specialized `update`.

- `crates/codegen/src/lib.rs`: `#[proc_macro] pub fn bayesian_filter(input: TokenStream) -> TokenStream`. Deps: `syn = { version = "2", features = ["full"] }`, `quote`, `proc-macro2`, `spec-loader`.
- `crates/codegen/src/parse.rs`: parses `spec = "path", name = Ident` via `syn::parse::Parse`.
- `crates/codegen/src/expand.rs`: dispatches per variant. Kalman → struct with `[f32; 4]` state, baked-in `K_INF`, straight-line arithmetic update. GammaPoisson → struct with `(α, β)`, integer-update method. EkfBearing → struct with `[f32; 4]` state and a `update(bearing: f32)` that calls into a runtime Jacobian helper. None of the emitted code depends on nalgebra.
- `crates/codegen-demo/src/lib.rs`: `bayesian_filter!{ spec = "../../model.ron", name = Filter4x2 }` (or sibling demos pointing at `gamma-poisson.ron`/`ekf-bearing.ron`).
- `crates/codegen-demo/tests/equivalence.rs`: per-model — for the Kalman demo, same 100-step trace through `Filter4x2` and `kalman::cv_2d`, assert max-abs-diff < 1e-4.
- **Done when**: `cargo expand -p codegen-demo` shows literal f32 constants in place of `K_INF`; equivalence test passes.
- **Pitfalls**: macro paths are relative to the *invoking* crate, so use absolute path or `MOONSHOT_MODEL` env var. Use `proc_macro2::Literal::f32_suffixed` for f32 literals — `quote!` Display loses precision. Output must not depend on nalgebra so `crates/embedded` can pull it in cheaply.

### Lab 5 — `crates/embedded`

Embassy bin, `thumbv7m-none-eabi`, runs in QEMU's `lm3s6965evb`.

- `Cargo.toml`: `embassy-executor = { version = "0.6", features = ["arch-cortex-m", "executor-thread", "integrated-timers"] }`, `embassy-time`, `cortex-m`, `cortex-m-rt`, `cortex-m-semihosting`, `panic-semihosting`, `codegen-demo` (path).
- `memory.x`: `FLASH 256K @ 0x00000000`, `RAM 64K @ 0x20000000` (lm3s6965 layout — 64K RAM, our use stays well under your 32K budget).
- `build.rs`: emits `cargo:rustc-link-arg=-Tlink.x`, copies `memory.x` to `OUT_DIR`.
- `src/main.rs`: `#![no_std] #![no_main]`. Embassy main spawns one task that walks a `const SAMPLES: [(f32, f32); 100]` (in `src/samples.rs`), calls `Filter4x2::update`, prints state via `cortex_m_semihosting::hprintln!`. Exit via `cortex_m_semihosting::debug::exit` after the trace.
- **Done when**: `just qemu` boots, prints 100 lines of `[k=NN] x=… y=… vx=… vy=…`, exits cleanly.
- **Pitfalls**: `defmt-rtt` won't work in plain QEMU — use semihosting. lm3s6965 has no FPU; soft-float is fine for the toy. `panic-semihosting` and `cortex-m-semihosting` share the underlying syscall — only one initializes it.

### End-to-end glue — `crates/end-to-end` + `xtask/`

- `end-to-end/src/main.rs`: runs the same trace through (a) `model::cv_2d`, (b) `codegen_demo::Filter4x2`, (c) optionally a QEMU trace dumped to `target/qemu-trace.bin`. Prints a side-by-side table; nonzero exit on max-diff > 1e-4.
- `xtask/src/main.rs`: `cargo xtask qemu --capture` spawns QEMU with stdout piped to `target/qemu-trace.bin`; `cargo xtask verify` runs end-to-end with `--with-qemu-trace`.

## Verification plan

`justfile` recipes that any future contributor (or you next week) can run:

- `just basis` → `basis-cli check --spec basis.yaml .` (architecture is intact — runs first in CI so layer drift fails fast).
- `just lab1` → `cargo test -p model` (also builds for `thumbv7m-none-eabi`).
- `just lab2` → `cargo test -p autodiff` (dual-number gradient matches numerical within 1e-3).
- `just lab3` → `cargo test -p partial-eval && cargo expand -p partial-eval | grep K_INF` (literal constant present).
- `just lab4` → `cargo test -p codegen-demo && cargo expand -p codegen-demo` (straight-line arithmetic, no `K_INF` symbol left).
- `just qemu` → `cargo xtask qemu --capture` (boots, runs trace, exits clean).
- `just verify` → `basis check && cargo tree -p embedded | grep -vq nalgebra && cargo run -p end-to-end -- --with-qemu-trace` (architecture intact, no nalgebra in MCU dep tree, all three implementations agree).
- `just size` → `cargo size -p embedded --release --target thumbv7m-none-eabi` (track flash/RAM cost as we specialize more).

## Critical files to create

- `Cargo.toml` — workspace root, member list, shared dep versions.
- `rust-toolchain.toml` — stable channel + thumbv7m-none-eabi target.
- `basis.yaml` — architectural governance (layers, boundaries, newtypes, unions, purity).
- `model.ron` — F, H, Q, R, dt for the 2D constant-velocity model. **Single source of truth read by both build.rs and the proc-macro.**
- `crates/spec/src/lib.rs` — inert types + newtypes + enums. The dictionary layer.
- `crates/spec-loader/src/lib.rs` — the file-IO bridge that prevents drift between Lab 3 and Lab 4.
- `crates/kalman/src/filter.rs` — reference Kalman filter; the comparison oracle for Lab 1a.
- `crates/gamma-poisson/src/lib.rs` — minimal conjugate update; the "no-matrix" architectural test.
- `crates/ekf-bearing/src/lib.rs` — non-linear filter; proves AD is genuinely load-bearing.
- `crates/codegen/src/expand.rs` — the heart of the compile-time specialization story.
- `crates/partial-eval/build.rs` — Riccati solver; the demonstration that "the unchanging math becomes a binary constant."

## Open risks

1. **`num_dual::DualVec<f32, U6>` may need extra trait-bound coaxing for nalgebra.** Should work out of the box in `num-dual` 0.10+ since both crates target the same `simba`/`num-traits` ecosystem, but generic bounds in Rust are notoriously fiddly. Fallback: hand-write a 6-element forward-mode dual struct (≈80 lines) with explicit `RealField` impl.
2. **Proc-macro path resolution is fiddly on Windows.** Use absolute paths via `MOONSHOT_MODEL` env var, not relative `"../../model.ron"`, to avoid surprises across `cargo expand` / `cargo test` invocation contexts.
3. **`f32` reordering between host (x86) and Cortex-M3 (soft-float)** produces small bit-level differences. Bit-identical equivalence has been dropped from scope — the contract is a per-component tolerance. Document the chosen ε with its basis (e.g., "1e-4 because 100-step trajectory accumulates < 1e-5 of FP reordering error in the worst case") so the number is a calibrated bound rather than a guess.
4. **Embassy executor on `lm3s6965evb` requires the right cortex-m-rt linker arguments.** This is well-trodden but easy to get wrong on the first attempt; budget some flailing time for the QEMU boot.
