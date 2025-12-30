jmp start

func:
    push 11
    push 10
    cmp
    ret

start:
    call func
    call func
