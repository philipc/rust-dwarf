use std;
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
            leb128::Error::IoError(e) => ReadError::Io(e),
            leb128::Error::Overflow => ReadError::Invalid("LEB128 overflow".to_string()),
        }
    }
}

impl<E: Endian> Sections<E> {
    pub fn compilation_units(&self) -> CompilationUnitIterator<E> {
        CompilationUnitIterator::new(self.endian, &*self.debug_info)
    }

    pub fn type_units(&self) -> TypeUnitIterator<E> {
        TypeUnitIterator::new(self.endian, &*self.debug_types)
    }

    pub fn abbrev<'a>(&self, unit: &UnitCommon<'a, E>) -> Result<AbbrevHash, ReadError> {
        unit.abbrev(&*self.debug_abbrev)
    }
}

#[cfg_attr(feature = "clippy", allow(should_implement_trait))]
impl<'a, E: Endian> CompilationUnitIterator<'a, E> {
    fn new(endian: E, data: &'a [u8]) -> Self {
        CompilationUnitIterator {
            endian: endian,
            data: data,
            offset: 0,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<CompilationUnit<'a, E>>, ReadError> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        let mut r = self.data;
        let unit = try!(CompilationUnit::read(&mut r, self.offset, self.endian));
        self.offset += self.data.len() - r.len();
        self.data = r;
        Ok(Some(unit))
    }
}

#[cfg_attr(feature = "clippy", allow(should_implement_trait))]
impl<'a, E: Endian> TypeUnitIterator<'a, E> {
    fn new(endian: E, data: &'a [u8]) -> Self {
        TypeUnitIterator {
            endian: endian,
            data: data,
            offset: 0,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<TypeUnit<'a, E>>, ReadError> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        let mut r = self.data;
        let unit = try!(TypeUnit::read(&mut r, self.offset, self.endian));
        self.offset += self.data.len() - r.len();
        self.data = r;
        Ok(Some(unit))
    }
}

impl<'a, E: Endian> CompilationUnit<'a, E> {
    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        endian: E,
    ) -> Result<CompilationUnit<'a, E>, ReadError> {
        let (mut common, data) = try!(UnitCommon::read(r, endian));
        common.data = From::from(data);
        Ok(CompilationUnit {
            offset: offset,
            common: common,
        })
    }

    pub fn abbrev(&self, debug_abbrev: &[u8]) -> Result<AbbrevHash, ReadError> {
        self.common.abbrev(debug_abbrev)
    }

    pub fn entries<'cursor>(
        &'a self,
        abbrev: &'cursor AbbrevHash,
    ) -> DieCursor<'cursor, 'a, 'a, E> {
        self.common.entries(self.data_offset(), abbrev)
    }

    pub fn entry<'cursor>(
        &'a self,
        offset: usize,
        abbrev: &'cursor AbbrevHash,
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        self.common.entry(self.data_offset(), offset, abbrev)
    }
}

impl<'a, E: Endian> TypeUnit<'a, E> {
    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        endian: E,
    ) -> Result<TypeUnit<'a, E>, ReadError> {
        let (mut common, mut data) = try!(UnitCommon::read(r, endian));

        // Read the remaining fields out of data
        let type_signature = try!(endian.read_u64(&mut data));
        let type_offset = try!(read_offset(&mut data, endian, common.offset_size));
        common.data = From::from(data);

        Ok(TypeUnit {
            offset: offset,
            type_signature: type_signature,
            type_offset: type_offset,
            common: common,
        })
    }

    pub fn abbrev(&self, debug_abbrev: &[u8]) -> Result<AbbrevHash, ReadError> {
        self.common.abbrev(debug_abbrev)
    }

    pub fn entries<'cursor>(
        &'a self,
        abbrev: &'cursor AbbrevHash,
    ) -> DieCursor<'cursor, 'a, 'a, E> {
        self.common.entries(self.data_offset(), abbrev, )
    }

    pub fn entry<'cursor>(
        &'a self,
        offset: usize,
        abbrev: &'cursor AbbrevHash,
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        self.common.entry(self.data_offset(), offset, abbrev)
    }

    pub fn type_entry<'cursor>(
        &'a self,
        abbrev: &'cursor AbbrevHash,
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        self.common.entry(self.data_offset(), self.type_offset as usize, abbrev)
    }
}

impl<'a, E: Endian> UnitCommon<'a, E> {
    pub fn read(
        r: &mut &'a [u8],
        endian: E,
    ) -> Result<(UnitCommon<'a, E>, &'a [u8]), ReadError> {
        let mut offset_size = 4;
        let mut len = try!(endian.read_u32(r)) as usize;
        if len == 0xffffffff {
            offset_size = 8;
            len = try!(endian.read_u64(r)) as usize;
        } else if len >= 0xfffffff0 {
            return Err(ReadError::Unsupported(format!("unit length {}", len)));
        }
        if len > r.len() {
            return Err(ReadError::Invalid(format!("unit length {}", len)));
        }

        // Tell the iterator we read the entire length, even if we don't parse it all now
        let mut data = &r[..len];
        *r = &r[len..];

        let version = try!(endian.read_u16(&mut data));
        // TODO: is this correct?
        if version < 2 || version > 4 {
            return Err(ReadError::Unsupported(format!("unit version {}", version)));
        }

        let abbrev_offset = try!(read_offset(&mut data, endian, offset_size));
        let address_size = try!(data.read_u8());

        Ok((UnitCommon {
            endian: endian,
            version: version,
            address_size: address_size,
            offset_size: offset_size,
            abbrev_offset: abbrev_offset,
            data: Default::default(),
        }, data))
    }
}

impl<'a, E: Endian> UnitCommon<'a, E> {
    pub fn abbrev(&self, debug_abbrev: &[u8]) -> Result<AbbrevHash, ReadError> {
        let offset = self.abbrev_offset as usize;
        let len = debug_abbrev.len();
        if offset >= len {
            return Err(ReadError::Invalid(format!("abbrev offset {} > {}", offset, len)));
        }
        AbbrevHash::read(&mut &debug_abbrev[offset..])
    }

    pub fn entries<'cursor>(
        &'a self,
        data_offset: usize,
        abbrev: &'cursor AbbrevHash,
    ) -> DieCursor<'cursor, 'a, 'a, E> {
        // Unfortunately, entry lifetime is restricted to that of self
        // because self.data might be owned
        DieCursor::new(self.data.deref(), data_offset, self, abbrev)
    }

    pub fn entry<'cursor>(
        &'a self,
        data_offset: usize,
        offset: usize,
        abbrev: &'cursor AbbrevHash,
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        if offset < data_offset {
            return None;
        }
        let relative_offset = offset - data_offset;
        if relative_offset >= self.data.len() {
            return None;
        }
        Some(DieCursor::new(&self.data[relative_offset..], offset, self, abbrev))
    }
}

#[cfg_attr(feature = "clippy", allow(should_implement_trait))]
impl<'a, 'entry, 'unit, E: Endian> DieCursor<'a, 'entry, 'unit, E> {
    pub fn new(
        r: &'entry [u8],
        offset: usize,
        unit: &'a UnitCommon<'unit, E>,
        abbrev: &'a AbbrevHash
    ) -> Self {
        DieCursor {
            r: r,
            offset: offset,
            unit: unit,
            abbrev: abbrev,
            next_child: false,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<Die<'entry>>, ReadError> {
        if self.r.len() == 0 {
            return Ok(None);
        }

        let mut r = self.r;
        let die = try!(Die::read(&mut r, self.offset, self.unit, self.abbrev));
        self.next_child = die.children;
        self.offset += self.r.len() - r.len();
        self.r = r;
        Ok(Some(die))
    }

    pub fn next_sibling(&mut self) -> Result<Option<Die<'entry>>, ReadError> {
        let mut depth = if self.next_child { 1 } else { 0 };
        while depth > 0 {
            match try!(self.next()) {
                Some(die) => {
                    if die.is_null() {
                        depth -= 1;
                    } else if self.next_child {
                        depth += 1;
                    }
                },
                None => return Ok(None),
            }
        }
        self.next()
    }
}

impl<'a, 'b> Die<'a> {
    pub fn read<E: Endian>(
        r: &mut &'a [u8],
        offset: usize,
        unit: &UnitCommon<'b, E>,
        abbrev_hash: &AbbrevHash,
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
            attributes.push(try!(Attribute::read(r, unit, abbrev_attribute)));
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

impl<'a, 'b> Attribute<'a> {
    pub fn read<E: Endian>(
        r: &mut &'a [u8],
        unit: &UnitCommon<'b, E>,
        abbrev: &AbbrevAttribute,
    ) -> Result<Attribute<'a>, ReadError> {
        let data = try!(AttributeData::read(r, unit, abbrev.form));
        Ok(Attribute {
            at: abbrev.at,
            data: data,
        })
    }
}

impl<'a, 'b> AttributeData<'a> {
    pub fn read<E: Endian>(
        r: &mut &'a [u8],
        unit: &UnitCommon<'b, E>,
        form: constant::DwForm,
    ) -> Result<AttributeData<'a>, ReadError> {
        let data = match form {
            constant::DW_FORM_addr => {
                let val = try!(read_address(r, unit.endian, unit.address_size));
                AttributeData::Address(val)
            }
            constant::DW_FORM_block2 => {
                let len = try!(unit.endian.read_u16(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_block4 => {
                let len = try!(unit.endian.read_u32(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_data2 => AttributeData::Data2(try!(unit.endian.read_u16(r))),
            constant::DW_FORM_data4 => AttributeData::Data4(try!(unit.endian.read_u32(r))),
            constant::DW_FORM_data8 => AttributeData::Data8(try!(unit.endian.read_u64(r))),
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
                let val = try!(read_offset(r, unit.endian, unit.offset_size));
                AttributeData::StringOffset(val)
            }
            constant::DW_FORM_udata => AttributeData::UData(try!(leb128::read_u64(r))),
            constant::DW_FORM_ref_addr => {
                let val = try!(read_offset(r, unit.endian, unit.offset_size));
                AttributeData::RefAddress(val)
            }
            constant::DW_FORM_ref1 => AttributeData::Ref(try!(r.read_u8()) as u64),
            constant::DW_FORM_ref2 => AttributeData::Ref(try!(unit.endian.read_u16(r)) as u64),
            constant::DW_FORM_ref4 => AttributeData::Ref(try!(unit.endian.read_u32(r)) as u64),
            constant::DW_FORM_ref8 => AttributeData::Ref(try!(unit.endian.read_u64(r)) as u64),
            constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r))),
            constant::DW_FORM_indirect => {
                let val = try!(leb128::read_u16(r));
                try!(AttributeData::read(r, unit, constant::DwForm(val)))
            }
            constant::DW_FORM_sec_offset => {
                // TODO: validate based on class
                let val = try!(read_offset(r, unit.endian, unit.offset_size));
                AttributeData::SecOffset(val)
            }
            constant::DW_FORM_exprloc => {
                let len = try!(leb128::read_u64(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::ExprLoc(val)
            }
            constant::DW_FORM_flag_present => AttributeData::Flag(true),
            constant::DW_FORM_ref_sig8 => AttributeData::RefSig(try!(unit.endian.read_u64(r))),
            _ => return Err(ReadError::Unsupported(format!("attribute form {}", form.0))),
        };
        Ok(data)
    }
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

fn read_offset<R: Read, E: Endian>(r: &mut R, endian: E, offset_size: u8) -> Result<u64, ReadError> {
    let val = match offset_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported(format!("offset size {}", offset_size))),
    };
    Ok(val)
}

fn read_address<R: Read, E: Endian>(r: &mut R, endian: E, address_size: u8) -> Result<u64, ReadError> {
    let val = match address_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(val)
}

impl AbbrevHash {
    pub fn read<R: Read>(r: &mut R) -> Result<AbbrevHash, ReadError> {
        let mut abbrev_hash = AbbrevHash::default();
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
