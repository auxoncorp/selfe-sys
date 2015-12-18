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
#![feature(lang_items, no_std, core_intrinsics)]

extern crate sel4_sys;
use sel4_sys::*;

pub static mut BOOTINFO: *mut seL4_BootInfo = (0 as *mut seL4_BootInfo);
static mut RUN_ONCE: bool = false;

#[doc(hidden)]
#[no_mangle]
/// Internal function which sets up the global `BOOTINFO`. Can only be called once - it sets a
/// private flag when it is called and will not modify `BOOTINFO` if that flag is set.
pub unsafe extern "C" fn __sel4_start_init_boot_info(bootinfo: *mut seL4_BootInfo) {
    if !RUN_ONCE {
        BOOTINFO = bootinfo;
        RUN_ONCE = true;
        seL4_SetUserData((*bootinfo).ipcBuffer as usize as seL4_Word);
    }
}

#[lang = "start"]
fn lang_start(main: *const u8, _argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        core::intrinsics::transmute::<_, fn()>(main)();
    }
    0
}

// the initial thread really ought not fail. but if it does, hang.
// eventually do something smarter. a backtrace might be nice.
#[lang = "panic_fmt"]
fn panic_fmt() {
    unsafe { core::intrinsics::abort(); }
}

#[lang = "eh_personality"]
fn eh_personality() {
    unsafe { core::intrinsics::abort(); }
}
