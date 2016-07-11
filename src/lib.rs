extern crate byteorder;

use std::borrow::Cow;

mod endian;
mod leb128;
mod read;
mod write;

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
    offset: usize,
    version: u16,
    address_size: u8,
    // TODO: offset_size: u8,
    abbrev_offset: usize,
    data: &'a [u8],
    data_offset: usize,
}

#[derive(Debug)]
pub struct DieBuffer<'a> {
    endian: Endian,
    address_size: u8,
    // TODO: offset_size: u8,
    debug_str: Cow<'a, [u8]>,
    abbrev: AbbrevHash,
    data: Cow<'a, [u8]>,
    offset: usize,
}

#[derive(Debug)]
pub struct DieCursor<'a> {
    r: &'a [u8],
    offset: usize,
    buffer: &'a DieBuffer<'a>,
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
    Ref(usize),
    RefAddress(u64),
    RefSig(u64),
    SecOffset(usize),
    ExprLoc(&'a [u8]),
}

#[derive(Debug)]
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

    pub fn iter<'a>(&'a self) -> std::slice::Iter<'a, Abbrev> {
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
