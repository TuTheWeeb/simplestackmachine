jmp start
triple_add:
    add8
    add8
    ret
start:
    push8 10
    push8 9
    push8 1
    jmp triple_add
    push8 50
    prt8
    jmp back
back:
