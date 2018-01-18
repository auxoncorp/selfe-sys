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
/// This is the entry point to the root task image. Set up the stack, stash the boot
/// info, then call the rust-generated main function.
///
/// The call chain from here will look like this:
///   sel4_start::_start ->
///   sel4_start::_real_start ->
///   <rust-generated>::main() ->
///   sel4_start::lang_start() (start lang item) ->
///   <user-defined>::main()
pub unsafe extern fn _start() -> ! {
    // LLVM clobbers r0 due to the way it does position independent code. We need to keep
    // r0 because it points to our bootinfo structure. Save it off in a temp register so
    // we can get to it later.
    //
    // Because LLVM loads the instruction pointer at the beginning of the function before
    // any of our code runs, we have to split this out into two functions.

    asm!(
        "
        /* save r0 into r8 */
        mov r8, r0
        b _real_start
        "
        :
        :
        : "r8"
        : "volatile"
    );

    core::intrinsics::unreachable();
}

#[doc(hidden)]
#[naked]
#[no_mangle]
pub unsafe extern fn _real_start() -> ! {
    asm!(
        "
        /* sp is currently bottom of stack, make it top of stack */
        add sp, sp, $1
        /* restore the saved r0 */
        mov r0, r8
        /* r0, the first arg in the calling convention, is set to the bootinfo
        * pointer on startup. */
        bl __sel4_start_init_boot_info
        /* zero argc, argv */
        mov r0, #0
        mov r1, #0
        /* Now go to the 'main' stub that rustc generates */
        bl main
        "
        :
        : "{sp}" (&(STACK.stack)),
          "i" (STACK_SIZE)
        : "sp", "r0", "r1"
        : "volatile"
    );

    core::intrinsics::unreachable();
}
