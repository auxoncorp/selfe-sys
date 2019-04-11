use crate::model;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use semver_parser::version::Version as SemVersion;

const CMAKELISTS_KERNEL: &str = include_str!("CMakeLists_kernel.txt");
const CMAKELISTS_LIB: &str = include_str!("CMakeLists_lib.txt");

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

pub struct ResolvedSeL4Source {
    pub kernel_dir: PathBuf,
    pub tools_dir: PathBuf,
}

/// dest_dir: Where downloaded source will be placed, if necessary
pub fn resolve_sel4_source(
    source: &model::SeL4Source,
    dest_dir: &Path,
) -> Result<ResolvedSeL4Source, String> {
    match &source {
        model::SeL4Source::LocalDirectories {
            kernel_dir,
            tools_dir,
        } => Ok(ResolvedSeL4Source {
            kernel_dir: kernel_dir.to_owned(),
            tools_dir: tools_dir.to_owned(),
        }),
        model::SeL4Source::Version(v) => {
            // Confirm we can support the requested version
            let _kernel_sha = version_to_sel4_kernel_release_sha(&v)
                .expect(&format!("Unsupported version: {}", v));
            let _tools_sha =
                version_to_sel4_tools_sha(&v).expect(&format!("Unsupported version: {}", v));

            let kernel_dir = dest_dir.join(format!("seL4_kernel-{}", v));
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

            let tools_dir = dest_dir.join(format!("seL4_tools-{}", v));
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

            Ok(ResolvedSeL4Source {
                kernel_dir,
                tools_dir,
            })
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum SeL4BuildMode {
    Kernel,
    Lib,
}

pub enum SeL4BuildOutcome {
    StaticLib {
        build_dir: PathBuf,
    },
    Kernel {
        build_dir: PathBuf,
        kernel_path: PathBuf,
    },
    KernelAndRootImage {
        build_dir: PathBuf,
        kernel_path: PathBuf,
        root_image_path: PathBuf,
    },
}

/// Return the cmake build dir
pub fn build_sel4(
    out_dir: &Path,
    kernel_dir: &Path,
    tools_dir: &Path,
    config: &model::contextualized::Contextualized,
    build_mode: SeL4BuildMode,
) -> SeL4BuildOutcome {
    let cmake_lists_content = match build_mode {
        SeL4BuildMode::Kernel => CMAKELISTS_KERNEL,
        SeL4BuildMode::Lib => CMAKELISTS_LIB,
    };

    let mut cmake_opts = BTreeMap::new();
    if let Some(prefix) = &config.build.cross_compiler_prefix {
        cmake_opts.insert("CROSS_COMPILER_PREFIX".to_string(), prefix.to_owned());
    }

    cmake_opts.insert(
        "CMAKE_TOOLCHAIN_FILE".to_string(),
        kernel_dir.join("gcc.cmake").display().to_string(),
    );
    cmake_opts.insert("KERNEL_PATH".to_string(), kernel_dir.display().to_string());

    if build_mode == SeL4BuildMode::Lib {
        cmake_opts.insert(
            "LibSel4FunctionAttributes".to_string(),
            "public".to_string(),
        );
    }

    for (k, v) in config.sel4_config.iter() {
        let v_str = match v {
            model::SingleValue::String(s) => s.to_owned(),
            model::SingleValue::Integer(i) => format!("{}", i),
            model::SingleValue::Boolean(b) => format!("{}", b),
        };

        cmake_opts.insert(k.to_owned(), v_str);
    }

    // create the build directory by hashing both input config and the actual
    // cmake options
    let mut hash_state = DefaultHasher::new();
    config.hash(&mut hash_state);
    cmake_opts.hash(&mut hash_state);
    cmake_lists_content.hash(&mut hash_state);
    // TODO hash relevant environment variables as well. Or tightly manage the target env.
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

    println!("Using build_dir={}", build_dir.display());
    fs::create_dir_all(&build_dir).expect("Failed to create build dir");
    fs::write(build_dir.join("CMakeLists.txt"), cmake_lists_content).unwrap();

    // Run CMake
    let mut cmake = Command::new("cmake");
    cmake
        .args(cmake_opts.iter().map(|(k, v)| format!("-D{}={}", k, v)))
        .arg("-G")
        .arg("Ninja")
        .arg(".")
        .current_dir(&build_dir)
        .env("SEL4_TOOLS_DIR", tools_dir.to_owned());

    if build_mode == SeL4BuildMode::Kernel {
        let rti = &config
            .build
            .root_task
            .as_ref()
            .expect("A build profile's  `root_task_image` is required for a kernel build")
            .image_path;
        cmake.env("ROOT_TASK_PATH", PathBuf::from(rti));
    }

    cmake.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    println!("Running cmake: {:?}", &cmake);

    let output = cmake.output().expect("failed to run cmake");
    assert!(output.status.success());

    // Run ninja
    let mut ninja = Command::new("ninja");
    ninja
        .arg(match build_mode {
            SeL4BuildMode::Kernel => "all",
            SeL4BuildMode::Lib => "libsel4.a",
        })
        .current_dir(&build_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    println!("Running ninja: {:?}", &ninja);

    let output = ninja.output().expect("failed to run ninja");
    assert!(output.status.success());

    let sel4_arch = cmake_opts
        .get("KernelSel4Arch")
        .expect("KernelSel4Arch missing but required as a sel4 config option");
    let kernel_platform = cmake_opts
        .get("KernelPlatform")
        .expect("KernelPlatform missing but required as a sel4 config option");
    match build_mode {
        SeL4BuildMode::Kernel => match config.context.target.as_ref() {
            "x86_64" | "x86" => SeL4BuildOutcome::KernelAndRootImage {
                build_dir: build_dir.clone(),
                kernel_path: build_dir
                    .join("images")
                    .join(format!("kernel-{}-{}", sel4_arch, kernel_platform)),
                root_image_path: build_dir
                    .join("images")
                    .join(format!("root_task-image-{}-{}", sel4_arch, kernel_platform)),
            },
            "arm" | "aarch32" | "arm32" | "aarch64" => SeL4BuildOutcome::Kernel {
                build_dir: build_dir.clone(),
                kernel_path: build_dir
                    .join("images")
                    .join(format!("root_task-image-arm-{}", kernel_platform)),
            },
            _ => panic!("Unsupported target"),
        },
        SeL4BuildMode::Lib => SeL4BuildOutcome::StaticLib { build_dir },
    }
}
