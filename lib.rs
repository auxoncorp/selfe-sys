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
#![feature(lang_items, global_allocator, allocator_api, alloc, core_intrinsics)]
#![doc(html_root_url = "https://doc.robigalia.org/")]

extern crate sel4_sys;
extern crate sel4;

use sel4_sys::*;

mod alloc;

pub use alloc::*;

pub static mut BOOTINFO: *mut seL4_BootInfo = (0 as *mut seL4_BootInfo);
static mut RUN_ONCE: bool = false;
#[global_allocator]
static ALLOCATOR: ScratchAlloc = ScratchAlloc { };

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
        seL4_SetUserData((*bootinfo).ipcBuffer as usize as seL4_Word);
    }
}

#[lang = "start"]
fn lang_start<T: Termination + 'static>(main: fn() -> T, _argc: isize, _argv: *const *const u8) -> isize {
    main();
    panic!("Root server shouldn't ever return from main!");
}

// the initial thread really ought not fail. but if it does, hang.
// eventually do something smarter. a backtrace might be nice.
#[lang = "panic_fmt"]
extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    use core::fmt::Write;
    let _ = write!(sel4::DebugOutHandle, "panic at {}:{}: ", file, line);
    let _ = sel4::DebugOutHandle.write_fmt(fmt);
    let _ = sel4::DebugOutHandle.write_char('\n');
    unsafe { core::intrinsics::abort(); }
}

#[lang = "eh_personality"]
fn eh_personality() {
    unsafe { core::intrinsics::abort(); }
}
