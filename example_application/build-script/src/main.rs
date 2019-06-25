use selfe_arc;
use std::path::Path;

fn main() {
    selfe_arc::build::link_with_archive(vec![("data_file.txt", Path::new("../data_file.txt"))]);
}
