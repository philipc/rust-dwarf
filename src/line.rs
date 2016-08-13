use std::borrow::Cow;

use endian::Endian;
use leb128;
use read::*;

#[derive(Debug, PartialEq, Eq)]
pub struct LineNumberProgram<'a, E: Endian> {
    pub offset: usize,
    pub endian: E,
    pub version: u16,
    pub address_size: u8,
    pub offset_size: u8,
    pub minimum_instruction_length: u8,
    pub maximum_operations_per_instruction: u8,
    pub default_is_stmt: bool,
    pub line_base: i8,
    pub line_range: i8,
    pub opcode_base: u8,
    pub standard_opcode_lengths: Cow<'a, [u8]>,
    pub include_directories: Vec<&'a [u8]>,
    pub files: Vec<FileEntry<'a>>,
    pub data: Cow<'a, [u8]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FileEntry<'a> {
    pub path: &'a [u8],
    pub directory: usize,
    pub time: u64,
    pub length: u64,
}

impl<'a, E: Endian> LineNumberProgram<'a, E> {
    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        endian: E,
        address_size: u8,
    ) -> Result<LineNumberProgram<'a, E>, ReadError> {
        let (offset_size, len) = try!(read_initial_length(r, endian));

        // Tell the iterator we read the entire length, even if we don't parse it all now
        let mut data = &r[..len];
        *r = &r[len..];

        let version = try!(endian.read_u16(&mut data));
        // TODO: is this correct?
        if version < 2 || version > 4 {
            return Err(ReadError::Unsupported);
        }

        let header_length = try!(read_offset(&mut data, endian, offset_size)) as usize;
        if header_length > data.len() {
            return Err(ReadError::Invalid);
        }
        let mut header = &data[..header_length];
        let data = &data[header_length..];

        let minimum_instruction_length = try!(read_u8(&mut header));
        let maximum_operations_per_instruction = if version >= 4 {
            try!(read_u8(&mut header))
        } else {
            1
        };
        let default_is_stmt = try!(read_u8(&mut header)) != 0;
        let line_base = try!(read_i8(&mut header));
        let line_range = try!(read_i8(&mut header));
        let opcode_base = try!(read_u8(&mut header));
        let standard_opcode_lengths = if opcode_base > 0 {
            try!(read_block(&mut header, opcode_base as usize - 1))
        } else {
            &[]
        };

        let mut include_directories = Vec::new();
        loop {
            if header.len() < 1 {
                return Err(ReadError::Invalid);
            }
            if header[0] == 0 {
                header = &header[1..];
                break;
            }
            include_directories.push(try!(read_string(&mut header)));
        }

        let mut files = Vec::new();
        loop {
            if header.len() < 1 {
                return Err(ReadError::Invalid);
            }
            if header[0] == 0 {
                //header = &header[1..];
                break;
            }
            let path = try!(read_string(&mut header));
            // Note: not validating this here
            let directory = try!(leb128::read_u64(&mut header)) as usize;
            let time = try!(leb128::read_u64(&mut header));
            let length = try!(leb128::read_u64(&mut header));
            files.push(FileEntry {
                path: path,
                directory: directory,
                time: time,
                length: length,
            });
        }

        Ok(LineNumberProgram {
            offset: offset,
            endian: endian,
            version: version,
            address_size: address_size,
            offset_size: offset_size,
            minimum_instruction_length: minimum_instruction_length,
            maximum_operations_per_instruction: maximum_operations_per_instruction,
            default_is_stmt: default_is_stmt,
            line_base: line_base,
            line_range: line_range,
            opcode_base: opcode_base,
            standard_opcode_lengths: From::from(standard_opcode_lengths),
            include_directories: include_directories,
            files: files,
            data: From::from(data),
        })
    }
}

/*
struct LineIterator {
    line: Line,
}

struct Line {
}
*/
