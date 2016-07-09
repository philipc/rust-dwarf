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
