use std::io::Write;
use std::ops::Deref;

use abbrev::AbbrevHash;
use constant;
use die::{DieCursor, AttributeData};
use endian::Endian;
use line::LineNumberProgram;
use read::*;
use write::*;

#[derive(Debug)]
pub struct CompilationUnitIterator<'a, E: Endian> {
    endian: E,
    data: &'a [u8],
    offset: usize,
}

impl<'a, E: Endian> CompilationUnitIterator<'a, E> {
    pub fn new(endian: E, data: &'a [u8]) -> Self {
        CompilationUnitIterator {
            endian: endian,
            data: data,
            offset: 0,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    #[cfg_attr(feature = "clippy", allow(should_implement_trait))]
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

#[derive(Debug, PartialEq, Eq)]
pub struct CompilationUnit<'a, E: Endian> {
    pub common: UnitCommon<'a, E>,
}

impl<'a, E: Endian + Default> Default for CompilationUnit<'a, E> {
    fn default() -> Self {
        CompilationUnit { common: Default::default() }
    }
}

impl<'a, E: Endian> CompilationUnit<'a, E> {
    fn base_header_len(offset_size: u8) -> usize {
        // version + abbrev_offset + address_size
        2 + offset_size as usize + 1
    }

    fn total_header_len(offset_size: u8) -> usize {
        // len + version + abbrev_offset + address_size
        // Includes an extra 4 bytes if offset_size is 8
        (offset_size as usize * 2 - 4) + Self::base_header_len(offset_size)
    }

    pub fn data(&'a self) -> &'a [u8] {
        self.common.data()
    }

    pub fn data_offset(&'a self) -> usize {
        self.common.offset + Self::total_header_len(self.common.offset_size)
    }

    pub fn abbrev(&self, debug_abbrev: &[u8]) -> Result<AbbrevHash, ReadError> {
        self.common.abbrev(debug_abbrev)
    }

    pub fn line_program<'line>(
        &self,
        debug_line: &'line [u8],
        abbrev: &AbbrevHash
    ) -> Result<Option<LineNumberProgram<'line, E>>, ReadError> {
        let mut entries = self.entries(abbrev);
        if let Some(entry) = try!(entries.next()) {
            if let Some(attr) = entry.attr(constant::DW_AT_stmt_list) {
                let offset = match *attr {
                    AttributeData::Data4(val) => val as usize,
                    AttributeData::SecOffset(val) => val as usize,
                    _ => return Err(ReadError::Invalid),
                };

                if offset >= debug_line.len() {
                    return Err(ReadError::Invalid);
                }
                let mut r = &debug_line[offset..];

                return LineNumberProgram::read(&mut r,
                                               offset,
                                               self.common.endian,
                                               self.common.address_size)
                    .map(Some);
            }
        }
        Ok(None)
    }

    pub fn entries<'cursor>(
        &'a self,
        abbrev: &'cursor AbbrevHash
    ) -> DieCursor<'cursor, 'a, 'a, E> {
        self.common.entries(self.data_offset(), abbrev)
    }

    pub fn entry<'cursor>(
        &'a self,
        offset: usize,
        abbrev: &'cursor AbbrevHash
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        self.common.entry(self.data_offset(), offset, abbrev)
    }

    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        endian: E
    ) -> Result<CompilationUnit<'a, E>, ReadError> {
        let (mut common, data) = try!(UnitCommon::read(r, offset, endian));
        common.data = data;
        Ok(CompilationUnit { common: common })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> Result<(), WriteError> {
        let len = Self::base_header_len(self.common.offset_size) + self.common.len();
        try!(self.common.write(w, len));
        try!(w.write_all(self.data()));
        Ok(())
    }
}

#[derive(Debug)]
pub struct TypeUnitIterator<'a, E: Endian> {
    endian: E,
    data: &'a [u8],
    offset: usize,
}

impl<'a, E: Endian> TypeUnitIterator<'a, E> {
    pub fn new(endian: E, data: &'a [u8]) -> Self {
        TypeUnitIterator {
            endian: endian,
            data: data,
            offset: 0,
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    #[cfg_attr(feature = "clippy", allow(should_implement_trait))]
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


#[derive(Debug, PartialEq, Eq)]
pub struct TypeUnit<'a, E: Endian> {
    pub common: UnitCommon<'a, E>,
    pub type_signature: u64,
    pub type_offset: u64,
}

impl<'a, E: Endian> TypeUnit<'a, E> {
    fn base_header_len(offset_size: u8) -> usize {
        // version + abbrev_offset + address_size + type_signature + type_offset
        2 + offset_size as usize + 1 + 8 + offset_size as usize
    }

    fn total_header_len(offset_size: u8) -> usize {
        // Includes an extra 4 bytes if offset_size is 8
        (offset_size as usize * 2 - 4) + Self::base_header_len(offset_size)
    }

    pub fn data(&'a self) -> &'a [u8] {
        self.common.data()
    }

    pub fn data_offset(&'a self) -> usize {
        self.common.offset + Self::total_header_len(self.common.offset_size)
    }

    pub fn abbrev(&self, debug_abbrev: &[u8]) -> Result<AbbrevHash, ReadError> {
        self.common.abbrev(debug_abbrev)
    }

    pub fn entries<'cursor>(
        &'a self,
        abbrev: &'cursor AbbrevHash
    ) -> DieCursor<'cursor, 'a, 'a, E> {
        self.common.entries(self.data_offset(), abbrev)
    }

    pub fn entry<'cursor>(
        &'a self,
        offset: usize,
        abbrev: &'cursor AbbrevHash
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        self.common.entry(self.data_offset(), offset, abbrev)
    }

    pub fn type_entry<'cursor>(
        &'a self,
        abbrev: &'cursor AbbrevHash
    ) -> Option<DieCursor<'cursor, 'a, 'a, E>> {
        self.common.entry(self.data_offset(), self.type_offset as usize, abbrev)
    }

    pub fn read(r: &mut &'a [u8], offset: usize, endian: E) -> Result<TypeUnit<'a, E>, ReadError> {
        let (mut common, mut data) = try!(UnitCommon::read(r, offset, endian));

        // Read the remaining fields out of data
        let type_signature = try!(endian.read_u64(&mut data));
        let type_offset = try!(read_offset(&mut data, endian, common.offset_size));
        common.data = data;

        Ok(TypeUnit {
            common: common,
            type_signature: type_signature,
            type_offset: type_offset,
        })
    }

    pub fn write<W: Write>(&self, w: &mut W) -> Result<(), WriteError> {
        let len = Self::base_header_len(self.common.offset_size) + self.common.len();
        try!(self.common.write(w, len));
        try!(self.common.endian.write_u64(w, self.type_signature));
        try!(write_offset(w,
                          self.common.endian,
                          self.common.offset_size,
                          self.type_offset));
        try!(w.write_all(self.data()));
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnitCommon<'a, E: Endian> {
    pub offset: usize,
    pub endian: E,
    pub version: u16,
    pub address_size: u8,
    pub offset_size: u8,
    pub abbrev_offset: u64,
    pub data: &'a [u8],
}

impl<'a, E: Endian + Default> Default for UnitCommon<'a, E> {
    fn default() -> Self {
        UnitCommon {
            offset: 0,
            endian: Default::default(),
            version: 4,
            address_size: 4,
            offset_size: 4,
            abbrev_offset: 0,
            data: &[],
        }
    }
}

#[cfg_attr(feature = "clippy", allow(len_without_is_empty))]
impl<'a, E: Endian> UnitCommon<'a, E> {
    pub fn data(&'a self) -> &'a [u8] {
        &*self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

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
        abbrev: &'cursor AbbrevHash
    ) -> DieCursor<'cursor, 'a, 'a, E> {
        // Unfortunately, entry lifetime is restricted to that of self
        // because self.data might be owned
        DieCursor::new(self.data.deref(), data_offset, self, abbrev)
    }

    pub fn entry<'cursor>(
        &'a self,
        data_offset: usize,
        offset: usize,
        abbrev: &'cursor AbbrevHash
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

    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        endian: E
    ) -> Result<(UnitCommon<'a, E>, &'a [u8]), ReadError> {
        let (offset_size, len) = try!(read_initial_length(r, endian));
        let mut data = &r[..len];

        let version = try!(endian.read_u16(&mut data));
        // TODO: is this correct?
        if version < 2 || version > 4 {
            return Err(ReadError::Unsupported);
        }

        let abbrev_offset = try!(read_offset(&mut data, endian, offset_size));
        let address_size = try!(read_u8(&mut data));

        *r = &r[len..];
        Ok((UnitCommon {
            offset: offset,
            endian: endian,
            version: version,
            address_size: address_size,
            offset_size: offset_size,
            abbrev_offset: abbrev_offset,
            data: Default::default(),
        },
            data))
    }

    pub fn write<W: Write>(&self, w: &mut W, len: usize) -> Result<(), WriteError> {
        match self.offset_size {
            4 => {
                if len >= 0xfffffff0 {
                    return Err(WriteError::Invalid(format!("compilation unit length {}", len)));
                }
                try!(self.endian.write_u32(w, len as u32));
            }
            8 => {
                try!(self.endian.write_u32(w, 0xffffffff));
                try!(self.endian.write_u64(w, len as u64));
            }
            _ => return Err(WriteError::Unsupported(format!("offset size {}", self.offset_size))),
        };
        try!(self.endian.write_u16(w, self.version));
        try!(write_offset(w, self.endian, self.offset_size, self.abbrev_offset));
        try!(write_u8(w, self.address_size));
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use endian::*;

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn compilation_unit_32() {
        let offset = 0;
        let offset_size = 4;
        let endian = LittleEndian;
        let data = [0x01, 0x23, 0x45, 0x67];
        let write_val = CompilationUnit {
            common: UnitCommon {
                offset: offset,
                endian: endian,
                version: 4,
                address_size: 4,
                offset_size: offset_size,
                abbrev_offset: 0x12,
                data: &data[..],
            },
        };

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = CompilationUnit::read(&mut r, offset, endian).unwrap();

        assert_eq!(&buf[..], [
            0x0b, 0x00, 0x00, 0x00,
            0x04, 0x00,
            0x12, 0x00, 0x00, 0x00,
            0x04,
            0x01, 0x23, 0x45, 0x67
        ]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn compilation_unit_64() {
        let offset = 0;
        let offset_size = 8;
        let endian = LittleEndian;
        let data = [0x01, 0x23, 0x45, 0x67];
        let write_val = CompilationUnit {
            common: UnitCommon {
                offset: offset,
                endian: endian,
                version: 4,
                address_size: 4,
                offset_size: offset_size,
                abbrev_offset: 0x12,
                data: &data,
            },
        };

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = CompilationUnit::read(&mut r, offset, endian).unwrap();

        assert_eq!(&buf[..], [
            0xff, 0xff, 0xff, 0xff, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0x00,
            0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04,
            0x01, 0x23, 0x45, 0x67
        ]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn type_unit_32() {
        let offset = 0;
        let offset_size = 4;
        let endian = LittleEndian;
        let data = [0x01, 0x23, 0x45, 0x67];
        let write_val = TypeUnit {
            common: UnitCommon {
                offset: offset,
                endian: endian,
                version: 4,
                address_size: 4,
                offset_size: offset_size,
                abbrev_offset: 0x12,
                data: &data,
            },
            type_signature: 0x0123456789abcdef,
            type_offset: 0x02,
        };

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = TypeUnit::read(&mut r, offset, endian).unwrap();

        assert_eq!(&buf[..], [
            0x17, 0x00, 0x00, 0x00,
            0x04, 0x00,
            0x12, 0x00, 0x00, 0x00,
            0x04,
            0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,
            0x02, 0x00, 0x00, 0x00,
            0x01, 0x23, 0x45, 0x67
        ]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn type_unit_64() {
        let offset = 0;
        let offset_size = 8;
        let endian = LittleEndian;
        let data = [0x01, 0x23, 0x45, 0x67];
        let write_val = TypeUnit {
            common: UnitCommon {
                offset: offset,
                endian: endian,
                version: 4,
                address_size: 4,
                offset_size: offset_size,
                abbrev_offset: 0x12,
                data: &data,
            },
            type_signature: 0x0123456789abcdef,
            type_offset: 0x02,
        };

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = TypeUnit::read(&mut r, offset, endian).unwrap();

        assert_eq!(buf, vec![
            0xff, 0xff, 0xff, 0xff, 0x1f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04, 0x00,
            0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x04,
            0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x23, 0x45, 0x67
        ]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }
}
