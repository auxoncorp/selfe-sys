#![no_std]
#![feature(core_intrinsics)]

pub mod debug;
mod libc;

#[cfg(feature = "panic_handler")]
mod panic;
