use std;

use endian::Endian;

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

#[inline]
pub fn read_i8(r: &mut &[u8]) -> Result<i8, ReadError> {
    read_u8(r).map(|val| val as i8)
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

pub fn read_initial_length<E: Endian>(r: &mut &[u8], endian: E) -> Result<(u8, usize), ReadError> {
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
        Ok((offset_size, len))
}
