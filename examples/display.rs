extern crate dwarf;

use std::env;
use std::path::Path;

fn main() {
    for file in env::args_os().skip(1) {
        display(file.as_ref()).unwrap();
    }
}

fn display(path: &Path) -> Result<(), dwarf::ReadError> {
    let sections = try!(dwarf::elf::load(path));
    let mut stdout = std::io::stdout();
    let mut f = dwarf::display::DefaultFormatter::new(&mut stdout, 4);

    let mut units = sections.compilation_units();
    while let Some(unit) = try!(units.next()) {
        let abbrev = try!(sections.abbrev(&unit.common));
        try!(unit.entries(&abbrev).display_depth(&mut f, 3));
    }

    let mut units = sections.type_units();
    while let Some(unit) = try!(units.next()) {
        let abbrev = try!(sections.abbrev(&unit.common));
        try!(unit.entries(&abbrev).display(&mut f));
    }

    Ok(())
}
