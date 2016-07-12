extern crate dwarf;

use std::borrow::Cow;

use dwarf::*;
use dwarf::constant::*;

#[test]
fn read_and_display() {
    let path = std::env::args_os().next().unwrap();
    let sections = dwarf::elf::load(path).unwrap();
    let mut units = sections.compilation_units();
    let mut buf = Vec::new();
    let mut f = dwarf::display::DefaultFormatter::new(&mut buf, 4);
    while let Some(unit) = units.next().unwrap() {
        let abbrev = unit.abbrev(&sections).unwrap();
        unit.die_buffer(&sections).entries(&abbrev).display(&mut f).unwrap();
    }
}

#[test]
fn die_buffer() {
    let path = std::env::args_os().next().unwrap();
    let sections = dwarf::elf::load(path).unwrap();
    let mut units = sections.compilation_units();
    while let Some(unit) = units.next().unwrap() {
        let abbrev = unit.abbrev(&sections).unwrap();

        let read_buffer = unit.die_buffer(&sections);
        let mut entries = read_buffer.entries(&abbrev);
        let mut write_buffer = DieBuffer::new(
            sections.endian, unit.address_size, Cow::Borrowed(&[]), unit.data_offset);
        while let Some(entry) = entries.next().unwrap() {
            entry.write(&mut write_buffer, &abbrev).unwrap();
        }

        let mut read_entries = read_buffer.entries(&abbrev);
        let mut write_entries = write_buffer.entries(&abbrev);
        loop {
            let read_entry = read_entries.next().unwrap();
            let write_entry = write_entries.next().unwrap();
            assert_eq!(read_entry, write_entry);
            if !read_entry.is_some() {
                break;
            }
        }

        assert_eq!(read_buffer.data(), write_buffer.data());
    }
}

#[test]
fn die() {
    let endian = Endian::Little;
    let address_size = 4;
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

    let mut buffer = DieBuffer::new(endian, address_size, Cow::Borrowed(&[]), 0);
    write_val.write(&mut buffer, &abbrev_hash).unwrap();

    let mut r = buffer.data();
    let read_val = Die::read(&mut r, write_val.offset, &buffer, &abbrev_hash).unwrap();

    assert_eq!(buffer.data(), [1, b't', b'e', b's', b't', 0]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn attribute() {
    let endian = Endian::Little;
    let address_size = 4;
    let abbrev = AbbrevAttribute { at: DW_AT_sibling, form: DW_FORM_ref4 };
    let write_val = Attribute {
        at: DW_AT_sibling,
        data: AttributeData::Ref(0x01234567),
    };

    let mut buffer = DieBuffer::new(endian, address_size, Cow::Borrowed(&[]), 0);
    write_val.write(&mut buffer, &abbrev).unwrap();

    let mut r = buffer.data();
    let read_val = Attribute::read(&mut r, &buffer, &abbrev).unwrap();

    assert_eq!(buffer.data(), [0x67, 0x45, 0x23, 0x01]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, write_val);
}

#[test]
fn attribute_data() {
    let endian = Endian::Little;
    let address_size = 4;
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
        for &indirect in &[false, true] {
            let mut buffer = DieBuffer::new(endian, address_size, Cow::Borrowed(&[]), 0);
            write_val.write(&mut buffer, form, indirect).unwrap();
            let buf = buffer.data();

            let read_form = if indirect { DW_FORM_indirect } else { form };
            let mut r = buf;
            let read_val = AttributeData::read(&mut r, &buffer, read_form).unwrap();

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
