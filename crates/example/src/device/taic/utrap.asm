.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text.usertrap
    .globl __alltraps_u
    .globl __restore_u
    .align 4
__alltraps_u:
    addi sp, sp, -35*8; # sp = sp + -35*8
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)

    # t0-t2
    .set n, 5
    .rept 3
        SAVE_GP %n
        .set n, n+1
    .endr

    # a0-a7
    .set n, 10
    .rept 8
        SAVE_GP %n
        .set n, n+1
    .endr

    # t3-t6
    .set n, 28
    .rept 4
        SAVE_GP %n
        .set n, n+1
    .endr

    csrr t0, 0x000
    csrr t1, 0x041
    csrr t2, 0x005
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    sd t2, 34*8(sp)
    csrr t3, 0x040
    sd t3, 2*8(sp)
    mv  a0, sp # a0 = sp
    call user_trap_handler

__restore_u:
    mv sp, a0
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 34*8(sp)
    ld t3, 2*8(sp)
    csrw 0x000, t0
    csrw 0x041, t1
    csrw 0x005, t2
    csrw 0x040, t3
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)

    # t0-t2
    .set n, 5
    .rept 3
        LOAD_GP %n
        .set n, n+1
    .endr

    # a0-a7
    .set n, 10
    .rept 8
        LOAD_GP %n
        .set n, n+1
    .endr

    # t3-t6
    .set n, 28
    .rept 4
        LOAD_GP %n
        .set n, n+1
    .endr

    addi sp, sp, 35*8
    .word 0x00200073    # uret
