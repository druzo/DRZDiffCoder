; x86_64 Linux — syscall write "Hello, world!" + newline then exit.

global _start

section .data
msg     db  "Hello, world!", 10
msg_len equ $ - msg

section .text
_start:
    mov     rax, 1          ; sys_write
    mov     rdi, 1          ; stdout
    mov     rsi, msg
    mov     rdx, msg_len
    syscall

    mov     rax, 60         ; sys_exit
    xor     rdi, rdi        ; status 0
    syscall