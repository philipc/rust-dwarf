extern crate dwarf;

use dwarf::*;

#[test]
fn read_and_display() {
    let path = std::env::args_os().next().unwrap();
    let sections = dwarf::elf::load(path).unwrap();
    let mut units = sections.compilation_units();
    let mut buf = Vec::new();
    let mut f = dwarf::display::DefaultFormatter::new(&mut buf, 4);
    while let Some(unit) = units.next().unwrap() {
        let abbrev = sections.abbrev(&unit).unwrap();
        unit.entries(&abbrev).display(&mut f).unwrap();
    }
}

#[test]
fn read_and_write() {
    let path = std::env::args_os().next().unwrap();
    let sections = dwarf::elf::load(path).unwrap();
    let mut units = sections.compilation_units();
    while let Some(read_unit) = units.next().unwrap() {
        let abbrev = sections.abbrev(&read_unit).unwrap();

        let mut entries = read_unit.entries(&abbrev);
        let mut write_unit = CompilationUnit::new(read_unit.endian, read_unit.address_size);
        write_unit.data_offset = read_unit.data_offset;
        while let Some(entry) = entries.next().unwrap() {
            entry.write(&mut write_unit, &abbrev).unwrap();
        }

        let mut read_entries = read_unit.entries(&abbrev);
        let mut write_entries = write_unit.entries(&abbrev);
        loop {
            let read_entry = read_entries.next().unwrap();
            let write_entry = write_entries.next().unwrap();
            assert_eq!(read_entry, write_entry);
            if !read_entry.is_some() {
                break;
            }
        }

        assert_eq!(read_unit.data(), write_unit.data());
    }
}
