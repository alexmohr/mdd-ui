// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::kv_row;
use crate::tree::types::{
    CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
    DetailSectionType,
};

/// Build tabbed sections for MUXDOP
/// General tab: Switch Key (DOP→Link, Byte Pos, Bit Pos), Default Case (Short name)
/// Cases tab: table with Short Name | Struct (link) | Lower Limit | Upper Limit
pub(super) fn build_mux_dop_tabs(
    mux_dop: &cda_database::datatypes::MuxDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    // Clear default types rows — we build our own General section
    types_rows.clear();

    let mut general_rows = Vec::new();

    general_rows.push(DetailRow {
        cells: vec!["Switch Key".to_owned(), String::new()],
        cell_types: vec![CellType::Text, CellType::Text],
        cell_jump_targets: vec![None, None],
        indent: 0,
        row_type: DetailRowType::Header,
        metadata: None,
    });

    if let Some(switch_key) = mux_dop.switch_key() {
        if let Some(dop) = switch_key.dop() {
            let dop_name = dop.short_name().unwrap_or("?").to_owned();
            general_rows.push(kv_row("DOP", dop_name, CellType::DopReference, 1));
        }
        general_rows.push(kv_row(
            "Byte Pos",
            switch_key.byte_position().to_string(),
            CellType::NumericValue,
            1,
        ));
        if let Some(bit_pos) = switch_key.bit_position() {
            general_rows.push(kv_row(
                "Bit Pos",
                bit_pos.to_string(),
                CellType::NumericValue,
                1,
            ));
        }
    }

    general_rows.push(DetailRow {
        cells: vec!["Default Case".to_owned(), String::new()],
        cell_types: vec![CellType::Text, CellType::Text],
        cell_jump_targets: vec![None, None],
        indent: 0,
        row_type: DetailRowType::Header,
        metadata: None,
    });

    if let Some(default_case) = mux_dop.default_case() {
        let dc_name = default_case.short_name().unwrap_or("-").to_owned();
        general_rows.push(kv_row("Short Name", dc_name, CellType::Text, 1));
    }

    let general_header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    sections.push(
        DetailSectionData::new(
            "General".to_owned(),
            DetailContent::Table {
                header: general_header,
                rows: general_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    );

    sections.push(build_cases_section(mux_dop));
}

fn build_cases_section(mux_dop: &cda_database::datatypes::MuxDop<'_>) -> DetailSectionData {
    let Some(cases) = mux_dop.cases() else {
        return DetailSectionData {
            title: "Cases".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::PlainText(vec!["No cases".to_owned()]),
        };
    };

    let cases_header = DetailRow {
        cells: vec![
            "Short Name".to_owned(),
            "Struct".to_owned(),
            "Lower Limit".to_owned(),
            "Upper Limit".to_owned(),
        ],
        cell_types: vec![
            CellType::Text,
            CellType::Text,
            CellType::NumericValue,
            CellType::NumericValue,
        ],
        indent: 0,
        ..Default::default()
    };

    let rows: Vec<DetailRow> = cases
        .iter()
        .map(|case| {
            let name = case.short_name().unwrap_or("?").to_owned();
            let struct_name = case
                .structure()
                .and_then(|s| s.short_name())
                .unwrap_or("-")
                .to_owned();
            let has_struct = struct_name != "-";
            let lower = case
                .lower_limit()
                .map(|l| format!("{l:?}"))
                .unwrap_or_default();
            let upper = case
                .upper_limit()
                .map(|l| format!("{l:?}"))
                .unwrap_or_default();

            let dop_jump = if has_struct {
                Some(crate::tree::CellJumpTarget::Dop {
                    name: struct_name.clone(),
                })
            } else {
                None
            };

            DetailRow {
                cells: vec![name, struct_name, lower, upper],
                cell_types: vec![
                    CellType::Text,
                    if has_struct {
                        CellType::DopReference
                    } else {
                        CellType::Text
                    },
                    CellType::NumericValue,
                    CellType::NumericValue,
                ],
                cell_jump_targets: vec![None, dop_jump, None, None],
                indent: 0,
                row_type: DetailRowType::Normal,
                metadata: None,
            }
        })
        .collect();

    DetailSectionData {
        title: "Cases".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header: cases_header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }
}
