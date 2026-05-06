# Binary size

**Date:** 2026-05-06
**Target:** `thumbv7m-none-eabi`
**Profile:** release (`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `overflow-checks = false`)
**Build command:** `cargo build -p embedded --target thumbv7m-none-eabi --release`
**Measurement command:** `llvm-size -A target/thumbv7m-none-eabi/release/embedded`

## Result

```
section              size        addr
.vector_table        1024           0
.text               28572        1024
.rodata              3060       29600
.data                   0   536870912
.gnu.sgstubs            0       32672
.bss                   12   536870912
.uninit                 0   536870924
.comment              153           0
.ARM.attributes        48           0
Total               32869
```

## Interpretation

**Flash image (what gets programmed onto the chip):** `vector_table + text + rodata + data` = `1024 + 28572 + 3060 + 0` = **32,656 bytes**.

**Static RAM (what's reserved before main runs):** `bss + data` = `12 + 0` = **12 bytes**. Stack growth is dynamic and not included in this number.

**ELF on disk (`ls -l`):** ~133 KB. The difference between this and the flash image is debug info, symbol tables, DWARF data, and section metadata — none of which is flashed. Reading `ls`-reported file size as "binary size" overstates the embedded footprint by ~4×.

## What this proves and doesn't prove

- ✓ Flash image fits under 32 KB (32,656 bytes ≤ 32,768 bytes), with **112 bytes of headroom**. Tight.
- ✓ Static RAM use is trivial (12 bytes). The 32 KB RAM constraint in [memory.x](../crates/embedded/memory.x) is nowhere close to being threatened.
- ✗ **Stack usage is not measured.** Maximum stack depth during `update()` and the surrounding semihosting calls has not been bounded.
- ✗ **The "32 KB chip" framing is ambiguous.** Real chips with that label vary: nRF52810 (24 KB RAM, 192 KB flash), STM32F103C6 (10 KB RAM, 32 KB flash), STM32G0 family. We fit easily on RAM-constrained chips with more flash; we are tight to the line on flash-constrained chips.

## Where the 32 KB of flash actually goes

Most of `.text` is not the filter. The macro-emitted `Filter4x2::update()` is on the order of a few hundred bytes — it's 16 multiply-adds and a return. The bulk of the 28.5 KB `.text` section is `core::fmt` and `cortex_m_semihosting`'s formatting machinery, pulled in by the 100 `hprintln!` calls in [crates/embedded/src/main.rs](../crates/embedded/src/main.rs). Replacing semihosting with RTT, or stripping output entirely, would likely drop the binary into the single-digit-KB range. This is a property of the demo, not the kernel.

## Reproducing this

```bash
rustup component add llvm-tools-preview
cargo build -p embedded --target thumbv7m-none-eabi --release
SYSROOT=$(rustc --print sysroot)
BINDIR=$(ls -d "$SYSROOT/lib/rustlib"/*/bin/)
"$BINDIR/llvm-size" -A target/thumbv7m-none-eabi/release/embedded
```
