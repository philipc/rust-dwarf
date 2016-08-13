mod endian;
mod leb128;
mod read;
mod write;

pub mod abbrev;
pub mod constant;
pub mod die;
pub mod display;
pub mod elf;
pub mod line;
pub mod unit;

pub use endian::{AnyEndian, Endian, LittleEndian, BigEndian, NativeEndian};
pub use read::ReadError;
pub use write::WriteError;

#[derive(Debug)]
pub struct Sections<E: Endian> {
    pub endian: E,
    pub debug_abbrev: Vec<u8>,
    pub debug_info: Vec<u8>,
    pub debug_line: Vec<u8>,
    pub debug_str: Vec<u8>,
    pub debug_types: Vec<u8>,
}

impl<E: Endian> Sections<E> {
    pub fn compilation_units(&self) -> unit::CompilationUnitIterator<E> {
        unit::CompilationUnitIterator::new(self.endian, &*self.debug_info)
    }

    pub fn type_units(&self) -> unit::TypeUnitIterator<E> {
        unit::TypeUnitIterator::new(self.endian, &*self.debug_types)
    }

    pub fn abbrev<'a>(&self, unit: &unit::UnitCommon<'a, E>) -> Result<abbrev::AbbrevHash, ReadError> {
        unit.abbrev(&*self.debug_abbrev)
    }
}
