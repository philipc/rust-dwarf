extern crate byteorder;

mod leb128;
mod decode;

pub mod constant;
pub mod display;
pub mod elf;

pub use decode::DecodeError;

#[derive(Clone, Copy, Debug)]
pub enum Endian {
    Little,
    Big,
}

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
    sections: &'a Sections,
    offset: usize,
    version: u16,
    address_size: u8,
    // TODO: offset_size: u8,
    abbrev: AbbrevHash,
    data: &'a [u8],
}

#[derive(Debug)]
pub struct DieCursor<'a> {
    unit: &'a CompilationUnit<'a>,
    data: &'a [u8],
    offset: usize,
    next_child: bool,
}

#[derive(Debug)]
pub struct Die<'a> {
    pub offset: usize,
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug)]
pub struct Attribute<'a> {
    pub at: constant::DwAt,
    pub data: AttributeData<'a>,
}

#[derive(Debug)]
pub enum AttributeData<'a> {
    Address(usize),
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
    RefAddress(usize),
    RefSig(u64),
    SecOffset(usize),
    ExprLoc(&'a [u8]),
}

#[derive(Debug)]
pub struct AbbrevHash(std::collections::HashMap<u64, Abbrev>);

#[derive(Debug)]
pub struct Abbrev {
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<AbbrevAttribute>,
}

#[derive(Debug)]
pub struct AbbrevAttribute {
    pub at: constant::DwAt,
    pub form: constant::DwForm,
}
