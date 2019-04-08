/* Copyright (c) 2015 The Robigalia Project Developers
 * Licensed under the Apache License, Version 2.0
 * <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT
 * license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
 * at your option. All files in the project carrying such
 * notice may not be copied, modified, or distributed except
 * according to those terms.
 */
#![no_std]
#![feature(lang_items, core_intrinsics, asm, naked_functions)]
#![doc(html_root_url = "https://doc.robigalia.org/")]

use core::alloc::Layout;
extern crate sel4_sys;
use sel4_sys::*;

#[repr(align(4096))]
#[doc(hidden)]
/// A wrapper around our stack so that we can specify its alignment requirement.
struct Stack {
    stack: [u8; STACK_SIZE],
}

pub static mut BOOTINFO: *mut seL4_BootInfo = (0 as *mut seL4_BootInfo);
static mut RUN_ONCE: bool = false;

#[used]
#[doc(hidden)]
static ENVIRONMENT_STRING: &'static [u8] = b"seL4=1\0\0";

#[used]
#[doc(hidden)]
static PROG_NAME: &'static [u8] = b"rootserver\0";

/// The size of the initial root thread stack. This stack is located in the root task image data
/// section.
pub const STACK_SIZE: usize = 1024 * 68;

#[used]
#[doc(hidden)]
/// The stack for our initial root task thread.
static mut STACK: Stack = Stack { stack: [0u8; STACK_SIZE] };

#[lang = "termination"]
trait Termination {
    fn report(self) -> i32;
}

impl Termination for () {
    fn report(self) -> i32 {
        0
    }
}

#[doc(hidden)]
#[no_mangle]
/// Internal function which sets up the global `BOOTINFO`. Can only be called once - it sets a
/// private flag when it is called and will not modify `BOOTINFO` if that flag is set.
pub unsafe extern "C" fn __sel4_start_init_boot_info(bootinfo: *mut seL4_BootInfo) {
    if !RUN_ONCE {
        BOOTINFO = bootinfo;
        RUN_ONCE = true;
        seL4_SetUserData((*bootinfo).ipcBuffer as usize);
    }
}

#[lang = "start"]
fn lang_start<T: Termination + 'static>(main: fn() -> T, _argc: isize, _argv: *const *const u8) -> isize {
    main();
    0
}

struct DebugOutHandle;

// TODO conditional compilation for this
impl ::core::fmt::Write for DebugOutHandle {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for &b in s.as_bytes() {
            unsafe { seL4_DebugPutChar(b as i8) };
        }
        Ok(())
    }
}


// the initial thread really ought not fail. but if it does, hang.
// eventually do something smarter. a backtrace might be nice.
// #[lang = "panic_fmt"]
// TODO conditional compilation for this
pub fn default_panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    use core::fmt::Write;
    let _ = write!(DebugOutHandle, "panic at {}:{}: ", file, line);
    let _ = DebugOutHandle.write_fmt(fmt);
    let _ = DebugOutHandle.write_char('\n');
    unsafe { core::intrinsics::abort(); }
}

#[lang = "oom"]
#[no_mangle]
pub fn rust_oom(_layout: Layout) -> ! {
    panic!("Root server has run out of memory!");
}

#[lang = "eh_personality"]
fn eh_personality() {
    unsafe { core::intrinsics::abort(); }
}

/// Returns the address of the bottom of the stack for the initial root task thread.
pub fn get_stack_bottom_addr() -> usize {
    unsafe { (&(STACK.stack)).as_ptr() as usize }
}

#[cfg(target_arch = "x86")]
include!("x86.rs");

#[cfg(target_arch = "x86_64")]
include!("x86_64.rs");

#[cfg(all(target_arch = "arm", target_pointer_width = "32"))]
include!("arm.rs");
