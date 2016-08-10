use std;
use std::ops::Deref;

use super::*;
use abbrev::*;
use die::*;

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

pub fn read_block<'a>(r: &mut &'a [u8], len: usize) -> Result<&'a [u8], ReadError> {
    if len > r.len() {
        return Err(ReadError::Invalid);
    }
    let val = &r[..len];
    *r = &r[len..];
    Ok(val)
}

pub fn read_string<'a>(r: &mut &'a [u8]) -> Result<&'a [u8], ReadError> {
    let len = match r.iter().position(|&x| x == 0) {
        Some(len) => len,
        None => return Err(ReadError::Invalid),
    };
    let val = &r[..len];
    *r = &r[len + 1..];
    Ok(val)
}

pub fn read_offset<E: Endian>(r: &mut &[u8], endian: E, offset_size: u8) -> Result<u64, ReadError> {
    let val = match offset_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported),
    };
    Ok(val)
}

pub fn read_address<E: Endian>(r: &mut &[u8], endian: E, address_size: u8) -> Result<u64, ReadError> {
    let val = match address_size {
        4 => try!(endian.read_u32(r)) as u64,
        8 => try!(endian.read_u64(r)),
        _ => return Err(ReadError::Unsupported),
    };
    Ok(val)
}
