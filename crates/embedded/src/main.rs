#![no_std]
#![no_main]

use core::sync::atomic::{compiler_fence, Ordering};
use cortex_m::interrupt;
use cortex_m::peripheral::{Peripherals, SYST};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use panic_halt as _;

mod samples;

/// 24-bit max — SysTick counter wraps every 2^24 ticks. The KF update is on
/// the order of 10^2 ticks, so a single update never wraps.
const SYST_RELOAD: u32 = 0x00FF_FFFF;

#[entry]
fn main() -> ! {
    // Configure SysTick as a free-running 24-bit countdown timer at the core
    // clock rate. QEMU's lm3s6965evb DWT::CYCCNT is unimplemented (always
    // returns 0), but its SysTick *is* wired to the virtual clock under
    // `-icount shift=0`, so SysTick gives us deterministic per-instruction
    // tick counts that demonstrate the same invariance property.
    let mut cp = Peripherals::take().expect("Peripherals::take must succeed at boot");
    let syst = &mut cp.SYST;
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(SYST_RELOAD);
    syst.clear_current();
    syst.enable_counter();

    let mut filter = codegen_demo::Filter4x2::new();
    #[cfg(feature = "naive-baseline")]
    let mut naive = kalman::cv_2d::<f32>(0.1, 0.01, 0.1);

    let mut min_ticks = u32::MAX;
    let mut max_ticks = 0u32;
    #[cfg(feature = "naive-baseline")]
    let mut min_ticks_naive = u32::MAX;
    #[cfg(feature = "naive-baseline")]
    let mut max_ticks_naive = 0u32;

    for (k, &(z_x, z_y)) in samples::SAMPLES.iter().enumerate() {
        // `interrupt::free` plus the compiler fences pin the measurement window:
        // no ISR fires inside it, and the optimiser cannot move the update body
        // across the SysTick reads. SysTick counts *down*, so before > after.
        let ticks = interrupt::free(|_cs| {
            compiler_fence(Ordering::SeqCst);
            let before = SYST::get_current();
            compiler_fence(Ordering::SeqCst);
            filter.update(z_x, z_y);
            compiler_fence(Ordering::SeqCst);
            let after = SYST::get_current();
            compiler_fence(Ordering::SeqCst);
            before.wrapping_sub(after) & SYST_RELOAD
        });

        if ticks < min_ticks {
            min_ticks = ticks;
        }
        if ticks > max_ticks {
            max_ticks = ticks;
        }

        #[cfg(feature = "naive-baseline")]
        let ticks_naive = {
            let z = nalgebra::SMatrix::<f32, 2, 1>::new(z_x, z_y);
            interrupt::free(|_cs| {
                compiler_fence(Ordering::SeqCst);
                let before = SYST::get_current();
                compiler_fence(Ordering::SeqCst);
                naive.predict();
                naive.update(&z);
                compiler_fence(Ordering::SeqCst);
                let after = SYST::get_current();
                compiler_fence(Ordering::SeqCst);
                before.wrapping_sub(after) & SYST_RELOAD
            })
        };
        #[cfg(feature = "naive-baseline")]
        {
            if ticks_naive < min_ticks_naive {
                min_ticks_naive = ticks_naive;
            }
            if ticks_naive > max_ticks_naive {
                max_ticks_naive = ticks_naive;
            }
        }

        // Always print state — both to be a useful debug artifact and to keep
        // the optimiser from eliminating the macro update as dead code (its
        // only side effect is the state mutation, so if `filter.x` is never
        // read, the whole branchless body collapses to a no-op under LTO).
        hprintln!(
            "update #{:>3}: {:>4} ticks  x={:>8.3} y={:>8.3} vx={:>6.3} vy={:>6.3}",
            k,
            ticks,
            filter.x[0],
            filter.x[1],
            filter.x[2],
            filter.x[3]
        );
        #[cfg(feature = "naive-baseline")]
        hprintln!(
            "  naive #{:>3}: {:>4} ticks  x={:>8.3} y={:>8.3} vx={:>6.3} vy={:>6.3}",
            k,
            ticks_naive,
            naive.x[0],
            naive.x[1],
            naive.x[2],
            naive.x[3]
        );
    }

    if min_ticks == max_ticks {
        hprintln!(
            "FLAT (macro): all {} updates ran in exactly {} ticks.",
            samples::SAMPLES.len(),
            min_ticks
        );
    } else {
        hprintln!(
            "VARIABLE (macro): min={} ticks, max={} ticks, spread={} ticks over {} updates.",
            min_ticks,
            max_ticks,
            max_ticks - min_ticks,
            samples::SAMPLES.len()
        );
    }

    #[cfg(feature = "naive-baseline")]
    {
        if min_ticks_naive == max_ticks_naive {
            hprintln!(
                "FLAT (naive):   all {} updates ran in exactly {} ticks.",
                samples::SAMPLES.len(),
                min_ticks_naive
            );
        } else {
            hprintln!(
                "VARIABLE (naive): min={} ticks, max={} ticks, spread={} ticks over {} updates.",
                min_ticks_naive,
                max_ticks_naive,
                max_ticks_naive - min_ticks_naive,
                samples::SAMPLES.len()
            );
        }
    }

    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
