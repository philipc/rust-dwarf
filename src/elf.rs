extern crate elf;

use std;

use super::{ReadError, AnyEndian, Sections};

impl std::convert::From<elf::ParseError> for ReadError {
    fn from(e: elf::ParseError) -> Self {
        match e {
            elf::ParseError::IoError(e) => ReadError::Io(e),
            elf::ParseError::InvalidMagic => ReadError::Invalid("elf magic".to_string()),
            elf::ParseError::InvalidFormat(_) => ReadError::Invalid("elf format".to_string()),
            elf::ParseError::NotImplemented => ReadError::Unsupported("elf format".to_string()),
        }
    }
}

pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Sections<AnyEndian>, ReadError> {
    let mut file = try!(elf::File::open_path(path.as_ref()));
    let endian = match file.ehdr.data {
        elf::types::ELFDATA2LSB => AnyEndian::Little,
        elf::types::ELFDATA2MSB => AnyEndian::Big,
        data => return Err(ReadError::Unsupported(format!("elf data: {}", data.0))),
    };
    let debug_abbrev = get_section(&mut file, ".debug_abbrev");
    let debug_info = get_section(&mut file, ".debug_info");
    let debug_str = get_section(&mut file, ".debug_str");
    let debug_types = get_section(&mut file, ".debug_types");
    Ok(Sections {
        endian: endian,
        debug_abbrev: debug_abbrev,
        debug_info: debug_info,
        debug_str: debug_str,
        debug_types: debug_types,
    })
}

fn get_section(file: &mut elf::File, name: &str) -> Vec<u8> {
    match file.sections.iter().position(|section| section.shdr.name == name) {
        Some(index) => file.sections.swap_remove(index).data,
        None => Vec::new(),
    }
}
