//! helper fns for build script integration

use crate::pack;
use std::{env, path};

/// Add the given files to a selfe archive, turn it into a binary, and instruct
/// cargo to include it when linking.
pub fn link_with_archive<'a, 'b, I>(named_files: I)
where
    I: IntoIterator<Item = (&'a str, &'b path::Path)>,
{
    let mut archive = pack::Archive::new();
    for (name, path) in named_files {
        archive.add_file(name, &path).unwrap_or_else(|_| panic!("Error adding file {} from path {} to archive",
            name,
            path.display()));
    }

    // rustc links with gcc; we need ld proper
    let ld = env::var("RUSTC_LINKER")
        .unwrap_or("gcc".to_string())
        .replace("gcc", "ld");

    let target_arch =
        env::var("CARGO_CFG_TARGET_ARCH").expect("Can't get CARGO_CFG_TARGET_ARCH from env");
    let out_dir = env::var("OUT_DIR").expect("Can't get OUT_DIR from env");
    let out_dir = path::Path::new(&out_dir);

    let elf_file = out_dir.join("libselfe_arc_data.a");
    archive
        .write_object_file(elf_file, ld, &target_arch)
        .expect("Error creating object file");

    println!("cargo:rustc-link-lib=static=selfe_arc_data");
    println!("cargo:rustc-link-search={}", out_dir.display());
}
