# example

An example seL4 application which uses [selfe-sys](../README.md)
to make syscalls and [selfe-start](./selfe-start/README.md) to bridge the gap between
a bare-bones Rust `#[no_std]` application and one that will work on the seL4 microkernel.

## Highlights

A [sel4.toml](sel4.toml) file sits at the project root, next to the Cargo.toml,
and provides the build configuration for the underlying seL4 kernel.

In order for the Rust project to link properly with the seL4 code, the following
rustflags are set in [.cargo/config](.cargo/config):

```toml
[build]
rustflags = ["-C", "link-arg=-no-pie", "-C", "link-arg=-nostdlib"]

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
```

Note also the presence of a selected linker to support the cross-compilation-for-arm
use case.

### Dependencies
Note that `selfe-sys` and `selfe-start` are included as regular Cargo.toml dependencies.

### Language Items

In order to let application-builders pick the level of secrecy they want around their failure
cases, [main.rs](src/main.rs) defines a `#[panic_handler]` implementation (albeit one that
immediately delegates to an optional helper from `selfe-start`)

### Boot Info

`selfe-start` exposes to the root task a static `selfe_start::BOOTINFO` item
that represents the kernel-supplied `seL4_BootInfo` instance from which
most information necessary to work with seL4 can derived.

```root
    let bootinfo: &'static seL4_BootInfo = unsafe { &*selfe_start::BOOTINFO };
    // Do work with the boot info instance here
```

## Building

You can build directly with [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild) or
[xargo](https://github.com/japaric/xargo), specifying your Rust target triple as an explicit argument.

Optionally use the environment variable `SEL4_CONFIG_PATH` to point to your sel4.toml configuration file 
and the `SEL4_PLATFORM` env-var to select your desired platform target.
```
SEL4_PLATFORM=pc99 cargo xbuild --target x86_64-unknown-linux-gnu
SEL4_PLATFORM=sabre cargo xbuild --target armv7-unknown-linux-gnueabihf
```

Alternately, you can build or run with the [selfe](../selfe-config/README.md)
tool, executed from this example project's directory.

```
selfe build --sel4_arch x86_64 --platform pc99 --debug
selfe build --sel4_arch x86_64 --platform pc99 --release

selfe build --sel4_arch aarch32 --platform sabre

selfe simulate --sel4_arch aarch32 --platform sabre
```
