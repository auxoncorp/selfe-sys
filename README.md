# under-named seL4 configuration/build libraries and tool

## Overview
* [confignoble](confignoble) is a library that defines a seL4 configuration format (sel4.toml) and utilities for building seL4 with that config
  * Also includes a binary tool for building seL4 applications with the help of a sel4.toml config file
* [libsel4-sys-gen](libsel4-sys-gen) uses `confignoble` to build libsel4 and provide generated bindings atop it.
* [sel4-start](sel4-start) is a library that defines Rust lang-items required for `no_std` Rust applications running on seL4.
* [example](example) is a Rust seL4 application that depends on sel4-start for its root-task setup and libsel4-sys-gen for access to syscalls. It can be built/run independently or with `cotransport`.

See the READMEs of the subdirectories for more detailed explanations.

## Usage

Install the build toolchain.

```
cargo install --git ssh://git@github.com/auxoncorp/confignoble.git confignoble --bin cotransport --features bin --force
cargo install cargo-xbuild
```

Note that Python, CMake, Ninja, QEMU, and others are lurking as indirect dependencies.


## Example project

Here's how to build the example application.
```
cd example
cotransport build --sel4_arch x86_64 --platform pc99
cotransport build --sel4_arch arm --platform sabre
```

Run the example application in QEMU:

```
cotransport simulate --sel4_arch arm --platform sabre
```

