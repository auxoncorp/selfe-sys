use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

extern crate confignoble;
const CMAKELISTS_TXT: &str = include_str!("CMakeLists.txt");

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

/// Walk up the directory tree from `start_dir`, looking for "sel4.toml"
fn find_sel4_toml(start_dir: &Path) -> Option<PathBuf> {
    assert!(
        start_dir.is_dir(),
        "{} is not a directory",
        start_dir.display()
    );

    let toml = start_dir.join("sel4.toml");
    if toml.exists() {
        return Some(toml);
    } else {
        match start_dir.parent() {
            Some(d) => find_sel4_toml(d),
            None => None,
        }
    }
}

/// Return the cmake build dir
fn build_sel4(
    out_dir: &Path,
    kernel_dir: &Path,
    tools_dir: &Path,
    config: &confignoble::model::contextualized::Contextualized,
) -> PathBuf {
    let mut opts = BTreeMap::new();

    if let Some(prefix) = &config.build.cross_compiler_prefix {
        opts.insert("CROSS_COMPILER_PREFIX".to_string(), prefix.to_owned());
    }

    opts.insert(
        "CMAKE_TOOLCHAIN_FILE".to_string(),
        kernel_dir.join("gcc.cmake").display().to_string(),
    );
    opts.insert("KERNEL_PATH".to_string(), kernel_dir.display().to_string());

    for (k, v) in config.sel4_config.iter() {
        let v_str = match v {
            confignoble::model::SingleValue::String(s) => s.to_owned(),
            confignoble::model::SingleValue::Integer(i) => format!("{}", i),
            confignoble::model::SingleValue::Boolean(b) => format!("{}", b),
        };

        opts.insert(k.to_owned(), v_str);
    }

    // create the build directory by hashing both input config and the actual
    // cmake options
    let mut hash_state = DefaultHasher::new();
    config.hash(&mut hash_state);
    opts.hash(&mut hash_state);
    CMAKELISTS_TXT.hash(&mut hash_state);
    // TODO hash relevant environment variables as well
    let config_hash = hash_state.finish();

    let build_dir = out_dir
        .join("sel4-build")
        .join(format!("{:x}", config_hash));
    if build_dir.exists() && !build_dir.is_dir() {
        panic!(
            "{} already exists, and is not a directory",
            build_dir.to_str().unwrap()
        );
    }

    fs::create_dir_all(&build_dir).expect("Failed to create build dir");

    fs::write(build_dir.join("CMakeLists.txt"), CMAKELISTS_TXT).unwrap();

    with_working_dir(&build_dir, || {
        let mut cmake = Command::new("cmake");
        cmake
            .args(opts.iter().map(|(k, v)| format!("-D{}={}", k, v)))
            .arg("-G")
            .arg("Ninja")
            .arg(".")
            .env("SEL4_TOOLS_DIR", tools_dir.to_owned())
            // TODO wire this up to the config
            .env("ROOT_TASK_PATH", "/home/mullr/devel/confignoble/example/target/x86_64-unknown-linux-gnu/debug/example")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        println!("Running cmake: {:?}", &cmake);

        let output = cmake.output().expect("failed to run cmake");
        assert!(output.status.success());

        let mut ninja = Command::new("ninja");
        ninja.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        println!("Running ninja: {:?}", &ninja);

        let output = ninja.output().expect("failed to run ninja");
        assert!(output.status.success());
    });

    build_dir
}

fn main() {
    let sel4_platform = "pc99";
    let is_debug = true;
    let target_arch = "x86_64";

    let pwd = &env::current_dir().unwrap();

    let config_file_path = find_sel4_toml(&pwd).unwrap_or_else(|| {
        let cfg = env::var("SEL4_CONFIG_PATH")
            .expect("sel4.toml was not found in the current tree, and SEL4_CONFIG was not set");
        PathBuf::from(&cfg)
    });

    let config_content = fs::read_to_string(&config_file_path).expect(&format!(
        "Can't read config file: {}",
        config_file_path.display()
    ));

    let config = confignoble::model::contextualized::Contextualized::from_str(
        &config_content,
        target_arch.to_owned(),
        is_debug,
        Some(sel4_platform.to_owned()),
    )
    .expect("Can't process config");

    let (kernel_dir, tools_dir) = match &config.sel4_source {
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
        confignoble::model::SeL4Source::Version(_) => unimplemented!(),
    };

    build_sel4(&pwd.join("target"), &kernel_dir, &tools_dir, &config);
}
