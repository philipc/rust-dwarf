use std;
use std::convert::From;
use std::io::{Read, Write};
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

pub fn read_u64<R: Read>(r: &mut R) -> Result<u64, Error> {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let byte = try!(r.read_u8());
        if shift == 63 && byte != 0x00 && byte != 0x01 {
            return Err(Error::Overflow);
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

pub fn read_i64<R: Read>(r: &mut R) -> Result<i64, Error> {
    let mut result = 0;
    let mut shift = 0;
    let size = 64;
    loop {
        let byte = try!(r.read_u8());
        if shift == 63 && byte != 0x00 && byte != 0x7f {
            return Err(Error::Overflow);
        }
        result |= i64::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if shift < size && (byte & 0x40) != 0 {
                // Sign extend
                result |= !0 << shift;
            }
            return Ok(result);
        }
    }
}

pub fn read_u16<R: Read>(r: &mut R) -> Result<u16, Error> {
    let val = try!(read_u64(r));
    if val > std::u16::MAX as u64 {
            return Err(Error::Overflow);
    }
    Ok(val as u16)
}

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

pub fn write_u16<W: Write>(w: &mut W, value: u16) -> std::io::Result<()> {
    write_u64(w, value as u64)
}

#[cfg(test)]
mod test {
    use super::*;
    use std;

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
        ] {
            assert_eq!(read_u16(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read overflow
        for &(mut r,) in &[
            (&[0xff,0xff,0x07][..],),
        ] {
            match read_u16(&mut r) {
                Err(Error::Overflow) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff][..],),
        ] {
            match read_u16(&mut r) {
                Err(Error::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
                otherwise => panic!("{:?}", otherwise),
            };
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
        ] {
            assert_eq!(read_u64(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read overflow
        for &(mut r,) in &[
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x02][..],),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x7f][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff][..],),
        ] {
            match read_u64(&mut r) {
                Err(Error::Overflow) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80][..],),
            (&[0xff,0xff][..],),
        ] {
            match read_u64(&mut r) {
                Err(Error::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Write EOF
        {
            let mut buf = &mut [0; 2][..];
            match write_u64(&mut buf, 0xffff) {
                Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::WriteZero),
                otherwise => panic!("{:?}", otherwise),
            };
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
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x7f][..], -0x8000000000000000),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x7f][..], -1),
        ] {
            assert_eq!(read_i64(&mut r).unwrap(), value);
            assert_eq!(r.len(), 0);
        }

        // Read overflow
        for &(mut r,) in &[
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x01][..],),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x3f][..],),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x40][..],),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x41][..],),
            (&[0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x01][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0x3f][..],),
            (&[0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff][..],),
        ] {
            match read_i64(&mut r) {
                Err(Error::Overflow) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80][..],),
            (&[0xff,0xff][..],),
        ] {
            match read_i64(&mut r) {
                Err(Error::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Write EOF
        {
            let mut buf = &mut [0; 2][..];
            match write_i64(&mut buf, 0xffff) {
                Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::WriteZero),
                otherwise => panic!("{:?}", otherwise),
            };
        }
    }
}
