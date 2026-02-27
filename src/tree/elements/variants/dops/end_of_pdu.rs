/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{kv_row, push_types_section};
use crate::tree::types::{CellType, DetailRow, DetailSectionData};

/// Build tabbed sections for `EndOfPdu` DOP
/// Shows basic structure ref (linked) + min, max values
pub(super) fn build_end_of_pdu_dop_tabs(
    eof_field: &cda_database::datatypes::EndOfPdu<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    // Remove the DOP Variant row — not needed per requirements
    types_rows.retain(|row| row.cells.first().map(String::as_str) != Some("DOP Variant"));

    if let Some(field) = eof_field.field()
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

    if let Some(min_items) = eof_field.min_number_of_items() {
        types_rows.push(kv_row(
            "Min Number of Items",
            min_items.to_string(),
            CellType::NumericValue,
            0,
        ));
    }

    if let Some(max_items) = eof_field.max_number_of_items() {
        types_rows.push(kv_row(
            "Max Number of Items",
            max_items.to_string(),
            CellType::NumericValue,
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);
}
