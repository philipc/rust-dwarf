extern crate dwarf;

#[test]
fn decode_and_display() {
    let path = std::env::args_os().next().unwrap();
    let sections = dwarf::elf::load(path).unwrap();
    let mut units = sections.compilation_units();
    let mut buf = Vec::new();
    let mut f = dwarf::display::DefaultFormatter::new(&mut buf, 4);
    while let Some(unit) = units.next().unwrap() {
        unit.display(&mut f).unwrap();
    }
}
