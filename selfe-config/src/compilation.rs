use crate::model::{self, Arch};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const CMAKELISTS_KERNEL: &str = include_str!("CMakeLists_kernel.txt");
const CMAKELISTS_LIB: &str = include_str!("CMakeLists_lib.txt");

fn clone_at_rev(repo: &str, rev: &str, dir: &Path) -> Result<(), String> {
    let mut git_clone_command = Command::new("git");
    git_clone_command
        .arg("clone")
        .arg(repo)
        .arg(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    println!("Running git: {:?}", &git_clone_command);
    let clone_output = git_clone_command
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;
    if !clone_output.status.success() {
        return Err("git clone command did not report success".to_string());
    }
    let mut git_reset_command = Command::new("git");
    git_reset_command
        .arg("reset")
        .arg("--hard")
        .arg(rev)
        .current_dir(dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    println!("Running git: {:?}", &git_reset_command);
    let reset_output = git_reset_command
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;
    if !reset_output.status.success() {
        return Err("git reset command did not report success".to_string());
    }
    Ok(())
}

fn clone_at_branch_or_tag(repo: &str, branch_or_tag: &str, dir: &Path) -> Result<(), String> {
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
    let output = git_clone_command
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err("git clone command did not report success".to_string())
    }
}

fn is_dir_absent_or_empty(dir_path: &Path) -> bool {
    if dir_path.exists() {
        if !dir_path.is_dir() {
            panic!(
                "Found pre-existing file at {} where either nothing or an empty dir was expected",
                dir_path.display()
            );
        }
        std::fs::read_dir(dir_path)
            .map_err(|e| format!("Could not read directory {} : {:?}", dir_path.display(), e))
            .unwrap()
            .count()
            == 0
    } else {
        true
    }
}

pub struct ResolvedSeL4Source {
    pub kernel_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub util_libs_dir: PathBuf,
}

/// dest_dir: Where downloaded source will be placed, if necessary
pub fn resolve_sel4_sources(
    source: &model::SeL4Sources,
    dest_dir: &Path,
    is_verbose: bool,
) -> Result<ResolvedSeL4Source, String> {
    fn resolve_repo_source(
        source: &model::RepoSource,
        name_hint: &str,
        dest_dir: &Path,
        is_verbose: bool,
    ) -> Result<PathBuf, String> {
        use model::{GitTarget, RepoSource};
        match source {
            RepoSource::LocalPath(p) => Ok(p.clone()),
            RepoSource::RemoteGit { url, target } => {
                let target_kind = target.kind();
                let name_suffix = format!("{}-{}-{}", name_hint, target_kind, target.value());
                let dir = dest_dir.join(name_suffix);
                let dir_needs_content = is_dir_absent_or_empty(&dir);
                if is_verbose {
                    println!(
                        "Git based source directory {:?} {} need fresh content",
                        dir,
                        if dir_needs_content { "DID" } else { " did not" }
                    );
                }
                fs::create_dir_all(&dir).expect("Failed to create dir");
                let dir = fs::canonicalize(&dir).unwrap_or_else(|_| {
                    panic!(
                        "Failed to canonicalize {} dir: {}",
                        name_hint,
                        &dir.display()
                    )
                });

                if dir_needs_content {
                    match target {
                        GitTarget::Branch(v) | GitTarget::Tag(v) => {
                            //"git://github.com/seL4/seL4_tools.git",
                            clone_at_branch_or_tag(url, v, &dir)?;
                        }
                        GitTarget::Rev(rev) => {
                            clone_at_rev(url, rev, &dir)?;
                        }
                    };
                }
                Ok(dir)
            }
        }
    }
    Ok(ResolvedSeL4Source {
        kernel_dir: resolve_repo_source(&source.kernel, "kernel", dest_dir, is_verbose)?,
        tools_dir: resolve_repo_source(&source.tools, "seL4_tools", dest_dir, is_verbose)?,
        util_libs_dir: resolve_repo_source(&source.util_libs, "util_libs", dest_dir, is_verbose)?,
    })
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
        root_image_path: Option<PathBuf>,
    },
}

/// Return the cmake build dir
pub fn build_sel4(
    out_dir: &Path,
    kernel_dir: &Path,
    tools_dir: &Path,
    util_libs_dir: &Path,
    config: &model::contextualized::Contextualized,
    build_mode: SeL4BuildMode,
) -> SeL4BuildOutcome {
    if let Some(ref build_dir) = config.build_dir {
	match build_mode {
	    SeL4BuildMode::Lib => {
		return SeL4BuildOutcome::StaticLib {
		    build_dir: build_dir.to_path_buf()
		}
	    },
	    SeL4BuildMode::Kernel => {
		panic!("Kernel build not supported when build_dir is provided");
	    }
	}
    }

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
        let rti = PathBuf::from(
            &config
                .build
                .root_task
                .as_ref()
                .expect("A build profile's  `root_task_image` is required for a kernel build")
                .image_path,
        );

        let util_libs_bin_dir = build_dir.join("util_libs");
        fs::create_dir_all(&build_dir).expect("Failed to create util libs build dir");
        println!("ROOT_TASK_PATH={}", rti.display());
        cmake
            .env("ROOT_TASK_PATH", rti)
            .env("UTIL_LIBS_SOURCE_PATH", util_libs_dir)
            .env("UTIL_LIBS_BIN_PATH", &util_libs_bin_dir);
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
    // TODO - should we enforce that this value matches the resolved config platform name?
    if cmake_opts.get("KernelPlatform").is_some() {
        panic!("Explicitly supplying a KernelPlatform property interferes with the inner workings of the seL4 cmake build")
    }
    let kernel_platform = cmake_opts.get("KernelX86Platform").unwrap_or_else(|| {
        cmake_opts.get("KernelARMPlatform").expect(
            "KernelARMPlatform or KernelX86Platform missing but required as a sel4 config option",
        )
    });
    match build_mode {
        SeL4BuildMode::Kernel => match config.context.arch {
            Arch::X86 => SeL4BuildOutcome::Kernel {
                build_dir: build_dir.clone(),
                kernel_path: build_dir
                    .join("images")
                    .join(format!("kernel-{}-{}", sel4_arch, kernel_platform)),
                root_image_path: Some(
                    build_dir
                        .join("images")
                        .join(format!("root_task-image-{}-{}", sel4_arch, kernel_platform)),
                ),
            },
            Arch::Arm => SeL4BuildOutcome::Kernel {
                build_dir: build_dir.clone(),
                kernel_path: build_dir
                    .join("images")
                    .join(format!("root_task-image-arm-{}", kernel_platform)),
                root_image_path: None,
            },
            _ => panic!("Unsupported target"),
        },
        SeL4BuildMode::Lib => SeL4BuildOutcome::StaticLib { build_dir },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    #[test]
    fn is_dir_absent_or_empty_when_absent() {
        assert!(is_dir_absent_or_empty(Path::new(
            "/314159/2653789/totally_not_a_real_path"
        )));
    }
    #[test]
    fn is_dir_empty_negative_when_empty() {
        let t = tempdir().expect("Could not make a temp dir");
        assert!(is_dir_absent_or_empty(t.path()));
    }
    #[test]
    fn is_dir_empty_negative_when_full() {
        let t = tempdir().expect("Could not make a temp dir");
        let file_path = t.path().join("my-temporary-note.txt");
        let mut file = File::create(file_path).expect("Could not create file in temp dir");
        writeln!(file, "A tiny bit of content").expect("Could not write content to dummy file");
        file.flush().expect("Could not flush to file");
        assert!(!is_dir_absent_or_empty(t.path()));
    }
}
