extern crate byteorder;

use std::borrow::Cow;

mod endian;
mod leb128;
mod read;
mod write;
#[cfg(test)]
mod test;

pub mod constant;
pub mod display;
pub mod elf;

pub use endian::{AnyEndian, Endian, LittleEndian, BigEndian, NativeEndian};
pub use read::ReadError;
pub use write::WriteError;

#[derive(Debug)]
pub struct Sections<E: Endian> {
    pub endian: E,
    pub debug_info: Vec<u8>,
    pub debug_str: Vec<u8>,
    pub debug_abbrev: Vec<u8>,
}

#[derive(Debug)]
pub struct CompilationUnitIterator<'a, E: Endian> {
    endian: E,
    data: &'a [u8],
    offset: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompilationUnit<'a, E: Endian> {
    pub offset: usize,
    pub endian: E,
    pub version: u16,
    pub address_size: u8,
    pub offset_size: u8,
    pub abbrev_offset: u64,
    pub data: Cow<'a, [u8]>,
}

#[derive(Debug)]
pub struct DieCursor<'a, 'entry, 'unit: 'a, E: 'a+Endian> {
    r: &'entry [u8],
    offset: usize,
    unit: &'a CompilationUnit<'unit, E>,
    abbrev: &'a AbbrevHash,
    next_child: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Die<'a> {
    pub offset: usize,
    pub code: u64,
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attribute<'a> {
    pub at: constant::DwAt,
    pub data: AttributeData<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeData<'a> {
    Address(u64),
    Block(&'a [u8]),
    Data1(u8),
    Data2(u16),
    Data4(u32),
    Data8(u64),
    UData(u64),
    SData(i64),
    Flag(bool),
    String(&'a str),
    StringOffset(u64),
    Ref(u64),
    RefAddress(u64),
    RefSig(u64),
    SecOffset(u64),
    ExprLoc(&'a [u8]),
}

#[derive(Debug, Default)]
pub struct AbbrevHash(std::collections::HashMap<u64, Abbrev>);

#[derive(Debug)]
pub struct AbbrevVec(Vec<Abbrev>);

#[derive(Debug, PartialEq, Eq)]
pub struct Abbrev {
    pub code: u64,
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<AbbrevAttribute>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AbbrevAttribute {
    pub at: constant::DwAt,
    pub form: constant::DwForm,
}

impl<'a, E: Endian+Default> Default for CompilationUnit<'a, E> {
    fn default() -> Self {
        CompilationUnit {
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

impl<'a, E: Endian> CompilationUnit<'a, E> {
    fn base_header_len(&self) -> usize {
        // version + abbrev_offset + address_size
        2 + self.offset_size as usize + 1
    }

    fn total_header_len(&self) -> usize {
        // len + version + abbrev_offset + address_size
        // Includes an extra 4 bytes if offset_size is 8
        (self.offset_size as usize * 2 - 4) + self.base_header_len()
    }

    fn base_len(&self) -> usize {
        self.base_header_len() + self.data.len()
    }

    pub fn data(&'a self) -> &'a [u8] {
        &*self.data
    }

    pub fn data_offset(&self) -> usize {
        self.offset + self.total_header_len()
    }
}

impl<'a> Die<'a> {
    pub fn null(offset: usize) -> Self {
        Die {
            offset: offset,
            code: 0,
            tag: constant::DW_TAG_null,
            children: false,
            attributes: Vec::new(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.tag == constant::DW_TAG_null
    }
}

impl AbbrevHash {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<u64, Abbrev> {
        self.0.iter()
    }

    pub fn get(&self, code: u64) -> Option<&Abbrev> {
        self.0.get(&code)
    }

    pub fn insert(&mut self, abbrev: Abbrev) -> Option<Abbrev> {
        self.0.insert(abbrev.code, abbrev)
    }
}

impl AbbrevVec {
    pub fn new(val: Vec<Abbrev>) -> Self {
        AbbrevVec(val)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<Abbrev> {
        self.0.iter()
    }
}

impl AbbrevAttribute {
    pub fn null() -> Self {
        AbbrevAttribute {
            at: constant::DW_AT_null,
            form: constant::DW_FORM_null,
        }
    }

    pub fn is_null(&self) -> bool {
        self.at == constant::DW_AT_null && self.form == constant::DW_FORM_null
    }
}
