extern crate dwarf;

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
        unit.display(&mut f).unwrap();
    }
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
        (AttributeData::String("test"), DW_FORM_strp, &[0; 4][..]),
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
            let mut debug_str = Vec::new();
            let mut buf = Vec::new();
            write_val.write(
                &mut buf, endian, address_size, &mut debug_str, form, indirect).unwrap();

            let read_form = if indirect { DW_FORM_indirect } else { form };
            let mut r = &buf[..];
            let read_val = AttributeData::read(
                &mut r, endian, address_size, &debug_str[..], read_form).unwrap();

            if indirect {
                assert_eq!(buf[0] as u16, form.0);
                assert_eq!(&buf[1..], expect);
            } else {
                assert_eq!(&buf[..], expect);
            }
            assert_eq!(r.len(), 0);
            assert_eq!(read_val, *write_val);
            if form == DW_FORM_strp {
                assert_eq!(debug_str, [b't', b'e', b's', b't', 0]);
            } else {
                assert_eq!(debug_str.len(), 0);
            }
        }
    }
}

#[test]
fn abbrev_container() {
    let write_val = AbbrevVec::new(vec![
        Abbrev {
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
    for (code, abbrev) in write_val.iter() {
        assert_eq!(Some(abbrev), read_val.get(code));
    }
}

#[test]
fn abbrev() {
    let write_code = 1;
    let write_val = Abbrev {
        tag: DW_TAG_namespace,
        children: true,
        attributes: vec![
            AbbrevAttribute { at: DW_AT_name, form: DW_FORM_strp },
        ],
    };

    let mut buf = Vec::new();
    write_val.write(&mut buf, write_code).unwrap();

    let mut r = &buf[..];
    let read_val = Abbrev::read(&mut r).unwrap();

    assert_eq!(&buf[..], [1, 57, 1, 3, 14, 0, 0]);
    assert_eq!(r.len(), 0);
    assert_eq!(read_val, Some((write_code, write_val)));
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
