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
    let units: Vec<dwarf::CompilationUnit> = sections.compilation_units().collect().unwrap();
    b.iter(|| {
        for unit in &units {
            format!("{}", unit);
        }
    });
}
