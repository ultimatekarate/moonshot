//! Three-way trajectory equivalence harness.
//!
//! What we compare:
//! - **Reference** (`kalman::cv_2d` with runtime nalgebra) vs **truth**: confirms
//!   the reference filter actually tracks. Tolerance: noise scale (sqrt(R) ≈ 0.3).
//! - **Macro** (`codegen_demo::Filter4x2`, steady-state straight-line) vs **truth**:
//!   confirms the proc-macro emits a *correct* filter. Same noise-scale bound.
//! - **Macro on host** vs **macro on QEMU** (when `--with-qemu-trace` is passed):
//!   same code, two platforms — only differ by f32 reordering in soft-float vs
//!   host hardware. Tight 1e-4 bound is meaningful here.
//!
//! What we do *not* check: bit-equivalence between reference and macro. Both are
//! valid filters but use different P trajectories (time-varying K vs steady-state
//! K_inf), so they only converge to each other asymptotically. We report the
//! diff for transparency but don't fail the run on it.

use std::env;
use std::path::Path;
use std::process::ExitCode;

const TRACE_LEN: usize = 100;
/// Skip the early portion of the trace where the time-varying-K reference is
/// still in transient and trajectories naturally diverge.
const TRANSIENT: usize = 20;
/// Each filter must track the noise-free truth within ~1·sqrt(R) for position.
/// R = 0.1 in `model.ron`, so 0.5 is generous.
const TRUTH_TOLERANCE: f32 = 0.5;
/// Same code, two platforms — diffs come from f32 reordering only.
const QEMU_TOLERANCE: f32 = 1e-4;

fn main() -> ExitCode {
    let with_qemu_trace = env::args().any(|a| a == "--with-qemu-trace");

    let samples = synth_trace();
    let truth = synth_truth();
    let reference_trace = run_reference(&samples);
    let macro_trace = run_macro(&samples);

    let ref_vs_truth = max_abs_diff(&reference_trace, &truth, TRANSIENT);
    let macro_vs_truth = max_abs_diff(&macro_trace, &truth, TRANSIENT);
    let ref_vs_macro = max_abs_diff(&reference_trace, &macro_trace, TRANSIENT);

    println!("After {}-sample transient:", TRANSIENT);
    println!("  reference (runtime-K) vs truth:");
    print_max_diff(&ref_vs_truth);
    println!("  macro (steady-state K_inf) vs truth:");
    print_max_diff(&macro_vs_truth);
    println!("  reference vs macro (informational — not asserted):");
    print_max_diff(&ref_vs_macro);

    let mut violation = ref_vs_truth.iter().any(|&d| d > TRUTH_TOLERANCE)
        || macro_vs_truth.iter().any(|&d| d > TRUTH_TOLERANCE);

    if with_qemu_trace {
        let trace_path = Path::new("target/qemu-trace.bin");
        if !trace_path.exists() {
            eprintln!("\n--with-qemu-trace specified but {} not found.", trace_path.display());
            eprintln!("  Run `cargo xtask qemu --capture` first.");
            return ExitCode::from(2);
        }
        match parse_qemu_trace(trace_path) {
            Ok(qemu_trace) => {
                let qemu_vs_macro = max_abs_diff(&qemu_trace, &macro_trace, 0);
                println!("\nQEMU macro vs host macro (same code, no transient skip):");
                print_max_diff(&qemu_vs_macro);
                if qemu_vs_macro.iter().any(|&d| d > QEMU_TOLERANCE) {
                    violation = true;
                }
            }
            Err(e) => {
                eprintln!("\nfailed to parse QEMU trace: {}", e);
                return ExitCode::from(2);
            }
        }
    }

    if violation {
        eprintln!("\nFAILED: tolerance violation (truth: {:.0e}, qemu: {:.0e})", TRUTH_TOLERANCE, QEMU_TOLERANCE);
        ExitCode::from(1)
    } else {
        println!("\nOK: truth-tracking and (if checked) QEMU equivalence within bounds.");
        ExitCode::SUCCESS
    }
}

/// Same trace the embedded binary walks: noise-free constant-velocity
/// trajectory `(0.1·k, 0.05·k)` for k in 0..100.
fn synth_trace() -> Vec<(f32, f32)> {
    (0..TRACE_LEN)
        .map(|k| {
            let t = k as f32 * 0.1;
            (t, 0.5 * t)
        })
        .collect()
}

/// Ground truth: position is exactly the (noise-free) measurement, velocity is
/// the constant generator: vx=1.0, vy=0.5.
fn synth_truth() -> Vec<[f32; 4]> {
    (0..TRACE_LEN)
        .map(|k| {
            let t = k as f32 * 0.1;
            [t, 0.5 * t, 1.0, 0.5]
        })
        .collect()
}

fn run_reference(samples: &[(f32, f32)]) -> Vec<[f32; 4]> {
    let mut filter = kalman::cv_2d::<f32>(0.1, 0.01, 0.1);
    samples
        .iter()
        .map(|&(z_x, z_y)| {
            filter.predict();
            let z = nalgebra::SMatrix::<f32, 2, 1>::new(z_x, z_y);
            filter.update(&z);
            [filter.x[0], filter.x[1], filter.x[2], filter.x[3]]
        })
        .collect()
}

fn run_macro(samples: &[(f32, f32)]) -> Vec<[f32; 4]> {
    let mut filter = codegen_demo::Filter4x2::new();
    samples
        .iter()
        .map(|&(z_x, z_y)| {
            filter.update(z_x, z_y);
            filter.x
        })
        .collect()
}

fn max_abs_diff(a: &[[f32; 4]], b: &[[f32; 4]], skip: usize) -> [f32; 4] {
    let mut out = [0.0_f32; 4];
    let n = a.len().min(b.len());
    for k in skip..n {
        for c in 0..4 {
            let d = (a[k][c] - b[k][c]).abs();
            if d > out[c] {
                out[c] = d;
            }
        }
    }
    out
}

fn print_max_diff(d: &[f32; 4]) {
    let labels = ["x", "y", "vx", "vy"];
    for c in 0..4 {
        println!("    {:>2}: max-abs-diff = {:>10.6}", labels[c], d[c]);
    }
}

/// Parses lines emitted by `crates/embedded/src/main.rs` (default features only).
/// Format: `update #  N:    K cycles  x=  V y=  V vx=  V vy=  V`.
/// Returns the trajectory as `[x, y, vx, vy]` per sample. Skips lines that
/// don't match (FLAT/VARIABLE summaries, semihosting noise).
fn parse_qemu_trace(path: &Path) -> Result<Vec<[f32; 4]>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut trace = Vec::with_capacity(TRACE_LEN);
    for line in content.lines() {
        if !line.contains("update #") {
            continue;
        }
        let mut x = None;
        let mut y = None;
        let mut vx = None;
        let mut vy = None;
        for token in line.split_whitespace() {
            if let Some(v) = token.strip_prefix("x=") {
                x = v.parse().ok();
            } else if let Some(v) = token.strip_prefix("y=") {
                y = v.parse().ok();
            } else if let Some(v) = token.strip_prefix("vx=") {
                vx = v.parse().ok();
            } else if let Some(v) = token.strip_prefix("vy=") {
                vy = v.parse().ok();
            }
        }
        if let (Some(x), Some(y), Some(vx), Some(vy)) = (x, y, vx, vy) {
            trace.push([x, y, vx, vy]);
        }
    }
    Ok(trace)
}
