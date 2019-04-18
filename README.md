# under-named seL4 configuration/build libraries and tool

## Usage

Install the build toolchain.

```
cargo install --git ssh://git@github.com/auxoncorp/confignoble.git cotransport --force
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

