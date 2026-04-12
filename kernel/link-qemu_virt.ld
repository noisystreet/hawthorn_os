/* QEMU virt machine: RAM at 0x4000_0000 (see docs/PORTING.md).
 * Canonical copy for `hawthorn_kernel` and `hawthorn_qemu_minimal`. */
ENTRY(_start)

MEMORY {
  RAM : ORIGIN = 0x40000000, LENGTH = 128M
}

SECTIONS {
  .text : {
    KEEP(*(.text.boot))
    *(.text .text.*)
  } > RAM

  .rodata : {
    *(.rodata .rodata.*)
  } > RAM

  .data : {
    *(.data .data.*)
  } > RAM

  .bss (NOLOAD) : {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  } > RAM

  /* Stack grows down; SP starts at last 16-byte-aligned word inside RAM. */
  __stack_top = ORIGIN(RAM) + LENGTH(RAM) - 16;
}
