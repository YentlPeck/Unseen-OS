/* The kernel is loaded at 1 MiB. The multiboot header must come first. */
ENTRY(_start)

SECTIONS
{
    . = 1M;
    __load_start = .;

    .text : {
        KEEP(*(.multiboot))
        *(.boot)
        *(.text .text.*)
    }
    .rodata : ALIGN(16) { *(.rodata .rodata.*) }
    .data : ALIGN(16) { *(.data .data.*) *(.got .got.*) }

    __load_end = .;

    .bss : ALIGN(4096) { *(.bss .bss.*) *(COMMON) }

    __bss_end = .;

    /DISCARD/ : { *(.eh_frame*) *(.comment) *(.note*) }
}
