extern crate byteorder;
extern crate fallible_iterator;

mod leb128;
mod display;
mod parse;

pub mod constant;
pub mod elf;

pub use fallible_iterator::FallibleIterator;
pub use parse::ParseError;

#[derive(Debug)]
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
pub struct CompilationUnitHeader<'a> {
    sections: &'a Sections,
    version: u16,
    address_size: u8,
    // TODO: offset_size: u8,
    abbrev: &'a [u8],
    data: &'a [u8],
}

#[derive(Debug)]
pub struct CompilationUnit<'a> {
    pub die: Vec<Die<'a>>,
}

#[derive(Debug)]
pub struct Die<'a> {
    pub tag: constant::DwTag,
    pub attributes: Vec<Attribute<'a>>,
    pub children: Option<Vec<Die<'a>>>,
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
    Ref(usize), // TODO: convert to DIE index
    RefAddress(usize),
    RefSig(u64),
    SecOffset(usize),
    ExprLoc(&'a [u8]),
}

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
