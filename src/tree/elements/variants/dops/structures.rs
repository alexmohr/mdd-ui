// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::kv_row;
use crate::tree::types::{
    CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
    DetailSectionType,
};

/// Build tabbed sections for Structure DOP: Overview + Params tabs
pub(super) fn build_structure_dop_tabs(
    structure: &cda_database::datatypes::StructureDop<'_>,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    let mut overview_rows = vec![kv_row(
        "Is Visible",
        structure.is_visible().to_string(),
        CellType::Text,
        0,
    )];

    if let Some(byte_size) = structure.byte_size() {
        overview_rows.push(kv_row(
            "Byte Size",
            byte_size.to_string(),
            CellType::NumericValue,
            0,
        ));
    }

    if let Some(params) = structure.params() {
        overview_rows.push(kv_row(
            "Param Count",
            params.len().to_string(),
            CellType::NumericValue,
            0,
        ));
    }

    // Drop the default types_rows (Short Name, DOP Variant, etc.) — not needed for structures
    types_rows.clear();

    let overview_header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    sections.push(
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header: overview_header,
                rows: overview_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    );

    if let Some(params) = structure.params() {
        let params_header = DetailRow {
            cells: vec![
                "Short Name".to_owned(),
                "Byte".to_owned(),
                "Bit\nLen".to_owned(),
                "Byte\nLen".to_owned(),
                "Value".to_owned(),
                "DOP".to_owned(),
                "Semantic".to_owned(),
            ],
            cell_types: vec![
                CellType::Text,
                CellType::NumericValue,
                CellType::NumericValue,
                CellType::NumericValue,
                CellType::NumericValue,
                CellType::Text,
                CellType::Text,
            ],
            indent: 0,
            ..Default::default()
        };

        let rows: Vec<DetailRow> = params
            .iter()
            .map(|p| {
                let param = cda_database::datatypes::Parameter(p);
                let name = param.short_name().unwrap_or("?").to_owned();
                let byte_pos = param.byte_position();
                let bit_len = "-".to_owned();
                let byte_len = "-".to_owned();
                let value = crate::tree::elements::variants::services::extract_coded_value(&param);
                let dop_name = crate::tree::elements::variants::services::extract_dop_name(&param);
                let semantic = param.semantic().unwrap_or_default().to_owned();
                let has_dop = !dop_name.is_empty();
                let param_id = param.id();

                DetailRow {
                    cells: vec![
                        name,
                        byte_pos.to_string(),
                        bit_len,
                        byte_len,
                        value,
                        dop_name,
                        semantic,
                    ],
                    cell_types: vec![
                        CellType::ParameterName,
                        CellType::NumericValue,
                        CellType::Text,
                        CellType::Text,
                        CellType::NumericValue,
                        if has_dop {
                            CellType::DopReference
                        } else {
                            CellType::Text
                        },
                        CellType::Text,
                    ],
                    indent: 0,
                    row_type: DetailRowType::Normal,
                    metadata: Some(crate::tree::RowMetadata::ParameterRow { param_id }),
                }
            })
            .collect();

        sections.push(DetailSectionData {
            title: "Params".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Requests,
            content: DetailContent::Table {
                header: params_header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Fixed(4),
                    ColumnConstraint::Fixed(4),
                    ColumnConstraint::Fixed(5),
                    ColumnConstraint::Percentage(15),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(20),
                ],
                use_row_selection: false,
            },
        });
    }
}
