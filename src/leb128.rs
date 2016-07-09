use std;
use std::convert::From;
use std::io::{Read, Write};
use std::ops::{BitOrAssign, Not, Shl};
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Overflow,
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

fn read_unsigned<R, T>(r: &mut R, size: usize, zero: T) -> Result<T, Error>
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
            return Ok(result);
        }
        shift += 7;
        if shift >= size {
            return Err(Error::Overflow);
        }
    }
}

fn read_signed<R, T>(r: &mut R, size: usize, zero: T) -> Result<T, Error>
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
            return Ok(result);
        }
        if shift >= size {
            return Err(Error::Overflow);
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

#[allow(dead_code)]
pub fn write_u64<W: Write>(w: &mut W, mut value: u64) -> std::io::Result<()> {
    loop {
        let byte = value as u8 & 0x7f;
        value >>= 7;
        if value == 0 {
            try!(w.write_u8(byte));
            return Ok(());
        }
        try!(w.write_u8(byte | 0x80));
    }
}

#[allow(dead_code)]
pub fn write_i64<W: Write>(w: &mut W, mut value: i64) -> std::io::Result<()> {
    loop {
        let byte = value as u8 & 0x7f;
        let sign = (byte & 0x40) != 0;
        value >>= 7;
        if value == 0 && !sign || value == -1 && sign {
            try!(w.write_u8(byte));
            return Ok(());
        }
        try!(w.write_u8(byte | 0x80));
    }
}

#[allow(dead_code)]
pub fn write_u16<W: Write>(w: &mut W, value: u16) -> std::io::Result<()> {
    write_u64(w, value as u64)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_u16() {
        let mut vec = Vec::new();

        // Read/write normal encodings
        for &(mut r, value) in &[
            (&[0x00][..], 0),
            (&[0x01][..], 1),
            (&[0x02][..], 2),
            (&[0x7f][..], 0x7f),
            (&[0x81,0x02][..], 0x101),
            (&[0xff,0x7f][..], 0x3fff),
            (&[0xff,0xff,0x03][..], 0xffff),
        ] {
            vec.clear();
            write_u16(&mut vec, value).unwrap();
            assert_eq!(vec, r);

            assert_eq!(read_u16(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read alternative encodings
        for &(mut r, value) in &[
            (&[0x80,0x00][..], 0),
            (&[0x81,0x00][..], 1),
            (&[0x80,0x80,0x00][..], 0),
            (&[0xff,0xff,0x00][..], 0x3fff),
            (&[0xff,0xff,0x7f][..], 0xffff),
        ] {
            assert_eq!(read_u16(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read overflow
        for &(mut r,) in &[
            (&[0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff][..],),
        ] {
            assert!(match read_u16(&mut r) {
                Err(Error::Overflow) => true,
                _ => false,
            });
        }
    }

    #[test]
    fn test_u64() {
        let mut vec = Vec::new();

        // Read/write normal encodings
        for &(mut r, value) in &[
            (&[0x00][..], 0),
            (&[0x01][..], 1),
            (&[0x02][..], 2),
            (&[0x7f][..], 0x7f),
            (&[0x81,0x02][..], 0x101),
            (&[0xff,0x7f][..], 0x3fff),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x7f][..], 0x7fffffffffffffff),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x01][..], 0xffffffffffffffff),
        ] {
            vec.clear();
            write_u64(&mut vec, value).unwrap();
            assert_eq!(vec, r);

            assert_eq!(read_u64(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read alternative encodings
        for &(mut r, value) in &[
            (&[0x80,0x00][..], 0),
            (&[0x81,0x00][..], 1),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x00][..], 0),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x00][..], 0x7fffffffffffffff),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x7f][..], 0xffffffffffffffff),
        ] {
            assert_eq!(read_u64(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read overflow
        for &(mut r,) in &[
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff][..],),
        ] {
            assert!(match read_u64(&mut r) {
                Err(Error::Overflow) => true,
                _ => false,
            });
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80][..],),
            (&[0xff,0xff][..],),
        ] {
            assert!(match read_u64(&mut r) {
                Err(Error::Io(_)) => true,
                _ => false,
            });
        }

        // Write EOF
        {
            let mut buf = &mut [0; 2][..];
            assert!(match write_u64(&mut buf, 0xffff) {
                Err(_) => true,
                _ => false,
            });
        }
    }

    #[test]
    fn test_i64() {
        let mut vec = Vec::new();

        // Read/write normal encodings
        for &(mut r, value) in &[
            (&[0x00][..], 0),
            (&[0x01][..], 1),
            (&[0x02][..], 2),
            (&[0x3f][..], 0x3f),
            (&[0x40][..], -0x40),
            (&[0x7f][..], -1),
            (&[0xff,0x00][..], 0x7f),
            (&[0x80,0x01][..], 0x80),
            (&[0x81,0x01][..], 0x81),
            (&[0xff,0x7e][..], -0x81),
            (&[0x80,0x7f][..], -0x80),
            (&[0x81,0x7f][..], -0x7f),
            (&[0xff,0x3f][..], 0x1fff),
            (&[0x80,0x40][..], -0x2000),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x3f][..], 0x3fffffffffffffff),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x40][..], -0x4000000000000000), // sign extend 63
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0xc0,0x00][..], 0x4000000000000000),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x00][..], 0x7fffffffffffffff),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x7f][..], -0x8000000000000000),
        ] {
            vec.clear();
            write_i64(&mut vec, value).unwrap();
            assert_eq!(vec, r);

            assert_eq!(read_i64(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read alternative encodings
        for &(mut r, value) in &[
            (&[0x80,0x00][..], 0),
            (&[0x81,0x00][..], 1),
            (&[0xff,0x7f][..], -1),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x00][..], 0),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x01][..], -0x8000000000000000),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x3f][..], -0x8000000000000000),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x40][..], 0),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x41][..], -0x8000000000000000),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x7f][..], -0x8000000000000000),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x01][..], -1),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x3f][..], -1),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x7f][..], -1),
        ] {
            assert_eq!(read_i64(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read overflow
        for &(mut r,) in &[
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff][..],),
        ] {
            assert!(match read_u64(&mut r) {
                Err(Error::Overflow) => true,
                _ => false,
            });
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80][..],),
            (&[0xff,0xff][..],),
        ] {
            assert!(match read_i64(&mut r) {
                Err(Error::Io(_)) => true,
                _ => false,
            });
        }

        // Write EOF
        {
            let mut buf = &mut [0; 2][..];
            assert!(match write_i64(&mut buf, 0xffff) {
                Err(_) => true,
                _ => false,
            });
        }
    }
}
