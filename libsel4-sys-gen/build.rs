extern crate bindgen;
use bindgen::Builder;
use std::path::{Path, PathBuf};

extern crate confignoble;
use confignoble::build_helpers::*;
use confignoble::compilation::{
    build_sel4, resolve_sel4_source, ResolvedSeL4Source, SeL4BuildMode, SeL4BuildOutcome,
};

const BLACKLIST_TYPES: &'static [&'static str] = &[
    "seL4_CPtr",
    "seL4_Word",
    "seL4_Int8",
    "seL4_Int16",
    "seL4_Int32",
    "seL4_Int64",
    "seL4_Uint8",
    "seL4_Uint16",
    "seL4_Uint32",
    "seL4_Uint64",
];

const BUILD_INCLUDE_DIRS: &'static [&'static str] = &[
    "libsel4/include",
    "libsel4/autoconf",
    "kernel/gen_config",
    "libsel4/gen_config",
    "libsel4/arch_include/$ARCH$",
    "libsel4/sel4_arch_include/$SEL4_ARCH$",
];

const KERNEL_INCLUDE_DIRS: &'static [&'static str] = &[
    "libsel4/include",
    "libsel4/arch_include/$ARCH$",
    "libsel4/sel4_arch_include/$SEL4_ARCH$",
    "libsel4/mode_include/$PTR_WIDTH$",
];

fn expand_include_dir(d: &str, arch: &str, sel4_arch: &str, ptr_width: usize) -> String {
    d.replace("$ARCH$", arch)
        .replace("$SEL4_ARCH$", sel4_arch)
        .replace("$PTR_WIDTH$", &format!("{}", ptr_width))
}

fn gen_bindings(
    out_dir: &Path,
    kernel_path: &Path,
    libsel4_build_path: &Path,
    arch: &str,
    sel4_arch: &str,
    ptr_width: usize,
) {
    println!("cargo:rerun-if-file-changed=src/bindgen_wrapper.h");

    let mut bindings = Builder::default()
        .header("src/bindgen_wrapper.h")
        .use_core()
        .ctypes_prefix("ctypes");

    for t in BLACKLIST_TYPES {
        bindings = bindings.blacklist_type(t);
    }

    for d in BUILD_INCLUDE_DIRS {
        bindings = bindings.clang_arg(format!(
            "-I{}",
            libsel4_build_path
                .join(expand_include_dir(d, arch, sel4_arch, ptr_width))
                .display()
        ));
    }

    for d in KERNEL_INCLUDE_DIRS {
        bindings = bindings.clang_arg(format!(
            "-I{}",
            kernel_path
                .join(expand_include_dir(d, arch, sel4_arch, ptr_width))
                .display()
        ));
    }

    let bindings = bindings.generate().expect("bindgen didn't work");

    bindings
        .write_to_file(PathBuf::from(out_dir).join("bindings.rs"))
        .expect("couldn't write bindings");
}

// TODO arm_hyp
fn rust_arch_to_sel4_arch(arch: &str) -> String {
    match arch {
        "arm" => "arm".to_owned(),
        "armv7" => "arm".to_owned(),
        "aarch32" => "arm".to_owned(),
        "aarch64" => "arm".to_owned(),
        "i386" => "x86".to_owned(),
        "i586" => "x86".to_owned(),
        "i686" => "x86".to_owned(),
        "x86_64" => "x86".to_owned(),
        _ => panic!("Unknown arch"),
    }
}

fn rust_arch_to_arch(arch: &str) -> String {
    match arch {
        "arm" => "aarch32".to_owned(),
        "armv7" => "aarch32".to_owned(),
        "aarch32" => "aarch32".to_owned(),
        "aarch64" => "aarch64".to_owned(),
        "i386" => "ia32".to_owned(),
        "i586" => "ia32".to_owned(),
        "i686" => "ia32".to_owned(),
        "x86_64" => "x86_64".to_owned(),
        _ => panic!("Unknown arch"),
    }
}

fn main() {
    BuildEnv::request_reruns();
    let BuildEnv {
        cargo_cfg_target_arch,
        cargo_cfg_target_pointer_width,
        out_dir,
        ..
    } = BuildEnv::from_env_vars();
    println!("cargo:rerun-if-file-changed=build.rs");
    println!("cargo:rerun-if-file-changed=src/lib.rs");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");

    let config = load_config_from_env_or_default();
    config.print_boolean_feature_flags();
    let sel4_arch = rust_arch_to_sel4_arch(&cargo_cfg_target_arch);
    let arch = rust_arch_to_arch(&cargo_cfg_target_arch);

    let ResolvedSeL4Source {
        kernel_dir,
        tools_dir,
    } = resolve_sel4_source(&config.sel4_source, &out_dir.join("sel4_source"))
        .expect("resolve sel4 source");

    let build_dir = if let SeL4BuildOutcome::StaticLib { build_dir } = build_sel4(
        &out_dir,
        &kernel_dir,
        &tools_dir,
        &config,
        SeL4BuildMode::Lib,
    ) {
        build_dir
    } else {
        panic!("build_sel4 built us something other than a static library");
    };

    println!("cargo:rustc-link-lib=static=sel4");
    println!(
        "cargo:rustc-link-search=native={}/libsel4",
        build_dir.display()
    );

    gen_bindings(
        &out_dir,
        &kernel_dir,
        &build_dir,
        &sel4_arch,
        &arch,
        cargo_cfg_target_pointer_width,
    );
}
