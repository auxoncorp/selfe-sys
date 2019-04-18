# libsel4-sys-gen

A generated thin wrapper around libsel4.a

## Usage

Add a dependency to this library in your Cargo.toml manifest:

```toml
[dependencies]
libsel4-sys-gen = { git = "ssh://git@github.com/auxoncorp/confignoble.git" }
```

And then in your Rust project:

```rust
extern crate sel4_sys;

use sel4_sys::{seL4_CapInitThreadTCB, seL4_TCB_Suspend};

fn main() {

    let _suspend_error = unsafe {
        seL4_TCB_Suspend(seL4_CapInitThreadTCB as usize)
    };

}

```

Furthermore, your library may require the following rustflags set in your `.cargo/config` file
in order to link successfully.
```toml
[build]
rustflags = ["-C", "link-args=-no-pie"]
```

Note that the Rust-available library name is `sel4_sys`

## Library Contents

`sel4_sys` contains the syscalls, constants, API functions, and type definitions
from [libsel4](https://github.com/seL4/seL4/tree/master/libsel4).

The majority of these bindings are generated with [bindgen](https://github.com/rust-lang/rust-bindgen)
from the seL4 kernel source specified in the project's relevant sel4.toml, as managed by
[confignoble](../confignoble/README.md). The exact contents of the `sel4-sys` package
will depend on the configuration flags set in that sel4.toml file, as they affect
the headers in seL4 used as input to the binding generation.

The goal here is to be able to track against changes to seL4 with as little manual
effort as reasonable.

Because these bindings are intended to be zero-overhead, the output is not particularly
Rust-idiomatic.  A notable no-cost ergonomics addition is that
`seL4_Word` and `seL4_CPtr` have been defined to be the same as regular Rust `usize`.

## Building

Default configuration is provided such that a regular `cargo build` will work
even without supplying a specific `SEL4_CONFIG_PATH` environment variable pointing at a sel4.toml file.

Cross-compilation is also possible with [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild) or
[xargo](https://github.com/japaric/xargo). Specify your Rust target triple as an argument explicitly,
and use the environment variable `SEL4_CONFIG_PATH` to point to your sel4.toml configuration file
and the `SEL4_PLATFORM` env-var to select your desired platform target.

### Cross-Compilation Examples

```
SEL4_CONFIG_PATH=/home/other/sel4.toml SEL4_PLATFORM=pc99 cargo xbuild --target=x86_64-unknown-linux-gnu
```

The embedded default configuration file minimally supports the `sabre` and `pc99` platforms

```
SEL4_PLATFORM=sabre cargo xbuild --target armv7-unknown-linux-gnueabihf
```

```
SEL4_PLATFORM=pc99 cargo xbuild --target x86_64-unknown-linux-gnu
```

# Tests

This library contains three kinds of tests:

* Compile time tests that assert and confirm the expected structure of the bindgen output
* Runtime unit tests produced by bindgen itself that check layout details
* Runtime property based tests which ensure that some non-kernel interacting functions
behave reasonably. These target the "bitfield" based structures which are generated behind inside seL4's build system
with a combination of custom parsing and Python-template-based code creation.

Run them all on an `x86_64` type host dev machine with `cargo test`
