fn main() {
    selfe_arc::build::link_with_archive(vec![(
        "data_file.txt",
        std::path::Path::new("data_file.txt"),
    )]);
}
