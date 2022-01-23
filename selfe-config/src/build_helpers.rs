//! Functions that can be called from build.rs, for when libraries need access
//! to the sel4 configuration

use crate::model::{self, Arch, Platform, RustArch, SeL4Arch};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};

pub struct BuildEnv {
    pub cargo_cfg_target_arch: String,
    pub cargo_cfg_target_pointer_width: usize,
    pub out_dir: PathBuf,
    pub profile: BuildProfile,
    pub sel4_config_path: Option<PathBuf>,
    pub sel4_override_arch: Option<String>,
    pub sel4_override_sel4_arch: Option<String>,
    pub sel4_platform: Option<String>,
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
            "SEL4_OVERRIDE_SEL4_ARCH",
            "SEL4_OVERRIDE_ARCH",
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
            env::var(var).unwrap_or_else(|_| panic!("{} must be set", var))
        }
        let raw_profile = get_env("PROFILE");
        let cargo_cfg_target_arch = get_env("CARGO_CFG_TARGET_ARCH");

        BuildEnv {
            cargo_cfg_target_arch,
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
            sel4_override_arch: env::var("SEL4_OVERRIDE_ARCH").ok(),
            sel4_override_sel4_arch: env::var("SEL4_OVERRIDE_SEL4_ARCH").ok(),
            sel4_platform: env::var("SEL4_PLATFORM").ok(),
        }
    }
}

/// This should be run from a build.rs
pub fn load_config_from_env_or_default() -> model::contextualized::Contextualized {
    let BuildEnv {
        cargo_cfg_target_arch,
        profile,
        sel4_config_path,
        sel4_override_arch,
        sel4_override_sel4_arch,
        sel4_platform,
        ..
    } = BuildEnv::from_env_vars();

    let (full_config, config_dir) = sel4_config_path
        .map(|config_file_path| {
            let config_file_path =
                fs::canonicalize(&Path::new(&config_file_path)).unwrap_or_else(|_| {
                    panic!(
                        "Config file could not be canonicalized: {}",
                        config_file_path.display()
                    )
                });

            let config_file_dir = config_file_path
                .parent()
                .expect("Can't get parent of config file path");
            println!("cargo:rerun-if-changed={}", config_file_path.display());
            let config_content = fs::read_to_string(&config_file_path).unwrap_or_else(|_| {
                panic!("Can't read config file: {}", config_file_path.display())
            });
            (
                model::full::Full::from_str(&config_content).expect("Error processing config file"),
                Some(config_file_dir.to_owned()),
            )
        })
        .unwrap_or_else(|| {
            println!("Using default config content");
            (model::get_default_config(), None)
        });

    let rust_arch = RustArch::from_str(&cargo_cfg_target_arch);

    let sel4_arch = match sel4_override_sel4_arch {
        Some(s) => SeL4Arch::from_str(&s)
            .expect("Can't parse SEL4_OVERRIDE_SEL4_ARCH as a known sel4_arch value"),
        None => SeL4Arch::from_rust_arch(rust_arch.unwrap())
            .expect("Can't find a sel4_arch for the current cargo target"),
    };

    let arch = match sel4_override_arch {
        Some(s) => {
            Arch::from_str(&s).expect("Can't parse SEL4_OVERRIDE_ARCH as a known arch value")
        }
        None => Arch::from_sel4_arch(sel4_arch),
    };

    let platform = Platform(sel4_platform.unwrap_or_else(|| {
        let auto_val = match arch {
            Arch::Arm => "sabre".to_owned(),
            Arch::X86 => "pc99".to_owned(),
            Arch::Riscv => panic!("Can't choose a default platform for riscv"),
        };
        println!(
            "cargo:warning=Using auto-detected value for SEL4_PLATFORM: '{}'",
            auto_val
        );
        auto_val
    }));

    model::contextualized::Contextualized::from_full(
        &full_config,
        arch,
        sel4_arch,
        profile.is_debug(),
        platform,
        config_dir.as_deref(),
    )
    .expect("Error resolving config file")
}

impl model::contextualized::Contextualized {
    pub fn print_boolean_feature_flags(&self) {
        for (k, v) in self.sel4_config.iter() {
            if let model::SingleValue::Boolean(true) = v {
                println!("cargo:rustc-cfg={}", k)
            };
        }
    }
}
