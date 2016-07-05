use std;
use std::fmt;

use super::*;

pub trait Formatter {
    fn indent(&mut self);
    fn unindent(&mut self);
    fn write_fmt(&mut self, fmt: std::fmt::Arguments) -> Result<(), std::io::Error>;
    fn write_sep(&mut self) -> Result<(), std::io::Error>;
}

pub struct DefaultFormatter<'a> {
    w: &'a mut std::io::Write,
    indent: usize,
    current_indent: usize,
}

impl<'a> DefaultFormatter<'a> {
    pub fn new(w: &'a mut std::io::Write, indent: usize) -> Self {
        DefaultFormatter {
            w: w,
            indent: indent,
            current_indent: 0,
        }
    }
}

impl<'a> Formatter for DefaultFormatter<'a> {
    fn indent(&mut self) {
        self.current_indent += self.indent;
    }

    fn unindent(&mut self) {
        if self.current_indent >= self.indent {
            self.current_indent -= self.indent;
        }
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments) -> Result<(), std::io::Error> {
        for _ in 0..self.current_indent {
            try!(write!(self.w, " "));
        }
        try!(self.w.write_fmt(fmt));
        Ok(())
    }

    fn write_sep(&mut self) -> Result<(), std::io::Error> {
        try!(write!(self.w, "\n"));
        Ok(())
    }
}

impl<'a> CompilationUnit<'a> {
    pub fn display<F: Formatter>(&self, f: &mut F) -> Result<(), ParseError> {
        let mut iter = try!(self.entries());
        while let Some(die) = try!(iter.next()) {
            if die.is_null() {
                f.unindent();
            } else {
                try!(die.display(f));
                try!(f.write_sep());
                if die.children {
                    f.indent();
                }
            }
        }
        Ok(())
    }
}

impl<'a> Die<'a> {
    pub fn display<F: Formatter>(&self, f: &mut F) -> Result<(), std::io::Error> {
        try!(write!(f, "{}\n", self.tag));
        for attribute in &self.attributes {
            try!(write!(f, "{}\n", attribute));
        }
        Ok(())
    }
}

impl<'a> fmt::Display for Attribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: interpret data based on attribute type
        write!(f, "{}: {}", self.at, self.data)
    }
}

impl<'a> fmt::Display for AttributeData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AttributeData::Address(val) => write!(f, "(address) {:x}", val),
            AttributeData::Block(val) => write!(f, "(block) len {}", val.len()),
            AttributeData::Data1(val) => write!(f, "(data1) {:x}", val),
            AttributeData::Data2(val) => write!(f, "(data2) {:x}", val),
            AttributeData::Data4(val) => write!(f, "(data4) {:x}", val),
            AttributeData::Data8(val) => write!(f, "(data8) {:x}", val),
            AttributeData::UData(val) => write!(f, "(udata) {:x}", val),
            AttributeData::SData(val) => write!(f, "(sdata) {:x}", val),
            AttributeData::Flag(val) => write!(f, "(flag) {}", val),
            AttributeData::String(val) => write!(f, "(string) {}", val),
            AttributeData::Ref(val) => write!(f, "(ref) {}", val),
            AttributeData::RefAddress(val) => write!(f, "(ref_address) {}", val),
            AttributeData::RefSig(val) => write!(f, "(ref_sig) {:x}", val),
            AttributeData::SecOffset(val) => write!(f, "(sec_offset) {:x}", val),
            AttributeData::ExprLoc(val) => write!(f, "(expr_loc) len {}", val.len()),
        }
    }
}

impl fmt::Display for constant::DwTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            constant::DW_TAG_array_type => write!(f, "array_type"),
            constant::DW_TAG_class_type => write!(f, "class_type"),
            constant::DW_TAG_entry_point => write!(f, "entry_point"),
            constant::DW_TAG_enumeration_type => write!(f, "enumeration_type"),
            constant::DW_TAG_formal_parameter => write!(f, "formal_parameter"),
            constant::DW_TAG_imported_declaration => write!(f, "imported_declaration"),
            constant::DW_TAG_label => write!(f, "label"),
            constant::DW_TAG_lexical_block => write!(f, "lexical_block"),
            constant::DW_TAG_member => write!(f, "member"),
            constant::DW_TAG_pointer_type => write!(f, "pointer_type"),
            constant::DW_TAG_reference_type => write!(f, "reference_type"),
            constant::DW_TAG_compile_unit => write!(f, "compile_unit"),
            constant::DW_TAG_string_type => write!(f, "string_type"),
            constant::DW_TAG_structure_type => write!(f, "structure_type"),
            constant::DW_TAG_subroutine_type => write!(f, "subroutine_type"),
            constant::DW_TAG_typedef => write!(f, "typedef"),
            constant::DW_TAG_union_type => write!(f, "union_type"),
            constant::DW_TAG_unspecified_parameters => write!(f, "unspecified_parameters"),
            constant::DW_TAG_variant => write!(f, "variant"),
            constant::DW_TAG_common_block => write!(f, "common_block"),
            constant::DW_TAG_common_inclusion => write!(f, "common_inclusion"),
            constant::DW_TAG_inheritance => write!(f, "inheritance"),
            constant::DW_TAG_inlined_subroutine => write!(f, "inlined_subroutine"),
            constant::DW_TAG_module => write!(f, "module"),
            constant::DW_TAG_ptr_to_member_type => write!(f, "ptr_to_member_type"),
            constant::DW_TAG_set_type => write!(f, "set_type"),
            constant::DW_TAG_subrange_type => write!(f, "subrange_type"),
            constant::DW_TAG_with_stmt => write!(f, "with_stmt"),
            constant::DW_TAG_access_declaration => write!(f, "access_declaration"),
            constant::DW_TAG_base_type => write!(f, "base_type"),
            constant::DW_TAG_catch_block => write!(f, "catch_block"),
            constant::DW_TAG_const_type => write!(f, "const_type"),
            constant::DW_TAG_constant => write!(f, "constant"),
            constant::DW_TAG_enumerator => write!(f, "enumerator"),
            constant::DW_TAG_file_type => write!(f, "file_type"),
            constant::DW_TAG_friend => write!(f, "friend"),
            constant::DW_TAG_namelist => write!(f, "namelist"),
            constant::DW_TAG_namelist_item => write!(f, "namelist_item"),
            constant::DW_TAG_packed_type => write!(f, "packed_type"),
            constant::DW_TAG_subprogram => write!(f, "subprogram"),
            constant::DW_TAG_template_type_parameter => write!(f, "template_type_parameter"),
            constant::DW_TAG_template_value_parameter => write!(f, "template_value_parameter"),
            constant::DW_TAG_thrown_type => write!(f, "thrown_type"),
            constant::DW_TAG_try_block => write!(f, "try_block"),
            constant::DW_TAG_variant_part => write!(f, "variant_part"),
            constant::DW_TAG_variable => write!(f, "variable"),
            constant::DW_TAG_volatile_type => write!(f, "volatile_type"),
            constant::DW_TAG_dwarf_procedure => write!(f, "dwarf_procedure"),
            constant::DW_TAG_restrict_type => write!(f, "restrict_type"),
            constant::DW_TAG_interface_type => write!(f, "interface_type"),
            constant::DW_TAG_namespace => write!(f, "namespace"),
            constant::DW_TAG_imported_module => write!(f, "imported_module"),
            constant::DW_TAG_unspecified_type => write!(f, "unspecified_type"),
            constant::DW_TAG_partial_unit => write!(f, "partial_unit"),
            constant::DW_TAG_imported_unit => write!(f, "imported_unit"),
            constant::DW_TAG_condition => write!(f, "condition"),
            constant::DW_TAG_shared_type => write!(f, "shared_type"),
            constant::DW_TAG_type_unit => write!(f, "type_unit"),
            constant::DW_TAG_rvalue_reference_type => write!(f, "rvalue_reference_type"),
            constant::DW_TAG_template_alias => write!(f, "template_alias"),
            _ => write!(f, "tag({})", self.0),
        }
    }
}

impl fmt::Display for constant::DwAt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            constant::DW_AT_sibling => write!(f, "sibling"),
            constant::DW_AT_location => write!(f, "location"),
            constant::DW_AT_name => write!(f, "name"),
            constant::DW_AT_ordering => write!(f, "ordering"),
            constant::DW_AT_byte_size => write!(f, "byte_size"),
            constant::DW_AT_bit_offset => write!(f, "bit_offset"),
            constant::DW_AT_bit_size => write!(f, "bit_size"),
            constant::DW_AT_stmt_list => write!(f, "stmt_list"),
            constant::DW_AT_low_pc => write!(f, "low_pc"),
            constant::DW_AT_high_pc => write!(f, "high_pc"),
            constant::DW_AT_language => write!(f, "language"),
            constant::DW_AT_discr => write!(f, "discr"),
            constant::DW_AT_discr_value => write!(f, "discr_value"),
            constant::DW_AT_visibility => write!(f, "visibility"),
            constant::DW_AT_import => write!(f, "import"),
            constant::DW_AT_string_length => write!(f, "string_length"),
            constant::DW_AT_common_reference => write!(f, "common_reference"),
            constant::DW_AT_comp_dir => write!(f, "comp_dir"),
            constant::DW_AT_const_value => write!(f, "const_value"),
            constant::DW_AT_containing_type => write!(f, "containing_type"),
            constant::DW_AT_default_value => write!(f, "default_value"),
            constant::DW_AT_inline => write!(f, "inline"),
            constant::DW_AT_is_optional => write!(f, "is_optional"),
            constant::DW_AT_lower_bound => write!(f, "lower_bound"),
            constant::DW_AT_producer => write!(f, "producer"),
            constant::DW_AT_prototyped => write!(f, "prototyped"),
            constant::DW_AT_return_addr => write!(f, "return_addr"),
            constant::DW_AT_start_scope => write!(f, "start_scope"),
            constant::DW_AT_bit_stride => write!(f, "bit_stride"),
            constant::DW_AT_upper_bound => write!(f, "upper_bound"),
            constant::DW_AT_abstract_origin => write!(f, "abstract_origin"),
            constant::DW_AT_accessibility => write!(f, "accessibility"),
            constant::DW_AT_address_class => write!(f, "address_class"),
            constant::DW_AT_artificial => write!(f, "artificial"),
            constant::DW_AT_base_types => write!(f, "base_types"),
            constant::DW_AT_calling_convention => write!(f, "calling_convention"),
            constant::DW_AT_count => write!(f, "count"),
            constant::DW_AT_data_member_location => write!(f, "data_member_location"),
            constant::DW_AT_decl_column => write!(f, "decl_column"),
            constant::DW_AT_decl_file => write!(f, "decl_file"),
            constant::DW_AT_decl_line => write!(f, "decl_line"),
            constant::DW_AT_declaration => write!(f, "declaration"),
            constant::DW_AT_discr_list => write!(f, "discr_list"),
            constant::DW_AT_encoding => write!(f, "encoding"),
            constant::DW_AT_external => write!(f, "external"),
            constant::DW_AT_frame_base => write!(f, "frame_base"),
            constant::DW_AT_friend => write!(f, "friend"),
            constant::DW_AT_identifier_case => write!(f, "identifier_case"),
            constant::DW_AT_macro_info => write!(f, "macro_info"),
            constant::DW_AT_namelist_item => write!(f, "namelist_item"),
            constant::DW_AT_priority => write!(f, "priority"),
            constant::DW_AT_segment => write!(f, "segment"),
            constant::DW_AT_specification => write!(f, "specification"),
            constant::DW_AT_static_link => write!(f, "static_link"),
            constant::DW_AT_type => write!(f, "type"),
            constant::DW_AT_use_location => write!(f, "use_location"),
            constant::DW_AT_variable_parameter => write!(f, "variable_parameter"),
            constant::DW_AT_virtuality => write!(f, "virtuality"),
            constant::DW_AT_vtable_elem_location => write!(f, "vtable_elem_location"),
            constant::DW_AT_allocated => write!(f, "allocated"),
            constant::DW_AT_associated => write!(f, "associated"),
            constant::DW_AT_data_location => write!(f, "data_location"),
            constant::DW_AT_byte_stride => write!(f, "byte_stride"),
            constant::DW_AT_entry_pc => write!(f, "entry_pc"),
            constant::DW_AT_use_UTF8 => write!(f, "use_UTF8"),
            constant::DW_AT_extension => write!(f, "extension"),
            constant::DW_AT_ranges => write!(f, "ranges"),
            constant::DW_AT_trampoline => write!(f, "trampoline"),
            constant::DW_AT_call_column => write!(f, "call_column"),
            constant::DW_AT_call_file => write!(f, "call_file"),
            constant::DW_AT_call_line => write!(f, "call_line"),
            constant::DW_AT_description => write!(f, "description"),
            constant::DW_AT_binary_scale => write!(f, "binary_scale"),
            constant::DW_AT_decimal_scale => write!(f, "decimal_scale"),
            constant::DW_AT_small => write!(f, "small"),
            constant::DW_AT_decimal_sign => write!(f, "decimal_sign"),
            constant::DW_AT_digit_count => write!(f, "digit_count"),
            constant::DW_AT_picture_string => write!(f, "picture_string"),
            constant::DW_AT_mutable => write!(f, "mutable"),
            constant::DW_AT_threads_scaled => write!(f, "threads_scaled"),
            constant::DW_AT_explicit => write!(f, "explicit"),
            constant::DW_AT_object_pointer => write!(f, "object_pointer"),
            constant::DW_AT_endianity => write!(f, "endianity"),
            constant::DW_AT_elemental => write!(f, "elemental"),
            constant::DW_AT_pure => write!(f, "pure"),
            constant::DW_AT_recursive => write!(f, "recursive"),
            constant::DW_AT_signature => write!(f, "signature"),
            constant::DW_AT_main_subprogram => write!(f, "main_subprogram"),
            constant::DW_AT_data_bit_offset => write!(f, "data_bit_offset"),
            constant::DW_AT_const_expr => write!(f, "const_expr"),
            constant::DW_AT_enum_class => write!(f, "enum_class"),
            constant::DW_AT_linkage_name => write!(f, "linkage_name"),
            _ => write!(f, "attr({})", self.0),
        }
    }
}
