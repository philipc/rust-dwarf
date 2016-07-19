extern crate leb128;

use std;
use std::io::{Read, Write};

pub use self::leb128::read::Error;
pub use self::leb128::read::unsigned as read_u64;
pub use self::leb128::read::signed as read_i64;

pub fn read_u16<R: Read>(r: &mut R) -> Result<u16, Error> {
    let val = try!(read_u64(r));
    if val > std::u16::MAX as u64 {
            return Err(Error::Overflow);
    }
    Ok(val as u16)
}

pub fn write_u64<W: Write>(w: &mut W, value: u64) -> std::io::Result<()> {
    try!(leb128::write::unsigned(w, value));
    Ok(())
}

pub fn write_i64<W: Write>(w: &mut W, value: i64) -> std::io::Result<()> {
    try!(leb128::write::signed(w, value as i64));
    Ok(())
}

pub fn write_u16<W: Write>(w: &mut W, value: u16) -> std::io::Result<()> {
    try!(write_u64(w, value as u64));
    Ok(())
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
                Err(Error::IoError(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
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
                Err(Error::IoError(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
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
                Err(Error::IoError(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
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
