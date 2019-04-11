#![no_std]
#![feature(lang_items, core_intrinsics)]

use core::panic::PanicInfo;

use core::fmt::Write;
use sel4_start::{self, DebugOutHandle};

fn main() {
    let bootinfo = unsafe { &*sel4_start::BOOTINFO };
    writeln!(DebugOutHandle, "Hello fancy world!").unwrap();
    let num_nodes = bootinfo.numNodes; // Pull out a reference to resolve packed-struct misalignment risk
    writeln!(
        DebugOutHandle,
        "Thing from bootinfo: numNodes={}",
        num_nodes
    )
    .unwrap();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    sel4_start::debug_panic_handler(&info)
}
