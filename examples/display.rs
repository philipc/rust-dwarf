extern crate dwarf;

use std::env;
use std::path::Path;
use dwarf::FallibleIterator;

fn main() {
    for file in env::args_os().skip(1) {
        display(file.as_ref()).unwrap();
    }
}

fn display(path: &Path) -> Result<(), dwarf::ParseError> {
    let sections = try!(dwarf::elf::load(path));
    let mut units = sections.compilation_units();
    let mut stdout = std::io::stdout();
    let mut f = dwarf::display::DefaultFormatter::new(&mut stdout, 4);
    while let Some(unit) = try!(units.next()) {
        try!(unit.display(&mut f));
    }
    Ok(())
}
