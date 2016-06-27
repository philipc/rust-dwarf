use std;
use std::convert::From;
use std::io::Read;
use std::ops::{BitOrAssign, Not, Shl};
use byteorder::{ReadBytesExt};

pub enum Error {
    Io(std::io::Error),
    Overflow,
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

fn read_unsigned<R, T>(r: &mut Read, size: usize, zero: T)
    -> Result<T, Error>
    where
        R: Read,
        T: BitOrAssign + Shl<usize, Output=T> + From<u8>
{
    let mut result = zero;
    let mut shift = 0;
    loop {
        let byte = try!(r.read_u8());
        result |= T::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result)
        }
        shift += 7;
        if shift >= size {
            return Err(Error::Overflow)
        }
    }
}

fn read_signed<R, T>(r: &mut R, size: usize, zero: T)
    -> Result<T, Error>
    where
        R: Read,
        T: Copy + BitOrAssign + Not<Output=T> + Shl<usize, Output=T> + From<u8>
{
    let mut result = zero;
    let mut shift = 0;
    loop {
        let byte = try!(r.read_u8());
        result |= T::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < size && (byte & 0x40) != 0 {
                // Sign extend
                result |= !zero << shift;
            }
            return Ok(result)
        }
        if shift >= size {
            return Err(Error::Overflow)
        }
    }
}

pub fn read_u16<R: Read>(r: &mut R) -> Result<u16, Error> {
    read_unsigned::<R, u16>(r, 16, 0u16)
}

pub fn read_u64<R: Read>(r: &mut R) -> Result<u64, Error> {
    read_unsigned::<R, u64>(r, 64, 0u64)
}

pub fn read_i64<R: Read>(r: &mut R) -> Result<i64, Error> {
    read_signed::<R, i64>(r, 64, 0i64)
}

// TODO: tests
