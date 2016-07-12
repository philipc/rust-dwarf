use std;
use std::borrow::Cow;
use std::io::Read;
use std::ops::Deref;
use byteorder::{ReadBytesExt};

use super::*;
use leb128;

#[derive(Debug)]
pub enum ReadError {
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
    Invalid(String),
    Unsupported(String),
}

impl std::convert::From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        ReadError::Io(e)
    }
}

impl std::convert::From<std::str::Utf8Error> for ReadError {
    fn from(e: std::str::Utf8Error) -> Self {
        ReadError::Utf8(e)
    }
}

impl std::convert::From<leb128::Error> for ReadError {
    fn from(e: leb128::Error) -> Self {
        match e {
            leb128::Error::Io(e) => ReadError::Io(e),
            leb128::Error::Overflow => ReadError::Invalid("LEB128 overflow".to_string()),
        }
    }
}

impl Sections {
    pub fn compilation_units(&self) -> CompilationUnitIterator {
        CompilationUnitIterator::new(self)
    }
}

#[cfg_attr(feature = "clippy", allow(should_implement_trait))]
impl<'a> CompilationUnitIterator<'a> {
    fn new(sections: &'a Sections) -> Self {
        CompilationUnitIterator {
            sections: sections,
            data: &sections.debug_info[..],
            offset: 0,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<CompilationUnit<'a>>, ReadError> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        let mut r = self.data;
        let unit = try!(CompilationUnit::read(&mut r, self.sections, self.offset));
        self.offset += self.data.len() - r.len();
        self.data = r;
        Ok(Some(unit))
    }
}

impl<'a> CompilationUnit<'a> {
    pub fn read(
        r: &mut &'a [u8],
        sections: &'a Sections,
        offset: usize,
    ) -> Result<CompilationUnit<'a>, ReadError> {
        let total_len = r.len();

        let len = try!(sections.endian.read_u32(r)) as usize;
        // TODO: 64 bit
        if len >= 0xfffffff0 {
            return Err(ReadError::Unsupported(format!("compilation unit length {}", len)));
        }
        if len > r.len() {
            return Err(ReadError::Invalid(format!("compilation unit length {}", len)));
        }

        // Tell the caller we read the entire length, even if we don't parse it all now
        let mut data = &r[..len];
        *r = &r[len..];

        let version = try!(sections.endian.read_u16(&mut data));
        // TODO: is this correct?
        if version < 2 || version > 4 {
            return Err(ReadError::Unsupported(format!("compilation unit version {}", version)));
        }

        let abbrev_offset = try!(read_offset(&mut data, sections.endian));
        let address_size = try!(data.read_u8());

        // Calculate offset of first DIE
        let data_offset = offset + (total_len - r.len() - data.len());

        Ok(CompilationUnit {
            offset: offset,
            version: version,
            address_size: address_size,
            abbrev_offset: abbrev_offset,
            data: data,
            data_offset: data_offset,
        })
    }

    pub fn abbrev(&'a self, sections: &'a Sections) -> Result<AbbrevHash, ReadError> {
        let offset = self.abbrev_offset;
        let len = sections.debug_abbrev.len();
        if offset >= len {
            return Err(ReadError::Invalid(format!("abbrev offset {} > {}", offset, len)));
        }
        AbbrevHash::read(&mut &sections.debug_abbrev[offset..])
    }

    pub fn die_buffer(&'a self, sections: &'a Sections) -> DieBuffer<'a> {
        DieBuffer::new(
            sections.endian,
            self.address_size,
            Cow::Borrowed(self.data),
            self.data_offset,
        )
    }
}

impl<'a> DieBuffer<'a> {
    pub fn entries(&'a self, abbrev: &'a AbbrevHash) -> DieCursor<'a> {
        DieCursor::new(self.data.deref(), self.offset, self, abbrev)
    }

    pub fn entry(&'a self, offset: usize, abbrev: &'a AbbrevHash) -> Option<DieCursor<'a>> {
        if offset < self.offset {
            return None;
        }
        let relative_offset = offset - self.offset;
        if relative_offset >= self.data.len() {
            return None;
        }
        Some(DieCursor::new(&self.data[relative_offset..], offset, self, abbrev))
    }
}

#[cfg_attr(feature = "clippy", allow(should_implement_trait))]
impl<'a> DieCursor<'a> {
    pub fn new(r: &'a [u8], offset: usize, buffer: &'a DieBuffer<'a>, abbrev: &'a AbbrevHash) -> Self {
        DieCursor {
            r: r,
            offset: offset,
            buffer: buffer,
            abbrev: abbrev,
            next_child: false,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<Die<'a>>, ReadError> {
        if self.r.len() == 0 {
            return Ok(None);
        }

        let mut r = self.r;
        let die = try!(Die::read(&mut r, self.offset, self.buffer, self.abbrev));
        self.next_child = die.children;
        self.offset += self.r.len() - r.len();
        self.r = r;
        Ok(Some(die))
    }

    pub fn next_sibling(&mut self) -> Result<Option<Die<'a>>, ReadError> {
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
    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        buffer: &'a DieBuffer<'a>,
        abbrev_hash: &'a AbbrevHash,
    ) -> Result<Die<'a>, ReadError> {
        let code = try!(leb128::read_u64(r));
        if code == 0 {
            return Ok(Die::null(offset));
        }

        let abbrev = match abbrev_hash.get(code) {
            Some(abbrev) => abbrev,
            None => return Err(ReadError::Invalid(format!("missing abbrev {}", code))),
        };

        let mut attributes = Vec::new();
        for abbrev_attribute in &abbrev.attributes {
            attributes.push(try!(Attribute::read(r, buffer, abbrev_attribute)));
        }

        Ok(Die {
            offset: offset,
            code: code,
            tag: abbrev.tag,
            children: abbrev.children,
            attributes: attributes,
        })
    }
}

impl<'a> Attribute<'a> {
    pub fn read(
        r: &mut &'a [u8],
        buffer: &'a DieBuffer<'a>,
        abbrev: &AbbrevAttribute,
    ) -> Result<Attribute<'a>, ReadError> {
        let data = try!(AttributeData::read(r, buffer, abbrev.form));
        Ok(Attribute {
            at: abbrev.at,
            data: data,
        })
    }
}

impl<'a> AttributeData<'a> {
    pub fn read(
        r: &mut &'a [u8],
        buffer: &'a DieBuffer<'a>,
        form: constant::DwForm,
    ) -> Result<AttributeData<'a>, ReadError> {
        let data = match form {
            constant::DW_FORM_addr => {
                let val = try!(read_address(r, buffer.endian, buffer.address_size));
                AttributeData::Address(val)
            }
            constant::DW_FORM_block2 => {
                let len = try!(buffer.endian.read_u16(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_block4 => {
                let len = try!(buffer.endian.read_u32(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_data2 => AttributeData::Data2(try!(buffer.endian.read_u16(r))),
            constant::DW_FORM_data4 => AttributeData::Data4(try!(buffer.endian.read_u32(r))),
            constant::DW_FORM_data8 => AttributeData::Data8(try!(buffer.endian.read_u64(r))),
            constant::DW_FORM_string => AttributeData::String(try!(read_string(r))),
            constant::DW_FORM_block => {
                let len = try!(leb128::read_u64(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_block1 => {
                let len = try!(r.read_u8()) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_data1 => AttributeData::Data1(try!(r.read_u8())),
            constant::DW_FORM_flag => AttributeData::Flag(try!(r.read_u8()) != 0),
            constant::DW_FORM_sdata => AttributeData::SData(try!(leb128::read_i64(r))),
            constant::DW_FORM_strp => {
                let val = try!(read_offset(r, buffer.endian));
                AttributeData::StringOffset(val)
            }
            constant::DW_FORM_udata => AttributeData::UData(try!(leb128::read_u64(r))),
            constant::DW_FORM_ref_addr => {
                let val = try!(read_address(r, buffer.endian, buffer.address_size));
                AttributeData::RefAddress(val)
            }
            constant::DW_FORM_ref1 => AttributeData::Ref(try!(r.read_u8()) as usize),
            constant::DW_FORM_ref2 => AttributeData::Ref(try!(buffer.endian.read_u16(r)) as usize),
            constant::DW_FORM_ref4 => AttributeData::Ref(try!(buffer.endian.read_u32(r)) as usize),
            constant::DW_FORM_ref8 => AttributeData::Ref(try!(buffer.endian.read_u64(r)) as usize),
            constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r)) as usize),
            constant::DW_FORM_indirect => {
                let val = try!(leb128::read_u16(r));
                try!(AttributeData::read(r, buffer, constant::DwForm(val)))
            }
            constant::DW_FORM_sec_offset => {
                // TODO: validate based on class
                let val = try!(read_offset(r, buffer.endian));
                AttributeData::SecOffset(val)
            }
            constant::DW_FORM_exprloc => {
                let len = try!(leb128::read_u64(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::ExprLoc(val)
            }
            constant::DW_FORM_flag_present => AttributeData::Flag(true),
            constant::DW_FORM_ref_sig8 => AttributeData::RefSig(try!(buffer.endian.read_u64(r))),
            _ => return Err(ReadError::Unsupported(format!("attribute form {}", form.0))),
        };
        Ok(data)
    }
}

fn read_offset<R: Read>(r: &mut R, endian: Endian) -> Result<usize, ReadError> {
    // TODO: 64 bit
    let offset = try!(endian.read_u32(r)) as usize;
    Ok(offset)
}

fn read_block<'a>(r: &mut &'a [u8], len: usize) -> Result<&'a [u8], ReadError> {
    if len > r.len() {
        return Err(ReadError::Invalid(format!("block length {} > {}", len, r.len())));
    }
    let val = &r[..len];
    *r = &r[len..];
    Ok(val)
}

fn read_string<'a>(r: &mut &'a [u8]) -> Result<&'a str, ReadError> {
    let len = match r.iter().position(|&x| x == 0) {
        Some(len) => len,
        None => return Err(ReadError::Invalid("unterminated string".to_string())),
    };
    let val = try!(std::str::from_utf8(&r[..len]));
    *r = &r[len + 1..];
    Ok(val)
}

fn read_address<R: Read>(r: &mut R, endian: Endian, address_size: u8) -> Result<u64, ReadError> {
    let val = match address_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(val)
}

impl AbbrevHash {
    pub fn read<R: Read>(r: &mut R) -> Result<AbbrevHash, ReadError> {
        let mut abbrev_hash = AbbrevHash::new();
        while let Some(abbrev) = try!(Abbrev::read(r)) {
            let code = abbrev.code;
            if abbrev_hash.insert(abbrev).is_some() {
                return Err(ReadError::Invalid(format!("duplicate abbrev code {}", code)));
            }
        }
        Ok(abbrev_hash)
    }
}

impl Abbrev {
    pub fn read<R: Read>(r: &mut R) -> Result<Option<Abbrev>, ReadError> {
        let code = try!(leb128::read_u64(r));
        if code == 0 {
            return Ok(None);
        }

        let tag = try!(leb128::read_u16(r));

        let children = match constant::DwChildren(try!(r.read_u8())) {
            constant::DW_CHILDREN_no => false,
            constant::DW_CHILDREN_yes => true,
            val => return Err(ReadError::Invalid(format!("DW_CHILDREN {}", val.0))),
        };

        let mut attributes = Vec::new();
        while let Some(attribute) = try!(AbbrevAttribute::read(r)) {
            attributes.push(attribute);
        }

        Ok(Some(Abbrev {
            code: code,
            tag: constant::DwTag(tag),
            children: children,
            attributes: attributes,
        }))
    }
}

impl AbbrevAttribute {
    pub fn read<R: Read>(r: &mut R) -> Result<Option<AbbrevAttribute>, ReadError> {
        let at = try!(leb128::read_u16(r));
        let form = try!(leb128::read_u16(r));
        let attribute = AbbrevAttribute {
            at: constant::DwAt(at),
            form: constant::DwForm(form),
        };
        if attribute.is_null() {
            Ok(None)
        } else {
            Ok(Some(attribute))
        }
    }
}
