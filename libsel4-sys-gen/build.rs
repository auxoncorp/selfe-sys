extern crate bindgen;
use bindgen::Builder;
use semver_parser::version::Version as SemVersion;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::{env, fs};

extern crate confignoble;

/// cd to `dir`, call `f`, then cd back to the previous working directory.
fn with_working_dir<F>(dir: &PathBuf, f: F)
where
    F: Fn() -> (),
{
    let pwd = env::current_dir().expect("Failed to get current dir");
    env::set_current_dir(&dir).expect(format!("Can't cd to {}", dir.display()).as_str());

    f();

    env::set_current_dir(&pwd).expect("Can't cd back to initial working dir");
}

/// Return the cmake build dir
fn build_libsel4(
    out_dir: &Path,
    cargo_manifest_dir: &Path,
    kernel_path: &Path,
    tools_path: &Path,
    config: &confignoble::model::contextualized::Contextualized,
) -> PathBuf {
    let build_dir = out_dir.join("libsel4-build");
    if build_dir.exists() {
        if !build_dir.is_dir() {
            panic!(
                "{} already exists, and is not a directory",
                build_dir.to_str().unwrap()
            );
        } else {
            fs::remove_dir_all(&build_dir).expect("Failed to remove existing build dir");
        }
    }

    fs::create_dir(&build_dir).expect("Failed to create build dir");

    let mut opts = HashMap::new();

    if let Some(prefix) = &config.build.cross_compiler_prefix {
        opts.insert("CROSS_COMPILER_PREFIX".to_string(), prefix.to_owned());
    }

    opts.insert(
        "CMAKE_TOOLCHAIN_FILE".to_string(),
        kernel_path.join("gcc.cmake").display().to_string(),
    );
    opts.insert("KERNEL_PATH".to_string(), kernel_path.display().to_string());
    opts.insert(
        "LibSel4FunctionAttributes".to_string(),
        "public".to_string(),
    );

    for (k, v) in config.sel4_config.iter() {
        let v_str = match v {
            confignoble::model::SingleValue::String(s) => s.to_owned(),
            confignoble::model::SingleValue::Integer(i) => format!("{}", i),
            confignoble::model::SingleValue::Boolean(b) => format!("{}", b),
        };

        opts.insert(k.to_owned(), v_str);
    }

    with_working_dir(&build_dir, || {
        let mut cmake = Command::new("cmake");
        cmake
            .args(opts.iter().map(|(k, v)| format!("-D{}={}", k, v)))
            .arg("-G")
            .arg("Ninja")
            .arg(cargo_manifest_dir)
            .env("SEL4_TOOLS_DIR", tools_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        println!("Running cmake: {:?}", &cmake);

        let output = cmake.output().expect("failed to run cmake");
        assert!(output.status.success());

        let mut ninja = Command::new("ninja");
        ninja
            .arg("libsel4.a")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        println!("Running ninja: {:?}", &ninja);

        let output = ninja.output().expect("failed to run ninja");
        assert!(output.status.success());
    });

    build_dir
}

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
    cargo_manifest_dir: PathBuf,
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
            cargo_manifest_dir: PathBuf::from(get_env("CARGO_MANIFEST_DIR")),
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

/// Finds the relevant sha in the seL4/seL4_tools github repository that is supposedly
/// compatible with the supplied version of seL4.
fn version_to_sel4_tools_sha(version: &SemVersion) -> Option<&'static str> {
    match (version.major, version.minor) {
        (2, 0) => Some("eaf835135acf8bc5ae631b021eddb93816ec873b"),
        (2, 1) => Some("215b802f2addeb99a7e3a733acc3eb2fffd23d2c"),
        (3, 0) => Some("dfd11fe348bd95577686e376abe68c945a0244e0"),
        (3, 1) => Some("c9db51f0396970fd4db224325d73c9ee3d7f0eb3"),
        (3, 2) => Some("9d75312da3338b913d1a7e48497b0fa6d4faa2ed"),
        (4, 0) => Some("d185d574db8332d83da01f3799d9ef53bebba80e"),
        (5, 0) => Some("fe5db52215e551771fe6662591edcacd727de0d9"),
        (5, 1) => Some("a9295b6efd52f0956040e0ff9e6674af867f2b61"),
        (5, 2) => Some("ee37ef2615a2acbd3688e66136f5b3deaadab125"),
        (6, 0) => Some("7223b43dd0151933e64bca509b2a471770ccff05"),
        (7, 0) => Some("695e63c327f979c5e3c9624d9e8d0fa765bf4393"),
        (8, 0) => Some("afe398cd09bc35c5a4d573006ed19284a164ad80"),
        (9, 0) => Some("87642544dbb767806d9c1f0fb673eb5ac86c242b"),
        (10, 0) => Some("9cd9d57ec8783db3d3bcea1821d7e7e1fe84d34e"),
        (10, 1) => Some("e8c8f9e1a3c37508fb1c395884a11d2a569e1ef6"),
        _ => None,
    }
}

fn version_to_sel4_kernel_release_sha(version: &SemVersion) -> Option<&'static str> {
    match (version.major, version.minor, version.patch) {
        (2, 0, 0) => Some("2a4ee9ba912c195f526163dca27d48e06d8a81f8"),
        (2, 1, 0) => Some("0115ad1d0d7a871d637f8ceb79e37b46f9981249"),
        (3, 0, 0) => Some("634d4681a88bdb43fa1e1f8fd8c330f43f6d6712"),
        (3, 0, 1) => Some("94e13b7e605676b4d5bd61497d38f0858cf2bb2f"),
        (3, 1, 0) => Some("8150e914c377bb2d94796c6857d08f64973a7295"),
        (3, 2, 0) => Some("e70cd7613bb3aed71d3df58c72146cc44a60190e"),
        (4, 0, 0) => Some("7d1df6af0027e87a66351d01804b00eb2637b63e"),
        (5, 0, 0) => Some("5453060b9530567d2709d0815c1b114cb1a2be6a"),
        (5, 1, 0) => Some("598c9d1efc2b10c475a92fd5775d8280c766b77f"),
        (5, 2, 0) => Some("3695232f9603af60d56f97072082d90f30e98b0e"),
        (6, 0, 0) => Some("8564ace4dfb622ec69e0f7d762ebfbc8552ec918"),
        (7, 0, 0) => Some("220ed968b1b1569586b26f297f37a8f7e7d7b961"),
        (8, 0, 0) => Some("396315f3bfb2592e2fe61eaaeee916b626f26e68"),
        (9, 0, 0) => Some("f58d22af8b6ce8bfccaa4bac393a31cad670e7c1"),
        (9, 0, 1) => Some("0dd40b6c43a290173ea7782b97afbbbddfa23b36"),
        (10, 0, 0) => Some("5c7f7844a6225acd0865d4c063ddac4c1a518963"),
        (10, 1, 0) => Some("a3c341adc4ee61cdfe68d64245c71ca9b171dd15"),
        (10, 1, 1) => Some("57e5417ce24ad6a37912dda47495f02b8c7eb60f"),
        _ => None,
    }
}

fn clone_at_branch_or_tag(repo: &str, branch_or_tag: &str, dir: &Path) {
    let mut git_clone_command = Command::new("git");
    git_clone_command
        .arg("clone")
        .arg("--depth=1")
        .arg("--single-branch")
        .arg("--branch")
        .arg(branch_or_tag)
        .arg(repo)
        .arg(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    println!("Running git: {:?}", &git_clone_command);
    let output = git_clone_command.output().expect("failed to run git");
    assert!(output.status.success());
}

fn is_dir_absent_or_empty(dir_path: &Path) -> bool {
    if dir_path.exists() {
        if !dir_path.is_dir() {
            panic!(
                "Found pre-existing file at {} where either nothing or an empty dir was expected",
                dir_path.display()
            );
        }
        std::fs::read_dir(dir_path).iter().len() == 0
    } else {
        true
    }
}

fn main() {
    BuildEnv::request_reruns();
    let BuildEnv {
        cargo_cfg_target_arch,
        cargo_cfg_target_pointer_width,
        cargo_manifest_dir,
        out_dir,
        profile,
        sel4_config_path,
        sel4_platform,
    } = BuildEnv::from_env_vars();
    println!("cargo:rerun-if-file-changed=build.rs");
    println!("cargo:rerun-if-file-changed=src/lib.rs");
    println!("cargo:rerun-if-file-changed=CMakeLists.txt");

    let full_config = sel4_config_path
        .map(|config_file_path| {
            let config_file_path =
                fs::canonicalize(&Path::new(&config_file_path)).expect(&format!(
                    "Config file could not be canonicalized: {}",
                    config_file_path.display()
                ));
            println!("cargo:rerun-if-file-changed={}", config_file_path.display());
            let config_content = fs::read_to_string(&config_file_path).expect(&format!(
                "Can't read config file: {}",
                config_file_path.display()
            ));
            confignoble::model::full::Full::from_str(&config_content)
                .expect("Error processing config file")
        })
        .unwrap_or_else(|| {
            println!("Using default config content in libsel4-sys-gen");
            confignoble::model::get_default_config()
        });

    let config = confignoble::model::contextualized::Contextualized::from_full(
        full_config,
        cargo_cfg_target_arch.to_owned(),
        profile.is_debug(),
        sel4_platform,
    )
    .expect("Error resolving config file");

    let sel4_arch = rust_arch_to_sel4_arch(&cargo_cfg_target_arch);
    let arch = rust_arch_to_arch(&cargo_cfg_target_arch);

    let (sel4_path, tools_path) = match &config.sel4_source {
        confignoble::model::SeL4Source::LocalDirectories {
            kernel_dir,
            tools_dir,
        } => (
            fs::canonicalize(&kernel_dir).expect(&format!(
                "Canonicalization failed for local kernel dir: {}",
                &kernel_dir.display()
            )),
            fs::canonicalize(&tools_dir).expect(&format!(
                "Canonicalization failed for local tools dir: {}",
                &tools_dir.display()
            )),
        ),
        confignoble::model::SeL4Source::Version(v) => {
            // Confirm we can support the requested version
            let _kernel_sha = version_to_sel4_kernel_release_sha(&v)
                .expect(&format!("Unsupported version: {}", v));
            let _tools_sha =
                version_to_sel4_tools_sha(&v).expect(&format!("Unsupported version: {}", v));

            let kernel_dir = out_dir.join(format!("seL4_kernel-{}", v));
            let kernel_needs_content = is_dir_absent_or_empty(&kernel_dir);
            fs::create_dir_all(&kernel_dir).expect("Failed to create kernel dir");
            let kernel_dir = fs::canonicalize(&kernel_dir)
                .expect(&format!("Kernel dir: {}", &kernel_dir.display()));
            if kernel_needs_content {
                clone_at_branch_or_tag(
                    "git://github.com/seL4/seL4.git",
                    &format!("{}", v),
                    &kernel_dir,
                )
            }

            let tools_dir = out_dir.join(format!("seL4_tools-{}", v));
            let tools_needs_content = is_dir_absent_or_empty(&tools_dir);
            fs::create_dir_all(&tools_dir).expect("Failed to create tools dir");
            let tools_dir = fs::canonicalize(&tools_dir)
                .expect(&format!("Tools dir: {}", &tools_dir.display()));
            if tools_needs_content {
                clone_at_branch_or_tag(
                    "git://github.com/seL4/seL4_tools.git",
                    &format!("{}.{}.x-compatible", v.major, v.minor),
                    &tools_dir,
                );
            }
            (kernel_dir, tools_dir)
        }
    };

    let build_dir = build_libsel4(
        &out_dir,
        &cargo_manifest_dir,
        &sel4_path,
        &tools_path,
        &config,
    );
    config.print_boolean_feature_flags();

    println!("cargo:rerun-if-env-changed=RUSTFLAGS");
    println!("cargo:rustc-link-lib=static=sel4");
    println!(
        "cargo:rustc-link-search=native={}/libsel4",
        build_dir.display()
    );

    gen_bindings(
        &out_dir,
        &sel4_path,
        &build_dir,
        &sel4_arch,
        &arch,
        cargo_cfg_target_pointer_width,
    );
}
