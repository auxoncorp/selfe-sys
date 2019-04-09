use std::path::{Path, PathBuf};
use std::{env, fs};

extern crate confignoble;
use confignoble::compilation::{build_sel4, SeL4BuildMode, resolve_sel4_source, ResolvedSeL4Source};

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

    let out_dir = pwd.join("target").join("sel4");

    let ResolvedSeL4Source {
        kernel_dir,
        tools_dir,
    } = resolve_sel4_source(&config.sel4_source, &out_dir.join("source"))
        .expect("resolve sel4 source");

    build_sel4(&out_dir.join("build"), &kernel_dir, &tools_dir, &config, SeL4BuildMode::Kernel);
}
