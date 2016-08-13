use std::borrow::Cow;

use constant;
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
    pub line_range: u8,
    pub opcode_base: u8,
    pub standard_opcode_lengths: Cow<'a, [u8]>,
    pub include_directories: Vec<&'a [u8]>,
    pub files: Vec<FileEntry<'a>>,
    pub data: Cow<'a, [u8]>,
}

impl<'a, E: Endian> LineNumberProgram<'a, E> {
    pub fn lines(&self) -> LineIterator<E> {
        LineIterator::new(self)
    }

    pub fn read(
        r: &mut &'a [u8],
        offset: usize,
        endian: E,
        address_size: u8,
    ) -> Result<LineNumberProgram<'a, E>, ReadError> {
        let (offset_size, len) = try!(read_initial_length(r, endian));
        let mut data = &r[..len];

        let version = try!(endian.read_u16(&mut data));
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
        if minimum_instruction_length == 0 {
            return Err(ReadError::Invalid);
        }

        let maximum_operations_per_instruction = if version >= 4 {
            try!(read_u8(&mut header))
        } else {
            1
        };
        if maximum_operations_per_instruction == 0 {
            return Err(ReadError::Invalid);
        }

        let default_is_stmt = try!(read_u8(&mut header)) != 0;
        let line_base = try!(read_i8(&mut header));
        let line_range = try!(read_u8(&mut header));
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
                header = &header[1..];
                break;
            }
            files.push(try!(FileEntry::read(&mut header)));
        }

        if header.len() != 0 {
            return Err(ReadError::Invalid);
        }

        *r = &r[len..];
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

pub struct LineIterator<'a, E: 'a+Endian> {
    program: &'a LineNumberProgram<'a, E>,
    data: &'a [u8],
    files: Vec<FileEntry<'a>>,
    line: Line<'a>,
    file: usize,
    copy: bool,
    reset: bool,
}

impl<'a, E: Endian> LineIterator<'a, E> {
    pub fn new(program: &'a LineNumberProgram<'a, E>) -> Self {
        LineIterator {
            program: program,
            data: program.data.as_ref(),
            files: Vec::new(),
            line: Line::new(program.default_is_stmt),
            file: 1,
            copy: false,
            reset: false,
        }
    }

    pub fn next(&mut self) -> Result<Option<&Line>, ReadError> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        if self.line.end_sequence {
            self.line = Line::new(self.program.default_is_stmt);
            self.file = 1;
        }

        if self.reset {
            self.line.basic_block = false;
            self.line.prologue_end = false;
            self.line.epilogue_begin = false;
            self.line.discriminator = 0;
            self.reset = false;
        }

        let mut r = self.data;
        loop {
            try!(self.next_opcode(&mut r));
            self.data = r;
            if self.copy {
                self.copy = false;
                self.set_file();
                return Ok(Some(&self.line));
            }
        }
    }

    fn next_opcode(&mut self, r: &mut &'a [u8]) -> Result<(), ReadError> {
        let opcode = try!(read_u8(r));
        match constant::DwLns(opcode) {
            constant::DW_LNS_extended => try!(self.next_extended(r)),
            constant::DW_LNS_copy => {
                self.copy = true;
                self.reset = true;
            }
            constant::DW_LNS_advance_pc => self.advance_pc(try!(leb128::read_u64(r))),
            constant::DW_LNS_advance_line => self.advance_line(try!(leb128::read_i64(r))),
            constant::DW_LNS_set_file => self.file = try!(leb128::read_u64(r)) as usize,
            constant::DW_LNS_set_column => self.line.column = try!(leb128::read_u64(r)),
            constant::DW_LNS_negate_stmt => self.line.is_stmt = !self.line.is_stmt,
            constant::DW_LNS_set_basic_block => self.line.basic_block = true,
            constant::DW_LNS_const_add_pc => {
                let op_delta = (255 - self.program.opcode_base) / self.program.line_range;
                self.advance_pc(op_delta as u64);
            }
            constant::DW_LNS_fixed_advance_pc => {
                self.line.address += try!(self.program.endian.read_u16(r)) as u64;
                self.line.op_index = 0;
            }
            constant::DW_LNS_set_prologue_end => self.line.prologue_end = true,
            constant::DW_LNS_set_epilogue_begin => self.line.epilogue_begin = true,
            constant::DW_LNS_set_isa => self.line.isa = try!(leb128::read_u64(r)),
            _ => {
                let opcode = opcode as usize;
                if opcode < self.program.opcode_base as usize {
                    // Unknown opcode, skip over it
                    if opcode - 1 >= self.program.standard_opcode_lengths.len() {
                        return Err(ReadError::Invalid);
                    }
                    for _ in 0..self.program.standard_opcode_lengths[opcode - 1] {
                        try!(leb128::read_u64(r));
                    }
                } else {
                    self.advance_special(opcode as u64);
                    self.copy = true;
                }
            }
        }
        Ok(())
    }

    fn next_extended(&mut self, r: &mut &'a [u8]) -> Result<(), ReadError> {
        let len = try!(leb128::read_u64(r)) as usize;
        if len > r.len() {
            return Err(ReadError::Invalid);
        }
        let mut data = &r[..len];
        *r = &r[len..];

        let opcode = try!(read_u8(&mut data));
        match constant::DwLne(opcode) {
            constant::DwLne(0) => return Err(ReadError::Invalid),
            constant::DW_LNE_end_sequence => {
                self.line.end_sequence = true;
                self.copy = true;
            }
            constant::DW_LNE_set_address => {
                self.line.address = try!(read_address(&mut data, self.program.endian, self.program.address_size));
                self.line.op_index = 0;
            }
            constant::DW_LNE_define_file => {
                self.files.push(try!(FileEntry::read(&mut data)));
            }
            constant::DW_LNE_set_discriminator => {
                self.line.discriminator = try!(leb128::read_u64(&mut data));
            }
            _ => {},
        }
        Ok(())
    }

    fn advance_special(&mut self, opcode: u64) {
        let delta = opcode - self.program.opcode_base as u64;
        let op_delta = delta / self.program.line_range as u64;
        let line_delta = delta % self.program.line_range as u64;
        let line_delta = self.program.line_base as i64 + line_delta as i64;
        self.advance_pc(op_delta);
        self.advance_line(line_delta);
    }

    fn advance_pc(&mut self, op_delta: u64) {
        let op_index = self.line.op_index + op_delta;
        let address_delta = op_index / self.program.maximum_operations_per_instruction as u64;
        self.line.op_index = op_index % self.program.maximum_operations_per_instruction as u64;
        self.line.address += self.program.minimum_instruction_length as u64 * address_delta;
    }

    fn advance_line(&mut self, delta: i64) {
        self.line.line = self.line.line.wrapping_add(delta as u64);
    }

    // TODO: return Result?
    fn set_file(&mut self) {
        let mut file = self.file;
        if file < 1 {
            self.line.file = FileEntry::default();
            return;
        }
        file -= 1;

        if file < self.program.files.len() {
            self.line.file = self.program.files[file].clone();
            return;
        }
        file -= self.program.files.len();

        if file < self.files.len() {
            self.line.file = self.files[file].clone();
            return;
        }

        self.line.file = FileEntry::default();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Line<'a> {
    pub address: u64,
    pub op_index: u64,
    pub file: FileEntry<'a>,
    pub line: u64,
    pub column: u64,
    pub is_stmt: bool,
    pub basic_block: bool,
    pub end_sequence: bool,
    pub prologue_end: bool,
    pub epilogue_begin: bool,
    pub isa: u64,
    pub discriminator: u64,
}

impl<'a> Line<'a> {
    fn new(is_stmt: bool) -> Self {
        Line {
            address: 0,
            op_index: 0,
            file: FileEntry::default(),
            line: 1,
            column: 0,
            is_stmt: is_stmt,
            basic_block: false,
            end_sequence: false,
            prologue_end: false,
            epilogue_begin: false,
            isa: 0,
            discriminator: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry<'a> {
    pub path: &'a [u8],
    pub directory: usize,
    pub timestamp: u64,
    pub length: u64,
}

impl<'a> Default for FileEntry<'a> {
    fn default() -> Self {
        FileEntry {
            path: &[],
            directory: 0,
            timestamp: 0,
            length: 0,
        }
    }
}

impl<'a> FileEntry<'a> {
    pub fn read(r: &mut &'a [u8]) -> Result<FileEntry<'a>, ReadError> {
        let path = try!(read_string(r));
        // Note: not validating this here
        let directory = try!(leb128::read_u64(r)) as usize;
        let timestamp = try!(leb128::read_u64(r));
        let length = try!(leb128::read_u64(r));
        Ok(FileEntry {
            path: path,
            directory: directory,
            timestamp: timestamp,
            length: length,
        })
    }
}
