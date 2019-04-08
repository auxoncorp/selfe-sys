extern crate bindgen;
use bindgen::Builder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

extern crate confignoble;

/// Get the environment variable `var`, or panic with a helpful message if it's
/// not set.
fn get_env(var: &str) -> String {
    env::var(var).expect(&format!("{} must be set", var))
}

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
    kernel_path: &Path,
    tools_path: &Path,
    config: &confignoble::contextualized::Contextualized,
) -> PathBuf {
    let out_dir = get_env("OUT_DIR");
    let out_dir = Path::new(&out_dir);

    let manifest_dir = get_env("CARGO_MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);

    let build_dir = out_dir.join("libsel4-build");
    if build_dir.exists() {
        if !build_dir.is_dir() {
            panic!(
                "{} already exists, and is not a directory",
                build_dir.to_str().unwrap()
            );
        }
        else {
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
            confignoble::SingleValue::String(s) => s.to_owned(),
            confignoble::SingleValue::Integer(i) => format!("{}", i),
            confignoble::SingleValue::Float(f) => format!("{}", f),
            confignoble::SingleValue::Boolean(b) => format!("{}", b),
        };

        opts.insert(k.to_owned(), v_str);
        if let confignoble::SingleValue::Boolean(b) = v {
            if *b {
                println!("cargo:rustc-cfg={}", k);
            }
        }
    }

    with_working_dir(&build_dir, || {
        let mut cmake = Command::new("cmake");
        cmake
            .args(opts.iter().map(|(k, v)| format!("-D{}={}", k, v)))
            .arg("-G")
            .arg("Ninja")
            .arg(manifest_dir)
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

fn expand_include_dir(d: &str, arch: &str, sel4_arch: &str, ptr_width: &str) -> String {
    d.replace("$ARCH$", arch)
        .replace("$SEL4_ARCH$", sel4_arch)
        .replace("$PTR_WIDTH$", ptr_width)
}

fn gen_bindings(
    kernel_path: &Path,
    libsel4_build_path: &Path,
    arch: &str,
    sel4_arch: &str,
    ptr_width: &str,
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

    let out_dir = get_env("OUT_DIR");
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
    // TODO load a real default config
    match env::var("SEL4_CONFIG_PATH") {
        Err(_) => {
            env::set_var(
                "SEL4_CONFIG_PATH",
                "/home/mullr/devel/confignoble/default_config.toml",
            );
        }
        _ => (),
    }

    let config_file_path = get_env("SEL4_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=SEL4_CONFIG_PATH");

    let config_file_path = fs::canonicalize(&Path::new(&config_file_path))
        .expect(&format!("Config file: {}", config_file_path));
    println!("cargo:rerun-if-file-changed={}", config_file_path.display());

    let config_content = fs::read_to_string(&config_file_path).expect(&format!(
        "Can't read config file: {}",
        config_file_path.display()
    ));

    let rust_arch = get_env("CARGO_CFG_TARGET_ARCH");

    let profile = get_env("PROFILE");
    let debug = match profile.as_str() {
        "debug" => true,
        "release" => false,
        _ => panic!("Unexpected value for PROFILE: {}", profile),
    };

    let platform = env::var("SEL4_PLATFORM").ok();
    println!("cargo:rerun-if-env-changed=SEL4_PLATFORM");

    let config = confignoble::contextualized::Contextualized::from_str(
        &config_content,
        rust_arch.to_owned(),
        debug,
        platform,
    )
    .expect("Error processing config file");

    let sel4_arch = rust_arch_to_sel4_arch(&rust_arch);
    let arch = rust_arch_to_arch(&rust_arch);

    let sel4_path = fs::canonicalize(&config.kernel_dir)
        .expect(&format!("Kernel dir: {}", config.kernel_dir.display()));
    let tools_path = fs::canonicalize(&config.tools_dir)
        .expect(&format!("Tools dir: {}", config.tools_dir.display()));

    let target_ptr_width = get_env("CARGO_CFG_TARGET_POINTER_WIDTH");

    let build_dir = build_libsel4(&sel4_path, &tools_path, &config);
    gen_bindings(&sel4_path, &build_dir, &sel4_arch, &arch, &target_ptr_width);
}
