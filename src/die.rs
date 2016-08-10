use std;
use std::io::Write;

use ::UnitCommon;
use abbrev::*;
use constant;
use endian::Endian;
use leb128;
use read::*;
use write::*;

#[derive(Debug)]
pub struct DieCursor<'a, 'entry, 'unit: 'a, E: 'a+Endian> {
    r: &'entry [u8],
    offset: usize,
    unit: &'a UnitCommon<'unit, E>,
    abbrev: &'a AbbrevHash,
    entry: Die<'entry>,
}

#[cfg_attr(feature = "clippy", allow(should_implement_trait))]
impl<'a, 'entry, 'unit, E: Endian> DieCursor<'a, 'entry, 'unit, E> {
    pub fn new(
        r: &'entry [u8],
        offset: usize,
        unit: &'a UnitCommon<'unit, E>,
        abbrev: &'a AbbrevHash
    ) -> Self {
        DieCursor {
            r: r,
            offset: offset,
            unit: unit,
            abbrev: abbrev,
            entry: Die::null(0),
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn next(&mut self) -> Result<Option<&Die<'entry>>, ReadError> {
        if self.r.len() == 0 {
            return Ok(None);
        }

        let mut r = self.r;
        try!(self.entry.read(&mut r, self.offset, self.unit, self.abbrev));
        self.offset += self.r.len() - r.len();
        self.r = r;
        Ok(Some(&self.entry))
    }

    pub fn next_sibling(&mut self) -> Result<Option<&Die<'entry>>, ReadError> {
        let mut depth = if self.entry.children { 1 } else { 0 };
        while depth > 0 {
            let mut sibling_offset = 0;
            for attribute in &self.entry.attributes {
                if attribute.at == constant::DW_AT_sibling {
                    if let AttributeData::Ref(offset) = attribute.data {
                        sibling_offset = self.unit.offset + offset as usize;
                    }
                    break;
                }
            }
            if sibling_offset > self.offset {
                let relative_offset = sibling_offset - self.offset;
                if relative_offset <= self.r.len() {
                    self.entry.set_null(0);
                    self.offset = sibling_offset;
                    self.r = &self.r[relative_offset..];
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            match try!(self.next()) {
                Some(die) => {
                    if die.is_null() {
                        depth -= 1;
                    } else if die.children {
                        depth += 1;
                    }
                },
                None => return Ok(None),
            }
        }
        self.next()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Die<'a> {
    pub offset: usize,
    pub code: u64,
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<Attribute<'a>>,
}

impl<'a> Die<'a> {
    pub fn null(offset: usize) -> Self {
        Die {
            offset: offset,
            code: 0,
            tag: constant::DW_TAG_null,
            children: false,
            attributes: Vec::new(),
        }
    }

    pub fn set_null(&mut self, offset: usize) {
        self.offset = offset;
        self.code = 0;
        self.tag = constant::DW_TAG_null;
        self.children = false;
        self.attributes.clear();
    }

    pub fn is_null(&self) -> bool {
        self.code == 0
    }

    pub fn read<'unit, E: Endian>(
        &mut self,
        r: &mut &'a [u8],
        offset: usize,
        unit: &UnitCommon<'unit, E>,
        abbrev_hash: &AbbrevHash,
    ) -> Result<(), ReadError> {
        self.set_null(offset);

        self.code = try!(leb128::read_u64(r));
        if self.code == 0 {
            return Ok(());
        }

        let abbrev = match abbrev_hash.get(self.code) {
            Some(abbrev) => abbrev,
            None => return Err(ReadError::Invalid),
        };

        self.tag = abbrev.tag;
        self.children = abbrev.children;
        let len = abbrev.attributes.len();
        self.attributes.reserve(len);
        unsafe {
            self.attributes.set_len(len);
            for i in 0..len {
                if let Err(e) = self.attributes[i].read(r, unit, &abbrev.attributes[i]) {
                    self.attributes.clear();
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub fn write_null<'unit, E: Endian>(unit: &mut UnitCommon<'unit, E>) -> std::io::Result<()> {
        let w = unit.data.to_mut();
        leb128::write_u64(w, 0)
    }

    pub fn write<'unit, E: Endian>(
        &self,
        unit: &mut UnitCommon<'unit, E>,
        abbrev_hash: &AbbrevHash,
    ) -> Result<(), WriteError> {
        if self.code == 0 {
            try!(Die::write_null(unit));
            return Ok(());
        }
        let abbrev = match abbrev_hash.get(self.code) {
            Some(abbrev) => abbrev,
            None => return Err(WriteError::Invalid(format!("missing abbrev {}", self.code))),
        };
        if self.children != abbrev.children {
            return Err(WriteError::Invalid("die/abbrev children mismatch".to_string()));
        }
        if self.attributes.len() != abbrev.attributes.len() {
            return Err(WriteError::Invalid("die/abbrev attribute length mismatch".to_string()));
        }
        try!(leb128::write_u64(unit.data.to_mut(), abbrev.code));
        for (attribute, abbrev_attribute) in self.attributes.iter().zip(&abbrev.attributes) {
            try!(attribute.write(unit, abbrev_attribute));
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attribute<'a> {
    pub at: constant::DwAt,
    pub data: AttributeData<'a>,
}

impl<'a> Attribute<'a> {
    pub fn null() -> Self {
        Attribute {
            at: constant::DW_AT_null,
            data: AttributeData::Null,
        }
    }

    pub fn read<'unit, E: Endian>(
        &mut self,
        r: &mut &'a [u8],
        unit: &UnitCommon<'unit, E>,
        abbrev: &AbbrevAttribute,
    ) -> Result<(), ReadError> {
        self.at = abbrev.at;
        try!(self.data.read(r, unit, abbrev.form));
        Ok(())
    }

    pub fn write<'unit, E: Endian>(
        &self,
        unit: &mut UnitCommon<'unit, E>,
        abbrev: &AbbrevAttribute,
    ) -> Result<(), WriteError> {
        if self.at != abbrev.at {
            return Err(WriteError::Invalid("attribute type mismatch".to_string()));
        }
        try!(self.data.write(unit, abbrev.form, false));
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeData<'a> {
    Null,
    Address(u64),
    Block(&'a [u8]),
    Data1(u8),
    Data2(u16),
    Data4(u32),
    Data8(u64),
    UData(u64),
    SData(i64),
    Flag(bool),
    String(&'a [u8]),
    StringOffset(u64),
    Ref(u64),
    RefAddress(u64),
    RefSig(u64),
    SecOffset(u64),
    ExprLoc(&'a [u8]),
}

impl<'a> AttributeData<'a> {
    pub fn read<'unit, E: Endian>(
        &mut self,
        r: &mut &'a [u8],
        unit: &UnitCommon<'unit, E>,
        form: constant::DwForm,
    ) -> Result<(), ReadError> {
        *self = match form {
            constant::DW_FORM_addr => {
                let val = try!(read_address(r, unit.endian, unit.address_size));
                AttributeData::Address(val)
            }
            constant::DW_FORM_block2 => {
                let len = try!(unit.endian.read_u16(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_block4 => {
                let len = try!(unit.endian.read_u32(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_data2 => AttributeData::Data2(try!(unit.endian.read_u16(r))),
            constant::DW_FORM_data4 => AttributeData::Data4(try!(unit.endian.read_u32(r))),
            constant::DW_FORM_data8 => AttributeData::Data8(try!(unit.endian.read_u64(r))),
            constant::DW_FORM_string => AttributeData::String(try!(read_string(r))),
            constant::DW_FORM_block => {
                let len = try!(leb128::read_u64(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_block1 => {
                let len = try!(read_u8(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::Block(val)
            }
            constant::DW_FORM_data1 => AttributeData::Data1(try!(read_u8(r))),
            constant::DW_FORM_flag => AttributeData::Flag(try!(read_u8(r)) != 0),
            constant::DW_FORM_sdata => AttributeData::SData(try!(leb128::read_i64(r))),
            constant::DW_FORM_strp => {
                let val = try!(read_offset(r, unit.endian, unit.offset_size));
                AttributeData::StringOffset(val)
            }
            constant::DW_FORM_udata => AttributeData::UData(try!(leb128::read_u64(r))),
            constant::DW_FORM_ref_addr => {
                let val = try!(read_offset(r, unit.endian, unit.offset_size));
                AttributeData::RefAddress(val)
            }
            constant::DW_FORM_ref1 => AttributeData::Ref(try!(read_u8(r)) as u64),
            constant::DW_FORM_ref2 => AttributeData::Ref(try!(unit.endian.read_u16(r)) as u64),
            constant::DW_FORM_ref4 => AttributeData::Ref(try!(unit.endian.read_u32(r)) as u64),
            constant::DW_FORM_ref8 => AttributeData::Ref(try!(unit.endian.read_u64(r)) as u64),
            constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r))),
            constant::DW_FORM_indirect => {
                let val = try!(leb128::read_u16(r));
                return self.read(r, unit, constant::DwForm(val))
            }
            constant::DW_FORM_sec_offset => {
                // TODO: validate based on class
                let val = try!(read_offset(r, unit.endian, unit.offset_size));
                AttributeData::SecOffset(val)
            }
            constant::DW_FORM_exprloc => {
                let len = try!(leb128::read_u64(r)) as usize;
                let val = try!(read_block(r, len));
                AttributeData::ExprLoc(val)
            }
            constant::DW_FORM_flag_present => AttributeData::Flag(true),
            constant::DW_FORM_ref_sig8 => AttributeData::RefSig(try!(unit.endian.read_u64(r))),
            _ => return Err(ReadError::Unsupported),
        };
        Ok(())
    }

    pub fn write<'unit, E: Endian>(
        &self,
        unit: &mut UnitCommon<'unit, E>,
        form: constant::DwForm,
        indirect: bool,
    ) -> Result<(), WriteError> {
        let w = unit.data.to_mut();
        if indirect {
            try!(leb128::write_u16(w, form.0));
        }
        match (self, form) {
            (&AttributeData::Address(ref val), constant::DW_FORM_addr) => {
                try!(write_address(w, unit.endian, unit.address_size, *val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block1) => {
                try!(write_u8(w, val.len() as u8));
                try!(w.write_all(val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block2) => {
                try!(unit.endian.write_u16(w, val.len() as u16));
                try!(w.write_all(val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block4) => {
                try!(unit.endian.write_u32(w, val.len() as u32));
                try!(w.write_all(val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block) => {
                try!(leb128::write_u64(w, val.len() as u64));
                try!(w.write_all(val));
            },
            (&AttributeData::Data1(ref val), constant::DW_FORM_data1) => {
                try!(write_u8(w, *val));
            },
            (&AttributeData::Data2(ref val), constant::DW_FORM_data2) => {
                try!(unit.endian.write_u16(w, *val));
            },
            (&AttributeData::Data4(ref val), constant::DW_FORM_data4) => {
                try!(unit.endian.write_u32(w, *val));
            },
            (&AttributeData::Data8(ref val), constant::DW_FORM_data8) => {
                try!(unit.endian.write_u64(w, *val));
            },
            (&AttributeData::UData(ref val), constant::DW_FORM_udata) => {
                try!(leb128::write_u64(w, *val));
            },
            (&AttributeData::SData(ref val), constant::DW_FORM_sdata) => {
                try!(leb128::write_i64(w, *val));
            },
            (&AttributeData::Flag(ref val), constant::DW_FORM_flag) => {
                try!(write_u8(w, if *val { 1 } else { 0 }));
            },
            (&AttributeData::Flag(ref val), constant::DW_FORM_flag_present) => {
                assert!(*val);
            },
            (&AttributeData::String(ref val), constant::DW_FORM_string) => {
                try!(w.write_all(val));
                try!(write_u8(w, 0));
            },
            (&AttributeData::StringOffset(ref val), constant::DW_FORM_strp) => {
                try!(write_offset(w, unit.endian, unit.offset_size, *val));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref1) => {
                try!(write_u8(w, *val as u8));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref2) => {
                try!(unit.endian.write_u16(w, *val as u16));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref4) => {
                try!(unit.endian.write_u32(w, *val as u32));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref8) => {
                try!(unit.endian.write_u64(w, *val as u64));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref_udata) => {
                try!(leb128::write_u64(w, *val as u64));
            },
            (&AttributeData::RefAddress(ref val), constant::DW_FORM_ref_addr) => {
                try!(write_offset(w, unit.endian, unit.offset_size, *val));
            },
            (&AttributeData::RefSig(ref val), constant::DW_FORM_ref_sig8) => {
                try!(unit.endian.write_u64(w, *val));
            },
            (&AttributeData::SecOffset(ref val), constant::DW_FORM_sec_offset) => {
                try!(write_offset(w, unit.endian, unit.offset_size, *val));
            },
            (&AttributeData::ExprLoc(ref val), constant::DW_FORM_exprloc) => {
                try!(leb128::write_u64(w, val.len() as u64));
                try!(w.write_all(val));
            },
            _ => return Err(WriteError::Unsupported(format!("attribute form {}", form.0))),
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use abbrev::*;
    use constant::*;
    use endian::*;
    use ::UnitCommon;

    #[test]
    fn die_cursor() {
        let mut abbrev_hash = AbbrevHash::new();
        abbrev_hash.insert(Abbrev {
            code: 1,
            tag: DW_TAG_namespace,
            children: true,
            attributes: vec![
                AbbrevAttribute { at: DW_AT_name, form: DW_FORM_string },
            ],
        });
        abbrev_hash.insert(Abbrev {
            code: 2,
            tag: DW_TAG_namespace,
            children: false,
            attributes: vec![
                AbbrevAttribute { at: DW_AT_name, form: DW_FORM_string },
            ],
        });

        fn entry<'a>(name: &'a str, children: bool) -> Die<'a> {
            Die {
                offset: 0,
                code: if children { 1 } else { 2},
                tag: DW_TAG_namespace,
                children: children,
                attributes: vec![
                    Attribute { at: DW_AT_name, data: AttributeData::String(name.as_bytes()) },
                ],
            }
        }

        let mut write_val = [
            entry("0", true),
                entry("1", false),
                entry("2", true),
                    Die::null(0),
                entry("4", true),
                    entry("5", false),
                    Die::null(0),
                entry("7", true),
                    entry("8", true),
                        entry("9", true),
                            Die::null(0),
                        Die::null(0),
                    Die::null(0),
                entry("13", false),
                Die::null(0),
            entry("15", false),
        ];
        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
        for mut entry in &mut write_val {
            entry.offset = unit.len();
            entry.write(&mut unit, &abbrev_hash).unwrap();
        }

        let mut entries = unit.entries(0, &abbrev_hash);
        for i in 0..write_val.len() {
            match entries.next() {
                Ok(Some(read_val)) => assert_eq!(*read_val, write_val[i]),
                _ => panic!(),
            }
        }
        assert!(entries.next().unwrap().is_none());

        let mut entries = unit.entries(0, &abbrev_hash);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[0]);
        assert_eq!(*entries.next().unwrap().unwrap(), write_val[1]);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[2]);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[4]);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[7]);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[13]);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[14]);
        assert_eq!(*entries.next_sibling().unwrap().unwrap(), write_val[15]);
        assert!(entries.next_sibling().unwrap().is_none());

        // TODO test DW_AT_sibling
    }

    #[test]
    fn die() {
        let mut abbrev_hash = AbbrevHash::new();
        let code = 1;
        abbrev_hash.insert(Abbrev {
            code: code,
            tag: DW_TAG_namespace,
            children: true,
            attributes: vec![
                AbbrevAttribute { at: DW_AT_name, form: DW_FORM_string },
            ],
        });
        let write_val = Die {
            offset: 0,
            code: code,
            tag: DW_TAG_namespace,
            children: true,
            attributes: vec![
                Attribute { at: DW_AT_name, data: AttributeData::String(b"test") },
            ],
        };

        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
        write_val.write(&mut unit, &abbrev_hash).unwrap();

        let mut r = unit.data();
        let mut read_val = Die::null(0);
        read_val.read(&mut r, write_val.offset, &unit, &abbrev_hash).unwrap();

        assert_eq!(unit.data(), [1, b't', b'e', b's', b't', 0]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    fn attribute() {
        let abbrev = AbbrevAttribute { at: DW_AT_sibling, form: DW_FORM_ref4 };
        let write_val = Attribute {
            at: DW_AT_sibling,
            data: AttributeData::Ref(0x01234567),
        };

        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
        write_val.write(&mut unit, &abbrev).unwrap();

        let mut r = unit.data();
        let mut read_val = Attribute::null();
        read_val.read(&mut r, &unit, &abbrev).unwrap();

        assert_eq!(unit.data(), [0x67, 0x45, 0x23, 0x01]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    fn attribute_data() {
        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };

        unit.address_size = 4;
        unit.offset_size = 4;
        for &(ref write_val, form, expect) in &[
            (AttributeData::Address(0x12345678), DW_FORM_addr, &[0x78, 0x56, 0x34, 0x12][..]),
            (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block1, &[0x3, 0x11, 0x22, 0x33][..]),
            (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block2, &[0x3, 0x00, 0x11, 0x22, 0x33][..]),
            (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block4, &[0x3, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33][..]),
            (AttributeData::Block(&[0x11, 0x22, 0x33]), DW_FORM_block, &[0x3, 0x11, 0x22, 0x33][..]),
            (AttributeData::Data1(0x01), DW_FORM_data1, &[0x01][..]),
            (AttributeData::Data2(0x0123), DW_FORM_data2, &[0x23, 0x01][..]),
            (AttributeData::Data4(0x01234567), DW_FORM_data4, &[0x67, 0x45, 0x23, 0x01][..]),
            (AttributeData::Data8(0x0123456789abcdef), DW_FORM_data8, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01][..]),
            (AttributeData::UData(0x01234567), DW_FORM_udata, &[231, 138, 141, 9][..]),
            (AttributeData::SData(0x01234567), DW_FORM_sdata, &[231, 138, 141, 9][..]),
            (AttributeData::SData(-0x01234567), DW_FORM_sdata, &[153, 245, 242, 118][..]),
            (AttributeData::Flag(false), DW_FORM_flag, &[0][..]),
            (AttributeData::Flag(true), DW_FORM_flag, &[1][..]),
            (AttributeData::Flag(true), DW_FORM_flag_present, &[][..]),
            (AttributeData::String(b"test"), DW_FORM_string, &[b't', b'e', b's', b't', 0][..]),
            (AttributeData::StringOffset(0x01234567), DW_FORM_strp, &[0x67, 0x45, 0x23, 0x01][..]),
            (AttributeData::Ref(0x01), DW_FORM_ref1, &[0x01][..]),
            (AttributeData::Ref(0x0123), DW_FORM_ref2, &[0x23, 0x01][..]),
            (AttributeData::Ref(0x01234567), DW_FORM_ref4, &[0x67, 0x45, 0x23, 0x01][..]),
            (AttributeData::Ref(0x0123456789abcdef), DW_FORM_ref8, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01][..]),
            (AttributeData::Ref(0x01234567), DW_FORM_ref_udata, &[231, 138, 141, 9][..]),
            (AttributeData::RefAddress(0x12345678), DW_FORM_ref_addr, &[0x78, 0x56, 0x34, 0x12][..]),
            (AttributeData::RefSig(0x0123456789abcdef), DW_FORM_ref_sig8, &[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01][..]),
            (AttributeData::SecOffset(0x12345678), DW_FORM_sec_offset, &[0x78, 0x56, 0x34, 0x12][..]),
            (AttributeData::ExprLoc(&[0x11, 0x22, 0x33]), DW_FORM_exprloc, &[0x3, 0x11, 0x22, 0x33][..]),
        ] {
            attribute_data_inner(&mut unit, write_val, form, expect);
        }

        unit.address_size = 8;
        unit.offset_size = 4;
        for &(ref write_val, form, expect) in &[
            (AttributeData::Address(0x0123456789), DW_FORM_addr,
                &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
        ] {
            attribute_data_inner(&mut unit, write_val, form, expect);
        }

        unit.address_size = 4;
        unit.offset_size = 8;
        for &(ref write_val, form, expect) in &[
            (AttributeData::StringOffset(0x0123456789), DW_FORM_strp,
                &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
            (AttributeData::RefAddress(0x0123456789), DW_FORM_ref_addr,
                &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
            (AttributeData::SecOffset(0x0123456789), DW_FORM_sec_offset,
                &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
        ] {
            attribute_data_inner(&mut unit, write_val, form, expect);
        }
    }

    fn attribute_data_inner<'a, 'b, E: Endian>(
        unit: &mut UnitCommon<'a, E>,
        write_val: &AttributeData<'b>,
        form: DwForm, expect: &[u8],
    ) {
        for &indirect in &[false, true] {
            unit.data = Default::default();
            write_val.write(unit, form, indirect).unwrap();
            let buf = unit.data();

            let read_form = if indirect { DW_FORM_indirect } else { form };
            let mut r = buf;
            let mut read_val = AttributeData::Null;
            read_val.read(&mut r, unit, read_form).unwrap();

            if indirect {
                assert_eq!(buf[0] as u16, form.0);
                assert_eq!(&buf[1..], expect);
            } else {
                assert_eq!(&buf[..], expect);
            }
            assert_eq!(r.len(), 0);
            assert_eq!(read_val, *write_val);
        }
    }
}
