use std;
use std::io::Write;
use byteorder::WriteBytesExt;

use super::*;
use leb128;

impl Abbrev {
    pub fn write_null<W: Write>(w: &mut W) -> std::io::Result<()> {
        leb128::write_u64(w, 0)
    }

    pub fn write<W: Write>(&self, w: &mut W, code: u64) -> std::io::Result<()> {
        try!(leb128::write_u64(w, code));
        // This probably should never happen
        if code == 0 {
            return Ok(());
        }

        try!(leb128::write_u16(w, self.tag.0));

        let children = if self.children {
            constant::DW_CHILDREN_yes
        } else {
            constant::DW_CHILDREN_no
        };
        try!(w.write_u8(children.0));

        for attribute in &self.attributes {
            try!(attribute.write(w));
        }
        try!(AbbrevAttribute::write_null(w));

        Ok(())
    }
}

impl AbbrevAttribute {
    pub fn write_null<W: Write>(w: &mut W) -> std::io::Result<()> {
        Self::null().write(w)
    }

    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        try!(leb128::write_u16(w, self.at.0));
        try!(leb128::write_u16(w, self.form.0));
        Ok(())
    }
}
