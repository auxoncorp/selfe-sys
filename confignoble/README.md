# confignoble

A seL4 configuration format, managed by a library.

## Usage

Direct use of this library is largely not necessary.  End users
will usually just deal with the [toml format](#Toml-Format)
when they need to tweak the version or compilation-options of
the seL4 they wish to build against when using [libsel4-sys-gen](../libsel4-sys-gen/README.md)

## Library Contents

### model module

In addition to handling deserialization, serialization, and in-memory representation of
a `full::Full` configuration model, the `model` module provides a `contextualized::Contextualized`
type that narrows the configuration options to those applicable to a specific `contextualized::Context`.

## compilation module

The `compilation` module provides the `build_sel4` function for compiling either the `seL4` client library
or a `seL4` kernel (and optionally-distinct root task artifact)

## build_helpers module

`build_helpers` provides utilities for use in the `build.rs` files of libraries or applications
that want to standardize on a shared configuration. End users may consider using this module
to apply their sel4.toml configuration as Rust compile-time feature flags to improve portability.

In build.rs:
```
use confignoble::build_helpers::*;

fn main() {
    /// Rerun this build script if any of the config-driving environment variables change
    BuildEnv::request_reruns();

    /// Like it says on the tin, paying particular attention to the SEL4_CONFIG_PATH env-var
    let config = load_config_from_env_or_default();

    /// Tells cargo to build the current library with feature-flags set
    /// based on the content of the selected-or-default sel4.toml configuration
    config.print_boolean_feature_flags();
}
```

## Toml Format

See [default_config.toml](src/default_config.toml) for a minimal example of the format materialized as toml,
or consider the following commented walkthrough.

```toml
# Location of the required source repositories: seL4 (kernel), seL4_tools (tools), and util_libs
# Any of these three may be specified using the `git` or `path` approach.
[sel4]
kernel = { git = "https://github.com/seL4/seL4" , tag = "10.1.1" }
tools = { git = "https://github.com/seL4/seL4_tools" , branch = "10.1.x-compatible" }
util_libs  = { path = "../misc/util_libs" }
util_libs  = { path = "../misc/util_libs" }

# seL4 kernel and library configuration properties go in [sel4.config.*] tables.
# These properties are ultimately passed to seL4's CMake build system.
# Such tables must only contain string, integer, or boolean properties.
#
# This table corresponds to seL4's notion of an 'arch', matching `model::Arch`
# for the in-memory representation. "x86" would be another reasonable table name.
[sel4.config.arm]
KernelArch = 'arm'

# This table corresponds to seL4's notion of a 'sel4_arch', matching `model::SeL4Arch`
# for the in-memory representation. "aarch64" or "x86_64" would be other comparable names.
[sel4.config.aarch32]
KernelSel4Arch = 'aarch32'
KernelArmSel4Arch = 'aarch32'


# This table corresponds to seL4's notion of a 'platform', matching `model::Platform`
# for the in-memory representation. Platform names are largely unrestricted.
[sel4.config.sabre]
KernelARMPlatform = 'imx6'
KernelHaveFPU = true

# The toml may contain tables for multiple arch/sel4_arch/platform options.
# The precise set of properties is settled through explicit contextualization.
[sel4.config.some-other-platform]
KernelARMPlatform = 'whatever'

# The [sel4.config.debug] and [sel4.config.release] tables correspond to the
# project's expected compilation profile
[sel4.config.debug]
KernelPrinting = true
KernelDebugBuild = true

[sel4.config.release]
KernelPrinting = false
KernelDebugBuild = false

# The [build.*] tables have names corresponding to seL4 platforms,
# and contain an optional `cross_compiler_prefix`, used when
# building libsel4 or seL4 kernels / root tasks.
[build.sabre]
cross_compiler_prefix = "arm-linux-gnueabihf-"

# For application/root task builds, please also supply the command
# necessary to create the project's root task, and the
# expected output location of that task, scoped to the
# platform and build profile  like [build.PLATFORM.debug]
# and [build.PLATFORM.release]
#
# These properties are used by the cotransport build tool in particular,
# and are not relevant for libraries.
[build.sabre.debug]
make_root_task = "cargo xbuild --target=armv7-unknown-linux-gnueabihf"
root_task_image = "target/armv7-unknown-linux-gnueabihf/debug/example"

# Note that the `make_root_task` property is technically optional
# if the creation of the root task image is managed at a different
# level of the toolchain.
[build.sabre.release]
make_root_task = "cargo xbuild --target=armv7-unknown-linux-gnueabihf --release"
root_task_image = "target/armv7-unknown-linux-gnueabihf/release/example"
```

## binary tool

A seL4 application build and simulation tool.

This tool's job is to orchestrate the construction of seL4 applications.

It uses a sel4.toml file sitting in a project's root dir to establish a canonical configuration
source and pipes that configuration, along with explicit output platform expectations
down through the application's build steps.
