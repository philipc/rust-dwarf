extern crate byteorder;
extern crate elf;
extern crate dwarf;

use std::env;
use std::path::Path;
use byteorder::{BigEndian, LittleEndian};

fn main() {
    for file in env::args_os().skip(1) {
        display(file.as_ref());
    }
}

fn display(path: &Path) {
    let file = elf::File::open_path(path).unwrap();
    let info = &file.get_section(".debug_info").unwrap().data;
    let debug_str = &file.get_section(".debug_str").unwrap().data;
    let abbrev = &file.get_section(".debug_abbrev").unwrap().data;
    let units = match file.ehdr.data {
        elf::types::ELFDATA2LSB => { dwarf::parse_debug_info::<LittleEndian>(info, debug_str, abbrev).unwrap() }
        elf::types::ELFDATA2MSB => { dwarf::parse_debug_info::<BigEndian>(info, debug_str, abbrev).unwrap() }
        _ => { panic!("Unable to resolve file endianness"); }
    };
    for unit in units {
        println!("{}", unit);
    }
}
