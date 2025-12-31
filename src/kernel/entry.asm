extern enter

global _start
_start:
    jmp enter
    cli
    hlt
