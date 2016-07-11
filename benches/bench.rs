#![feature(test)]

extern crate test;
extern crate dwarf;

#[bench]
fn read(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let mut units = sections.compilation_units();
        while let Some(unit) = units.next().unwrap() {
            let buffer = unit.die_buffer(&sections).unwrap();
            let mut entries = buffer.entries();
            while let Some(_) = entries.next().unwrap() {
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
            let buffer = unit.die_buffer(&sections).unwrap();
            buffer.entries().display(&mut f).unwrap();
        }
    });
}
