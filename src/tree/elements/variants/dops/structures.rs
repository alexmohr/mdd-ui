/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::kv_row;
use crate::tree::types::{
    CellJumpTargetType, CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow,
    DetailSectionData, DetailSectionType,
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

    let overview_header = DetailRow::header(vec![DetailCell::text("Property"), DetailCell::text("Value")]);

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

    sections.push(build_params_section(structure));
}

fn build_params_section(
    structure: &cda_database::datatypes::StructureDop<'_>,
) -> DetailSectionData {
    let Some(params) = structure.params() else {
        return DetailSectionData {
            title: "Params".to_owned(),
            render_as_header: false,
            section_type: DetailSectionType::Requests,
            content: DetailContent::PlainText(vec!["No params".to_owned()]),
        };
    };
    let params_header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::new("Byte", CellType::NumericValue),
        DetailCell::new("Bit\nLen", CellType::NumericValue),
        DetailCell::new("Byte\nLen", CellType::NumericValue),
        DetailCell::new("Value", CellType::NumericValue),
        DetailCell::text("DOP"),
        DetailCell::text("Semantic"),
    ]);

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

            let dop_jump = if has_dop {
                Some(crate::tree::CellJumpTarget::new(CellJumpTargetType::Dop {
                    name: dop_name.clone(),
                }))
            } else {
                None
            };

            {
                let dop_cell_type = if has_dop { CellType::DopReference } else { CellType::Text };
                let mut dop_cell = DetailCell::new(dop_name, dop_cell_type);
                if let Some(jump) = dop_jump {
                    dop_cell = dop_cell.with_jump(jump);
                }
                let mut row = DetailRow::normal(
                    vec![
                        DetailCell::new(name, CellType::ParameterName)
                            .with_jump(crate::tree::CellJumpTarget::new(
                                CellJumpTargetType::Parameter { param_id },
                            )),
                        DetailCell::new(byte_pos.to_string(), CellType::NumericValue),
                        DetailCell::text(bit_len),
                        DetailCell::text(byte_len),
                        DetailCell::new(value, CellType::NumericValue),
                        dop_cell,
                        DetailCell::text(semantic),
                    ],
                    0,
                );
                row.metadata = Some(crate::tree::RowMetadata::ParameterRow { param_id });
                row
            }
        })
        .collect();

    DetailSectionData {
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
    }
}
