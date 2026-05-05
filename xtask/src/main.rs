fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("qemu") => {
            // Stub: spawn `cargo run -p embedded --target thumbv7m-none-eabi --release`
            // with stdout piped to target/qemu-trace.bin when --capture is passed.
            // See docs/plan.md "End-to-end glue".
            println!("xtask qemu — stub");
        }
        Some("verify") => {
            // Stub: invoke `just verify` equivalent.
            println!("xtask verify — stub");
        }
        Some(other) => {
            eprintln!("unknown task: {other}");
            std::process::exit(2);
        }
        None => {
            eprintln!("usage: cargo xtask <qemu|verify>");
            std::process::exit(2);
        }
    }
}
