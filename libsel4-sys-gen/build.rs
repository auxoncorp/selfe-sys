extern crate bindgen;
use bindgen::Builder;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};

extern crate confignoble;
use confignoble::compilation::{
    build_sel4, resolve_sel4_source, ResolvedSeL4Source, SeL4BuildMode, SeL4BuildOutcome
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

pub struct BuildEnv {
    cargo_cfg_target_arch: String, // something like x86_64 or arm
    cargo_cfg_target_pointer_width: usize,
    out_dir: PathBuf,
    profile: BuildProfile,
    sel4_config_path: Option<PathBuf>,
    sel4_platform: Option<String>,
}

pub enum BuildProfile {
    Debug,
    Release,
}

impl BuildProfile {
    pub fn is_debug(&self) -> bool {
        match self {
            BuildProfile::Debug => true,
            _ => false,
        }
    }
}

impl BuildEnv {
    pub fn request_reruns() {
        for e in [
            "CARGO_CFG_TARGET_ARCH",
            "CARGO_CFG_TARGET_POINTER_WIDTH",
            "CARGO_MANIFEST_DIR",
            "OUT_DIR",
            "PROFILE",
            "SEL4_CONFIG_PATH",
            "SEL4_PLATFORM",
        ]
        .iter()
        {
            println!("cargo:rerun-if-env-changed={}", e);
        }
    }

    pub fn from_env_vars() -> Self {
        /// Get the environment variable `var`, or panic with a helpful message if it's
        /// not set.
        fn get_env(var: &str) -> String {
            env::var(var).expect(&format!("{} must be set", var))
        }
        let raw_profile = get_env("PROFILE");
        BuildEnv {
            cargo_cfg_target_arch: get_env("CARGO_CFG_TARGET_ARCH"),
            cargo_cfg_target_pointer_width: get_env("CARGO_CFG_TARGET_POINTER_WIDTH")
                .parse()
                .expect("Could not parse CARGO_CFG_TARGET_POINTER_WIDTH as an unsigned integer"),
            out_dir: PathBuf::from(get_env("OUT_DIR")),
            profile: match raw_profile.as_str() {
                "debug" => BuildProfile::Debug,
                "release" => BuildProfile::Release,
                _ => panic!("Unexpected value for PROFILE: {}", raw_profile),
            },
            sel4_config_path: env::var("SEL4_CONFIG_PATH").ok().map(PathBuf::from),
            sel4_platform: env::var("SEL4_PLATFORM").ok(),
        }
    }
}

fn main() {
    BuildEnv::request_reruns();
    let BuildEnv {
        cargo_cfg_target_arch,
        cargo_cfg_target_pointer_width,
        out_dir,
        profile,
        sel4_config_path,
        sel4_platform,
    } = BuildEnv::from_env_vars();
    println!("cargo:rerun-if-file-changed=build.rs");
    println!("cargo:rerun-if-file-changed=src/lib.rs");
    println!("cargo:rerun-if-env-changed=RUSTFLAGS");

    let (full_config, config_dir) = sel4_config_path
        .map(|config_file_path| {
            let config_file_path =
                fs::canonicalize(&Path::new(&config_file_path)).expect(&format!(
                    "Config file could not be canonicalized: {}",
                    config_file_path.display()
                ));

            let config_file_dir = config_file_path
                .parent()
                .expect("Can't get parent of config file path");
            println!("cargo:rerun-if-file-changed={}", config_file_path.display());
            let config_content = fs::read_to_string(&config_file_path).expect(&format!(
                "Can't read config file: {}",
                config_file_path.display()
            ));
            (
                confignoble::model::full::Full::from_str(&config_content)
                    .expect("Error processing config file"),
                Some(config_file_dir.to_owned()),
            )
        })
        .unwrap_or_else(|| {
            println!("Using default config content in libsel4-sys-gen");
            (confignoble::model::get_default_config(), None)
        });

    let config = confignoble::model::contextualized::Contextualized::from_full(
        full_config,
        cargo_cfg_target_arch.to_owned(),
        profile.is_debug(),
        sel4_platform,
        config_dir.as_ref().map(|pb| pb.as_path()),
    )
    .expect("Error resolving config file");
    config.print_boolean_feature_flags();

    let sel4_arch = rust_arch_to_sel4_arch(&cargo_cfg_target_arch);
    let arch = rust_arch_to_arch(&cargo_cfg_target_arch);

    let ResolvedSeL4Source {
        kernel_dir,
        tools_dir,
    } = resolve_sel4_source(&config.sel4_source, &out_dir.join("sel4_source"))
        .expect("resolve sel4 source");

    let build_dir = if let SeL4BuildOutcome::StaticLib { build_dir }= build_sel4(
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
