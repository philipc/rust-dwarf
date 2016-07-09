use std;
use std::io::Read;
use byteorder;
use byteorder::{ReadBytesExt};

#[derive(Clone, Copy, Debug)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
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
}

