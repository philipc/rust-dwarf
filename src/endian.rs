use std;
use std::io::{Read, Write};
use byteorder;
use byteorder::{ReadBytesExt, WriteBytesExt};

pub trait Endian: Copy {
    fn read_u16<R: Read>(&self, r: &mut R) -> Result<u16, std::io::Error>;
    fn read_u32<R: Read>(&self, r: &mut R) -> Result<u32, std::io::Error>;
    fn read_u64<R: Read>(&self, r: &mut R) -> Result<u64, std::io::Error>;
    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error>;
    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error>;
    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LittleEndian;

impl Endian for LittleEndian {
    fn read_u16<R: Read>(&self, r: &mut R) -> Result<u16, std::io::Error> {
        r.read_u16::<byteorder::LittleEndian>()
    }

    fn read_u32<R: Read>(&self, r: &mut R) -> Result<u32, std::io::Error> {
        r.read_u32::<byteorder::LittleEndian>()
    }

    fn read_u64<R: Read>(&self, r: &mut R) -> Result<u64, std::io::Error> {
        r.read_u64::<byteorder::LittleEndian>()
    }

    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        w.write_u16::<byteorder::LittleEndian>(val)
    }

    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        w.write_u32::<byteorder::LittleEndian>(val)
    }

    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        w.write_u64::<byteorder::LittleEndian>(val)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BigEndian;

impl Endian for BigEndian {
    fn read_u16<R: Read>(&self, r: &mut R) -> Result<u16, std::io::Error> {
        r.read_u16::<byteorder::BigEndian>()
    }

    fn read_u32<R: Read>(&self, r: &mut R) -> Result<u32, std::io::Error> {
        r.read_u32::<byteorder::BigEndian>()
    }

    fn read_u64<R: Read>(&self, r: &mut R) -> Result<u64, std::io::Error> {
        r.read_u64::<byteorder::BigEndian>()
    }

    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        w.write_u16::<byteorder::BigEndian>(val)
    }

    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        w.write_u32::<byteorder::BigEndian>(val)
    }

    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        w.write_u64::<byteorder::BigEndian>(val)
    }
}


#[cfg(target_endian = "little")]
pub type NativeEndian = LittleEndian;
#[cfg(target_endian = "big")]
pub type NativeEndian = BigEndian;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnyEndian {
    Little,
    Big,
}

impl Default for AnyEndian {
    fn default() -> Self {
        AnyEndian::native()
    }
}

impl AnyEndian {
    #[cfg(target_endian = "little")]
    fn native() -> Self {
        AnyEndian::Little
    }

    #[cfg(target_endian = "big")]
    fn native() -> Self {
        AnyEndian::Big
    }
}

impl Endian for AnyEndian {
    fn read_u16<R: Read>(&self, r: &mut R) -> Result<u16, std::io::Error> {
        match *self {
            AnyEndian::Little => r.read_u16::<byteorder::LittleEndian>(),
            AnyEndian::Big => r.read_u16::<byteorder::BigEndian>(),
        }
    }

    fn read_u32<R: Read>(&self, r: &mut R) -> Result<u32, std::io::Error> {
        match *self {
            AnyEndian::Little => r.read_u32::<byteorder::LittleEndian>(),
            AnyEndian::Big => r.read_u32::<byteorder::BigEndian>(),
        }
    }

    fn read_u64<R: Read>(&self, r: &mut R) -> Result<u64, std::io::Error> {
        match *self {
            AnyEndian::Little => r.read_u64::<byteorder::LittleEndian>(),
            AnyEndian::Big => r.read_u64::<byteorder::BigEndian>(),
        }
    }

    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        match *self {
            AnyEndian::Little => w.write_u16::<byteorder::LittleEndian>(val),
            AnyEndian::Big => w.write_u16::<byteorder::BigEndian>(val),
        }
    }

    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        match *self {
            AnyEndian::Little => w.write_u32::<byteorder::LittleEndian>(val),
            AnyEndian::Big => w.write_u32::<byteorder::BigEndian>(val),
        }
    }

    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        match *self {
            AnyEndian::Little => w.write_u64::<byteorder::LittleEndian>(val),
            AnyEndian::Big => w.write_u64::<byteorder::BigEndian>(val),
        }
    }
}