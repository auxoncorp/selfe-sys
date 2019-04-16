//! Functions that can be called from build.rs, for when libraries need access
//! to the sel4 configuration

use crate::model;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};

pub struct BuildEnv {
    pub cargo_cfg_target_arch: String, // something like x86_64 or arm
    pub cargo_cfg_target_pointer_width: usize,
    pub out_dir: PathBuf,
    pub profile: BuildProfile,
    pub sel4_config_path: Option<PathBuf>,
    pub sel4_platform: String,
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
            sel4_platform: env::var("SEL4_PLATFORM").unwrap_or_else(|_| {
                let host = env::var("HOST").expect("Could not get HOST env-var as fallback to determine a default host platform");
                if let Some(arch_bit) = host.split("-").next() {
                    match arch_bit.to_lowercase().as_ref() {
                        "arm" | "armv7" | "aarch32" | "aarch64" => "sabre".to_owned(),
                        "x86" | "x86_64" | "ia32"=> "pc99".to_owned(),
                        _ => panic!("No SEL4_PLATFORM was set and could not determine a fallback platform from the HOST triple, {}", host),
                    }
                } else {
                    panic!("HOST env-var was expected to be a target triple, but instead contained: {}", host);
                }
            }),
        }
    }
}

/// This should be run from a build.rs
pub fn load_config_from_env_or_default() -> model::contextualized::Contextualized {
    let BuildEnv {
        cargo_cfg_target_arch,
        profile,
        sel4_config_path,
        sel4_platform,
        ..
    } = BuildEnv::from_env_vars();

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
                model::full::Full::from_str(&config_content).expect("Error processing config file"),
                Some(config_file_dir.to_owned()),
            )
        })
        .unwrap_or_else(|| {
            println!("Using default config content in libsel4-sys-gen");
            (model::get_default_config(), None)
        });

    model::contextualized::Contextualized::from_full(
        full_config,
        &cargo_cfg_target_arch,
        profile.is_debug(),
        &sel4_platform,
        config_dir.as_ref().map(|pb| pb.as_path()),
    )
    .expect("Error resolving config file")
}
