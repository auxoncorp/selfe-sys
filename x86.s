/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */
    .global _sel4_start
    .global _start
    .text

_start:
_sel4_start:
    leal    _stack_top, %esp
    /* Setup segment selector for IPC buffer access. */
    movw    $((7 << 3) | 3), %ax
    movw    %ax, %gs
    /* Setup the global "bootinfo" structure. */
    pushl   %ebx
    call    __sel4_start_init_boot_info
    /* We drop another word off the stack pointer so that rustc's generated
     * main can scrape the "argc" and "argv" off the stack. We set them to 0
     * and NULL though. */
    subl    $4, %esp
    movw $0, -4(%esp)
    movw $0, -4(%esp)
    /* Now go to the "main" stub that rustc generates */
    call main
    /* if main returns, die a loud and painful death. */
    ud2
    .data
    .align 4
    .bss
    .align  8
_stack_bottom:
    .space  16384
_stack_top:
