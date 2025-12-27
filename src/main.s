jmp start

loop:
    inc
    dup
    push 1000000000
    cmp
    jnz loop
    ret

start:
    push 0
    call loop
