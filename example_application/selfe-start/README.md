# selfe-start

This is a local fork of [sel4-start](https://gitlab.com/robigalia/sel4-start).

[![Crates.io](https://img.shields.io/crates/v/sel4-start.svg?style=flat-square)](https://crates.io/crates/sel4-start)

[Documentation](https://doc.robigalia.org/sel4_start)

This crate defines the entry point `_sel4_start` and a Rust `#[lang =
"start"]` entry point which the `_sel4_start` calls after initializing the
global `BootInfo` instance which is also defined in this crate. This is used
only for the "initial thread" of the system, which is the first thread that
seL4 starts. In the `BootInfo` are many wonderous things. See the [seL4
manual](http://sel4.systems/Info/Docs/seL4-manual-2.0.0.pdf), table 9.2 on
page 39, for canonical information about the content of the `BootInfo`.

The initial thread is created with 16K of stack space.

## Status

Complete.
