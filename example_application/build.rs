#[cfg(not(workaround_build))]
fn main() {
    println!("cargo:rerun-if-changed=build-script");
    cargo_5730::run_build_script();
}

#[cfg(workaround_build)]
fn main() {
    selfe_arc::build::link_with_archive(vec![(
        "data_file.txt",
        std::path::Path::new("data_file.txt"),
    )]);
}
