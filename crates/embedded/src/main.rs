#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use panic_halt as _;

mod samples;

#[entry]
fn main() -> ! {
    let mut filter = codegen_demo::Filter4x2::new();

    for (k, &(z_x, z_y)) in samples::SAMPLES.iter().enumerate() {
        filter.update(z_x, z_y);
        hprintln!(
            "[k={:>3}] x={:>8.3} y={:>8.3} vx={:>6.3} vy={:>6.3}",
            k, filter.x[0], filter.x[1], filter.x[2], filter.x[3]
        );
    }

    hprintln!("done");
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
