// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::{kv_row, push_types_section};
use crate::tree::types::{CellType, DetailRow, DetailSectionData};

/// Build tabbed sections for `DynamicLengthField` DOP
/// Shows bit/byte position and data object prop ref (link), not DOP variant/short name
pub(super) fn build_dynamic_length_field_dop_tabs(
    dyn_field: &cda_database::datatypes::DynamicLengthDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    // Remove DOP Variant and Short Name rows — not wanted for dynamic length fields
    types_rows.retain(|row| {
        let key = row.cells.first().map(std::string::String::as_str);
        !matches!(key, Some("DOP Variant" | "Short Name"))
    });

    let offset = dyn_field.offset();
    types_rows.push(kv_row(
        "Offset",
        offset.to_string(),
        CellType::NumericValue,
        0,
    ));

    if let Some(det) = dyn_field.determine_number_of_items() {
        types_rows.push(kv_row(
            "Byte Position",
            det.byte_position().to_string(),
            CellType::NumericValue,
            0,
        ));
        types_rows.push(kv_row(
            "Bit Position",
            det.bit_position().to_string(),
            CellType::NumericValue,
            0,
        ));

        if let Some(dop) = det.dop() {
            let dop_name = dop.short_name().unwrap_or("?").to_owned();
            types_rows.push(kv_row(
                "Data Object Prop",
                dop_name,
                CellType::DopReference,
                0,
            ));
        }
    }

    if let Some(field) = dyn_field.field()
        && let Some(basic_struct) = field.basic_structure()
    {
        let struct_name = basic_struct.short_name().unwrap_or("?").to_owned();
        types_rows.push(kv_row(
            "Basic Structure",
            struct_name,
            CellType::DopReference,
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);
}
