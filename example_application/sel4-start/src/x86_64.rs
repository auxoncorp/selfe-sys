/* Copyright (c) 2017 The Robigalia Project Developers
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
#[cfg(not(test))]
/// This is the entry point to the root task image. Set up the stack, stash the
/// boot info, then call the rust-generated main function.
///
/// The call chain from here will look like this:
///   sel4_start::_start ->
///   <rust-generated>::main() ->
///   sel4_start::lang_start() (start lang item) ->
///   <user-defined>::main()
pub unsafe extern "C" fn _start() -> ! {
    // setup stack pointer
    // don't mess up rdi which we need next
    llvm_asm!(
        "
        /* rsp is currently bottom of stack, make it top of stack */
        addq $1, %rsp
        /* put a nonsensical value in rbp so we fail fast if we touch it */
        movq $$0xdeadbeef, %rbp
        "
        :
        : "{rsp}" (&(STACK.stack))
          "i" (STACK_SIZE)
        : "rdi", "rsp", "rbp"
        : "volatile"
    );

    // setup the global 'bootinfo' structure
    // The argument to this function has been put into rdi for us by sel4
    llvm_asm!("call __sel4_start_init_boot_info" :::: "volatile");

    // Call main stub that rustc generates
    llvm_asm!(
        "
        /* N.B. rsp MUST be aligned to a 16-byte boundary when main is called.
         * Insert or remove padding here to make that happen.
         */
        pushq $$0
        /* Null terminate auxv */
        pushq $$0
        pushq $$0
        /* Null terminate envp */
        pushq $$0
        /* add at least one environment string (why?) */
        pushq $0
        /* Null terminate argv */
        pushq $$0
        /* Give an argv[0] (why?) */
        pushq $1
        /* Give argc */
        pushq $$1
        /* No atexit */
        movq $$0, %rdx

        /* Now go to the 'main' stub that rustc generates */
        call main
        "
        :
        : "{rax}" (ENVIRONMENT_STRING as *const [u8] as *const u8),
          "{rbx}" (PROG_NAME as *const [u8] as *const u8)
        :
        : "volatile"
    );

    // if main returns, die a loud and painful death.
    core::intrinsics::unreachable()
}
