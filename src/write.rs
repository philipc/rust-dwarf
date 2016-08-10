use std;
use std::io::Write;

use super::*;

#[derive(Debug)]
pub enum WriteError {
    Io(std::io::Error),
    Invalid(String),
    Unsupported(String),
}

impl std::convert::From<std::io::Error> for WriteError {
    fn from(e: std::io::Error) -> Self {
        WriteError::Io(e)
    }
}

#[inline]
pub fn write_u8<W: Write>(w: &mut W, val: u8) -> Result<(), std::io::Error> {
    let buf = [val];
    w.write_all(&buf)
}

impl<'a, E: Endian> CompilationUnit<'a, E> {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<(), WriteError> {
        let len = Self::base_header_len(self.common.offset_size) + self.common.len();
        try!(self.common.write(w, len));
        try!(w.write_all(self.data()));
        Ok(())
    }
}

impl<'a, E: Endian> TypeUnit<'a, E> {
    pub fn write<W: Write>(&self, w: &mut W) -> Result<(), WriteError> {
        let len = Self::base_header_len(self.common.offset_size) + self.common.len();
        try!(self.common.write(w, len));
        try!(self.common.endian.write_u64(w, self.type_signature));
        try!(write_offset(w, self.common.endian, self.common.offset_size, self.type_offset));
        try!(w.write_all(self.data()));
        Ok(())
    }
}

impl<'a, E: Endian> UnitCommon<'a, E> {
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

pub fn write_offset<W: Write, E: Endian>(w: &mut W, endian: E, offset_size: u8, val: u64) -> Result<(), WriteError> {
    match offset_size {
        4 => try!(endian.write_u32(w, val as u32)),
        8 => try!(endian.write_u64(w, val)),
        _ => return Err(WriteError::Unsupported(format!("offset size {}", offset_size))),
    };
    Ok(())
}

pub fn write_address<W: Write, E: Endian>(w: &mut W, endian: E, address_size: u8, val: u64) -> Result<(), WriteError> {
    match address_size {
        4 => try!(endian.write_u32(w, val as u32)),
        8 => try!(endian.write_u64(w, val)),
        _ => return Err(WriteError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(())
}
