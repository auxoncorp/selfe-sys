use crate::layout::{self, ArchiveHeader, DirectoryEntry};

pub struct Archive<'a>(&'a [u8]);

#[derive(Debug)]
pub enum ReadError {
    InvalidMagicNumber,
    InvalidVersion,
    FileNotFound,
    FileOffsetTooLarge,
    LayoutError(layout::ReadError),
}

impl core::convert::From<layout::ReadError> for ReadError {
    fn from(e: layout::ReadError) -> ReadError {
        ReadError::LayoutError(e)
    }
}

pub struct DirectoryEntryIterator<'a> {
    remaining_files: usize,
    data: &'a [u8],
}

impl<'a> Iterator for DirectoryEntryIterator<'a> {
    type Item = Result<DirectoryEntry, layout::ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_files > 0 {
            let entry = DirectoryEntry::read(&self.data);
            self.remaining_files = self.remaining_files - 1;
            self.data = &self.data[DirectoryEntry::serialized_size()..];
            Some(entry)
        } else {
            None
        }
    }
}

impl<'a> Archive<'a> {
    pub fn from_slice(sl: &'a [u8]) -> Archive<'a> {
        Archive(sl)
    }

    fn header(&self) -> Result<ArchiveHeader, ReadError> {
        // TODO: verify header crc
        let header = ArchiveHeader::read(self.0)?;

        if header.magic != *layout::MAGIC {
            return Err(ReadError::InvalidMagicNumber);
        }

        if header.version != layout::VERSION_1 {
            return Err(ReadError::InvalidVersion);
        }

        Ok(header)
    }

    pub fn all_files(&'a self) -> Result<DirectoryEntryIterator<'a>, ReadError> {
        let header = self.header()?;
        Ok(DirectoryEntryIterator {
            remaining_files: header.file_count as usize,
            data: &self.0[ArchiveHeader::serialized_size()..],
        })
    }

    pub fn file(&'a self, name: &'a str) -> Result<&'a [u8], ReadError> {
        let mut dir_entry = None;
        for res in self.all_files()? {
            if let Ok(entry) = res {
                if let Ok(entry_name) = entry.name() {
                    if entry_name == name {
                        dir_entry = Some(entry);
                    }
                }
            }
        }

        let dir_entry = dir_entry.ok_or_else(|| ReadError::FileNotFound)?;
        println!("found entry: {:?}", dir_entry);
        let header = self.header()?;
        let data_slice = &self.0[header.data_start as usize..];
        Ok(&data_slice
            [dir_entry.offset as usize..dir_entry.offset as usize + dir_entry.length as usize])
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::pack;
    use proptest::prelude::*;
    use proptest::{collection, num};
    use std::collections::HashSet;
    use std::io::{Read, Write};
    use std::path::Path;
    use std::{fs, io};
    use tempfile::{NamedTempFile, TempPath};

    #[test]
    fn write_and_read() {
        let mut data = Vec::<u8>::new();

        {
            let mut ar = pack::Archive::new();
            ar.add_file("lib.rs", Path::new("./src/lib.rs"));
            ar.add_file("pack.rs", Path::new("./src/pack.rs"));

            let mut writer = io::BufWriter::new(&mut data);
            ar.write(&mut writer).unwrap();
        }

        let ar = Archive::from_slice(&data);

        // check directory
        let dir = ar.all_files().unwrap();
        let files = dir
            .map(|dir_entry| dir_entry.unwrap().name().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(files, vec!("lib.rs", "pack.rs"));

        // check lib.rs content
        {
            let actual_data = ar.file("lib.rs").unwrap();
            let mut expected_data = Vec::new();
            let mut f = fs::File::open("./src/lib.rs").unwrap();
            f.read_to_end(&mut expected_data).unwrap();
            assert_eq!(expected_data, actual_data);
        }

        // check pack.rs content
        {
            let actual_data = ar.file("pack.rs").unwrap();
            let mut expected_data = Vec::new();
            let mut f = fs::File::open("./src/pack.rs").unwrap();
            f.read_to_end(&mut expected_data).unwrap();
            assert_eq!(expected_data, actual_data);
        }
    }

    fn gen_test_file(
        max_name_size: usize,
        max_file_size: usize,
    ) -> impl Strategy<Value = (String, TempPath)> {
        (
            ".{0,256}".prop_filter("string is too long", move |s| {
                s.bytes().len() <= max_name_size
            }),
            collection::vec(num::u8::ANY, 0..max_file_size),
        )
            .prop_map(|(name, data)| {
                let mut file = NamedTempFile::new().unwrap();
                file.write(&data).unwrap();

                (name, file.into_temp_path())
            })
    }

    fn files_should_round_trip(
        files: Vec<(String, TempPath)>,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        let mut data = Vec::<u8>::new();

        {
            let mut ar = pack::Archive::new();
            for (name, path) in files.iter() {
                ar.add_file(name, path);
            }

            let mut writer = io::BufWriter::new(&mut data);
            ar.write(&mut writer).unwrap();
        }

        let ar = Archive::from_slice(&data);

        let dir = ar.all_files().unwrap();
        let dir_files = dir
            .map(|dir_entry| dir_entry.unwrap().name().unwrap().to_owned())
            .collect::<HashSet<_>>();

        for (name, path) in files.iter() {
            prop_assert!(dir_files.contains(name));

            let actual_data = ar.file(name);
            prop_assert!(actual_data.is_ok());
            let actual_data = actual_data.unwrap();

            let mut expected_data = Vec::new();
            let mut f = fs::File::open(path).unwrap();
            f.read_to_end(&mut expected_data).unwrap();
            prop_assert_eq!(expected_data, actual_data);
        }
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 30, .. ProptestConfig::default()
        })]
        #[test]
        fn write_and_read_small_files(files in collection::vec(gen_test_file(255, 0x4000), 1..10)) {
            // TODO ^^^ try 256 ^^^
            files_should_round_trip(files)?
        }
    }

    // This is just way too slow in debug mode
    #[cfg(not(debug_assertions))]
    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 5, .. ProptestConfig::default()
        })]
        #[test]
        fn write_and_read_large_files(files in collection::vec(gen_test_file(255, 0x4000000), 1..10)) {
            files_should_round_trip(files)?
        }
    }

}
