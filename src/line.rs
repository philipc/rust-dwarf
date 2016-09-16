use constant;
use endian::Endian;
use leb128;
use read::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineProgram<'data, E: Endian> {
    pub offset: usize,
    pub endian: E,
    pub version: u16,
    pub address_size: u8,
    pub offset_size: u8,
    pub address_step: u8,
    pub operation_range: u8,
    pub default_statement: bool,
    pub line_base: i8,
    pub line_range: u8,
    pub opcode_base: u8,
    pub standard_opcode_lengths: &'data [u8],
    pub include_directories: Vec<&'data [u8]>,
    pub files: Vec<FileEntry<'data>>,
    pub data: &'data [u8],
}

impl<'data, E: Endian> LineProgram<'data, E> {
    pub fn lines(&self) -> LineIterator<'data, E> {
        LineIterator::new(self.clone())
    }

    pub fn into_lines(self) -> LineIterator<'data, E> {
        LineIterator::new(self)
    }

    pub fn read(
        r: &mut &'data [u8],
        offset: usize,
        endian: E,
        address_size: u8,
        comp_dir: &'data [u8],
        comp_name: &'data [u8]
    ) -> Result<LineProgram<'data, E>, ReadError> {
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

        let address_step = try!(read_u8(&mut header));
        if address_step == 0 {
            return Err(ReadError::Invalid);
        }

        let operation_range = if version >= 4 {
            try!(read_u8(&mut header))
        } else {
            1
        };
        if operation_range == 0 {
            return Err(ReadError::Invalid);
        }

        let default_statement = try!(read_u8(&mut header)) != 0;
        let line_base = try!(read_i8(&mut header));

        let line_range = try!(read_u8(&mut header));
        if line_range == 0 {
            return Err(ReadError::Invalid);
        }

        let opcode_base = try!(read_u8(&mut header));
        if opcode_base == 0 {
            return Err(ReadError::Invalid);
        }

        let standard_opcode_lengths = try!(read_block(&mut header, opcode_base as usize - 1));

        let mut include_directories = vec![comp_dir];
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

        let mut files = vec![FileEntry {
                                 path: comp_name,
                                 directory: 0,
                                 timestamp: 0,
                                 length: 0,
                             }];
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
        Ok(LineProgram {
            offset: offset,
            endian: endian,
            version: version,
            address_size: address_size,
            offset_size: offset_size,
            address_step: address_step,
            operation_range: operation_range,
            default_statement: default_statement,
            line_base: line_base,
            line_range: line_range,
            opcode_base: opcode_base,
            standard_opcode_lengths: standard_opcode_lengths,
            include_directories: include_directories,
            files: files,
            data: data,
        })
    }
}

// Since line entries can modify the file entry array, the ownership
// gets a bit awkard unless the iterator takes ownership of the header.
// When reading, if you want to read the line number information more
// than once, then you should cache it. If you don't want to cache it,
// then reparsing the header is a small cost.

pub struct LineIterator<'data, E: 'data + Endian> {
    program: LineProgram<'data, E>,
    line: Line,
    copy: bool,
    data: &'data [u8],
}

impl<'data, E: Endian> LineIterator<'data, E> {
    pub fn new(program: LineProgram<'data, E>) -> Self {
        let default_statement = program.default_statement;
        let data = program.data;
        LineIterator {
            program: program,
            line: Line::new(default_statement),
            copy: false,
            data: data,
        }
    }

    pub fn directories(&self) -> &Vec<&'data [u8]> {
        &self.program.include_directories
    }

    pub fn files(&self) -> &Vec<FileEntry> {
        &self.program.files
    }

    #[cfg_attr(feature = "clippy", allow(should_implement_trait))]
    pub fn next(&mut self) -> Result<Option<(&LineIterator<E>, &Line)>, ReadError> {
        if self.data.len() == 0 {
            return Ok(None);
        }

        if self.line.sequence_end {
            self.line = Line::new(self.program.default_statement);
        } else {
            self.line.basic_block = false;
            self.line.prologue_end = false;
            self.line.epilogue_begin = false;
            self.line.discriminator = 0;
        }

        let mut r = self.data;
        loop {
            try!(self.next_opcode(&mut r));
            self.data = r;
            if self.copy {
                self.copy = false;
                return Ok(Some((self, &self.line)));
            }
        }
    }

    fn next_opcode(&mut self, r: &mut &'data [u8]) -> Result<(), ReadError> {
        let opcode = try!(read_u8(r));
        match constant::DwLns(opcode) {
            constant::DW_LNS_extended => try!(self.next_extended(r)),
            constant::DW_LNS_copy => self.copy = true,
            constant::DW_LNS_advance_pc => self.advance_pc(try!(leb128::read_u64(r))),
            constant::DW_LNS_advance_line => self.advance_line(try!(leb128::read_i64(r))),
            constant::DW_LNS_set_file => self.line.file = try!(leb128::read_u64(r)),
            constant::DW_LNS_set_column => self.line.column = try!(leb128::read_u64(r)),
            constant::DW_LNS_negate_stmt => self.line.statement = !self.line.statement,
            constant::DW_LNS_set_basic_block => self.line.basic_block = true,
            constant::DW_LNS_const_add_pc => {
                let op_delta = (255 - self.program.opcode_base) / self.program.line_range;
                self.advance_pc(op_delta as u64);
            }
            constant::DW_LNS_fixed_advance_pc => {
                self.line.address += try!(self.program.endian.read_u16(r)) as u64;
                self.line.operation = 0;
            }
            constant::DW_LNS_set_prologue_end => self.line.prologue_end = true,
            constant::DW_LNS_set_epilogue_begin => self.line.epilogue_begin = true,
            constant::DW_LNS_set_isa => self.line.isa = try!(leb128::read_u64(r)),
            _ => {
                if opcode < self.program.opcode_base {
                    // Unknown opcode, skip over it
                    let index = opcode as usize - 1;
                    if index >= self.program.standard_opcode_lengths.len() {
                        return Err(ReadError::Invalid);
                    }
                    for _ in 0..self.program.standard_opcode_lengths[index] {
                        try!(leb128::read_u64(r));
                    }
                } else {
                    self.advance_special(opcode);
                    self.copy = true;
                }
            }
        }
        Ok(())
    }

    fn next_extended(&mut self, r: &mut &'data [u8]) -> Result<(), ReadError> {
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
                self.line.sequence_end = true;
                self.copy = true;
            }
            constant::DW_LNE_set_address => {
                self.line.address =
                    try!(read_address(&mut data, self.program.endian, self.program.address_size));
                self.line.operation = 0;
            }
            constant::DW_LNE_define_file => {
                self.program.files.push(try!(FileEntry::read(&mut data)));
            }
            constant::DW_LNE_set_discriminator => {
                self.line.discriminator = try!(leb128::read_u64(&mut data));
            }
            _ => {
                // Unknown opcode, we've already skipped over it
            }
        }
        Ok(())
    }

    fn advance_special(&mut self, opcode: u8) {
        let delta = opcode - self.program.opcode_base;
        let op_delta = delta / self.program.line_range;
        let line_delta = delta % self.program.line_range;
        let line_delta = self.program.line_base as i64 + line_delta as i64;
        self.advance_pc(op_delta as u64);
        self.advance_line(line_delta);
    }

    fn advance_pc(&mut self, op_delta: u64) {
        let operation = self.line.operation + op_delta;
        let address_delta = operation / self.program.operation_range as u64;
        self.line.operation = operation % self.program.operation_range as u64;
        self.line.address += self.program.address_step as u64 * address_delta;
    }

    fn advance_line(&mut self, delta: i64) {
        self.line.line = self.line.line.wrapping_add(delta as u64);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Line {
    pub address: u64,
    pub operation: u64,
    pub file: u64,
    pub line: u64,
    pub column: u64,
    pub statement: bool,
    pub basic_block: bool,
    pub sequence_end: bool,
    pub prologue_end: bool,
    pub epilogue_begin: bool,
    pub isa: u64,
    pub discriminator: u64,
}

impl Line {
    fn new(statement: bool) -> Self {
        Line {
            address: 0,
            operation: 0,
            file: 1,
            line: 1,
            column: 0,
            statement: statement,
            basic_block: false,
            sequence_end: false,
            prologue_end: false,
            epilogue_begin: false,
            isa: 0,
            discriminator: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry<'data> {
    pub path: &'data [u8],
    pub directory: u64,
    pub timestamp: u64,
    pub length: u64,
}

impl<'data> Default for FileEntry<'data> {
    fn default() -> Self {
        FileEntry {
            path: &[],
            directory: 0,
            timestamp: 0,
            length: 0,
        }
    }
}

impl<'data> FileEntry<'data> {
    pub fn read(r: &mut &'data [u8]) -> Result<FileEntry<'data>, ReadError> {
        let path = try!(read_string(r));
        // Note: not validating this here
        let directory = try!(leb128::read_u64(r));
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
