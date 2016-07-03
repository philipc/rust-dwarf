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
    b.iter(|| {
        let units: Vec<dwarf::CompilationUnit> = sections.compilation_units().collect().unwrap();
        for unit in &units {
            format!("{}", unit);
        }
    });
}

#[bench]
fn display_flat(b: &mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        let units: Vec<dwarf::CompilationUnitHeader> = sections.compilation_unit_headers().collect().unwrap();
        for unit in units {
            let mut entries = unit.entries().unwrap();
            while let Some(entry) = entries.next().unwrap() {
                format!("{}", entry);
            }
        }
    });
}
