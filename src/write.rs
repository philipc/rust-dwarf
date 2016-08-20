use std;
use std::io::Write;

use endian::Endian;

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

pub fn write_offset<W: Write, E: Endian>(
    w: &mut W,
    endian: E,
    offset_size: u8,
    val: u64
) -> Result<(), WriteError> {
    match offset_size {
        4 => try!(endian.write_u32(w, val as u32)),
        8 => try!(endian.write_u64(w, val)),
        _ => return Err(WriteError::Unsupported(format!("offset size {}", offset_size))),
    };
    Ok(())
}

pub fn write_address<W: Write, E: Endian>(
    w: &mut W,
    endian: E,
    address_size: u8,
    val: u64
) -> Result<(), WriteError> {
    match address_size {
        4 => try!(endian.write_u32(w, val as u32)),
        8 => try!(endian.write_u64(w, val)),
        _ => return Err(WriteError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(())
}
