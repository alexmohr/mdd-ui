/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::{DataOperationVariant, Parameter};

use crate::tree::{
    CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow, DetailSectionData,
    DetailSectionType,
};

/// Extracted byte-field info from a single parameter.
struct ByteField {
    byte_pos: u32,
    bit_pos: u32,
    bit_length: Option<u32>,
    name: String,
    type_label: &'static str,
    coded_value: Option<u64>,
    param_id: u32,
}

impl ByteField {
    fn from_param(param: &Parameter<'_>) -> Self {
        let name = param.short_name().unwrap_or("?").to_owned();
        let type_label = param
            .param_type()
            .map_or("?", |pt| crate::tree::param_type_label(&pt));
        let bit_pos = param.bit_position();
        let byte_pos = param.byte_position();

        let coded_value = extract_coded_value(param);
        let bit_length = extract_bit_length(param);

        Self {
            byte_pos,
            bit_pos,
            bit_length,
            name,
            type_label,
            coded_value,
            param_id: param.id(),
        }
    }

    /// Number of bytes this field spans (rounded up). Returns 1 if unknown.
    fn byte_span(&self) -> u32 {
        self.bit_length.map_or(1, |bl| {
            let effective_start = if self.bit_pos == crate::tree::BIT_POSITION_UNSET {
                0
            } else {
                self.bit_pos
            };
            effective_start
                .saturating_add(bl)
                .saturating_add(7)
                .checked_div(8)
                .unwrap_or(1)
        })
    }
}

/// Extract coded value from any const-like parameter type.
fn extract_coded_value(param: &Parameter<'_>) -> Option<u64> {
    // CodedConst
    if let Some(cc) = param.specific_data_as_coded_const() {
        return cc.coded_value().and_then(|v| v.parse::<u64>().ok());
    }

    // NrcConst – takes the first coded value
    if let Some(nrc) = param.specific_data_as_nrc_const()
        && let Some(vals) = nrc.coded_values()
        && !vals.is_empty()
    {
        return vals.get(0).parse::<u64>().ok();
    }

    // PhysConst – physical constant value (needs DOP for internal→coded conversion,
    // but for display purposes we parse the raw value)
    if let Some(pc) = param.specific_data_as_phys_const() {
        return pc.phys_constant_value().and_then(|v| v.parse::<u64>().ok());
    }

    None
}

/// Extract bit length from a parameter's type-specific data.
fn extract_bit_length(param: &Parameter<'_>) -> Option<u32> {
    // CodedConst
    if let Some(cc) = param.specific_data_as_coded_const()
        && let Some(dct) = cc.diag_coded_type()
        && let Ok(owned) = cda_database::datatypes::DiagCodedType::try_from(dct)
    {
        return owned.bit_len();
    }

    // NrcConst
    if let Some(nrc) = param.specific_data_as_nrc_const()
        && let Some(dct) = nrc.diag_coded_type()
        && let Ok(owned) = cda_database::datatypes::DiagCodedType::try_from(dct)
    {
        return owned.bit_len();
    }

    // Reserved
    if let Some(res) = param.specific_data_as_reserved() {
        return Some(res.bit_length());
    }

    // MatchingRequestParam
    if let Some(mrp) = param.specific_data_as_matching_request_param() {
        return Some(mrp.byte_length().saturating_mul(8));
    }

    // PhysConst / Value / System / LengthKeyRef – navigate to NormalDop
    let dop_opt = param
        .specific_data_as_phys_const()
        .and_then(|pc| pc.dop())
        .or_else(|| param.specific_data_as_value().and_then(|v| v.dop()))
        .or_else(|| param.specific_data_as_system().and_then(|s| s.dop()))
        .or_else(|| {
            param
                .specific_data_as_length_key_ref()
                .and_then(|l| l.dop())
        });

    if let Some(dop) = dop_opt {
        let data_op = cda_database::datatypes::DataOperation(dop);
        if let Ok(DataOperationVariant::Normal(normal_dop)) = data_op.variant()
            && let Ok(diag_type) = normal_dop.diag_coded_type()
        {
            return diag_type.bit_len();
        }
    }

    None
}

/// Build just the byte/bit pattern rows (without wrapping in a section).
/// Used by `build_param_section` to embed the rows directly in the section.
#[must_use]
pub fn build_byte_pattern_rows(params: &[Parameter<'_>]) -> Vec<DetailRow> {
    let mut fields: Vec<ByteField> = params.iter().map(ByteField::from_param).collect();
    fields.sort_by_key(|f| (f.byte_pos, effective_bit_pos(f.bit_pos)));

    let mut rows = Vec::new();
    let mut i = 0;
    while i < fields.len() {
        let byte_pos = fields.get(i).map_or(0, |f| f.byte_pos);
        let group_start = i;
        while i < fields.len() && fields.get(i).is_some_and(|f| f.byte_pos == byte_pos) {
            i = i.saturating_add(1);
        }
        let group = fields.get(group_start..i).unwrap_or_default();
        if group.len() == 1 {
            let Some(f) = group.first() else { continue };
            rows.push(build_field_row(f, 0));
        } else {
            for f in group {
                rows.push(build_field_row(f, 1));
            }
        }
    }
    rows
}

/// Build a "Byte Pattern" detail section visualising the byte/bit layout of a
/// PDU (request or response).
///
/// Each parameter is rendered as a row showing its byte offset, bit range,
/// hex/binary value (for coded constants), name and type.  Sub-byte
/// parameters sharing the same byte position are shown with indentation to
/// indicate bit-level drill-down.
#[must_use]
pub fn build_byte_pattern_section(title: &str, params: &[Parameter<'_>]) -> DetailSectionData {
    let rows = build_byte_pattern_rows(params);

    let header = DetailRow::header(vec![
        DetailCell::new("Offset", CellType::NumericValue),
        DetailCell::text("Bits"),
        DetailCell::text("Hex"),
        DetailCell::text("Binary"),
        DetailCell::text("Parameter"),
        DetailCell::text("Type"),
    ]);

    DetailSectionData {
        title: title.to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Fixed(8),
                ColumnConstraint::Fixed(7),
                ColumnConstraint::Fixed(8),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(40),
                ColumnConstraint::Percentage(15),
            ],
            use_row_selection: true,
        },
        byte_pattern_rows: None,
    }
}

// ------------------------------------------------------------------
// Formatting helpers
// ------------------------------------------------------------------

fn effective_bit_pos(bit_pos: u32) -> u32 {
    if bit_pos == crate::tree::BIT_POSITION_UNSET {
        0
    } else {
        bit_pos
    }
}

fn build_field_row(f: &ByteField, indent: usize) -> DetailRow {
    let offset = format_offset(f);
    let bits = format_bit_range(f);
    let hex = format_hex(f);
    let binary = format_binary(f);

    let mut row = DetailRow::normal(
        vec![
            DetailCell::new(offset, CellType::NumericValue),
            DetailCell::text(bits),
            DetailCell::new(hex, CellType::NumericValue),
            DetailCell::text(binary),
            DetailCell::new(f.name.clone(), CellType::ParameterName).with_jump(
                crate::tree::CellJumpTarget::new(crate::tree::CellJumpTargetType::Parameter {
                    param_id: f.param_id,
                }),
            ),
            DetailCell::text(f.type_label),
        ],
        indent,
    );
    row.metadata = Some(crate::tree::RowMetadata::ParameterRow {
        param_id: f.param_id,
    });
    row
}

/// Format the byte offset column: `"0"`, `"1-2"`, or `"3..n"` for
/// variable-length fields.
fn format_offset(f: &ByteField) -> String {
    let span = f.byte_span();
    if span <= 1 {
        f.byte_pos.to_string()
    } else {
        let end = f.byte_pos.saturating_add(span).saturating_sub(1);
        format!("{}-{end}", f.byte_pos)
    }
}

/// Format the bit range: `"[7:0]"`, `"[7]"`, `"[15:0]"`, etc.
fn format_bit_range(f: &ByteField) -> String {
    let start_bit = effective_bit_pos(f.bit_pos);
    let Some(bl) = f.bit_length else {
        return if start_bit == 0 {
            "[7:0]".to_owned()
        } else {
            format!("[{start_bit}]")
        };
    };

    if bl == 0 {
        return String::new();
    }

    let end_bit = start_bit.saturating_add(bl).saturating_sub(1);
    if end_bit == start_bit {
        format!("[{start_bit}]")
    } else {
        format!("[{end_bit}:{start_bit}]")
    }
}

/// Format the hex value: `"0x22"` for coded constants, `"??"` for dynamic.
fn format_hex(f: &ByteField) -> String {
    let Some(val) = f.coded_value else {
        let byte_span = f.byte_span();
        return "??".repeat(byte_span as usize);
    };

    let Some(bl) = f.bit_length else {
        return format!("0x{val:02X}");
    };

    let byte_count = bl.saturating_add(7).checked_div(8).unwrap_or(1);
    match byte_count {
        0 | 1 => format!("0x{val:02X}"),
        2 => format!("0x{val:04X}"),
        3 => format!("0x{val:06X}"),
        4 => format!("0x{val:08X}"),
        _ => format!("0x{val:X}"),
    }
}

/// Format the binary representation: `"0010 0010"` for known values,
/// `"????????"` for dynamic.
fn format_binary(f: &ByteField) -> String {
    let bl = f.bit_length.unwrap_or(8);
    let Some(val) = f.coded_value else {
        return "?".repeat(bl as usize);
    };

    let raw = format!("{val:0width$b}", width = bl as usize);

    // Insert spaces every 4 bits for readability.
    let mut spaced = String::with_capacity(
        raw.len()
            .saturating_add(raw.len().checked_div(4).unwrap_or(0)),
    );
    for (i, ch) in raw.chars().enumerate() {
        if i > 0 && i.checked_rem(4) == Some(0) {
            spaced.push(' ');
        }
        spaced.push(ch);
    }
    spaced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_hex_coded_value() {
        let f = ByteField {
            byte_pos: 0,
            bit_pos: 0,
            bit_length: Some(8),
            name: "SID".to_owned(),
            type_label: "CodedConst",
            coded_value: Some(0x22),
            param_id: 0,
        };
        assert_eq!(format_hex(&f), "0x22");
    }

    #[test]
    fn format_hex_unknown() {
        let f = ByteField {
            byte_pos: 3,
            bit_pos: 0,
            bit_length: Some(16),
            name: "Data".to_owned(),
            type_label: "Value",
            coded_value: None,
            param_id: 1,
        };
        assert_eq!(format_hex(&f), "????");
    }

    #[test]
    fn format_binary_known() {
        let f = ByteField {
            byte_pos: 0,
            bit_pos: 0,
            bit_length: Some(8),
            name: "SID".to_owned(),
            type_label: "CodedConst",
            coded_value: Some(0x22),
            param_id: 0,
        };
        assert_eq!(format_binary(&f), "0010 0010");
    }

    #[test]
    fn format_binary_unknown() {
        let f = ByteField {
            byte_pos: 1,
            bit_pos: 0,
            bit_length: Some(8),
            name: "Dyn".to_owned(),
            type_label: "Value",
            coded_value: None,
            param_id: 2,
        };
        assert_eq!(format_binary(&f), "????????");
    }

    #[test]
    fn format_bit_range_full_byte() {
        let f = ByteField {
            byte_pos: 0,
            bit_pos: 0,
            bit_length: Some(8),
            name: String::new(),
            type_label: "",
            coded_value: None,
            param_id: 0,
        };
        assert_eq!(format_bit_range(&f), "[7:0]");
    }

    #[test]
    fn format_bit_range_single_bit() {
        let f = ByteField {
            byte_pos: 1,
            bit_pos: 7,
            bit_length: Some(1),
            name: String::new(),
            type_label: "",
            coded_value: None,
            param_id: 0,
        };
        assert_eq!(format_bit_range(&f), "[7]");
    }

    #[test]
    fn format_bit_range_sub_byte() {
        let f = ByteField {
            byte_pos: 1,
            bit_pos: 0,
            bit_length: Some(7),
            name: String::new(),
            type_label: "",
            coded_value: None,
            param_id: 0,
        };
        assert_eq!(format_bit_range(&f), "[6:0]");
    }

    #[test]
    fn format_offset_single_byte() {
        let f = ByteField {
            byte_pos: 5,
            bit_pos: 0,
            bit_length: Some(8),
            name: String::new(),
            type_label: "",
            coded_value: None,
            param_id: 0,
        };
        assert_eq!(format_offset(&f), "5");
    }

    #[test]
    fn format_offset_multi_byte() {
        let f = ByteField {
            byte_pos: 1,
            bit_pos: 0,
            bit_length: Some(16),
            name: String::new(),
            type_label: "",
            coded_value: None,
            param_id: 0,
        };
        assert_eq!(format_offset(&f), "1-2");
    }
}
