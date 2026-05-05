#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;

mod samples;

#[entry]
fn main() -> ! {
    // Stub: see docs/plan.md Lab 5.
    // Real implementation will:
    //   - spawn an embassy task
    //   - walk samples::SAMPLES, calling Filter4x2::update
    //   - hprintln! the state on each tick
    //   - exit via cortex_m_semihosting::debug::exit on completion
    let mut filter = codegen_demo::Filter4x2::new();
    for &(z_x, z_y) in samples::SAMPLES.iter() {
        filter.update(z_x, z_y);
    }
    loop {
        cortex_m::asm::wfi();
    }
}
