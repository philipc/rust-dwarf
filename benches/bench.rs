#![feature(test)]

extern crate test;
extern crate dwarf;

#[bench]
fn parse(b:&mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    b.iter(|| {
        dwarf::parse_sections(&sections).unwrap();
    });
}

#[bench]
fn display(b:&mut test::Bencher) {
    let path = std::env::args_os().next().unwrap(); // Note: not constant
    let sections = dwarf::elf::load(path).unwrap();
    let units = dwarf::parse_sections(&sections).unwrap();
    b.iter(|| {
        for unit in &units {
            format!("{}", unit);
        }
    });
}
