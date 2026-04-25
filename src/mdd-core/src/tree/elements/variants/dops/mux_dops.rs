/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::kv_row;
use crate::tree::types::{
    CellJumpTargetType, CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow,
    DetailSectionData, DetailSectionType,
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

    general_rows.push(DetailRow::header(vec![
        DetailCell::text("Switch Key"),
        DetailCell::text(""),
    ]));

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

    general_rows.push(DetailRow::header(vec![
        DetailCell::text("Default Case"),
        DetailCell::text(""),
    ]));

    if let Some(default_case) = mux_dop.default_case() {
        let dc_name = default_case.short_name().unwrap_or("-").to_owned();
        general_rows.push(kv_row("Short Name", dc_name, CellType::Text, 1));
    }

    let general_header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Value"),
    ]);

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

    let cases_header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::text("Struct"),
        DetailCell::new("Lower Limit", CellType::NumericValue),
        DetailCell::new("Upper Limit", CellType::NumericValue),
    ]);

    let rows: Vec<DetailRow> = cases
        .iter()
        .map(|case| {
            let name = case.short_name().unwrap_or("?").to_owned();
            let struct_name = case
                .structure()
                .and_then(|s| s.short_name())
                .map(str::to_owned);
            let lower = case
                .lower_limit()
                .map(|l| format!("{l:?}"))
                .unwrap_or_default();
            let upper = case
                .upper_limit()
                .map(|l| format!("{l:?}"))
                .unwrap_or_default();

            let dop_jump = struct_name.as_ref().map(|n| {
                crate::tree::CellJumpTarget::new(CellJumpTargetType::Dop {
                    index: usize::MAX,
                    name: n.clone(),
                })
            });
            let struct_display = struct_name.clone().unwrap_or_else(|| "-".to_owned());

            {
                let struct_cell_type = if struct_name.is_some() {
                    CellType::DopReference
                } else {
                    CellType::Text
                };
                let mut struct_cell = DetailCell::new(struct_display, struct_cell_type);
                if let Some(jump) = dop_jump {
                    struct_cell = struct_cell.with_jump(jump);
                }
                DetailRow::normal(
                    vec![
                        DetailCell::text(name),
                        struct_cell,
                        DetailCell::new(lower, CellType::NumericValue),
                        DetailCell::new(upper, CellType::NumericValue),
                    ],
                    0,
                )
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
