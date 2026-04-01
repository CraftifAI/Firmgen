/* CPU1 Flash sectors */

/* HEX directive required by HEX utility to generate the golden CMAC tag */
/* with one entry that represents all the allocated flash memory */
ROMS
{
  FLASH_BANK0_2: o=0x00080000 l=0x00060000, fill = 0xFFFF /* If fill not specified, then default is all 0s */
}
