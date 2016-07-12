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

pub use endian::Endian;
pub use read::ReadError;
pub use write::WriteError;

#[derive(Debug)]
pub struct Sections {
    pub endian: Endian,
    pub debug_info: Vec<u8>,
    pub debug_str: Vec<u8>,
    pub debug_abbrev: Vec<u8>,
}

#[derive(Debug)]
pub struct CompilationUnitIterator<'a> {
    sections: &'a Sections,
    data: &'a [u8],
    offset: usize,
}

#[derive(Debug)]
pub struct CompilationUnit<'a> {
    pub offset: usize,
    pub version: u16,
    pub endian: Endian,
    pub address_size: u8,
    // TODO: offset_size: u8,
    pub abbrev_offset: usize,
    pub data: Cow<'a, [u8]>,
    pub data_offset: usize,
}

#[derive(Debug)]
// TODO: use multiple lifetimes
pub struct DieCursor<'a> {
    r: &'a [u8],
    offset: usize,
    unit: &'a CompilationUnit<'a>,
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
    StringOffset(usize),
    Ref(usize),
    RefAddress(u64),
    RefSig(u64),
    SecOffset(usize),
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

impl<'a> CompilationUnit<'a> {
    pub fn new(
        endian: Endian,
        address_size: u8,
    ) -> CompilationUnit<'a> {
        CompilationUnit {
            offset: 0,
            version: 4,
            endian: endian,
            address_size: address_size,
            abbrev_offset: 0,
            data: Cow::Owned(Vec::new()),
            data_offset: 0,
        }
    }

    pub fn data(&'a self) -> &'a [u8] {
        &*self.data
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
        AbbrevHash(std::collections::HashMap::new())
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
