use std::convert::TryFrom;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::layout;

pub struct Archive {
    files: Vec<File>,
}

pub struct File {
    name: String,
    path: PathBuf,
}

struct ScheduledFile {
    path: PathBuf,
    size: u64,
    padding: u64,
}

#[derive(Debug)]
pub enum ArchiveWriteError {
    HeaderTooLarge,
    DataSegmentTooLarge,
    FileNameTooLong(String),
    IO(io::Error),
    UnsupportedTargetArch,
    LinkError,
}

impl std::convert::From<io::Error> for ArchiveWriteError {
    fn from(e: io::Error) -> ArchiveWriteError {
        ArchiveWriteError::IO(e)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum AddFileError {
    EmptyNameNotAllowed,
    NameConflict,
    FileNameTooLong(String),
}

const LINKER_SCRIPT: &str = r#"SECTIONS
{
  .rodata : ALIGN(8)
  {
    _selfe_arc_data_start = . ;
    *(.*) ;
    _selfe_arc_data_end = . ;
  }
}"#;

impl Archive {
    pub fn new() -> Archive {
        Archive { files: vec![] }
    }

    pub fn add_file<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<(), AddFileError> {
        let path = path.as_ref();

        if name.is_empty() {
            return Err(AddFileError::EmptyNameNotAllowed);
        }

        if self.files.iter().find(|f| f.name == name).is_some() {
            return Err(AddFileError::NameConflict);
        }

        if name.as_bytes().len() > layout::FILE_NAME_BYTES {
            return Err(AddFileError::FileNameTooLong(name.to_owned()));
        }

        self.files.push(File {
            name: name.to_owned(),
            path: path.to_owned(),
        });

        Ok(())
    }

    pub fn write<W: Write>(&self, mut writer: &mut W) -> Result<(), ArchiveWriteError> {
        let header_size = layout::ArchiveHeader::serialized_size();
        let dir_entry_size = layout::DirectoryEntry::serialized_size();

        let file_count =
            u32::try_from(self.files.len()).map_err(|_| ArchiveWriteError::HeaderTooLarge)?;
        let dir_size: u32 = file_count
            .checked_mul(
                u32::try_from(dir_entry_size).map_err(|_| ArchiveWriteError::HeaderTooLarge)?,
            )
            .ok_or(ArchiveWriteError::HeaderTooLarge)?;
        let data_start: u32 = dir_size
            .checked_add(u32::try_from(header_size).map_err(|_| ArchiveWriteError::HeaderTooLarge)?)
            .ok_or(ArchiveWriteError::HeaderTooLarge)?;

        // page align data_start
        let data_start = layout::align_addr(data_start);
        let initial_padding_size = data_start - (dir_size + header_size as u32);

        // header
        let header = layout::ArchiveHeader {
            magic: *layout::MAGIC,
            version: layout::VERSION_1,
            data_start,
            file_count,
        };

        header.write(&mut writer)?;

        // directory
        let mut scheduled_files = Vec::new();
        let mut data_cursor = 0u64;
        for (i, f) in self.files.iter().enumerate() {
            // files should always be page-aligned
            assert_eq!(data_cursor & 0xfff, 0);

            let name = f.name.as_bytes();
            if name.len() > layout::FILE_NAME_BYTES {
                return Err(ArchiveWriteError::FileNameTooLong(f.name.to_owned()));
            }

            let data_file = fs::File::open(&f.path)?;
            let file_size = data_file.metadata()?.len();

            let mut dir_entry = layout::DirectoryEntry {
                name_len: name.len() as u8,
                name_bytes: [0; layout::FILE_NAME_BYTES],
                offset: data_cursor,
                length: file_size,
            };

            // copy the name into the dir entry
            for (name_char, entry_char) in name.iter().zip(dir_entry.name_bytes.iter_mut()) {
                *entry_char = *name_char;
            }

            dir_entry.write(&mut writer)?;

            // pad to page boundaries, but not the last file.
            let is_last = i == self.files.len() - 1;
            let padding = if is_last {
                0
            } else {
                let alignment: u64 = layout::ALIGNMENT.into();
                let mask: u64 = layout::ALIGNMENT_MASK.into();
                alignment - (file_size & mask)
            };

            scheduled_files.push(ScheduledFile {
                path: f.path.to_owned(),
                size: file_size,
                padding,
            });

            data_cursor = data_cursor
                .checked_add(dir_entry.length)
                .ok_or(ArchiveWriteError::DataSegmentTooLarge)?
                .checked_add(padding)
                .ok_or(ArchiveWriteError::DataSegmentTooLarge)?;
        }

        // initial padding
        for _ in 0..initial_padding_size {
            writer.write(&[0])?;
        }

        // data
        for f in scheduled_files.iter() {
            let data_file = fs::File::open(&f.path).unwrap();
            let mut buf_reader = io::BufReader::new(data_file);
            let bytes_written = io::copy(&mut buf_reader, &mut writer)?;

            assert_eq!(bytes_written, f.size);

            for _ in 0..f.padding {
                writer.write(&[0])?;
            }
        }

        Ok(())
    }

    pub fn write_object_file<P: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        output: P,
        ld: P2,
        target_arch: &str,
    ) -> Result<(), ArchiveWriteError> {
        let output = output.as_ref();
        let ld = ld.as_ref();

        let archive_path = output.with_extension("selfearc");

        {
            let mut archive_file = fs::File::create(&archive_path)?;
            self.write(&mut archive_file)?;
        }

        let linker_script_path = output.with_extension("ld");

        {
            let mut linker_script_file = fs::File::create(&*linker_script_path)?;
            write!(&mut linker_script_file, "{}", LINKER_SCRIPT)?;
        }

        let output_format = match target_arch {
            "aarch64" => "elf64-littleaarch64",
            "arm" | "armv7" | "armebv7r" | "armv5te" | "armv7r" | "armv7s" => "elf32-littlearm",
            "i386" | "i586" | "i686" => "elf32-i386",
            "riscv32imac" | "riscv32imc" | "riscv64gc" | "riscv64imac" => "elf32-littleriscv",
            "thumbv7em" | "thumbv7m" | "thumbv7neon" => "elf32-littlearm",
            "thumbv8m.main" => "elf64-littleaarch64",
            "x86_64" => "elf64-x86-64",
            _ => return Err(ArchiveWriteError::UnsupportedTargetArch),
        };

        let mut ld = Command::new(ld);
        ld.arg("-T")
            .arg(linker_script_path)
            .arg("--oformat")
            .arg(output_format)
            .arg("-r")
            .arg("-b")
            .arg("binary")
            .arg(archive_path)
            .arg("-o")
            .arg(output)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        println!("running ld command: {:?}", ld);

        let output = ld.output()?;
        if !output.status.success() {
            return Err(ArchiveWriteError::LinkError);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_empty_name() {
        let mut ar = Archive::new();
        let res = ar.add_file("", Path::new("doesn't_matter"));
        assert_eq!(res, Err(AddFileError::EmptyNameNotAllowed));
    }

    #[test]
    fn no_duplicate_name() {
        {
            let mut test_file = fs::File::create("/tmp/pack_test.txt").unwrap();
            test_file.write_all(b"test").unwrap();
        }

        let mut ar = Archive::new();
        let res = ar.add_file("test", Path::new("/tmp/pack_test.txt"));
        assert_eq!(res, Ok(()));

        let res = ar.add_file("test", Path::new("doesn't_matter"));
        assert_eq!(res, Err(AddFileError::NameConflict));
    }

    #[test]
    fn no_overlong_name() {
        let mut ar = Archive::new();
        let name = "dajlsdkfj alskdjflkasdjfkljasdkl fjalfj eliwjf lasdijflaksdjflkasjdlkfaj sdlfkjasldkf jalsdkjf laskjdf laskdjf lakwjflawjelf ijasdlkfjaslfiawejlfajsdkflasdkjflaskdjflaskdjflaskdjflaksjdflkasjdflaksdjflaskdjflaksdjflkasjdflkajsdflkajsdlkfjasldkfjlaksjdflkasjdflkajsdlkfjasldkjfaklsdjf";
        let res = ar.add_file(name, Path::new("foo"));
        assert_eq!(res, Err(AddFileError::FileNameTooLong(name.to_owned())));
    }

    #[test]
    fn pack_files() {
        {
            let mut test_file = fs::File::create("/tmp/pack_test.txt").unwrap();
            test_file.write_all(b"test").unwrap();
        }

        let mut ar = Archive::new();
        ar.add_file("test", Path::new("/tmp/pack_test.txt"))
            .unwrap();

        let mut actual_data = Vec::new();
        {
            let mut writer = io::BufWriter::new(&mut actual_data);
            ar.write(&mut writer).unwrap();
        }

        let mut expected_data = vec![];
        // ARCHIVE HEADER
        #[rustfmt::skip]
        expected_data.append(&mut vec!(
            // magic
            0x73, 0x65, 0x6c, 0x66, 0x65, 0x61, 0x72, 0x63,
            // version
            0x01,
            // data_start
            0x00, 0x10, 0x00, 0x00,
            // file_count
            0x01, 0x00, 0x00, 0x00,
        ));

        assert_eq!(
            expected_data.len(),
            layout::ArchiveHeader::serialized_size()
        );

        // DIRECTORY ENTRY 1/1
        #[rustfmt::skip]
        expected_data.append(&mut vec!(
            // len, name
            0x04, 0x74, 0x65, 0x73, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // offset
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // length
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ));

        assert_eq!(
            expected_data.len(),
            layout::ArchiveHeader::serialized_size() + layout::DirectoryEntry::serialized_size()
        );

        // PADDING
        expected_data.append(&mut [0u8; 3807].to_vec());

        // FILE 1/1
        expected_data.append(&mut vec![0x74, 0x65, 0x73, 0x74]);

        // double check file data alignment
        assert_eq!(
            expected_data
                .clone()
                .into_iter()
                .skip(0x1000)
                .collect::<Vec<u8>>(),
            vec!(0x74, 0x65, 0x73, 0x74)
        );

        assert_eq!(expected_data.len(), actual_data.len());
        for (i, (e, a)) in expected_data.iter().zip(actual_data.iter()).enumerate() {
            assert_eq!(
                e, a,
                "At byte {:#x}, expected {:#04x} but got {:#04x}",
                i, e, a
            );
        }
    }

    #[test]
    fn object_file() {
        use std::str;

        {
            let mut test_file = fs::File::create("/tmp/pack_test.txt").unwrap();
            test_file.write_all(b"test").unwrap();
        }

        let mut ar = Archive::new();
        ar.add_file("test", Path::new("/tmp/pack_test.txt"))
            .unwrap();

        ar.write_object_file("/tmp/pack_test.elf", "ld", "x86_64")
            .unwrap();

        let mut ld = Command::new("objdump");
        let out = ld.arg("-t").arg("/tmp/pack_test.elf").output().unwrap();
        let stdout = str::from_utf8(&out.stdout).unwrap();
        assert!(stdout.contains("_selfe_arc_data_start"));
        assert!(stdout.contains("_selfe_arc_data_end"));
    }
}
