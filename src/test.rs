use super::*;

#[test]
fn compilation_unit_32() {
    let offset = 0;
    let offset_size = 4;
    let endian = LittleEndian;
    let data = [0x01, 0x23, 0x45, 0x67];
    let write_val = CompilationUnit {
        common: UnitCommon {
            offset: offset,
            endian: endian,
            version: 4,
            address_size: 4,
            offset_size: offset_size,
            abbrev_offset: 0x12,
            data: From::from(&data[..]),
        },
    };

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = CompilationUnit::read(&mut r, offset, endian).unwrap();

    assert_eq!(&buf[..], [
        0x0b, 0x00, 0x00, 0x00,
        0x04, 0x00,
        0x12, 0x00, 0x00, 0x00,
        0x04,
        0x01, 0x23, 0x45, 0x67
    ]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn compilation_unit_64() {
    let offset = 0;
    let offset_size = 8;
    let endian = LittleEndian;
    let data = [0x01, 0x23, 0x45, 0x67];
    let write_val = CompilationUnit {
        common: UnitCommon {
            offset: offset,
            endian: endian,
            version: 4,
            address_size: 4,
            offset_size: offset_size,
            abbrev_offset: 0x12,
            data: From::from(&data[..]),
        },
    };

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = CompilationUnit::read(&mut r, offset, endian).unwrap();

    assert_eq!(&buf[..], [
        0xff, 0xff, 0xff, 0xff, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x04, 0x00,
        0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x04,
        0x01, 0x23, 0x45, 0x67
    ]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn type_unit_32() {
    let offset = 0;
    let offset_size = 4;
    let endian = LittleEndian;
    let data = [0x01, 0x23, 0x45, 0x67];
    let write_val = TypeUnit {
        common: UnitCommon {
            offset: offset,
            endian: endian,
            version: 4,
            address_size: 4,
            offset_size: offset_size,
            abbrev_offset: 0x12,
            data: From::from(&data[..]),
        },
        type_signature: 0x0123456789abcdef,
        type_offset: 0x02,
    };

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = TypeUnit::read(&mut r, offset, endian).unwrap();

    assert_eq!(&buf[..], [
        0x17, 0x00, 0x00, 0x00,
        0x04, 0x00,
        0x12, 0x00, 0x00, 0x00,
        0x04,
        0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,
        0x02, 0x00, 0x00, 0x00,
        0x01, 0x23, 0x45, 0x67
    ]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn type_unit_64() {
    let offset = 0;
    let offset_size = 8;
    let endian = LittleEndian;
    let data = [0x01, 0x23, 0x45, 0x67];
    let write_val = TypeUnit {
        common: UnitCommon {
            offset: offset,
            endian: endian,
            version: 4,
            address_size: 4,
            offset_size: offset_size,
            abbrev_offset: 0x12,
            data: From::from(&data[..]),
        },
        type_signature: 0x0123456789abcdef,
        type_offset: 0x02,
    };

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = TypeUnit::read(&mut r, offset, endian).unwrap();

    assert_eq!(buf, vec![
        0xff, 0xff, 0xff, 0xff, 0x1f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x04, 0x00,
        0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x04,
        0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01,
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x23, 0x45, 0x67
    ]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}
