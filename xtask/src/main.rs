//! Workspace tasks. Two real subcommands:
//!
//! - `cargo xtask qemu --capture`: build + run the embedded binary in QEMU,
//!   tee the semihosting trace to `target/qemu-trace.bin`. The end-to-end
//!   harness consumes that file when invoked with `--with-qemu-trace`.
//!
//! - `cargo xtask verify`: full pipeline check. Architecture lint, headline
//!   binary nalgebra-free, end-to-end equivalence (with QEMU trace).

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EMBEDDED_TARGET: &str = "thumbv7m-none-eabi";
const TRACE_FILE: &str = "target/qemu-trace.bin";

fn main() {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    let rest: Vec<String> = args.collect();

    let exit = match cmd.as_deref() {
        Some("qemu") => cmd_qemu(&rest),
        Some("verify") => cmd_verify(),
        Some(other) => {
            eprintln!("unknown task: {other}");
            usage();
            2
        }
        None => {
            usage();
            2
        }
    };
    std::process::exit(exit);
}

fn usage() {
    eprintln!("usage: cargo xtask <qemu [--capture]|verify>");
}

fn cmd_qemu(args: &[String]) -> i32 {
    let capture = args.iter().any(|a| a == "--capture");
    let workspace_root = workspace_root();

    if !capture {
        // Pass through to `cargo run -p embedded --target thumbv7m-none-eabi --release`.
        let status = Command::new("cargo")
            .current_dir(&workspace_root)
            .args(["run", "-p", "embedded", "--target", EMBEDDED_TARGET, "--release"])
            .status()
            .expect("spawn cargo run");
        return status.code().unwrap_or(1);
    }

    // Capture mode: spawn cargo run with stdout piped, tee to file + console.
    let trace_path = workspace_root.join(TRACE_FILE);
    if let Some(parent) = trace_path.parent() {
        fs::create_dir_all(parent).expect("create target dir");
    }
    let mut trace_file = fs::File::create(&trace_path).expect("create trace file");

    let mut child = Command::new("cargo")
        .current_dir(&workspace_root)
        .args(["run", "-p", "embedded", "--target", EMBEDDED_TARGET, "--release"])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn cargo run");

    let stdout = child.stdout.take().expect("captured stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).expect("read cargo stdout");
        if n == 0 {
            break;
        }
        // Tee: console + file.
        print!("{}", line);
        trace_file.write_all(line.as_bytes()).expect("write trace");
    }

    let status = child.wait().expect("wait cargo run");
    println!("captured {} ({} bytes)", trace_path.display(), fs::metadata(&trace_path).map(|m| m.len()).unwrap_or(0));
    status.code().unwrap_or(1)
}

fn cmd_verify() -> i32 {
    let workspace_root = workspace_root();

    // 1. basis architectural check.
    let basis = Command::new("basis-cli")
        .current_dir(&workspace_root)
        .args(["check", "--spec", "basis.yaml", "."])
        .status();
    match basis {
        Ok(s) if s.success() => println!("[OK] basis architectural check"),
        Ok(s) => {
            eprintln!("[FAIL] basis-cli reported violations (exit {:?})", s.code());
            return 1;
        }
        Err(_) => {
            // basis-cli is an optional external tool; if it's not installed at
            // all, warn but don't fail the whole verify.
            eprintln!("[WARN] basis-cli not found in PATH — skipping architectural check");
        }
    }

    // 2. Headline binary has no nalgebra in its dep tree.
    let tree = Command::new("cargo")
        .current_dir(&workspace_root)
        .args([
            "tree",
            "-p",
            "embedded",
            "--target",
            EMBEDDED_TARGET,
            "-e",
            "no-proc-macro",
        ])
        .output()
        .expect("cargo tree");
    if !tree.status.success() {
        eprintln!("[FAIL] cargo tree -p embedded: {:?}", tree.status.code());
        return 1;
    }
    let tree_str = String::from_utf8_lossy(&tree.stdout);
    if tree_str.contains("nalgebra") {
        eprintln!("[FAIL] nalgebra found in headline embedded dep tree");
        return 1;
    }
    println!("[OK] no nalgebra in headline embedded dep tree");

    // 3. Run end-to-end with the QEMU trace if it exists; otherwise host-only.
    let trace_exists = workspace_root.join(TRACE_FILE).exists();
    let mut e2e_args = vec!["run", "-p", "end-to-end", "--"];
    if trace_exists {
        e2e_args.push("--with-qemu-trace");
    }
    let e2e = Command::new("cargo")
        .current_dir(&workspace_root)
        .args(&e2e_args)
        .status()
        .expect("cargo run -p end-to-end");
    match e2e.code() {
        Some(0) => {
            println!(
                "[OK] end-to-end ({})",
                if trace_exists { "with QEMU trace" } else { "host-only" }
            );
            0
        }
        Some(c) => {
            eprintln!("[FAIL] end-to-end: exit {}", c);
            1
        }
        None => {
            eprintln!("[FAIL] end-to-end: terminated by signal");
            1
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at xtask/; parent is the workspace.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    Path::new(&manifest_dir)
        .parent()
        .expect("xtask dir has a parent (the workspace)")
        .to_path_buf()
}
