/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */

#[doc(hidden)]
#[naked]
#[no_mangle]
/// This is the entry point to the root task image. Set up the stack, stash the
/// boot info, then call the rust-generated main function.
///
/// The call chain from here will look like this:
///   sel4_start::_start ->
///   sel4_start::_real_start ->
///   <rust-generated>::main() ->
///   sel4_start::lang_start() (start lang item) ->
///   <user-defined>::main()
pub unsafe extern "C" fn _start() -> ! {
    // We are in a particularly precarious position with regards to the stack on
    // x86. In order to set the stack pointer to the address of our stack buffer
    // variable LLVM will calculate its offset from the instruction pointer, in
    // order to be "position independent" code. Unfortunately, there is no
    // instruction-pointer- offset addressing mode in x86 like there is in
    // x86_64, so what LLVM will do is CALL the address of the next instruction
    // and then POP the return address off the stack to get the instruction
    // pointer. THIS DOESN'T WORK VERY WELL WITHOUT A STACK.
    //
    // So there'a chicken and egg problem... we can't set the stack pointer without
    // having a stack!
    //
    // Our solution is to set the stack pointer to the bootinfo structure that sel4
    // gave us in ebx, and save a backup of the first word from that structure.
    // Then, once we set the stack pointer to its real value, we can fix the
    // clobbered first word in the bootinfo structure with our backup.
    //
    // Because LLVM does this CALL/POP magic at the beginning of the function before
    // any of our code runs, we have to setup our temporary stack first in a
    // function by itself that doesn't touch any variables so as to not need the
    // CALL/POP magic, then we jump to the real start function that does.

    // setup temporary stack pointer into the bootinfo structure and backup the
    // parts we will corrupt
    llvm_asm!(
        "
        /* set stack pointer to bootinfo structure */
        movl %ebx, %esp
        /* setup a one-word stack will overwrite the beginning of the bootinfo structure */
        addl $$4, %esp
        /* save a backup of the affected bootinfo word */
        movl (%ebx), %esi
        /* now do the important stuff */
        jmp _real_start
        "
        :
        :
        : "esp", "ebx", "esi", "memory"
        : "volatile"
    );

    // if main returns, die a loud and painful death.
    core::intrinsics::unreachable();
}

#[naked]
#[no_mangle]
#[doc(hidden)]
pub unsafe extern "C" fn _real_start() -> ! {
    // setup real stack pointer and fix the corrupted value in the bootinfo
    // structure don't mess with ebx which we need next
    llvm_asm!(
        "
        /* esp is currently bottom of stack, make it top of stack */
        addl $1, %esp
        /* put a nonsensical value in ebp so we fail fast if we touch it */
        movl $$0xdeadbeef, %ebp
        /* fix the corrupted value in the bootinfo structure */
        movl %esi, (%ebx)
        "
        :
        : "{esp}" (&(STACK.stack))
          "i" (STACK_SIZE)
        : "esp", "ebp", "ebx", "memory"
        : "volatile"
    );

    // Setup segment selector for IPC buffer access.
    // LLVM might be using eax so we save it manually to avoid having to worry about
    // how llvm might save it if we add it to the clobbers list
    llvm_asm!(
        "
        pushl %eax
        movw    $$((7 << 3) | 3), %ax
        movw    %ax, %fs
        popl %eax
        "
        :
        :
        :
        : "volatile"
    );

    // Setup the global "bootinfo" structure.
    // ebx was set by sel4 and contains the pointer to the bootinfo structure
    llvm_asm!(
        "
        pushl   %ebx
        call    __sel4_start_init_boot_info
        /* We drop another word off the stack pointer so that rustc's generated
         * main can scrape the 'argc' and 'argv' off the stack.
         * TODO: why is this necessary? Caller cleanup of above %ebx? */
        addl    $$4, %esp
        "
        :
        :
        :
        : "volatile"
    );

    // Call main stub that rustc generates
    llvm_asm!(
        "
        /* Null terminate auxv */
        pushl $$0
        pushl $$0
        /* Null terminate envp */
        pushl $$0
        /* add at least one environment string (why?) */
        pushl $0
        /* Null terminate argv */
        pushl $$0
        /* Give an argv[0] */
        pushl $1
        /* Give argc */
        pushl $$1
        /* No atexit */
        movl $$0, %edx

        /* Now go to the 'main' stub that rustc generates */
        call main
        "
        :
        : "{eax}" (ENVIRONMENT_STRING as *const [u8] as *const u8),
          "{ebx}" (PROG_NAME as *const [u8] as *const u8)
        :
        : "volatile"
    );

    // if main returns, die a loud and painful death.
    core::intrinsics::unreachable();
}
