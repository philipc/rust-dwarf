use super::*;
use constant::*;

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

#[test]
fn die_cursor() {
    let mut abbrev_hash = AbbrevHash::new();
    abbrev_hash.insert(Abbrev {
        code: 1,
        tag: DW_TAG_namespace,
        children: true,
        attributes: vec![
            AbbrevAttribute { at: DW_AT_name, form: DW_FORM_string },
        ],
    });
    abbrev_hash.insert(Abbrev {
        code: 2,
        tag: DW_TAG_namespace,
        children: false,
        attributes: vec![
            AbbrevAttribute { at: DW_AT_name, form: DW_FORM_string },
        ],
    });

    fn entry<'a>(name: &'a str, children: bool) -> Die<'a> {
        Die {
            offset: 0,
            code: if children { 1 } else { 2},
            tag: DW_TAG_namespace,
            children: children,
            attributes: vec![
                Attribute { at: DW_AT_name, data: AttributeData::String(name) },
            ],
        }
    }

    let mut write_val = [
        entry("0", true),
            entry("1", false),
            entry("2", true),
                Die::null(0),
            entry("4", true),
                entry("5", false),
                Die::null(0),
            entry("7", true),
                entry("8", true),
                    entry("9", true),
                        Die::null(0),
                    Die::null(0),
                Die::null(0),
            entry("13", false),
            Die::null(0),
        entry("15", false),
    ];
    let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
    for mut entry in &mut write_val {
        entry.offset = unit.len();
        entry.write(&mut unit, &abbrev_hash).unwrap();
    }

    let mut entries = unit.entries(0, &abbrev_hash);
    for i in 0..write_val.len() {
        match entries.next() {
            Ok(Some(read_val)) => assert_eq!(read_val, write_val[i]),
            _ => panic!(),
        }
    }
    assert!(entries.next().unwrap().is_none());

    let mut entries = unit.entries(0, &abbrev_hash);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[0]);
    assert_eq!(entries.next().unwrap().unwrap(), write_val[1]);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[2]);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[4]);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[7]);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[13]);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[14]);
    assert_eq!(entries.next_sibling().unwrap().unwrap(), write_val[15]);
    assert!(entries.next_sibling().unwrap().is_none());

    // TODO test DW_AT_sibling
}

#[test]
fn die() {
    let mut abbrev_hash = AbbrevHash::new();
    let code = 1;
    abbrev_hash.insert(Abbrev {
        code: code,
        tag: DW_TAG_namespace,
        children: true,
        attributes: vec![
            AbbrevAttribute { at: DW_AT_name, form: DW_FORM_string },
        ],
    });
    let write_val = Die {
        offset: 0,
        code: code,
        tag: DW_TAG_namespace,
        children: true,
        attributes: vec![
            Attribute { at: DW_AT_name, data: AttributeData::String("test") },
        ],
    };

    let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
    write_val.write(&mut unit, &abbrev_hash).unwrap();

    let mut r = unit.data();
    let read_val = Die::read(&mut r, write_val.offset, &unit, &abbrev_hash).unwrap();

    assert_eq!(unit.data(), [1, b't', b'e', b's', b't', 0]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn attribute() {
    let abbrev = AbbrevAttribute { at: DW_AT_sibling, form: DW_FORM_ref4 };
    let write_val = Attribute {
        at: DW_AT_sibling,
        data: AttributeData::Ref(0x01234567),
    };

    let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
    write_val.write(&mut unit, &abbrev).unwrap();

    let mut r = unit.data();
    let read_val = Attribute::read(&mut r, &unit, &abbrev).unwrap();

    assert_eq!(unit.data(), [0x67, 0x45, 0x23, 0x01]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn attribute_data() {
    let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };

    unit.address_size = 4;
    unit.offset_size = 4;
    for &(ref write_val, form, expect) in &[
        (AttributeData::Address(0x12345678), DW_FORM_addr, &[0x78, 0x56, 0x34, 0x12][..]),
        (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block1, &[0x3, 0x11, 0x22, 0x33][..]),
        (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block2, &[0x3, 0x00, 0x11, 0x22, 0x33][..]),
        (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block4, &[0x3, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33][..]),
        (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block, &[0x3, 0x11, 0x22, 0x33][..]),
        (AttributeData::Data1(0x01), DW_FORM_data1, &[0x01][..]),
        (AttributeData::Data2(0x0123), DW_FORM_data2, &[0x23, 0x01][..]),
        (AttributeData::Data4(0x01234567), DW_FORM_data4, &[0x67, 0x45, 0x23, 0x01][..]),
        (AttributeData::Data8(0x0123456789abcdef), DW_FORM_data8, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01][..]),
        (AttributeData::UData(0x01234567), DW_FORM_udata, &[231, 138, 141, 9][..]),
        (AttributeData::SData(0x01234567), DW_FORM_sdata, &[231, 138, 141, 9][..]),
        (AttributeData::SData(-0x01234567), DW_FORM_sdata, &[153, 245, 242, 118][..]),
        (AttributeData::Flag(false), DW_FORM_flag, &[0][..]),
        (AttributeData::Flag(true), DW_FORM_flag, &[1][..]),
        (AttributeData::Flag(true), DW_FORM_flag_present, &[][..]),
        (AttributeData::String("test"), DW_FORM_string, &[b't', b'e', b's', b't', 0][..]),
        (AttributeData::StringOffset(0x01234567), DW_FORM_strp, &[0x67, 0x45, 0x23, 0x01][..]),
        (AttributeData::Ref(0x01), DW_FORM_ref1, &[0x01][..]),
        (AttributeData::Ref(0x0123), DW_FORM_ref2, &[0x23, 0x01][..]),
        (AttributeData::Ref(0x01234567), DW_FORM_ref4, &[0x67, 0x45, 0x23, 0x01][..]),
        (AttributeData::Ref(0x0123456789abcdef), DW_FORM_ref8, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01][..]),
        (AttributeData::Ref(0x01234567), DW_FORM_ref_udata, &[231, 138, 141, 9][..]),
        (AttributeData::RefAddress(0x12345678), DW_FORM_ref_addr, &[0x78, 0x56, 0x34, 0x12][..]),
        (AttributeData::RefSig(0x0123456789abcdef), DW_FORM_ref_sig8, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01][..]),
        (AttributeData::SecOffset(0x12345678), DW_FORM_sec_offset, &[0x78, 0x56, 0x34, 0x12][..]),
        (AttributeData::ExprLoc(&[0x11, 0x22, 0x33]), DW_FORM_exprloc, &[0x3, 0x11, 0x22, 0x33][..]),
    ] {
        attribute_data_inner(&mut unit, write_val, form, expect);
    }

    unit.address_size = 8;
    unit.offset_size = 4;
    for &(ref write_val, form, expect) in &[
        (AttributeData::Address(0x0123456789), DW_FORM_addr,
            &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
    ] {
        attribute_data_inner(&mut unit, write_val, form, expect);
    }

    unit.address_size = 4;
    unit.offset_size = 8;
    for &(ref write_val, form, expect) in &[
        (AttributeData::StringOffset(0x0123456789), DW_FORM_strp,
            &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
        (AttributeData::RefAddress(0x0123456789), DW_FORM_ref_addr,
            &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
        (AttributeData::SecOffset(0x0123456789), DW_FORM_sec_offset,
            &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
    ] {
        attribute_data_inner(&mut unit, write_val, form, expect);
    }
}

fn attribute_data_inner<'a, 'b, E: Endian>(
    unit: &mut UnitCommon<'a, E>,
    write_val: &AttributeData<'b>,
    form: DwForm, expect: &[u8],
) {
    for &indirect in &[false, true] {
        unit.data = Default::default();
        write_val.write(unit, form, indirect).unwrap();
        let buf = unit.data();

        let read_form = if indirect { DW_FORM_indirect } else { form };
        let mut r = buf;
        let read_val = AttributeData::read(&mut r, unit, read_form).unwrap();

        if indirect {
            assert_eq!(buf[0] as u16, form.0);
            assert_eq!(&buf[1..], expect);
        } else {
            assert_eq!(&buf[..], expect);
        }
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, *write_val);
    }
}

#[test]
fn abbrev_container() {
    let write_val = AbbrevVec::new(vec![
        Abbrev {
            code: 1,
            tag: DW_TAG_namespace,
            children: true,
            attributes: vec![
                AbbrevAttribute { at: DW_AT_name, form: DW_FORM_strp },
            ],
        },
    ]);

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = AbbrevHash::read(&mut r).unwrap();

    assert_eq!(&buf[..], [1, 57, 1, 3, 14, 0, 0, 0]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val.len(), write_val.len());
    for abbrev in write_val.iter() {
        assert_eq!(Some(abbrev), read_val.get(abbrev.code));
    }
}

#[test]
fn abbrev() {
    let write_val = Abbrev {
        code: 1,
        tag: DW_TAG_namespace,
        children: true,
        attributes: vec![
            AbbrevAttribute { at: DW_AT_name, form: DW_FORM_strp },
        ],
    };

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = Abbrev::read(&mut r).unwrap();

    assert_eq!(&buf[..], [1, 57, 1, 3, 14, 0, 0]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, Some(write_val));
}

#[test]
fn abbrev_attribute() {
    let write_val = AbbrevAttribute { at: DW_AT_sibling, form: DW_FORM_ref4 };

    let mut buf = Vec::new();
    write_val.write(&mut buf).unwrap();

    let mut r = &buf[..];
    let read_val = AbbrevAttribute::read(&mut r).unwrap();

    assert_eq!(&buf[..], [1, 19]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, Some(write_val));
}