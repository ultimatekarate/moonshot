/* lm3s6965evb has 64K RAM physically; the moonshot's claimed budget is 32K.
 * Restricting the link script makes the constraint real and gives `just size`
 * something to bite — if the binary won't fit, the link fails loudly. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 256K
  RAM   : ORIGIN = 0x20000000, LENGTH = 32K
}
