/* lm3s6965evb (Cortex-M3) memory layout — matches qemu-system-arm -M lm3s6965evb. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 256K
  RAM   : ORIGIN = 0x20000000, LENGTH = 64K
}
