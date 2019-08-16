#![no_std]
#![feature(core_intrinsics)]

pub mod debug;

#[cfg(feature = "panic_handler")]
mod panic;
