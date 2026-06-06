/* INSERT directive: keep the default linker script intact and only
 * append our trampoline section. Without `INSERT`, this would replace
 * the entire default layout. */
SECTIONS {
    .kpti_trampoline ALIGN(4096) : ALIGN(4096)
    {
        __kpti_trampoline_start = .;
        KEEP(*(.kpti_trampoline))
        . = ALIGN(4096);
        KEEP(*(.kpti_trampoline_data))
        . = ALIGN(4096);
        __kpti_trampoline_end = .;
    }
} INSERT AFTER .text;
