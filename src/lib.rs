#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Allow std in tests
#[cfg(test)]
#[macro_use]
extern crate std;

mod compile_time_assertions;

type seL4_CPtr = usize;
type seL4_Word = usize;
type seL4_Int8 = i8;
type seL4_Int16 = i16;
type seL4_Int32 = i32;
type seL4_Int64 = i64;
type seL4_Uint8 = u8;
type seL4_Uint16 = u16;
type seL4_Uint32 = u32;
type seL4_Uint64 = u64;

pub const seL4_WordBits: usize = core::mem::size_of::<usize>() * 8;

#[cfg(any(target_arch = "arm", target_arch = "x86"))]
mod ctypes {
    pub type c_char = i8;
    pub type c_uint = u32;
    pub type c_int = i32;
    pub type c_ulong = u32;
}

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
pub mod ctypes {
    pub type c_char = i8;
    pub type c_uint = u32;
    pub type c_int = i32;
    pub type c_ulong = u64;
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));

