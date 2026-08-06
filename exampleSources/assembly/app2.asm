; x86_64 — iterative fibonacci printing each value then exit.

global _start

section .data
sep     db  " ", 0
nl      db  10, 0

section .bss
buf     resb 32

section .text
print_u64:
    mov     rbp, rsp
    lea     rcx, [buf + 31]
    mov     rbx, 10
    xor     rax, rax
    mov     rdx, rax
    div_loop:
        xor     rdx, rdx
        div     rbx
        add     dl, '0'
        dec     rcx
        mov     [rcx], dl
        test    rax, rax
        jnz     div_loop
    mov     rsi, rcx
    mov     rdx, rdi
    sub     rdx, rcx
    mov     rax, 1
    mov     rdi, 1
    syscall
    ret

_start:
    mov     r12, 0
    mov     r13, 1
    mov     r14, 10
.print_loop:
    mov     rdi, r12
    call    print_u64
    mov     rax, 1
    mov     rdi, 1
    lea     rsi, [rel sep]
    mov     rdx, 1
    syscall
    mov     rax, r12
    mov     r12, r13
    add     r13, rax
    dec     r14
    jnz     .print_loop

    mov     rax, 60
    xor     rdi, rdi
    syscall