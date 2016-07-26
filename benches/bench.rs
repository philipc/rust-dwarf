#![feature(test)]

extern crate test;
extern crate dwarf;
extern crate gimli;

#[bench]
fn read_dwarf(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut units = sections.compilation_units();
        while let Some(unit) = units.next().unwrap() {
            let abbrev = sections.abbrev(&unit.common).unwrap();
            let mut entries = unit.entries(&abbrev);
            while let Some(entry) = entries.next().unwrap() {
                for attribute in &entry.attributes {
                    test::black_box(attribute);
                }
            }
        }
    });
}

#[bench]
fn display(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    let mut buf = Vec::new();
    let mut f = dwarf::display::DefaultFormatter::new(&mut buf, 4);
    b.iter(|| {
        let mut units = sections.compilation_units();
        while let Some(unit) = units.next().unwrap() {
            let abbrev = sections.abbrev(&unit.common).unwrap();
            unit.entries(&abbrev).display(&mut f).unwrap();
        }
    });
}

#[bench]
fn read_gimli(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let debug_info = gimli::DebugInfo::<gimli::LittleEndian>::new(&sections.debug_info);
        let debug_abbrev = gimli::DebugAbbrev::<gimli::LittleEndian>::new(&sections.debug_abbrev);
        for unit in debug_info.units() {
            let unit = unit.unwrap();
            let abbrevs = unit.abbreviations(debug_abbrev).unwrap();
            let mut cursor = unit.entries(&abbrevs);
            loop {
                let entry = cursor.current().unwrap().unwrap();
                for attr in entry.attrs() {
                    test::black_box(attr.unwrap());
                }
                if let None = cursor.next_dfs() {
                    break;
                }
            }
        }
    });
}
