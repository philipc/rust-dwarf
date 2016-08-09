use std;
use std::io::Write;
use read::ReadError;

pub trait Endian: Copy {
    fn read_u16(&self, r: &mut &[u8]) -> Result<u16, ReadError>;
    fn read_u32(&self, r: &mut &[u8]) -> Result<u32, ReadError>;
    fn read_u64(&self, r: &mut &[u8]) -> Result<u64, ReadError>;
    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error>;
    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error>;
    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error>;
}

macro_rules! read_endian {
    ($r:ident, $ty:ty, $to:ident) => ({
        let len = std::mem::size_of::<$ty>();
        if $r.len() < len {
            return Err(ReadError::Eof);
        }
        let mut val: $ty = 0;
        unsafe {
            std::ptr::copy_nonoverlapping($r.as_ptr(), &mut val as *mut $ty as *mut u8, len);
        }
        *$r = &$r[len..];
        Ok(val.$to())
    });
}

macro_rules! write_endian {
    ($w:ident, $ty:ty, $to:ident, $val:ident) => ({
        let val: $ty = $val.$to();
        let buf = unsafe {
            std::slice::from_raw_parts(&val as *const $ty as *const u8, std::mem::size_of::<$ty>())
        };
        $w.write_all(buf)
    });
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LittleEndian;

impl Endian for LittleEndian {
    fn read_u16(&self, r: &mut &[u8]) -> Result<u16, ReadError> {
        read_endian!(r, u16, to_le)
    }

    fn read_u32(&self, r: &mut &[u8]) -> Result<u32, ReadError> {
        read_endian!(r, u32, to_le)
    }

    fn read_u64(&self, r: &mut &[u8]) -> Result<u64, ReadError> {
        read_endian!(r, u64, to_le)
    }

    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        write_endian!(w, u16, to_le, val)
    }

    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        write_endian!(w, u32, to_le, val)
    }

    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        write_endian!(w, u64, to_le, val)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BigEndian;

impl Endian for BigEndian {
    fn read_u16(&self, r: &mut &[u8]) -> Result<u16, ReadError> {
        read_endian!(r, u16, to_be)
    }

    fn read_u32(&self, r: &mut &[u8]) -> Result<u32, ReadError> {
        read_endian!(r, u32, to_be)
    }

    fn read_u64(&self, r: &mut &[u8]) -> Result<u64, ReadError> {
        read_endian!(r, u64, to_be)
    }

    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        write_endian!(w, u16, to_be, val)
    }

    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        write_endian!(w, u32, to_be, val)
    }

    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        write_endian!(w, u64, to_be, val)
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
    fn read_u16(&self, r: &mut &[u8]) -> Result<u16, ReadError> {
        match *self {
            AnyEndian::Little => read_endian!(r, u16, to_le),
            AnyEndian::Big => read_endian!(r, u16, to_be),
        }
    }

    fn read_u32(&self, r: &mut &[u8]) -> Result<u32, ReadError> {
        match *self {
            AnyEndian::Little => read_endian!(r, u32, to_le),
            AnyEndian::Big => read_endian!(r, u32, to_be),
        }
    }

    fn read_u64(&self, r: &mut &[u8]) -> Result<u64, ReadError> {
        match *self {
            AnyEndian::Little => read_endian!(r, u64, to_le),
            AnyEndian::Big => read_endian!(r, u64, to_be),
        }
    }

    fn write_u16<W: Write>(&self, w: &mut W, val: u16) -> Result<(), std::io::Error> {
        match *self {
            AnyEndian::Little => write_endian!(w, u16, to_le, val),
            AnyEndian::Big => write_endian!(w, u16, to_be, val),
        }
    }

    fn write_u32<W: Write>(&self, w: &mut W, val: u32) -> Result<(), std::io::Error> {
        match *self {
            AnyEndian::Little => write_endian!(w, u32, to_le, val),
            AnyEndian::Big => write_endian!(w, u32, to_be, val),
        }
    }

    fn write_u64<W: Write>(&self, w: &mut W, val: u64) -> Result<(), std::io::Error> {
        match *self {
            AnyEndian::Little => write_endian!(w, u64, to_le, val),
            AnyEndian::Big => write_endian!(w, u64, to_be, val),
        }
    }
}
