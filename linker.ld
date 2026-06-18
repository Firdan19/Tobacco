ENTRY(_start)

SECTIONS {
    . = 1M;

    .multiboot ALIGN(8) : {
        LONG(0xE85250D6)
        LONG(0)
        LONG(24)
        LONG(0x17ADAF12)
        LONG(0)
        LONG(8)
    }

    .text ALIGN(4K) : {
        *(.text.boot)
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : {
        *(COMMON)
        *(.bss .bss.*)
    }
}
