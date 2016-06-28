use std;
use byteorder;
use byteorder::{ByteOrder, ReadBytesExt};

use types::*;
use constant;
use leb128;

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Invalid(String),
    Unsupported(String),
}

impl std::convert::From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::Io(e)
    }
}

impl std::convert::From<std::str::Utf8Error> for ParseError {
    fn from(e: std::str::Utf8Error) -> Self {
        ParseError::Utf8(e)
    }
}

impl std::convert::From<leb128::Error> for ParseError {
    fn from(e: leb128::Error) -> Self {
        match e {
            leb128::Error::Io(e) => ParseError::Io(e),
            leb128::Error::Overflow => ParseError::Invalid("LEB128 overflow".to_string()),
        }
    }
}

pub fn parse_sections<'a>(sections: &'a Sections) -> Result<Vec<CompilationUnit<'a>>, ParseError> {
    match sections.endian {
        Endian::Little => parse_debug_info::<byteorder::LittleEndian>(sections),
        Endian::Big => parse_debug_info::<byteorder::BigEndian>(sections),
    }
}

fn parse_debug_info<'a, B: ByteOrder>(sections: &'a Sections) -> Result<Vec<CompilationUnit<'a>>, ParseError> {
    let mut info = &sections.debug_info[..];
    let mut result = Vec::new();
    while info.len() > 0 {
        let len = try!(info.read_u32::<B>()) as usize;
        // TODO: 64 bit
        if len >= 0xfffffff0 {
            return Err(ParseError::Unsupported(format!("compilation unit length {}", len)))
        }
        if len > info.len() {
            return Err(ParseError::Invalid(format!("compilation unit length {}", len)))
        }
        result.push(try!(parse_compilation_unit::<B>(sections, &info[..len])));
        info = &info[len..];
    }
    Ok(result)
}

fn parse_compilation_unit<'a, B: ByteOrder>(sections: &'a Sections, mut r: &'a [u8]) -> Result<CompilationUnit<'a>, ParseError> {
    let r = &mut r;

    let version = try!(r.read_u16::<B>());
    if version < 2 || version > 4 { // TODO: is this correct?
        return Err(ParseError::Unsupported(format!("compilation unit version {}", version)))
    }

    let abbrev_offset = try!(parse_offset::<B>(r, sections.debug_abbrev.len()));
    let address_size = try!(r.read_u8());

    let abbrev_hash = try!(parse_abbrev(&sections.debug_abbrev[abbrev_offset..]));
    let die = try!(parse_die_children::<B>(sections, r, &abbrev_hash, address_size));
    Ok(CompilationUnit {die: die})
}

fn parse_die_children<'a, B: ByteOrder>(sections: &'a Sections, r: &mut &'a [u8], abbrev_hash: &AbbrevHash, address_size: u8) -> Result<Vec<Die<'a>>, ParseError> {
    let mut die = Vec::new();
    while r.len() > 0 {
        let code = try!(leb128::read_u64(r));
        if code == 0 {
            break;
        }

        let abbrev = match abbrev_hash.get(&code) {
            Some(abbrev) => abbrev,
            None => return Err(ParseError::Invalid(format!("missing abbrev {}", code))),
        };

        let mut attribute = Vec::new();
        for abbrev_attribute in &abbrev.attribute {
            let data = try!(parse_attribute_data::<B>(sections, r, abbrev_attribute.form, address_size));
            attribute.push(Attribute {
                at: abbrev_attribute.at,
                data: data,
            });
        }

        let children = if abbrev.children {
            try!(parse_die_children::<B>(sections, r, abbrev_hash, address_size))
        } else {
            Vec::new()
        };

        die.push(Die {
            tag: abbrev.tag,
            attribute: attribute,
            children: children,
        });
    }
    Ok(die)
}

fn parse_attribute_data<'a, B: ByteOrder>(sections: &'a Sections, r: &mut &'a [u8], form: constant::DwForm, address_size: u8) -> Result<AttributeData<'a>, ParseError> {
    let data = match form {
        constant::DW_FORM_addr => AttributeData::Address(try!(parse_address::<B>(r, address_size))),
        constant::DW_FORM_block2 => {
            let len = try!(r.read_u16::<B>()) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        },
        constant::DW_FORM_block4 => {
            let len = try!(r.read_u32::<B>()) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        },
        constant::DW_FORM_data2 => AttributeData::Data2(try!(r.read_u16::<B>())),
        constant::DW_FORM_data4 => AttributeData::Data4(try!(r.read_u32::<B>())),
        constant::DW_FORM_data8 => AttributeData::Data8(try!(r.read_u64::<B>())),
        constant::DW_FORM_string => AttributeData::String(try!(parse_string(r))),
        constant::DW_FORM_block => {
            let len = try!(leb128::read_u64(r)) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        },
        constant::DW_FORM_block1 => {
            let len = try!(r.read_u8()) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        },
        constant::DW_FORM_data1 => AttributeData::Data1(try!(r.read_u8())),
        constant::DW_FORM_flag => AttributeData::Flag(try!(r.read_u8()) != 0),
        constant::DW_FORM_sdata => AttributeData::SData(try!(leb128::read_i64(r))),
        constant::DW_FORM_strp => {
            let offset = try!(parse_offset::<B>(r, sections.debug_str.len()));
            let mut str_r = &sections.debug_str[offset..];
            let val = try!(parse_string(&mut str_r));
            AttributeData::String(val)
        },
        constant::DW_FORM_udata => AttributeData::UData(try!(leb128::read_u64(r))),
        constant::DW_FORM_ref_addr => AttributeData::RefAddress(try!(parse_address::<B>(r, address_size))),
        constant::DW_FORM_ref1 => AttributeData::Ref(try!(r.read_u8()) as usize),
        constant::DW_FORM_ref2 => AttributeData::Ref(try!(r.read_u16::<B>()) as usize),
        constant::DW_FORM_ref4 => AttributeData::Ref(try!(r.read_u32::<B>()) as usize),
        constant::DW_FORM_ref8 => AttributeData::Ref(try!(r.read_u64::<B>()) as usize),
        constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r)) as usize),
        constant::DW_FORM_indirect => {
            let val = try!(leb128::read_u16(r));
            try!(parse_attribute_data::<B>(sections, r, constant::DwForm(val), address_size))
        },
        constant::DW_FORM_sec_offset => {
            // TODO: validate based on class
            AttributeData::SecOffset(try!(parse_offset::<B>(r, std::usize::MAX)))
        },
        constant::DW_FORM_exprloc => {
            let len = try!(leb128::read_u64(r)) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::ExprLoc(val)
        },
        constant::DW_FORM_flag_present => AttributeData::Flag(true),
        constant::DW_FORM_ref_sig8 => AttributeData::RefSig(try!(r.read_u64::<B>())),
        _ => return Err(ParseError::Unsupported(format!("attribute form {}", form.0))),
    };
    Ok(data)
}

fn parse_offset<B: ByteOrder>(r: &mut &[u8], len: usize) -> Result<usize, ParseError> {
    // TODO: 64 bit
    let offset = try!(r.read_u32::<B>()) as usize;
    if offset >= len {
        return Err(ParseError::Invalid(format!("offset {} > {}", offset, len)))
    }
    Ok(offset)
}

fn parse_block<'a>(r: &mut &'a[u8], len: usize) -> Result<&'a [u8], ParseError> {
    if len > r.len() {
        return Err(ParseError::Invalid(format!("block length {} > {}", len, r.len())))
    }
    let val = &r[..len];
    *r = &r[len..];
    Ok(val)
}

fn parse_string<'a>(r: &mut &'a[u8]) -> Result<&'a str, ParseError> {
    let len = match r.iter().position(|&x| x == 0) {
        Some(len) => len,
        None => return Err(ParseError::Invalid(format!("unterminated string"))),
    };
    let val = try!(std::str::from_utf8(&r[..len]));
    *r = &r[len+1..];
    Ok(val)
}

fn parse_address<B: ByteOrder>(r: &mut &[u8], address_size: u8) -> Result<usize, ParseError> {
    let val = match address_size {
        4 => try!(r.read_u32::<B>()) as usize,
        8 => try!(r.read_u64::<B>()) as usize,
        _ => return Err(ParseError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(val)
}

type AbbrevHash = std::collections::HashMap<u64, Abbrev>;

fn parse_abbrev(mut abbrev: &[u8]) -> Result<AbbrevHash, ParseError> {
    let mut abbrev_hash = AbbrevHash::new();
    loop {
        let code = try!(leb128::read_u64(&mut abbrev));
        if code == 0 {
            break;
        }

        let tag = try!(leb128::read_u16(&mut abbrev));
        let children = match constant::DwChildren(try!(abbrev.read_u8())) {
            constant::DW_CHILDREN_no => false,
            constant::DW_CHILDREN_yes => true,
            val => return Err(ParseError::Invalid(format!("DW_CHILDREN {}", val.0))),
        };

        let mut attribute = Vec::new();
        loop {
            let at = try!(leb128::read_u16(&mut abbrev));
            let form = try!(leb128::read_u16(&mut abbrev));
            if at == 0 && form == 0 {
                break;
            }
            attribute.push(AbbrevAttribute {
                at: constant::DwAt(at),
                form: constant::DwForm(form),
            });
        }

        if abbrev_hash.insert(code, Abbrev {
            tag: constant::DwTag(tag),
            children: children,
            attribute: attribute
        }).is_some() {
            return Err(ParseError::Invalid(format!("duplicate abbrev code {}", code)))
        }
    }
    Ok(abbrev_hash)
}
