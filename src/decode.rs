use std;
use byteorder;
use byteorder::{ReadBytesExt};

use super::*;
use leb128;

#[derive(Debug)]
pub enum DecodeError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Invalid(String),
    Unsupported(String),
}

impl std::convert::From<std::io::Error> for DecodeError {
    fn from(e: std::io::Error) -> Self {
        DecodeError::Io(e)
    }
}

impl std::convert::From<std::str::Utf8Error> for DecodeError {
    fn from(e: std::str::Utf8Error) -> Self {
        DecodeError::Utf8(e)
    }
}

impl std::convert::From<leb128::Error> for DecodeError {
    fn from(e: leb128::Error) -> Self {
        match e {
            leb128::Error::Io(e) => DecodeError::Io(e),
            leb128::Error::Overflow => DecodeError::Invalid("LEB128 overflow".to_string()),
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
    pub fn compilation_units(&self) -> CompilationUnitIterator {
        CompilationUnitIterator::new(self)
    }
}

impl<'a> CompilationUnitIterator<'a> {
    fn new(sections: &'a Sections) -> Self {
        CompilationUnitIterator {
            sections: sections,
            info: &sections.debug_info[..],
        }
    }

    pub fn next(&mut self) -> Result<Option<CompilationUnit<'a>>, DecodeError> {
        if self.info.len() == 0 {
            return Ok(None);
        }

        let len = try!(self.sections.endian.read_u32(&mut self.info)) as usize;
        // TODO: 64 bit
        if len >= 0xfffffff0 {
            return Err(DecodeError::Unsupported(format!("compilation unit length {}", len)));
        }
        if len > self.info.len() {
            return Err(DecodeError::Invalid(format!("compilation unit length {}", len)));
        }
        let result = try!(CompilationUnit::new(self.sections, &self.info[..len]));
        self.info = &self.info[len..];
        Ok(Some(result))
    }
}

impl<'a> CompilationUnit<'a> {
    pub fn new(
        sections: &'a Sections,
        mut r: &'a [u8]
    ) -> Result<CompilationUnit<'a>, DecodeError> {
        let r = &mut r;

        let version = try!(sections.endian.read_u16(r));
        if version < 2 || version > 4 { // TODO: is this correct?
            return Err(DecodeError::Unsupported(format!("compilation unit version {}", version)));
        }

        let abbrev_offset = try!(decode_offset(r, sections.endian, sections.debug_abbrev.len()));
        let abbrev = try!(AbbrevHash::decode(&sections.debug_abbrev[abbrev_offset..]));

        let address_size = try!(r.read_u8());

        Ok(CompilationUnit {
            sections: sections,
            version: version,
            address_size: address_size,
            abbrev: abbrev,
            data: r,
        })
    }

    pub fn entries(&'a self) -> Result<DieCursor<'a>, DecodeError> {
        Ok(DieCursor::new(self, &self.data, 0))
    }

    pub fn entry(&'a self, offset: usize) -> Result<DieCursor<'a>, DecodeError> {
        if offset >= self.data.len() {
            return Err(DecodeError::Invalid(format!("offset {} > {}", offset, self.data.len())));
        }
        Ok(DieCursor::new(self, &self.data[offset..], offset))
    }
}

impl<'a> DieCursor<'a> {
    fn new(unit: &'a CompilationUnit<'a>, data: &'a [u8], offset: usize) -> Self {
        DieCursor {
            unit: unit,
            data: data,
            offset: offset,
            next_child: false,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<Die<'a>>, DecodeError> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        let mut r = self.data;
        let mut die = try!(Die::decode(&mut r, self.unit));
        self.next_child = die.children;
        die.offset = self.offset;
        self.offset += self.data.len() - r.len();
        self.data = r;
        Ok(Some(die))
    }

    pub fn next_sibling(&mut self) -> Result<Option<Die<'a>>, DecodeError> {
        if self.next_child {
            self.next_child = false;
            loop {
                match try!(self.next_sibling()) {
                    Some(die) => if die.is_null() { break; },
                    None => return Ok(None),
                }
            }
        }
        self.next()
    }
}

impl<'a> Die<'a> {
    pub fn null() -> Self {
        Die {
            offset: 0,
            tag: constant::DwTag(0),
            children: false,
            attributes: Vec::new(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.tag == constant::DwTag(0)
    }

    pub fn decode(r: &mut &'a [u8], unit: &'a CompilationUnit<'a>) -> Result<Die<'a>, DecodeError> {
        let code = try!(leb128::read_u64(r));
        if code == 0 {
            return Ok(Die::null());
        }

        let abbrev = match unit.abbrev.get(code) {
            Some(abbrev) => abbrev,
            None => return Err(DecodeError::Invalid(format!("missing abbrev {}", code))),
        };

        let mut attributes = Vec::new();
        for abbrev_attribute in &abbrev.attributes {
            attributes.push(try!(Attribute::decode(r, unit, abbrev_attribute)));
        }

        Ok(Die {
            offset: 0,
            tag: abbrev.tag,
            children: abbrev.children,
            attributes: attributes,
        })
    }
}

impl<'a> Attribute<'a> {
    pub fn decode(
        r: &mut &'a [u8],
        unit: &'a CompilationUnit<'a>,
        abbrev: &AbbrevAttribute,
    ) -> Result<Attribute<'a>, DecodeError> {
        let data = try!(decode_attribute_data(r, unit, abbrev.form));
        Ok(Attribute {
            at: abbrev.at,
            data: data,
        })
    }
}

fn decode_attribute_data<'a>(
    r: &mut &'a [u8],
    unit: &'a CompilationUnit<'a>,
    form: constant::DwForm,
) -> Result<AttributeData<'a>, DecodeError> {
    let endian = unit.sections.endian;
    let data = match form {
        constant::DW_FORM_addr => AttributeData::Address(try!(decode_address(r, endian, unit.address_size))),
        constant::DW_FORM_block2 => {
            let len = try!(endian.read_u16(r)) as usize;
            let val = try!(decode_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_block4 => {
            let len = try!(endian.read_u32(r)) as usize;
            let val = try!(decode_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_data2 => AttributeData::Data2(try!(endian.read_u16(r))),
        constant::DW_FORM_data4 => AttributeData::Data4(try!(endian.read_u32(r))),
        constant::DW_FORM_data8 => AttributeData::Data8(try!(endian.read_u64(r))),
        constant::DW_FORM_string => AttributeData::String(try!(decode_string(r))),
        constant::DW_FORM_block => {
            let len = try!(leb128::read_u64(r)) as usize;
            let val = try!(decode_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_block1 => {
            let len = try!(r.read_u8()) as usize;
            let val = try!(decode_block(r, len));
            AttributeData::Block(val)
        }
        constant::DW_FORM_data1 => AttributeData::Data1(try!(r.read_u8())),
        constant::DW_FORM_flag => AttributeData::Flag(try!(r.read_u8()) != 0),
        constant::DW_FORM_sdata => AttributeData::SData(try!(leb128::read_i64(r))),
        constant::DW_FORM_strp => {
            let offset = try!(decode_offset(r, endian, unit.sections.debug_str.len()));
            let mut str_r = &unit.sections.debug_str[offset..];
            let val = try!(decode_string(&mut str_r));
            AttributeData::String(val)
        }
        constant::DW_FORM_udata => AttributeData::UData(try!(leb128::read_u64(r))),
        constant::DW_FORM_ref_addr => AttributeData::RefAddress(try!(decode_address(r, endian, unit.address_size))),
        constant::DW_FORM_ref1 => AttributeData::Ref(try!(r.read_u8()) as usize),
        constant::DW_FORM_ref2 => AttributeData::Ref(try!(endian.read_u16(r)) as usize),
        constant::DW_FORM_ref4 => AttributeData::Ref(try!(endian.read_u32(r)) as usize),
        constant::DW_FORM_ref8 => AttributeData::Ref(try!(endian.read_u64(r)) as usize),
        constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r)) as usize),
        constant::DW_FORM_indirect => {
            let val = try!(leb128::read_u16(r));
            try!(decode_attribute_data(r, unit, constant::DwForm(val)))
        }
        constant::DW_FORM_sec_offset => {
            // TODO: validate based on class
            AttributeData::SecOffset(try!(decode_offset(r, endian, std::usize::MAX)))
        }
        constant::DW_FORM_exprloc => {
            let len = try!(leb128::read_u64(r)) as usize;
            let val = try!(decode_block(r, len));
            AttributeData::ExprLoc(val)
        }
        constant::DW_FORM_flag_present => AttributeData::Flag(true),
        constant::DW_FORM_ref_sig8 => AttributeData::RefSig(try!(endian.read_u64(r))),
        _ => return Err(DecodeError::Unsupported(format!("attribute form {}", form.0))),
    };
    Ok(data)
}

fn decode_offset(r: &mut &[u8], endian: Endian, len: usize) -> Result<usize, DecodeError> {
    // TODO: 64 bit
    let offset = try!(endian.read_u32(r)) as usize;
    if offset >= len {
        return Err(DecodeError::Invalid(format!("offset {} > {}", offset, len)));
    }
    Ok(offset)
}

fn decode_block<'a>(r: &mut &'a [u8], len: usize) -> Result<&'a [u8], DecodeError> {
    if len > r.len() {
        return Err(DecodeError::Invalid(format!("block length {} > {}", len, r.len())));
    }
    let val = &r[..len];
    *r = &r[len..];
    Ok(val)
}

fn decode_string<'a>(r: &mut &'a [u8]) -> Result<&'a str, DecodeError> {
    let len = match r.iter().position(|&x| x == 0) {
        Some(len) => len,
        None => return Err(DecodeError::Invalid("unterminated string".to_string())),
    };
    let val = try!(std::str::from_utf8(&r[..len]));
    *r = &r[len + 1..];
    Ok(val)
}

fn decode_address(r: &mut &[u8], endian: Endian, address_size: u8) -> Result<usize, DecodeError> {
    let val = match address_size {
        4 => try!(endian.read_u32(r)) as usize,
        8 => try!(endian.read_u64(r)) as usize,
        _ => return Err(DecodeError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(val)
}

impl AbbrevHash {
    pub fn decode(mut r: &[u8]) -> Result<AbbrevHash, DecodeError> {
        let mut abbrev_hash = std::collections::HashMap::new();
        while let Some((code, abbrev)) = try!(Abbrev::decode(&mut r)) {
            if abbrev_hash.insert(code, abbrev).is_some() {
                return Err(DecodeError::Invalid(format!("duplicate abbrev code {}", code)));
            }
        }
        Ok(AbbrevHash(abbrev_hash))
    }

    pub fn get(&self, code: u64) -> Option<&Abbrev> {
        self.0.get(&code)
    }
}

impl Abbrev {
    pub fn decode(r: &mut &[u8]) -> Result<Option<(u64, Abbrev)>, DecodeError> {
        let code = try!(leb128::read_u64(r));
        if code == 0 {
            return Ok(None);
        }

        let tag = try!(leb128::read_u16(r));

        let children = match constant::DwChildren(try!(r.read_u8())) {
            constant::DW_CHILDREN_no => false,
            constant::DW_CHILDREN_yes => true,
            val => return Err(DecodeError::Invalid(format!("DW_CHILDREN {}", val.0))),
        };

        let mut attributes = Vec::new();
        while let Some(attribute) = try!(AbbrevAttribute::decode(r)) {
            attributes.push(attribute);
        }

        Ok(Some((code, Abbrev {
            tag: constant::DwTag(tag),
            children: children,
            attributes: attributes,
        })))
    }
}

impl AbbrevAttribute {
    pub fn decode(r: &mut &[u8]) -> Result<Option<AbbrevAttribute>, DecodeError> {
        let at = try!(leb128::read_u16(r));
        let form = try!(leb128::read_u16(r));
        if at == 0 && form == 0 {
            Ok(None)
        } else {
            Ok(Some(AbbrevAttribute {
                at: constant::DwAt(at),
                form: constant::DwForm(form),
            }))
        }
    }
}
