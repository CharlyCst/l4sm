SECTIONS
{
  /* Not needed with panic=abort, and would end up before .text in the flat binary */
  /DISCARD/ : { *(.eh_frame*) }

  /* Start address */
  . = 0x0e090000; /* We use the same address as RF-A on QEMU */

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

