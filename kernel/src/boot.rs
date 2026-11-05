//! Boot code. QEMU (acting as a multiboot loader) starts us in 32-bit mode.
//! We build page tables that map the first 1 GiB, switch the CPU to 64-bit
//! "long mode", and call `kernel_main` in Rust.

core::arch::global_asm!(
    r#"
.section .multiboot, "a"
.align 4
mb_header:
    .long 0x1BADB002                    # multiboot magic
    .long 0x00010003                    # flags: page align, memory info, load addresses below
    .long -(0x1BADB002 + 0x00010003)    # checksum
    .long mb_header
    .long __load_start
    .long __load_end
    .long __bss_end
    .long _start

.section .boot, "ax"
.code32
.global _start
_start:
    cli
    cld                                 # string ops go forward (multiboot doesn't promise this)
    mov $boot_stack_top, %esp
    mov %ebx, %esi                      # keep the multiboot info pointer

    # Zero all of .bss (Rust statics, page tables, stack, heap).
    mov $__load_end, %edi
    mov $__bss_end, %ecx
    sub %edi, %ecx
    xor %eax, %eax
    rep stosb

    # PML4[0] -> PDPT, PDPT[0] -> PD
    mov $pdpt, %eax
    or $0x3, %eax
    mov %eax, pml4
    mov $pd, %eax
    or $0x3, %eax
    mov %eax, pdpt

    # PD: 512 entries of 2 MiB pages = identity map of the first 1 GiB.
    xor %ecx, %ecx
1:
    mov %ecx, %eax
    shl $21, %eax
    or $0x83, %eax                      # present | writable | huge page
    mov %eax, pd(,%ecx,8)
    inc %ecx
    cmp $512, %ecx
    jne 1b

    mov $pml4, %eax
    mov %eax, %cr3
    mov %cr4, %eax
    or $(1 << 5), %eax                  # PAE
    mov %eax, %cr4
    mov $0xC0000080, %ecx               # EFER
    rdmsr
    or $(1 << 8), %eax                  # long mode enable
    wrmsr
    mov %cr0, %eax
    or $0x80000001, %eax                # paging + protected mode
    mov %eax, %cr0

    lgdt gdt64_ptr
    ljmp $0x08, $long_mode

.code64
long_mode:
    xor %ax, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss
    mov $boot_stack_top, %rsp
    mov %esi, %edi                      # multiboot info -> first argument
    call kernel_main
2:
    hlt
    jmp 2b

.section .rodata
.align 8
gdt64:
    .quad 0
    .quad 0x00AF9A000000FFFF            # 64-bit kernel code segment
gdt64_ptr:
    .word gdt64_ptr - gdt64 - 1
    .long gdt64

.section .bss, "aw", @nobits
.align 4096
pml4: .skip 4096
pdpt: .skip 4096
pd:   .skip 4096
boot_stack: .skip 262144  # 256 KB: chores need up to ~96 KB in the worst case
boot_stack_top:
"#,
    options(att_syntax)
);
