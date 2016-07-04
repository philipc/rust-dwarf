#![feature(test)]

extern crate test;
extern crate dwarf;

use dwarf::FallibleIterator;

#[bench]
fn parse(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let _: Vec<dwarf::CompilationUnit> = sections.compilation_units().collect().unwrap();
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
            unit.display(&mut f).unwrap();
        }
    });
}

#[bench]
fn display_headers(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    let mut buf = Vec::new();
    let mut f = dwarf::display::DefaultFormatter::new(&mut buf, 4);
    b.iter(|| {
        let mut units = sections.compilation_unit_headers();
        while let Some(unit) = units.next().unwrap() {
            unit.display(&mut f).unwrap();
        }
    });
}
