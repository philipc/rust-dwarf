extern crate byteorder;

pub use parse::*;

pub mod constant;
pub mod elf;

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
pub struct CompilationUnit<'a> {
    pub die: Vec<Die<'a>>,
}

#[derive(Debug)]
pub struct Die<'a> {
    pub tag: constant::DwTag,
    pub attribute: Vec<Attribute<'a>>,
    pub children: Vec<Die<'a>>,
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
    pub attribute: Vec<AbbrevAttribute>,
}

#[derive(Debug)]
pub struct AbbrevAttribute {
    pub at: constant::DwAt,
    pub form: constant::DwForm,
}

mod leb128;
mod display;
mod parse;
