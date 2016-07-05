extern crate elf;

use std;

use super::{DecodeError, Endian, Sections};

impl std::convert::From<elf::ParseError> for DecodeError {
    fn from(e: elf::ParseError) -> Self {
        match e {
            elf::ParseError::IoError(e) => DecodeError::Io(e),
            elf::ParseError::InvalidMagic => DecodeError::Invalid("elf magic".to_string()),
            elf::ParseError::InvalidFormat(_) => DecodeError::Invalid("elf format".to_string()),
            elf::ParseError::NotImplemented => DecodeError::Unsupported("elf format".to_string()),
        }
    }
}

pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Sections, DecodeError> {
    let mut file = try!(elf::File::open_path(path.as_ref()));
    let endian = match file.ehdr.data {
        elf::types::ELFDATA2LSB => Endian::Little,
        elf::types::ELFDATA2MSB => Endian::Big,
        data => return Err(DecodeError::Unsupported(format!("elf data: {}", data.0))),
    };
    let debug_info = try!(get_section(&mut file, ".debug_info")).data;
    let debug_str = try!(get_section(&mut file, ".debug_str")).data;
    let debug_abbrev = try!(get_section(&mut file, ".debug_abbrev")).data;
    Ok(Sections {
        endian: endian,
        debug_info: debug_info,
        debug_str: debug_str,
        debug_abbrev: debug_abbrev,
    })
}

fn get_section(file: &mut elf::File, name: &str) -> Result<elf::Section, DecodeError> {
    match file.sections.iter().position(|section| section.shdr.name == name) {
        Some(index) => Ok(file.sections.swap_remove(index)),
        None => Err(DecodeError::Invalid(format!("missing elf section: {}", name))),
    }
}
