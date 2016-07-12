use std;
use std::io::{Read, Write};
use byteorder;
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    #[cfg(target_endian = "little")]
    pub fn native() -> Self {
        Endian::Little
    }

    #[cfg(target_endian = "big")]
    pub fn native() -> Self {
        Endian::Big
    }

    pub fn read_u16<R: Read>(&self, r: &mut R) -> Result<u16, std::io::Error> {
        match *self {
            Endian::Little => r.read_u16::<byteorder::LittleEndian>(),
            Endian::Big => r.read_u16::<byteorder::BigEndian>(),
        }
    }

    pub fn read_u32<R: Read>(&self, r: &mut R) -> Result<u32, std::io::Error> {
        match *self {
            Endian::Little => r.read_u32::<byteorder::LittleEndian>(),
            Endian::Big => r.read_u32::<byteorder::BigEndian>(),
        }
    }

    pub fn read_u64<R: Read>(&self, r: &mut R) -> Result<u64, std::io::Error> {
        match *self {
            Endian::Little => r.read_u64::<byteorder::LittleEndian>(),
            Endian::Big => r.read_u64::<byteorder::BigEndian>(),
        }
    }

    pub fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        match *self {
            Endian::Little => w.write_u16::<byteorder::LittleEndian>(val),
            Endian::Big => w.write_u16::<byteorder::BigEndian>(val),
        }
    }

    pub fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        match *self {
            Endian::Little => w.write_u32::<byteorder::LittleEndian>(val),
            Endian::Big => w.write_u32::<byteorder::BigEndian>(val),
        }
    }

    pub fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        match *self {
            Endian::Little => w.write_u64::<byteorder::LittleEndian>(val),
            Endian::Big => w.write_u64::<byteorder::BigEndian>(val),
        }
    }
}
