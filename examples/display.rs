extern crate dwarf;

use std::env;
use std::path::Path;

fn main() {
    for file in env::args_os().skip(1) {
        display(file.as_ref());
    }
}

fn display(path: &Path) {
    let sections = dwarf::elf::load(path).unwrap();
    let units = dwarf::parse_sections(&sections).unwrap();
    for unit in &units {
        println!("{}", unit);
    }
}
