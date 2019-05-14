# selfe-arc

A simple file archive library, useful for bundling process binaries and
configuration files with your seL4 application. It takes special care to page
align all file data.

## Usage

The `pack` module can be used to create archives on a typical computer. This
will be done automatically when building with `selfe`. The `read` module can be
used to read archive data (which you embedded in your root task, and so is
mapped into memory).

## Library Contents

### layout module

The `layout` module contains the basic data structures and code that define the
serialization format. You typically won't use it directly.

### pack module

The `pack` module can be used to create archives from files on the filesystem. 

```rust
use selfe_arc::pack;

let mut ar = pack::Archive::new();
ar.add_file("test.txt", Path::new("./test.txt"));

let mut archive_data = Vec::new();
let mut writer = io::BufWriter::new(&mut archive_data);
ar.write(&mut writer).unwrap();
```

## read module

The `read` module, available with `nostd`, Allos you to read the data from
somewhere in memory. It deals with `&[u8]` slices, which you should be able to
create from a pointer and a length.

```rust
use selfe_arc::read;

let arc_data = unsafe { core::slice::from_raw_parts(_selfe_arc_data_ptr, _selfe_arc_len) };
let ar = read::Archive::from_slice(&arc_data);

let test_txt: &[u8] = ar.file("test.txt").unwrap();
```
