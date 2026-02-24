// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::{kv_row, push_types_section};
use crate::tree::types::{CellType, DetailRow, DetailSectionData};

/// Build tabbed sections for StaticField DOP
/// Shows byte size and fixed number of items (no DOP variant)
pub(super) fn build_static_field_dop_tabs(
    static_field: &cda_database::datatypes::StaticFieldDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    // Remove the DOP Variant row — keep Short Name
    types_rows.retain(|row| row.cells.first().map(|c| c.as_str()) != Some("DOP Variant"));

    types_rows.push(kv_row(
        "Fixed Number of Items",
        static_field.fixed_number_of_items().to_string(),
        CellType::NumericValue,
        0,
    ));

    types_rows.push(kv_row(
        "Item Byte Size",
        static_field.item_byte_size().to_string(),
        CellType::NumericValue,
        0,
    ));

    if let Some(field) = static_field.field() {
        types_rows.push(kv_row(
            "Is Visible",
            field.is_visible().to_string(),
            CellType::Text,
            0,
        ));

        if let Some(basic_struct) = field.basic_structure() {
            let struct_name = basic_struct.short_name().unwrap_or("?").to_owned();
            types_rows.push(kv_row(
                "Basic Structure",
                struct_name,
                CellType::DopReference,
                0,
            ));
        }
    }

    push_types_section(std::mem::take(types_rows), sections);
}
