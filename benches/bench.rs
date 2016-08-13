#![feature(test)]

extern crate test;
extern crate dwarf;

#[bench]
fn display_info(b: &mut test::Bencher) {
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
fn read_info(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut units = sections.compilation_units();
        while let Some(unit) = units.next().unwrap() {
            let abbrev = sections.abbrev(&unit.common).unwrap();
            let mut entries = unit.entries(&abbrev);
            while let Some(entry) = entries.next().unwrap() {
                test::black_box(entry.tag);
                for attribute in &entry.attributes {
                    test::black_box(attribute.at);
                    test::black_box(&attribute.data);
                }
            }
        }
    });
}

#[bench]
fn read_line(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut units = sections.compilation_units();
        while let Some(unit) = units.next().unwrap() {
            let abbrev = sections.abbrev(&unit.common).unwrap();
            if let Some(line_program) = sections.line_program(&unit, &abbrev).unwrap() {
                let mut lines = line_program.lines();
                while let Some(line) = lines.next().unwrap() {
                    test::black_box(line);
                }
            }
        }
    });
}
