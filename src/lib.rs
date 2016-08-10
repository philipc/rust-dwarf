use std::borrow::Cow;

mod endian;
mod leb128;
mod read;
mod write;
#[cfg(test)]
mod test;

pub mod abbrev;
pub mod constant;
pub mod die;
pub mod display;
pub mod elf;

pub use endian::{AnyEndian, Endian, LittleEndian, BigEndian, NativeEndian};
pub use read::ReadError;
pub use write::WriteError;

#[derive(Debug)]
pub struct Sections<E: Endian> {
    pub endian: E,
    pub debug_abbrev: Vec<u8>,
    pub debug_info: Vec<u8>,
    pub debug_str: Vec<u8>,
    pub debug_types: Vec<u8>,
}

#[derive(Debug)]
pub struct CompilationUnitIterator<'a, E: Endian> {
    endian: E,
    data: &'a [u8],
    offset: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompilationUnit<'a, E: Endian> {
    pub common: UnitCommon<'a, E>,
}

#[derive(Debug)]
pub struct TypeUnitIterator<'a, E: Endian> {
    endian: E,
    data: &'a [u8],
    offset: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeUnit<'a, E: Endian> {
    pub common: UnitCommon<'a, E>,
    pub type_signature: u64,
    pub type_offset: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnitCommon<'a, E: Endian> {
    pub offset: usize,
    pub endian: E,
    pub version: u16,
    pub address_size: u8,
    pub offset_size: u8,
    pub abbrev_offset: u64,
    pub data: Cow<'a, [u8]>,
}

impl<'a, E: Endian+Default> Default for CompilationUnit<'a, E> {
    fn default() -> Self {
        CompilationUnit {
            common: Default::default(),
        }
    }
}

impl<'a, E: Endian> CompilationUnit<'a, E> {
    fn base_header_len(offset_size: u8) -> usize {
        // version + abbrev_offset + address_size
        2 + offset_size as usize + 1
    }

    fn total_header_len(offset_size: u8) -> usize {
        // len + version + abbrev_offset + address_size
        // Includes an extra 4 bytes if offset_size is 8
        (offset_size as usize * 2 - 4) + Self::base_header_len(offset_size)
    }

    pub fn data(&'a self) -> &'a [u8] {
        self.common.data()
    }

    pub fn data_offset(&'a self) -> usize {
        self.common.offset + Self::total_header_len(self.common.offset_size)
    }
}

impl<'a, E: Endian> TypeUnit<'a, E> {
    fn base_header_len(offset_size: u8) -> usize {
        // version + abbrev_offset + address_size + type_signature + type_offset
        2 + offset_size as usize + 1 + 8 + offset_size as usize
    }

    fn total_header_len(offset_size: u8) -> usize {
        // Includes an extra 4 bytes if offset_size is 8
        (offset_size as usize * 2 - 4) + Self::base_header_len(offset_size)
    }

    pub fn data(&'a self) -> &'a [u8] {
        self.common.data()
    }

    pub fn data_offset(&'a self) -> usize {
        self.common.offset + Self::total_header_len(self.common.offset_size)
    }
}

impl<'a, E: Endian+Default> Default for UnitCommon<'a, E> {
    fn default() -> Self {
        UnitCommon {
            offset: 0,
            endian: Default::default(),
            version: 4,
            address_size: 4,
            offset_size: 4,
            abbrev_offset: 0,
            data: Cow::Owned(Vec::new()),
        }
    }
}

impl<'a, E: Endian> UnitCommon<'a, E> {
    pub fn data(&'a self) -> &'a [u8] {
        &*self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}
