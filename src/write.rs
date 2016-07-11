use std;
use std::io::Write;
use byteorder::WriteBytesExt;

use super::*;
use leb128;

#[derive(Debug)]
pub enum WriteError {
    Io(std::io::Error),
    Invalid(String),
    Unsupported(String),
}

impl std::convert::From<std::io::Error> for WriteError {
    fn from(e: std::io::Error) -> Self {
        WriteError::Io(e)
    }
}

impl<'a> Die<'a> {
    pub fn write_null<W: Write>(w: &mut W) -> std::io::Result<()> {
        leb128::write_u64(w, 0)
    }

    pub fn write<W: Write>(
        &self,
        w: &mut W,
        buffer: &mut DieBuffer<'a>,
        abbrev_hash: &AbbrevHash,
    ) -> Result<(), WriteError> {
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
        // This probably should never happen
        if abbrev.code == 0 {
            return Ok(());
        }
        for (attribute, abbrev_attribute) in self.attributes.iter().zip(&abbrev.attributes) {
            try!(attribute.write(w, buffer, abbrev_attribute));
        }
        Ok(())
    }
}

impl<'a> Attribute<'a> {
    pub fn write<W: Write>(
        &self,
        w: &mut W,
        buffer: &mut DieBuffer<'a>,
        abbrev: &AbbrevAttribute,
    ) -> Result<(), WriteError> {
        if self.at != abbrev.at {
            return Err(WriteError::Invalid("attribute type mismatch".to_string()));
        }
        try!(self.data.write(w, buffer, abbrev.form, false));
        Ok(())
    }
}

#[cfg_attr(feature = "clippy", allow(match_same_arms))]
impl<'a> AttributeData<'a> {
    pub fn write<W: Write>(
        &self,
        w: &mut W,
        buffer: &mut DieBuffer<'a>,
        form: constant::DwForm,
        indirect: bool,
    ) -> Result<(), WriteError> {
        if indirect {
            try!(leb128::write_u16(w, form.0));
        }
        match (self, form) {
            (&AttributeData::Address(ref val), constant::DW_FORM_addr) => {
                try!(write_address(w, buffer.endian, buffer.address_size, *val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block1) => {
                try!(w.write_u8(val.len() as u8));
                try!(w.write_all(val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block2) => {
                try!(buffer.endian.write_u16(w, val.len() as u16));
                try!(w.write_all(val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block4) => {
                try!(buffer.endian.write_u32(w, val.len() as u32));
                try!(w.write_all(val));
            },
            (&AttributeData::Block(ref val), constant::DW_FORM_block) => {
                try!(leb128::write_u64(w, val.len() as u64));
                try!(w.write_all(val));
            },
            (&AttributeData::Data1(ref val), constant::DW_FORM_data1) => {
                try!(w.write_u8(*val));
            },
            (&AttributeData::Data2(ref val), constant::DW_FORM_data2) => {
                try!(buffer.endian.write_u16(w, *val));
            },
            (&AttributeData::Data4(ref val), constant::DW_FORM_data4) => {
                try!(buffer.endian.write_u32(w, *val));
            },
            (&AttributeData::Data8(ref val), constant::DW_FORM_data8) => {
                try!(buffer.endian.write_u64(w, *val));
            },
            (&AttributeData::UData(ref val), constant::DW_FORM_udata) => {
                try!(leb128::write_u64(w, *val));
            },
            (&AttributeData::SData(ref val), constant::DW_FORM_sdata) => {
                try!(leb128::write_i64(w, *val));
            },
            (&AttributeData::Flag(ref val), constant::DW_FORM_flag) => {
                try!(w.write_u8(if *val { 1 } else { 0 }));
            },
            (&AttributeData::Flag(ref val), constant::DW_FORM_flag_present) => {
                assert!(*val);
            },
            (&AttributeData::String(ref val), constant::DW_FORM_string) => {
                try!(w.write_all(val.as_bytes()));
                try!(w.write_u8(0));
            },
            (&AttributeData::String(ref val), constant::DW_FORM_strp) => {
                // FIXME: reuse existing strings
                let debug_str = buffer.debug_str.to_mut();
                let offset = debug_str.len();
                try!(write_offset(w, buffer.endian, offset));
                try!(debug_str.write_all(val.as_bytes()));
                try!(debug_str.write_u8(0));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref1) => {
                try!(w.write_u8(*val as u8));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref2) => {
                try!(buffer.endian.write_u16(w, *val as u16));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref4) => {
                try!(buffer.endian.write_u32(w, *val as u32));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref8) => {
                try!(buffer.endian.write_u64(w, *val as u64));
            },
            (&AttributeData::Ref(ref val), constant::DW_FORM_ref_udata) => {
                try!(leb128::write_u64(w, *val as u64));
            },
            (&AttributeData::RefAddress(ref val), constant::DW_FORM_ref_addr) => {
                try!(write_address(w, buffer.endian, buffer.address_size, *val));
            },
            (&AttributeData::RefSig(ref val), constant::DW_FORM_ref_sig8) => {
                try!(buffer.endian.write_u64(w, *val));
            },
            (&AttributeData::SecOffset(ref val), constant::DW_FORM_sec_offset) => {
                try!(write_offset(w, buffer.endian, *val));
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

fn write_offset<W: Write>(w: &mut W, endian: Endian, val: usize) -> Result<(), WriteError> {
    // TODO: 64 bit
    try!(endian.write_u32(w, val as u32));
    Ok(())
}

fn write_address<W: Write>(w: &mut W, endian: Endian, address_size: u8, val: u64) -> Result<(), WriteError> {
    match address_size {
        4 => try!(endian.write_u32(w, val as u32)),
        8 => try!(endian.write_u64(w, val)),
        _ => return Err(WriteError::Unsupported(format!("address size {}", address_size))),
    };
    Ok(())
}

impl AbbrevVec {
    pub fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        for abbrev in self.iter() {
            try!(abbrev.write(w));
        }
        try!(Abbrev::write_null(w));
        Ok(())
    }
}

impl Abbrev {
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
