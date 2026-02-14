SECTIONS
{
  /* Start address */
  . = 0x1000; /* TODO: figure out what should be the start address */

  /* Output a text section, starting with the entry point */
  .text : ALIGN(0x4) {
    _start
    *(.text)
    *(.text.*)
  }

  /* Output the rodata */
  .rodata : ALIGN(0x8) {
    KEEP(*(__*))
    *(.rodata)
    *(.rodata.*)
  }

  /* Finally, all data                                         */
  /* NOTE: no need to page-align bss, both bss and data are RW */
  .data : ALIGN(0x8) {
    KEEP(*(__*))
    *(.data)
    *(.data.*)
  }
  .sdata : ALIGN(0x8) {
    KEEP(*(__*))
    *(.sdata)
    *(.sdata.*)
  }
  . = ALIGN(0x8);
  _bss_start = .;
  .sbss : {
    *(.sbss)
    *(.sbss.*)
  }
  .bss : ALIGN(0x8) {
    *(.bss)
    *(.bss.*)
  }
  _bss_stop = .;

  /* Then we mark the start of the stack (or the end, as the stack grows
   * downard). */
  . = ALIGN(0x1000);
  _stack_start = .;
}

