extern crate bindgen;
use bindgen::Builder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

extern crate confignoble;

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
    _tools_path: &Path,
    config: &confignoble::contextualized::Contextualized,
) -> PathBuf {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not defined");
    let out_dir = Path::new(&out_dir);

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not defined");
    let manifest_dir = Path::new(&manifest_dir);

    let build_dir = out_dir.join("libsel4-build");
    if build_dir.exists() && !build_dir.is_dir() {
        panic!(
            "{} already exists, and is not a directory",
            build_dir.to_str().unwrap()
        );
    }

    if !build_dir.exists() {
        fs::create_dir(&build_dir).expect("Failed to create build dir");
    }

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

    for (k, v) in config.config.iter() {
        match v {
            confignoble::SingleValue::String(s) => {
                opts.insert(k.to_owned(), s.to_owned());
            }
            confignoble::SingleValue::Integer(i) => {
                opts.insert(k.to_owned(), format!("{}", i));
            }
            confignoble::SingleValue::Float(f) => {
                opts.insert(k.to_owned(), format!("{}", f));
            }
            confignoble::SingleValue::Boolean(b) => {
                opts.insert(k.to_owned(), format!("{}", b));
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

fn gen_bindings(
    kernel_path: &Path,
    libsel4_build_path: &Path,
    arch: &str,
    sel4_arch: &str,
    ptr_width: &str,
) {
    let bindings = Builder::default()
        .header("src/bindgen_wrapper.h")
        .use_core()
        .ctypes_prefix("ctypes")
        .blacklist_type("seL4_CPtr")
        .blacklist_type("seL4_Word")
        .blacklist_type("seL4_Int8")
        .blacklist_type("seL4_Int16")
        .blacklist_type("seL4_Int32")
        .blacklist_type("seL4_Int64")
        .blacklist_type("seL4_Uint8")
        .blacklist_type("seL4_Uint16")
        .blacklist_type("seL4_Uint32")
        .blacklist_type("seL4_Uint64")
        .clang_arg(format!(
            "-I{}",
            libsel4_build_path.join("libsel4/include").display()
        ))
        .clang_arg(format!(
            "-I{}",
            libsel4_build_path.join("libsel4/autoconf").display()
        ))
        .clang_arg(format!(
            "-I{}",
            libsel4_build_path.join("kernel/gen_config").display()
        ))
        .clang_arg(format!(
            "-I{}",
            libsel4_build_path.join("libsel4/gen_config").display()
        ))
        .clang_arg(format!(
            "-I{}",
            libsel4_build_path
                .join(format!("libsel4/arch_include/{}", arch))
                .display()
        ))
        .clang_arg(format!(
            "-I{}",
            kernel_path.join("libsel4/include").display()
        ))
        .clang_arg(format!(
            "-I{}",
            kernel_path
                .join(format!("libsel4/arch_include/{}", arch))
                .display()
        ))
        .clang_arg(format!(
            "-I{}",
            kernel_path
                .join(format!("libsel4/sel4_arch_include/{}", sel4_arch))
                .display()
        ))
        .clang_arg(format!(
            "-I{}",
            libsel4_build_path
                .join(format!("libsel4/sel4_arch_include/{}", sel4_arch))
                .display()
        ))
        .clang_arg(format!(
            "-I{}",
            kernel_path
                .join(format!("libsel4/mode_include/{}", ptr_width))
                .display()
        ))
        .generate()
        .expect("bindgen didn't work");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not defined");
    bindings
        .write_to_file(PathBuf::from(out_dir).join("bindings.rs"))
        .expect("couldn't write bindings");
}

// TODO arm_hyp
fn rust_arch_to_sel4_arch_category(arch: &str) -> String {
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

fn rust_arch_to_sel4_arch(arch: &str) -> String {
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
    env::set_var(
        "SEL4_CONFIG_PATH",
        "/home/mullr/devel/confignoble/default_config.toml",
    );

    let config_file_path = env::var("SEL4_CONFIG_PATH").expect("SEL4_CONFIG_PATH must be set");
    let config_file_path = Path::new(&config_file_path);
    let config_content = fs::read_to_string(config_file_path).expect("Can't read config file");

    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH must be set");

    let profile = env::var("PROFILE").expect("PROFILE must be set");
    let debug = match profile.as_str() {
        "debug" => true,
        "release" => false,
        _ => panic!("Unexpected value for PROFILE"),
    };

    let config = confignoble::contextualized::Contextualized::from_str(
        &config_content,
        arch.to_owned(),
        debug,
        None,
    )
    .expect("Error processing config file");

    let sel4_arch_category = rust_arch_to_sel4_arch_category(&arch);
    let sel4_arch = rust_arch_to_sel4_arch(&arch);

    let sel4_path = fs::canonicalize(&Path::new("/home/mullr/devel/auxon-sel4")).unwrap();
    let tools_path = Path::new("/home/mullr/sel4/seL4_tools");

    let target_ptr_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH")
        .expect("CARGO_CFG_TARGET_POINTER_WIDTH must be set");

    let build_dir = build_libsel4(
        &sel4_path, tools_path, // cross_compiler_prefix,
        &config,
    );
    gen_bindings(
        &sel4_path,
        &build_dir,
        &sel4_arch_category,
        &sel4_arch,
        &target_ptr_width,
    );

    // let build_dir = build_libsel4(&sel4_path, tools_path, None);
    // gen_bindings(&sel4_path, &build_dir, "x86", "x86_64", 64);

    // let build_dir = build_libsel4(&sel4_path, tools_path, Some("arm-linux-gnueabi-"));
    // gen_bindings(&sel4_path, &build_dir, "arm", "aarch32", 32);
}
