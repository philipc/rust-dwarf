use std;
use std::ops::Deref;

use super::*;
use leb128;
use abbrev::*;

#[derive(Debug)]
pub enum ReadError {
    Io,
    Eof,
    Invalid,
    Unsupported,
    Overflow,
}

impl std::convert::From<std::io::Error> for ReadError {
    fn from(_: std::io::Error) -> Self {
        ReadError::Io
    }
}

#[inline]
pub fn read_u8(r: &mut &[u8]) -> Result<u8, ReadError> {
    if r.len() < 1 {
        return Err(ReadError::Eof);
    }
    let byte = r[0];
    *r = &r[1..];
    return Ok(byte)
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
        let (mut common, data) = try!(UnitCommon::read(r, offset, endian));
        common.data = From::from(data);
        Ok(CompilationUnit {
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
        let (mut common, mut data) = try!(UnitCommon::read(r, offset, endian));

        // Read the remaining fields out of data
        let type_signature = try!(endian.read_u64(&mut data));
        let type_offset = try!(read_offset(&mut data, endian, common.offset_size));
        common.data = From::from(data);

        Ok(TypeUnit {
            common: common,
            type_signature: type_signature,
            type_offset: type_offset,
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
        offset: usize,
        endian: E,
    ) -> Result<(UnitCommon<'a, E>, &'a [u8]), ReadError> {
        let mut offset_size = 4;
        let mut len = try!(endian.read_u32(r)) as usize;
        if len == 0xffffffff {
            offset_size = 8;
            len = try!(endian.read_u64(r)) as usize;
        } else if len >= 0xfffffff0 {
            return Err(ReadError::Unsupported);
        }
        if len > r.len() {
            return Err(ReadError::Invalid);
        }

        // Tell the iterator we read the entire length, even if we don't parse it all now
        let mut data = &r[..len];
        *r = &r[len..];

        let version = try!(endian.read_u16(&mut data));
        // TODO: is this correct?
        if version < 2 || version > 4 {
            return Err(ReadError::Unsupported);
        }

        let abbrev_offset = try!(read_offset(&mut data, endian, offset_size));
        let address_size = try!(read_u8(&mut data));

        Ok((UnitCommon {
            offset: offset,
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
            return Err(ReadError::Invalid);
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
            entry: Die::null(0),
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<&Die<'entry>>, ReadError> {
        if self.r.len() == 0 {
            return Ok(None);
        }

        let mut r = self.r;
        try!(self.entry.read(&mut r, self.offset, self.unit, self.abbrev));
        self.offset += self.r.len() - r.len();
        self.r = r;
        Ok(Some(&self.entry))
    }

    pub fn next_sibling(&mut self) -> Result<Option<&Die<'entry>>, ReadError> {
        let mut depth = if self.entry.children { 1 } else { 0 };
        while depth > 0 {
            let mut sibling_offset = 0;
            for attribute in &self.entry.attributes {
                if attribute.at == constant::DW_AT_sibling {
                    if let AttributeData::Ref(offset) = attribute.data {
                        sibling_offset = self.unit.offset + offset as usize;
                    }
                    break;
                }
            }
            if sibling_offset > self.offset {
                let relative_offset = sibling_offset - self.offset;
                if relative_offset <= self.r.len() {
                    self.entry.set_null(0);
                    self.offset = sibling_offset;
                    self.r = &self.r[relative_offset..];
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            match try!(self.next()) {
                Some(die) => {
                    if die.is_null() {
                        depth -= 1;
                    } else if die.children {
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
        &mut self,
        r: &mut &'a [u8],
        offset: usize,
        unit: &UnitCommon<'b, E>,
        abbrev_hash: &AbbrevHash,
    ) -> Result<(), ReadError> {
        self.set_null(offset);

        self.code = try!(leb128::read_u64(r));
        if self.code == 0 {
            return Ok(());
        }

        let abbrev = match abbrev_hash.get(self.code) {
            Some(abbrev) => abbrev,
            None => return Err(ReadError::Invalid),
        };

        self.tag = abbrev.tag;
        self.children = abbrev.children;
        let len = abbrev.attributes.len();
        self.attributes.reserve(len);
        unsafe {
            self.attributes.set_len(len);
            for i in 0..len {
                if let Err(e) = self.attributes[i].read(r, unit, &abbrev.attributes[i]) {
                    self.attributes.clear();
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

impl<'a, 'b> Attribute<'a> {
    pub fn read<E: Endian>(
        &mut self,
        r: &mut &'a [u8],
        unit: &UnitCommon<'b, E>,
        abbrev: &AbbrevAttribute,
    ) -> Result<(), ReadError> {
        self.at = abbrev.at;
        try!(self.data.read(r, unit, abbrev.form));
        Ok(())
    }
}

impl<'a, 'b> AttributeData<'a> {
    pub fn read<E: Endian>(
        &mut self,
        r: &mut &'a [u8],
        unit: &UnitCommon<'b, E>,
        form: constant::DwForm,
    ) -> Result<(), ReadError> {
        *self = match form {
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
                let len = try!(read_u8(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_data1 => AttributeData::Data1(try!(read_u8(r))),
            constant::DW_FORM_flag => AttributeData::Flag(try!(read_u8(r)) != 0),
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
            constant::DW_FORM_ref1 => AttributeData::Ref(try!(read_u8(r)) as u64),
            constant::DW_FORM_ref2 => AttributeData::Ref(try!(unit.endian.read_u16(r)) as u64),
            constant::DW_FORM_ref4 => AttributeData::Ref(try!(unit.endian.read_u32(r)) as u64),
            constant::DW_FORM_ref8 => AttributeData::Ref(try!(unit.endian.read_u64(r)) as u64),
            constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r))),
            constant::DW_FORM_indirect => {
                let val = try!(leb128::read_u16(r));
                return self.read(r, unit, constant::DwForm(val))
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
            _ => return Err(ReadError::Unsupported),
        };
        Ok(())
    }
}

fn read_block<'a>(r: &mut &'a [u8], len: usize) -> Result<&'a [u8], ReadError> {
    if len > r.len() {
        return Err(ReadError::Invalid);
    }
    let val = &r[..len];
    *r = &r[len..];
    Ok(val)
}

fn read_string<'a>(r: &mut &'a [u8]) -> Result<&'a [u8], ReadError> {
    let len = match r.iter().position(|&x| x == 0) {
        Some(len) => len,
        None => return Err(ReadError::Invalid),
    };
    let val = &r[..len];
    *r = &r[len + 1..];
    Ok(val)
}

fn read_offset<E: Endian>(r: &mut &[u8], endian: E, offset_size: u8) -> Result<u64, ReadError> {
    let val = match offset_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported),
    };
    Ok(val)
}

fn read_address<E: Endian>(r: &mut &[u8], endian: E, address_size: u8) -> Result<u64, ReadError> {
    let val = match address_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported),
    };
    Ok(val)
}
