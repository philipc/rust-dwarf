use std;
use std::io::Write;

use constant;
use leb128;
use read::{read_u8, ReadError};
use write::{write_u8};

#[derive(Debug, Default)]
pub struct AbbrevHash(std::collections::HashMap<u64, Abbrev>);

impl AbbrevHash {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<u64, Abbrev> {
        self.0.iter()
    }

    pub fn get(&self, code: u64) -> Option<&Abbrev> {
        self.0.get(&code)
    }

    pub fn insert(&mut self, abbrev: Abbrev) -> Option<Abbrev> {
        self.0.insert(abbrev.code, abbrev)
    }

    pub fn read(r: &mut &[u8]) -> Result<AbbrevHash, ReadError> {
        let mut abbrev_hash = AbbrevHash::default();
        while let Some(abbrev) = try!(Abbrev::read(r)) {
            if abbrev_hash.insert(abbrev).is_some() {
                return Err(ReadError::Invalid);
            }
        }
        Ok(abbrev_hash)
    }
}


#[derive(Debug)]
pub struct AbbrevVec(Vec<Abbrev>);

impl AbbrevVec {
    pub fn new(val: Vec<Abbrev>) -> Self {
        AbbrevVec(val)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<Abbrev> {
        self.0.iter()
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        for abbrev in self.iter() {
            try!(abbrev.write(w));
        }
        try!(Abbrev::write_null(w));
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Abbrev {
    pub code: u64,
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<AbbrevAttribute>,
}

impl Abbrev {
    pub fn read(r: &mut &[u8]) -> Result<Option<Abbrev>, ReadError> {
        let code = try!(leb128::read_u64(r));
        if code == 0 {
            return Ok(None);
        }

        let tag = try!(leb128::read_u16(r));

        let children = match constant::DwChildren(try!(read_u8(r))) {
            constant::DW_CHILDREN_no => false,
            constant::DW_CHILDREN_yes => true,
            _ => return Err(ReadError::Invalid),
        };

        let mut attributes = Vec::new();
        while let Some(attribute) = try!(AbbrevAttribute::read(r)) {
            attributes.push(attribute);
        }

        Ok(Some(Abbrev {
            code: code,
            tag: constant::DwTag(tag),
            children: children,
            attributes: attributes,
        }))
    }

    pub fn write_null<W: Write>(w: &mut W) -> std::io::Result<()> {
        leb128::write_u64(w, 0)
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        try!(leb128::write_u64(w, self.code));
        // This probably should never happen
        if self.code == 0 {
            return Ok(());
        }

        try!(leb128::write_u16(w, self.tag.0));

        let children = if self.children {
            constant::DW_CHILDREN_yes
        } else {
            constant::DW_CHILDREN_no
        };
        try!(write_u8(w, children.0));

        for attribute in &self.attributes {
            try!(attribute.write(w));
        }
        try!(AbbrevAttribute::write_null(w));

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AbbrevAttribute {
    pub at: constant::DwAt,
    pub form: constant::DwForm,
}

impl AbbrevAttribute {
    pub fn null() -> Self {
        AbbrevAttribute {
            at: constant::DW_AT_null,
            form: constant::DW_FORM_null,
        }
    }

    pub fn is_null(&self) -> bool {
        self.at == constant::DW_AT_null && self.form == constant::DW_FORM_null
    }

    pub fn read(r: &mut &[u8]) -> Result<Option<AbbrevAttribute>, ReadError> {
        let at = try!(leb128::read_u16(r));
        let form = try!(leb128::read_u16(r));
        let attribute = AbbrevAttribute {
            at: constant::DwAt(at),
            form: constant::DwForm(form),
        };
        if attribute.is_null() {
            Ok(None)
        } else {
            Ok(Some(attribute))
        }
    }

    pub fn write_null<W: Write>(w: &mut W) -> std::io::Result<()> {
        Self::null().write(w)
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        try!(leb128::write_u16(w, self.at.0));
        try!(leb128::write_u16(w, self.form.0));
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use constant::*;

    #[test]
    fn abbrev_container() {
        let write_val = AbbrevVec::new(vec![
            Abbrev {
                code: 1,
                tag: DW_TAG_namespace,
                children: true,
                attributes: vec![
                    AbbrevAttribute { at: DW_AT_name, form: DW_FORM_strp },
                ],
            },
        ]);

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = AbbrevHash::read(&mut r).unwrap();

        assert_eq!(&buf[..], [1, 57, 1, 3, 14, 0, 0, 0]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val.len(), write_val.len());
        for abbrev in write_val.iter() {
            assert_eq!(Some(abbrev), read_val.get(abbrev.code));
        }
    }

    #[test]
    fn abbrev() {
        let write_val = Abbrev {
            code: 1,
            tag: DW_TAG_namespace,
            children: true,
            attributes: vec![
                AbbrevAttribute { at: DW_AT_name, form: DW_FORM_strp },
            ],
        };

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = Abbrev::read(&mut r).unwrap();

        assert_eq!(&buf[..], [1, 57, 1, 3, 14, 0, 0]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, Some(write_val));
    }

    #[test]
    fn abbrev_attribute() {
        let write_val = AbbrevAttribute { at: DW_AT_sibling, form: DW_FORM_ref4 };

        let mut buf = Vec::new();
        write_val.write(&mut buf).unwrap();

        let mut r = &buf[..];
        let read_val = AbbrevAttribute::read(&mut r).unwrap();

        assert_eq!(&buf[..], [1, 19]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, Some(write_val));
    }
}
