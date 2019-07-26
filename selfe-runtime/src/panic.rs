use core::fmt::Write;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _res = writeln!(crate::debug::DebugOutHandle, "*** Panic: {:#?}", info);

    unsafe {
        core::intrinsics::abort();
    }
}
