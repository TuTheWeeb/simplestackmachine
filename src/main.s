jmp start

loop:
    push 10
    push 20
    uadd64
    ret

start:
    push 0
    call loop
