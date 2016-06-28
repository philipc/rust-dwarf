extern crate dwarf;

#[test]
fn parse_and_display() {
    let path = std::env::args_os().next().unwrap();
    let sections = dwarf::elf::load(path).unwrap();
    let units = dwarf::parse_sections(&sections).unwrap();
    for unit in &units {
        format!("{}", unit);
    }
}
