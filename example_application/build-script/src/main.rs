use selfe_arc;
use std::fs;

fn main() {
    let mut archive = selfe_arc::pack::Archive::new();
    archive.add_file("data_file.txt", "../data_file.txt").unwrap();

    let mut archive_file = fs::File::create("../target/selfe_arc_data").unwrap();
    archive.write(&mut archive_file).unwrap();
}
