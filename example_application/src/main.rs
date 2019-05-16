#![no_std]
#![feature(lang_items, core_intrinsics, asm, global_asm)]

use core::panic::PanicInfo;

use core::fmt::Write;
use sel4_start::{self, DebugOutHandle};
use selfe_arc;
use selfe_sys::{seL4_BootInfo, seL4_CapInitThreadTCB, seL4_TCB_Suspend};

extern "C" {
    static _selfe_arc_data_start: u8;
    static _selfe_arc_data_end: usize;
}

fn main() {
    #[cfg(target_arch = "arm")]
    let arch = "arm";
    #[cfg(target_arch = "x86_64")]
    let arch = "x86_64";

    writeln!(DebugOutHandle, "\n\nHello {} world!\n\n", arch).unwrap();

    let bootinfo: &'static seL4_BootInfo = unsafe { &*sel4_start::BOOTINFO };
    let num_nodes = bootinfo.numNodes; // Pull out a reference to resolve packed-struct misalignment risk
    writeln!(
        DebugOutHandle,
        "Thing from bootinfo: numNodes={}",
        num_nodes
    )
    .unwrap();

    let archive_slice: &[u8] = unsafe {
        core::slice::from_raw_parts(
            &_selfe_arc_data_start,
            &_selfe_arc_data_end as *const _ as usize - &_selfe_arc_data_start as *const _ as usize,
        )
    };

    let archive = selfe_arc::read::Archive::from_slice(archive_slice);
    let data_file_slice = archive.file("data_file.txt").unwrap();
    let content = core::str::from_utf8(data_file_slice).unwrap();

    writeln!(DebugOutHandle, "Read data file from selfe arc: {}", content).unwrap();

    let suspend_error = unsafe { seL4_TCB_Suspend(seL4_CapInitThreadTCB as usize) };
    if suspend_error != 0 {
        writeln!(
            DebugOutHandle,
            "Error suspending root task thread: {}",
            suspend_error
        )
        .unwrap();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    sel4_start::debug_panic_handler(&info)
}

global_asm!(
    r#"
.section ".rodata"
.balign 4096

.global _selfe_arc_data_start
.type _selfe_arc_data_start, object
_selfe_arc_data_start:
  .incbin "target/selfe_arc_data"

.balign 4096

.global _selfe_arc_data_end
_selfe_arc_data_end:
"#
);
