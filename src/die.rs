use std;
use std::io::Write;

use abbrev::{AbbrevHash, AbbrevAttribute};
use constant;
use endian::Endian;
use leb128;
use read::*;
use write::*;
use unit::UnitCommon;

#[derive(Debug)]
pub struct DieIterator<'a, 'data, E>
    where 'data: 'a,
          E: Endian + 'a
{
    r: &'data [u8],
    offset: usize,
    unit: &'a UnitCommon<'data, E>,
    abbrev: &'a AbbrevHash,
    entry: Die<'data>,
}

impl<'a, 'data, E: Endian> DieIterator<'a, 'data, E> {
    pub fn new(
        r: &'data [u8],
        offset: usize,
        unit: &'a UnitCommon<'data, E>,
        abbrev: &'a AbbrevHash
    ) -> Self {
        DieIterator {
            r: r,
            offset: offset,
            unit: unit,
            abbrev: abbrev,
            entry: Die::null(0),
        }
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    // Get the next entry.
    //
    // This may be a normal entry, or a null entry.
    // Returns `None` when the end of input is reached.
    #[cfg_attr(feature = "clippy", allow(should_implement_trait))]
    pub fn next(&mut self) -> Result<Option<&Die<'data>>, ReadError> {
        if self.r.len() == 0 {
            return Ok(None);
        }

        let mut r = self.r;
        try!(self.entry.read(&mut r, self.offset, self.unit, self.abbrev));
        self.offset += self.r.len() - r.len();
        self.r = r;
        Ok(Some(&self.entry))
    }

    // Get the next sibling entry.
    //
    // If the current entry has no children, or is a null, then this
    // simply returns the next entry.
    //
    // If the current entry has children, then its child entries (and
    // the associated null) are skipped over. The DW_AT_sibling attribute
    // is used to accelerate this if possible.
    //
    // If the returned entry is a null, then there are no more siblings.
    // Note that this means a subsequent call to this method will return
    // an entry at the next higher depth in the tree.
    //
    // Returns `None` when the end of input is reached.
    pub fn next_sibling(&mut self) -> Result<Option<&Die<'data>>, ReadError> {
        let mut depth = 0;
        loop {
            if self.entry.children {
                depth += 1;
                let mut sibling_offset = 0;
                for attribute in &self.entry.attributes {
                    if attribute.at == constant::DW_AT_sibling {
                        if let AttributeData::Ref(offset) = attribute.data {
                            sibling_offset = self.unit.offset + offset as usize;
                        }
                        break;
                    }
                }
                // This is outside the for loop due to borrow check
                if sibling_offset > self.offset {
                    let relative_offset = sibling_offset - self.offset;
                    if relative_offset <= self.r.len() {
                        self.entry.set_null(0);
                        self.offset = sibling_offset;
                        self.r = &self.r[relative_offset..];
                        depth -= 1;
                    }
                }
            }
            if try!(self.next()).is_none() {
                return Ok(None);
            }
            if depth <= 0 {
                return Ok(Some(&self.entry));
            }
            if self.entry.is_null() {
                depth -= 1;
            }
        }
    }

    pub fn tree(self) -> DieTree<'a, 'data, E> {
        DieTree::new(self)
    }
}

#[derive(Debug)]
pub struct DieTree<'a, 'data, E>
    where 'data: 'a,
          E: Endian + 'a
{
    iter: DieIterator<'a, 'data, E>,
    // The depth of the entry that DieIterator::next_sibling() will return.
    depth: isize,
}

impl<'a, 'data, E> DieTree<'a, 'data, E>
    where E: Endian
{
    fn new(iter: DieIterator<'a, 'data, E>) -> DieTree<'a, 'data, E> {
        DieTree {
            iter: iter,
            depth: 0,
        }
    }

    pub fn iter<'me>(&'me mut self) -> DieTreeIterator<'me, 'a, 'data, E> {
        let depth = self.depth;
        DieTreeIterator::new(self, depth)
    }

    // Move the iterator to the next entry at the specified depth.
    //
    // Assumes depth <= self.depth + 1.
    //
    // Returns true if successful.
    fn next<'me>(&'me mut self, depth: isize) -> Result<bool, ReadError> {
        if self.depth < depth {
            // The iterator is at the parent.
            debug_assert_eq!(self.depth + 1, depth);
            if !self.iter.entry.children {
                // No children, sorry.
                return Ok(false);
            }
            // The next entry is the child.
            if try!(self.iter.next()).is_none() {
                return Ok(false);
            }
            if self.iter.entry.is_null() {
                // No children, don't adjust depth.
                return Ok(false);
            } else {
                // Got a child, next_sibling is now at the child depth.
                self.depth += 1;
                return Ok(true);
            }
        }
        loop {
            if try!(self.iter.next_sibling()).is_none() {
                return Ok(false);
            }
            if self.depth == depth {
                if self.iter.entry.is_null() {
                    // No more entries at the target depth.
                    self.depth -= 1;
                    return Ok(false);
                } else {
                    // Got a child at the target depth.
                    return Ok(true);
                }
            }
            if self.iter.entry.is_null() {
                self.depth -= 1;
            }
        }
    }
}

#[derive(Debug)]
pub struct DieTreeIterator<'a, 'b, 'data, E>
    where 'b: 'a,
          'data: 'b,
          E: Endian + 'b
{
    tree: &'a mut DieTree<'b, 'data, E>,
    depth: isize,
    done: bool,
}

impl<'a, 'b, 'data, E> DieTreeIterator<'a, 'b, 'data, E>
    where E: Endian
{
    #[inline]
    fn new(tree: &'a mut DieTree<'b, 'data, E>, depth: isize) -> DieTreeIterator<'a, 'b, 'data, E> {
        DieTreeIterator {
            tree: tree,
            depth: depth,
            done: false,
        }
    }

    #[inline]
    pub fn entry(&self) -> &Die<'data> {
        &self.tree.iter.entry
    }

    #[inline]
    pub fn next<'me>(&'me mut self)
        -> Result<Option<DieTreeIterator<'me, 'b, 'data, E>>, ReadError> {
        if self.done {
            Ok(None)
        } else if try!(self.tree.next(self.depth)) {
            Ok(Some(DieTreeIterator::new(self.tree, self.depth + 1)))
        } else {
            self.done = true;
            Ok(None)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Die<'data> {
    pub offset: usize,
    pub code: u64,
    pub tag: constant::DwTag,
    pub children: bool,
    pub attributes: Vec<Attribute<'data>>,
}

impl<'data> Die<'data> {
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

    pub fn attr(&self, at: constant::DwAt) -> Option<&AttributeData<'data>> {
        self.attributes.iter().find(|attr| attr.at == at).map(|attr| &attr.data)
    }

    pub fn read<'unit, E: Endian>(
        &mut self,
        r: &mut &'data [u8],
        offset: usize,
        unit: &UnitCommon<'unit, E>,
        abbrev_hash: &AbbrevHash
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
        self.attributes.reserve(abbrev.attributes.len());
        for abbrev_attribute in &abbrev.attributes {
            self.attributes.push(try!(Attribute::read(r, unit, abbrev_attribute)));
        }

        Ok(())
    }

    pub fn write_null<W: Write>(w: &mut W) -> std::io::Result<()> {
        leb128::write_u64(w, 0)
    }

    pub fn write<'unit, E: Endian, W: Write>(
        &self,
        w: &mut W,
        unit: &UnitCommon<'unit, E>,
        abbrev_hash: &AbbrevHash
    ) -> Result<(), WriteError> {
        if self.code == 0 {
            try!(Die::write_null(w));
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
        try!(leb128::write_u64(w, abbrev.code));
        for (attribute, abbrev_attribute) in self.attributes.iter().zip(&abbrev.attributes) {
            try!(attribute.write(w, unit, abbrev_attribute));
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attribute<'data> {
    pub at: constant::DwAt,
    pub data: AttributeData<'data>,
}

impl<'data> Attribute<'data> {
    pub fn null() -> Self {
        Attribute {
            at: constant::DW_AT_null,
            data: AttributeData::Null,
        }
    }

    pub fn read<'unit, E: Endian>(
        r: &mut &'data [u8],
        unit: &UnitCommon<'unit, E>,
        abbrev: &AbbrevAttribute
    ) -> Result<Attribute<'data>, ReadError> {
        let data = try!(AttributeData::read(r, unit, abbrev.form));
        Ok(Attribute {
            at: abbrev.at,
            data: data,
        })
    }

    pub fn write<'unit, E: Endian, W: Write>(
        &self,
        w: &mut W,
        unit: &UnitCommon<'unit, E>,
        abbrev: &AbbrevAttribute
    ) -> Result<(), WriteError> {
        if self.at != abbrev.at {
            return Err(WriteError::Invalid("attribute type mismatch".to_string()));
        }
        try!(self.data.write(w, unit, abbrev.form, false));
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeData<'data> {
    Null,
    Address(u64),
    Block(&'data [u8]),
    Data1(u8),
    Data2(u16),
    Data4(u32),
    Data8(u64),
    UData(u64),
    SData(i64),
    Flag(bool),
    String(&'data [u8]),
    StringOffset(u64),
    Ref(u64),
    RefAddress(u64),
    RefSig(u64),
    SecOffset(u64),
    ExprLoc(&'data [u8]),
}

impl<'data> AttributeData<'data> {
    pub fn as_string(&self, debug_str: &'data [u8]) -> Option<&'data [u8]> {
        match *self {
            AttributeData::String(val) => Some(val),
            AttributeData::StringOffset(val) => {
                let val = val as usize;
                if val < debug_str.len() {
                    let mut r = &debug_str[val..];
                    read_string(&mut r).ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_offset(&self) -> Option<usize> {
        match *self {
            AttributeData::Data4(val) => Some(val as usize),
            AttributeData::SecOffset(val) => Some(val as usize),
            _ => None,
        }
    }

    pub fn read<'unit, E: Endian>(
        r: &mut &'data [u8],
        unit: &UnitCommon<'unit, E>,
        form: constant::DwForm
    ) -> Result<AttributeData<'data>, ReadError> {
        let data = match form {
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
                let val = if unit.version == 2 {
                    try!(read_address(r, unit.endian, unit.address_size))
                } else {
                    try!(read_offset(r, unit.endian, unit.offset_size))
                };
                AttributeData::RefAddress(val)
            }
            constant::DW_FORM_ref1 => AttributeData::Ref(try!(read_u8(r)) as u64),
            constant::DW_FORM_ref2 => AttributeData::Ref(try!(unit.endian.read_u16(r)) as u64),
            constant::DW_FORM_ref4 => AttributeData::Ref(try!(unit.endian.read_u32(r)) as u64),
            constant::DW_FORM_ref8 => AttributeData::Ref(try!(unit.endian.read_u64(r)) as u64),
            constant::DW_FORM_ref_udata => AttributeData::Ref(try!(leb128::read_u64(r))),
            constant::DW_FORM_indirect => {
                let val = try!(leb128::read_u16(r));
                try!(AttributeData::read(r, unit, constant::DwForm(val)))
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
        Ok(data)
    }

    #[cfg_attr(feature = "clippy", allow(match_same_arms))]
    pub fn write<'unit, E: Endian, W: Write>(
        &self,
        w: &mut W,
        unit: &UnitCommon<'unit, E>,
        form: constant::DwForm,
        indirect: bool
    ) -> Result<(), WriteError> {
        if indirect {
            try!(leb128::write_u16(w, form.0));
        }
        match (self, form) {
            (&AttributeData::Address(ref val), constant::DW_FORM_addr) => {
                try!(write_address(w, unit.endian, unit.address_size, *val));
            }
            (&AttributeData::Block(val), constant::DW_FORM_block1) => {
                try!(write_u8(w, val.len() as u8));
                try!(w.write_all(val));
            }
            (&AttributeData::Block(val), constant::DW_FORM_block2) => {
                try!(unit.endian.write_u16(w, val.len() as u16));
                try!(w.write_all(val));
            }
            (&AttributeData::Block(val), constant::DW_FORM_block4) => {
                try!(unit.endian.write_u32(w, val.len() as u32));
                try!(w.write_all(val));
            }
            (&AttributeData::Block(val), constant::DW_FORM_block) => {
                try!(leb128::write_u64(w, val.len() as u64));
                try!(w.write_all(val));
            }
            (&AttributeData::Data1(ref val), constant::DW_FORM_data1) => {
                try!(write_u8(w, *val));
            }
            (&AttributeData::Data2(ref val), constant::DW_FORM_data2) => {
                try!(unit.endian.write_u16(w, *val));
            }
            (&AttributeData::Data4(ref val), constant::DW_FORM_data4) => {
                try!(unit.endian.write_u32(w, *val));
            }
            (&AttributeData::Data8(ref val), constant::DW_FORM_data8) => {
                try!(unit.endian.write_u64(w, *val));
            }
            (&AttributeData::UData(ref val), constant::DW_FORM_udata) => {
                try!(leb128::write_u64(w, *val));
            }
            (&AttributeData::SData(ref val), constant::DW_FORM_sdata) => {
                try!(leb128::write_i64(w, *val));
            }
            (&AttributeData::Flag(ref val), constant::DW_FORM_flag) => {
                try!(write_u8(w, if *val { 1 } else { 0 }));
            }
            (&AttributeData::Flag(ref val), constant::DW_FORM_flag_present) => {
                assert!(*val);
            }
            (&AttributeData::String(val), constant::DW_FORM_string) => {
                try!(w.write_all(val));
                try!(write_u8(w, 0));
            }
            (&AttributeData::StringOffset(ref val), constant::DW_FORM_strp) => {
                try!(write_offset(w, unit.endian, unit.offset_size, *val));
            }
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref1) => {
                try!(write_u8(w, *val as u8));
            }
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref2) => {
                try!(unit.endian.write_u16(w, *val as u16));
            }
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref4) => {
                try!(unit.endian.write_u32(w, *val as u32));
            }
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref8) => {
                try!(unit.endian.write_u64(w, *val as u64));
            }
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref_udata) => {
                try!(leb128::write_u64(w, *val as u64));
            }
            (&AttributeData::RefAddress(ref val), constant::DW_FORM_ref_addr) => {
                if unit.version == 2 {
                    try!(write_address(w, unit.endian, unit.address_size, *val));
                } else {
                    try!(write_offset(w, unit.endian, unit.offset_size, *val));
                }
            }
            (&AttributeData::RefSig(ref val), constant::DW_FORM_ref_sig8) => {
                try!(unit.endian.write_u64(w, *val));
            }
            (&AttributeData::SecOffset(ref val), constant::DW_FORM_sec_offset) => {
                try!(write_offset(w, unit.endian, unit.offset_size, *val));
            }
            (&AttributeData::ExprLoc(val), constant::DW_FORM_exprloc) => {
                try!(leb128::write_u64(w, val.len() as u64));
                try!(w.write_all(val));
            }
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
    use unit::*;

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
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

        fn entry<'data>(name: &'data str, children: bool) -> Die<'data> {
            Die {
                offset: 0,
                code: if children { 1 } else { 2 },
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
        let mut data = Vec::new();
        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
        for mut entry in &mut write_val {
            entry.offset = data.len();
            entry.write(&mut data, &unit, &abbrev_hash).unwrap();
        }
        unit.data = &data[..];

        let mut entries = unit.entries(0, &abbrev_hash);
        for i in 0..write_val.len() {
            match entries.next() {
                Ok(Some(read_val)) => assert_eq!(*read_val, write_val[i]),
                otherwise => panic!("{:?}", otherwise),
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

        let mut tree = unit.entries(0, &abbrev_hash).tree();
        let mut tree = tree.iter();
        {
            let mut tree = tree.next().unwrap().unwrap();
            assert_eq!(*tree.entry(), write_val[0]);
            {
                let mut tree = tree.next().unwrap().unwrap();
                assert_eq!(*tree.entry(), write_val[1]);
                assert!(tree.next().unwrap().is_none());
            }
            {
                let mut tree = tree.next().unwrap().unwrap();
                assert_eq!(*tree.entry(), write_val[2]);
                assert!(tree.next().unwrap().is_none());
            }
            {
                let mut tree = tree.next().unwrap().unwrap();
                assert_eq!(*tree.entry(), write_val[4]);
                {
                    let mut tree = tree.next().unwrap().unwrap();
                    assert_eq!(*tree.entry(), write_val[5]);
                    assert!(tree.next().unwrap().is_none());
                }
                assert!(tree.next().unwrap().is_none());
            }
            {
                let mut tree = tree.next().unwrap().unwrap();
                assert_eq!(*tree.entry(), write_val[7]);
                {
                    let tree = tree.next().unwrap().unwrap();
                    assert_eq!(*tree.entry(), write_val[8]);
                    // Stop iterating here.
                }
            }
            {
                let tree = tree.next().unwrap().unwrap();
                assert_eq!(*tree.entry(), write_val[13]);
            }
            assert!(tree.next().unwrap().is_none());
        }
        {
            let mut tree = tree.next().unwrap().unwrap();
            assert_eq!(*tree.entry(), write_val[15]);
            assert!(tree.next().unwrap().is_none());
        }
        assert!(tree.next().unwrap().is_none());
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

        let mut data = Vec::new();
        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
        write_val.write(&mut data, &unit, &abbrev_hash).unwrap();
        unit.data = &data[..];

        let mut r = unit.data();
        let mut read_val = Die::null(0);
        read_val.read(&mut r, write_val.offset, &unit, &abbrev_hash).unwrap();

        assert_eq!(unit.data(), [1, b't', b'e', b's', b't', 0]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    fn attribute() {
        let abbrev = AbbrevAttribute {
            at: DW_AT_sibling,
            form: DW_FORM_ref4,
        };
        let write_val = Attribute {
            at: DW_AT_sibling,
            data: AttributeData::Ref(0x01234567),
        };

        let mut data = Vec::new();
        let mut unit = UnitCommon { endian: LittleEndian, ..Default::default() };
        write_val.write(&mut data, &unit, &abbrev).unwrap();
        unit.data = &data[..];

        let mut r = unit.data();
        let read_val = Attribute::read(&mut r, &unit, &abbrev).unwrap();

        assert_eq!(unit.data(), [0x67, 0x45, 0x23, 0x01]);
        assert_eq!(r.len(), 0);
        assert_eq!(read_val, write_val);
    }

    #[test]
    #[cfg_attr(rustfmt, rustfmt_skip)]
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

        unit.version = 2;
        unit.address_size = 8;
        unit.offset_size = 4;
        for &(ref write_val, form, expect) in &[
            (AttributeData::RefAddress(0x0123456789), DW_FORM_ref_addr,
                &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00][..]),
        ] {
            attribute_data_inner(&mut unit, write_val, form, expect);
        }
    }

    fn attribute_data_inner<'data, 'b, E: Endian>(
        unit: &UnitCommon<'data, E>,
        write_val: &AttributeData<'b>,
        form: DwForm,
        expect: &[u8]
    ) {
        for &indirect in &[false, true] {
            let mut data = Vec::new();
            write_val.write(&mut data, unit, form, indirect).unwrap();
            let buf = &data[..];

            let read_form = if indirect { DW_FORM_indirect } else { form };
            let mut r = buf;
            let read_val = AttributeData::read(&mut r, unit, read_form).unwrap();

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
