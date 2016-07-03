use std;
use byteorder;
use byteorder::{ReadBytesExt};

use super::*;
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

impl Endian {
    fn read_u16(&self, r: &mut &[u8]) -> Result<u16, std::io::Error> {
        match *self {
            Endian::Little => r.read_u16::<byteorder::LittleEndian>(),
            Endian::Big => r.read_u16::<byteorder::BigEndian>(),
        }
    }

    fn read_u32(&self, r: &mut &[u8]) -> Result<u32, std::io::Error> {
        match *self {
            Endian::Little => r.read_u32::<byteorder::LittleEndian>(),
            Endian::Big => r.read_u32::<byteorder::BigEndian>(),
        }
    }

    fn read_u64(&self, r: &mut &[u8]) -> Result<u64, std::io::Error> {
        match *self {
            Endian::Little => r.read_u64::<byteorder::LittleEndian>(),
            Endian::Big => r.read_u64::<byteorder::BigEndian>(),
        }
    }
}

impl Sections {
    pub fn compilation_unit_headers(&self) -> CompilationUnitHeaderIterator {
        CompilationUnitHeaderIterator::new(self)
    }

    pub fn compilation_units(&self) -> CompilationUnitIterator {
        CompilationUnitIterator::new(self)
    }
}

#[derive(Debug)]
pub struct CompilationUnitHeaderIterator<'a> {
    sections: &'a Sections,
    info: &'a [u8],
}

impl<'a> CompilationUnitHeaderIterator<'a> {
    fn new(sections: &'a Sections) -> Self {
        CompilationUnitHeaderIterator {
            sections: sections,
            info: &sections.debug_info[..],
        }
    }
}

impl<'a> FallibleIterator for CompilationUnitHeaderIterator<'a> {
    type Item = CompilationUnitHeader<'a>;
    type Error = ParseError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if self.info.len() == 0 {
            return Ok(None);
        }

        let len = try!(self.sections.endian.read_u32(&mut self.info)) as usize;
        // TODO: 64 bit
        if len >= 0xfffffff0 {
            return Err(ParseError::Unsupported(format!("compilation unit length {}", len)));
        }
        if len > self.info.len() {
            return Err(ParseError::Invalid(format!("compilation unit length {}", len)));
        }
        let result = try!(CompilationUnitHeader::new(self.sections, &self.info[..len]));
        self.info = &self.info[len..];
        Ok(Some(result))
    }
}

impl<'a> CompilationUnitHeader<'a> {
    pub fn new(
        sections: &'a Sections,
        mut r: &'a [u8]
    ) -> Result<CompilationUnitHeader<'a>, ParseError> {
        let r = &mut r;

        let version = try!(sections.endian.read_u16(r));
        if version < 2 || version > 4 { // TODO: is this correct?
            return Err(ParseError::Unsupported(format!("compilation unit version {}", version)));
        }

        let abbrev_offset = try!(parse_offset(sections, r, sections.debug_abbrev.len()));
        let abbrev = &sections.debug_abbrev[abbrev_offset..];

        let address_size = try!(r.read_u8());

        Ok(CompilationUnitHeader {
            sections: sections,
            version: version,
            address_size: address_size,
            abbrev: abbrev,
            data: r,
        })
    }

    pub fn parse(&self) -> Result<CompilationUnit<'a>, ParseError> {
        let mut iter = try!(self.entries());
        let mut entries = Vec::new();
        try!(iter.next_children(&mut entries));
        Ok(CompilationUnit { die: entries })
    }

    pub fn entries(&self) -> Result<DebuggingInformationEntryIterator<'a>, ParseError> {
        let abbrev = try!(parse_abbrev(self.abbrev));
        Ok(DebuggingInformationEntryIterator::new(self.sections, self.address_size, abbrev, self.data))
    }
}

#[derive(Debug)]
pub struct CompilationUnitIterator<'a>(CompilationUnitHeaderIterator<'a>);

impl<'a> CompilationUnitIterator<'a> {
    fn new(sections: &'a Sections) -> Self {
        CompilationUnitIterator(CompilationUnitHeaderIterator::new(sections))
    }
}

impl<'a> FallibleIterator for CompilationUnitIterator<'a> {
    type Item = CompilationUnit<'a>;
    type Error = ParseError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        match self.0.next() {
            Ok(Some(header)) => Ok(Some(try!(header.parse()))),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub struct DebuggingInformationEntryIterator<'a> {
    sections: &'a Sections,
    address_size: u8,
    abbrev: AbbrevHash,
    data: &'a [u8],
}

impl<'a> DebuggingInformationEntryIterator<'a> {
    fn new(sections: &'a Sections, address_size: u8, abbrev: AbbrevHash, data: &'a [u8]) -> Self {
        DebuggingInformationEntryIterator {
            sections: sections,
            address_size: address_size,
            abbrev: abbrev,
            data: data,
        }
    }

    fn next_children(&mut self, children: &mut Vec<Die<'a>>) -> Result<(), ParseError> {
        loop {
            let mut child = match try!(self.next()) {
                Some(child) => child,
                None => break,
            };
            if child.tag == constant::DwTag(0) {
                break;
            }

            match child.children {
                Some(ref mut children) => try!(self.next_children(children)),
                None => {},
            }

            children.push(child);
        }
        Ok(())
    }
}

impl<'a> FallibleIterator for DebuggingInformationEntryIterator<'a> {
    type Item = Die<'a>;
    type Error = ParseError;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        let code = try!(leb128::read_u64(&mut self.data));
        if code == 0 {
            return Ok(Some(Die {
                tag: constant::DwTag(0),
                attributes: Vec::new(),
                children: None,
            }));
        }

        let abbrev = match self.abbrev.get(&code) {
            Some(abbrev) => abbrev,
            None => return Err(ParseError::Invalid(format!("missing abbrev {}", code))),
        };

        let mut attributes = Vec::new();
        for abbrev_attribute in &abbrev.attributes {
            let data = try!(parse_attribute_data(self.sections, &mut self.data, abbrev_attribute.form, self.address_size));
            attributes.push(Attribute {
                at: abbrev_attribute.at,
                data: data,
            });
        }

        let children = if abbrev.children {
            Some(Vec::new())
        } else {
            None
        };

        Ok(Some(Die {
            tag: abbrev.tag,
            attributes: attributes,
            children: children,
        }))
    }
}

fn parse_attribute_data<'a>(
    sections: &'a Sections,
    r: &mut &'a [u8],
    form: constant::DwForm,
    address_size: u8
) -> Result<AttributeData<'a>, ParseError> {
    let data = match form {
        constant::DW_FORM_addr => AttributeData::Address(try!(parse_address(sections, r, address_size))),
        constant::DW_FORM_block2 => {
            let len = try!(sections.endian.read_u16(r)) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_block4 => {
            let len = try!(sections.endian.read_u32(r)) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_data2 => AttributeData::Data2(try!(sections.endian.read_u16(r))),
        constant::DW_FORM_data4 => AttributeData::Data4(try!(sections.endian.read_u32(r))),
        constant::DW_FORM_data8 => AttributeData::Data8(try!(sections.endian.read_u64(r))),
        constant::DW_FORM_string => AttributeData::String(try!(parse_string(r))),
        constant::DW_FORM_block => {
            let len = try!(leb128::read_u64(r)) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_block1 => {
            let len = try!(r.read_u8()) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_data1 => AttributeData::Data1(try!(r.read_u8())),
        constant::DW_FORM_flag => AttributeData::Flag(try!(r.read_u8()) != 0),
        constant::DW_FORM_sdata => AttributeData::SData(try!(leb128::read_i64(r))),
        constant::DW_FORM_strp => {
            let offset = try!(parse_offset(sections, r, sections.debug_str.len()));
            let mut str_r = &sections.debug_str[offset..];
            let val = try!(parse_string(&mut str_r));
            AttributeData::String(val)
        }
        constant::DW_FORM_udata => AttributeData::UData(try!(leb128::read_u64(r))),
        constant::DW_FORM_ref_addr => AttributeData::RefAddress(try!(parse_address(sections, r, address_size))),
        constant::DW_FORM_ref1 => AttributeData::Ref(try!(r.read_u8()) as usize),
        constant::DW_FORM_ref2 => AttributeData::Ref(try!(sections.endian.read_u16(r)) as usize),
        constant::DW_FORM_ref4 => AttributeData::Ref(try!(sections.endian.read_u32(r)) as usize),
        constant::DW_FORM_ref8 => AttributeData::Ref(try!(sections.endian.read_u64(r)) as usize),
        constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r)) as usize),
        constant::DW_FORM_indirect => {
            let val = try!(leb128::read_u16(r));
            try!(parse_attribute_data(sections, r, constant::DwForm(val), address_size))
        }
        constant::DW_FORM_sec_offset => {
            // TODO: validate based on class
            AttributeData::SecOffset(try!(parse_offset(sections, r, std::usize::MAX)))
        }
        constant::DW_FORM_exprloc => {
            let len = try!(leb128::read_u64(r)) as usize;
            let val = try!(parse_block(r, len));
            AttributeData::ExprLoc(val)
        }
        constant::DW_FORM_flag_present => AttributeData::Flag(true),
        constant::DW_FORM_ref_sig8 => AttributeData::RefSig(try!(sections.endian.read_u64(r))),
        _ => return Err(ParseError::Unsupported(format!("attribute form {}", form.0))),
    };
    Ok(data)
}

fn parse_offset(sections: &Sections, r: &mut &[u8], len: usize) -> Result<usize, ParseError> {
    // TODO: 64 bit
    let offset = try!(sections.endian.read_u32(r)) as usize;
    if offset >= len {
        return Err(ParseError::Invalid(format!("offset {} > {}", offset, len)));
    }
    Ok(offset)
}

fn parse_block<'a>(r: &mut &'a [u8], len: usize) -> Result<&'a [u8], ParseError> {
    if len > r.len() {
        return Err(ParseError::Invalid(format!("block length {} > {}", len, r.len())));
    }
    let val = &r[..len];
    *r = &r[len..];
    Ok(val)
}

fn parse_string<'a>(r: &mut &'a [u8]) -> Result<&'a str, ParseError> {
    let len = match r.iter().position(|&x| x == 0) {
        Some(len) => len,
        None => return Err(ParseError::Invalid("unterminated string".to_string())),
    };
    let val = try!(std::str::from_utf8(&r[..len]));
    *r = &r[len + 1..];
    Ok(val)
}

fn parse_address(sections: &Sections, r: &mut &[u8], address_size: u8) -> Result<usize, ParseError> {
    let val = match address_size {
        4 => try!(sections.endian.read_u32(r)) as usize,
        8 => try!(sections.endian.read_u64(r)) as usize,
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

        let mut attributes = Vec::new();
        loop {
            let at = try!(leb128::read_u16(&mut abbrev));
            let form = try!(leb128::read_u16(&mut abbrev));
            if at == 0 && form == 0 {
                break;
            }
            attributes.push(AbbrevAttribute {
                at: constant::DwAt(at),
                form: constant::DwForm(form),
            });
        }

        if abbrev_hash.insert(code, Abbrev {
            tag: constant::DwTag(tag),
            children: children,
            attributes: attributes,
        }).is_some() {
            return Err(ParseError::Invalid(format!("duplicate abbrev code {}", code)));
        }
    }
    Ok(abbrev_hash)
}
