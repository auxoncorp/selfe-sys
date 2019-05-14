use core::{fmt, str};
use core::mem;

#[cfg(feature = "std")]
use byteorder::{LittleEndian, WriteBytesExt};

////////////////
// Read Utils //
////////////////

#[derive(Debug)]
pub enum ReadError {
    BufferTooShort,
}

/// because try_from is only implemented for slices up to length 32
fn u8_slice_to_array_256(slice: &[u8]) -> Option<[u8; 256]> {
    if slice.len() != 256 {
        None
    } else {
        let ptr = slice.as_ptr() as *const [u8; 256];
        Some(unsafe { *ptr })
    }
}

/// Checked versions of the relevant byteorder read functions
mod read {
    use super::{u8_slice_to_array_256, ReadError};
    use byteorder::{ByteOrder, LittleEndian};
    use core::convert::TryInto;

    pub(super) fn read_u8(buf: &[u8]) -> Result<u8, ReadError> {
        if buf.len() < 1 {
            Err(ReadError::BufferTooShort)
        } else {
            Ok(buf[0])
        }
    }

    pub(super) fn read_u32(buf: &[u8]) -> Result<u32, ReadError> {
        if buf.len() < 4 {
            Err(ReadError::BufferTooShort)
        } else {
            Ok(LittleEndian::read_u32(buf))
        }
    }

    pub(super) fn read_u64(buf: &[u8]) -> Result<u64, ReadError> {
        if buf.len() < 8 {
            Err(ReadError::BufferTooShort)
        } else {
            Ok(LittleEndian::read_u64(buf))
        }
    }

    pub(super) fn read_8_bytes(buf: &[u8]) -> Result<[u8; 8], ReadError> {
        if buf.len() < 8 {
            Err(ReadError::BufferTooShort)
        } else {
            Ok(buf[0..8].try_into().unwrap())
        }
    }

    pub(super) fn read_256_bytes(buf: &[u8]) -> Result<[u8; 256], ReadError> {
        if buf.len() < 256 {
            Err(ReadError::BufferTooShort)
        } else {
            let slice = &buf[0..256];
            Ok(u8_slice_to_array_256(&slice).unwrap())
        }
    }
}

///////////////
// Constants //
///////////////

/// The selfarc magic number
pub const MAGIC: &[u8; 8] = b"selfarc!";

/// The file format version
pub const VERSION_1: u8 = 1;

/// Where to align file data
pub const ALIGNMENT: u64 = 0x1000;

/// The mask for aligning file addresses.
pub const ALIGNMENT_MASK: u64 = ALIGNMENT - 1;

pub fn align_addr(a: u64) -> u64 {
    let low_bits = a & ALIGNMENT_MASK;
    if low_bits == 0 {
        a
    } else {
        a + ALIGNMENT - low_bits
    }
}

///////////////////
// ArchiveHeader //
///////////////////

#[derive(Debug, PartialEq, Eq)]
pub struct ArchiveHeader {
    /// The magic number
    pub magic: [u8; 8],

    /// The archive format version
    pub version: u8,

    /// The offset of the start of file data, relative to the beginning of the
    /// archive data.
    pub data_start: u32,

    /// The number of files in this archive
    pub file_count: u32,
}

impl Default for ArchiveHeader {
    fn default() -> ArchiveHeader {
        ArchiveHeader {
            magic: *MAGIC,
            version: VERSION_1,
            data_start: 0,
            file_count: 0,
        }
    }
}

impl ArchiveHeader {
    pub const fn serialized_size() -> usize {
        mem::size_of::<[u8;8]>() // magic
            + mem::size_of::<u8>() // version
            + mem::size_of::<u32>() // data_start
            + mem::size_of::<u32>() // file_count
    }

    #[cfg(feature = "std")]
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write(&self.magic)?;
        writer.write_u8(self.version)?;
        writer.write_u32::<LittleEndian>(self.data_start)?;
        writer.write_u32::<LittleEndian>(self.file_count)?;
        Ok(())
    }

    pub fn read(mut buf: &[u8]) -> Result<ArchiveHeader, ReadError> {
        let mut header = ArchiveHeader::default();

        header.magic = read::read_8_bytes(buf)?;
        buf = &buf[8..];

        header.version = read::read_u8(buf)?;
        buf = &buf[1..];

        header.data_start = read::read_u32(buf)?;
        buf = &buf[4..];

        header.file_count = read::read_u32(buf)?;

        Ok(header)
    }
}

pub struct DirectoryEntry {
    /// The length of the file name in bytes.
    pub name_len: u8,

    /// The bytes of the file name, UTF-8 encoded.
    pub name_bytes: [u8; 256],

    /// The location of the file, as an offset from header.data_start.
    /// 4k-aligned.
    pub offset: u64,

    /// The length of the file, in bytes
    pub length: u64,
}

impl Default for DirectoryEntry {
    fn default() -> DirectoryEntry {
        DirectoryEntry {
            name_len: 0,
            name_bytes: [0; 256],
            offset: 0,
            length: 0,
        }
    }
}

impl fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DirectoryEntry {{ \n\tname_len: {}, \n\tname_bytes: {:?}, \n\tdecoded name: \"{}\",\n\toffset: {:x}, \n\tlength: {} \n}}",
            self.name_len,
            &self.name_bytes as &[u8],
            self.name().unwrap_or("Invalid UTF8"),
            self.offset,
            self.length
        )
    }
}

impl PartialEq for DirectoryEntry {
    fn eq(&self, other: &DirectoryEntry) -> bool {
        (self.name_len == other.name_len)
            && (self
                .name_bytes
                .iter()
                .zip(other.name_bytes.iter())
                .all(|(a, b)| a == b))
            && (self.offset == other.offset)
            && (self.length == other.length)
    }
}

impl Eq for DirectoryEntry {}

impl DirectoryEntry {
    pub const fn serialized_size() -> usize {
        mem::size_of::<u8>() // name_len
            + mem::size_of::<[u8;256]>() // name_bytes
            + mem::size_of::<u64>() // offset
            + mem::size_of::<u64>() // length
    }

    #[cfg(feature = "std")]
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_u8(self.name_len)?;
        writer.write(&self.name_bytes)?;
        writer.write_u64::<LittleEndian>(self.offset)?;
        writer.write_u64::<LittleEndian>(self.length)?;
        Ok(())
    }

    pub fn read(mut buf: &[u8]) -> Result<DirectoryEntry, ReadError> {
        let mut entry = DirectoryEntry::default();

        entry.name_len = read::read_u8(buf)?;
        buf = &buf[1..];

        entry.name_bytes = read::read_256_bytes(buf)?;
        buf = &buf[256..];

        entry.offset = read::read_u64(buf)?;
        buf = &buf[8..];

        entry.length = read::read_u64(buf)?;

        Ok(entry)
    }

    pub fn name(&self) -> Result<&str, core::str::Utf8Error> {
        str::from_utf8(&self.name_bytes[0..self.name_len as usize])
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::{array, collection, num};

    #[test]
    fn header_layout() {
        #[rustfmt::skip]
        let expected = vec!(
            // magic
            0x73, 0x65, 0x6c, 0x66, 0x61, 0x72, 0x63, 0x21,
            // version
            0x01,
            // data_start
            0x00, 0x10, 0x00, 0x00,
            // file_count
            0x02, 0x00, 0x00, 0x00);

        let mut actual = vec![];
        ArchiveHeader {
            magic: *MAGIC,
            version: VERSION_1,
            data_start: 0x1000,
            file_count: 2,
        }
        .write(&mut actual)
        .unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn directory_entry_layout() {
        #[rustfmt::skip]
        let expected = vec!(
            // name length
            0x04,
            // name
            0x74, 0x65, 0x73, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
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
            0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // length
            0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );

        let mut entry = DirectoryEntry {
            name_len: 0,
            name_bytes: [0; 256],
            offset: 0x2000,
            length: 0x4000,
        };

        let name = "test".as_bytes();
        entry.name_len = name.len() as u8;
        for (a, b) in name.iter().zip(entry.name_bytes.iter_mut()) {
            *b = *a;
        }

        let mut actual = vec![];
        entry.write(&mut actual).unwrap();

        assert_eq!(expected.len(), actual.len());
        assert_eq!(expected, actual);
    }

    fn gen_archive_header() -> impl Strategy<Value = ArchiveHeader> {
        (
            array::uniform8(num::u8::ANY), // magic
            num::u8::ANY,                  // version
            num::u32::ANY,                 // data_start
            num::u32::ANY,
        ) // file_count
            .prop_map(|(magic, version, data_start, file_count)| ArchiveHeader {
                magic,
                version,
                data_start,
                file_count,
            })
    }

    fn gen_directory_entry() -> impl Strategy<Value = DirectoryEntry> {
        (
            num::u8::ANY,                            // name_len
            collection::vec(num::u8::ANY, 256..257), // name_bytes
            num::u64::ANY,                           // offset
            num::u64::ANY,
        ) // length
            .prop_map(
                |(name_len, name_bytes_vec, offset, length)| DirectoryEntry {
                    name_len,
                    name_bytes: u8_slice_to_array_256(&name_bytes_vec).unwrap(),
                    offset,
                    length,
                },
            )
    }

    proptest! {
        // Archive header
        #[test]
        fn read_archive_header_doesnt_panic(bytes in collection::vec(num::u8::ANY, 0..18)) {
            let _ignore = ArchiveHeader::read(&bytes);
        }

        #[test]
        fn read_archive_header_errors_with_too_little_data(bytes in collection::vec(num::u8::ANY, 0..17)) {
            prop_assert!(ArchiveHeader::read(&bytes).is_err());
        }

        #[test]
        fn archive_header_round_trip(header in gen_archive_header()) {
            let mut ser = vec!();
            prop_assert!(header.write(&mut ser).is_ok());

            let deser = ArchiveHeader::read(&ser);
            prop_assert!(deser.is_ok());
            prop_assert_eq!(header, deser.unwrap());
        }

        // Directory entry
        #[test]
        fn read_directory_entry_doesnt_panic(bytes in collection::vec(num::u8::ANY, 0..266)) {
            let _ignore = DirectoryEntry::read(&bytes);
        }

        #[test]
        fn read_directory_entry_erros_with_too_little_data(bytes in collection::vec(num::u8::ANY, 0..265)) {
            prop_assert!(DirectoryEntry::read(&bytes).is_err());
        }

        #[test]
        fn directory_entry_round_trip(header in gen_directory_entry()) {
            let mut ser = vec!();
            prop_assert!(header.write(&mut ser).is_ok());

            let deser = DirectoryEntry::read(&ser);
            prop_assert!(deser.is_ok());
            prop_assert_eq!(header, deser.unwrap());
        }
    }
}
