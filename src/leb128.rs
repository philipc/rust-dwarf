use std;
use std::convert::From;
use std::io::Write;
use read::{read_u8, ReadError};
use write::write_u8;

pub fn read_u64(r: &mut &[u8]) -> Result<u64, ReadError> {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let byte = try!(read_u8(r));
        if shift == 63 && byte != 0x00 && byte != 0x01 {
            return Err(ReadError::Overflow);
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

pub fn read_i64(r: &mut &[u8]) -> Result<i64, ReadError> {
    let mut result = 0;
    let mut shift = 0;
    let size = 64;
    loop {
        let byte = try!(read_u8(r));
        if shift == 63 && byte != 0x00 && byte != 0x7f {
            return Err(ReadError::Overflow);
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

pub fn read_u16(r: &mut &[u8]) -> Result<u16, ReadError> {
    let val = try!(read_u64(r));
    if val > std::u16::MAX as u64 {
        return Err(ReadError::Overflow);
    }
    Ok(val as u16)
}

pub fn write_u64<W: Write>(w: &mut W, mut value: u64) -> std::io::Result<()> {
    loop {
        let byte = value as u8 & 0x7f;
        value >>= 7;
        if value == 0 {
            try!(write_u8(w, byte));
            return Ok(());
        }
        try!(write_u8(w, byte | 0x80));
    }
}

pub fn write_i64<W: Write>(w: &mut W, mut value: i64) -> std::io::Result<()> {
    loop {
        let byte = value as u8 & 0x7f;
        value >>= 6;
        if value == 0 || value == -1 {
            try!(write_u8(w, byte));
            return Ok(());
        }
        value >>= 1;
        try!(write_u8(w, byte | 0x80));
    }
}

pub fn write_u16<W: Write>(w: &mut W, value: u16) -> std::io::Result<()> {
    try!(write_u64(w, value as u64));
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use read::ReadError;
    use std;

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
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
                Err(ReadError::Overflow) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80,0x80,0x80][..],),
            (&[0xff,0xff,0xff][..],),
        ] {
            match read_u16(&mut r) {
                Err(ReadError::Eof) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
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
                Err(ReadError::Overflow) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80][..],),
            (&[0xff,0xff][..],),
        ] {
            match read_u64(&mut r) {
                Err(ReadError::Eof) => {},
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
    #[cfg_attr(rustfmt, rustfmt_skip)]
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
                Err(ReadError::Overflow) => {},
                otherwise => panic!("{:?}", otherwise),
            };
        }

        // Read EOF
        for &(mut r,) in &[
            (&[0x80][..],),
            (&[0xff,0xff][..],),
        ] {
            match read_i64(&mut r) {
                Err(ReadError::Eof) => {},
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
