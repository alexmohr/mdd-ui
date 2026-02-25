// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::{ParsedDopName, push_types_section};
use crate::tree::types::{
    CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
    DetailSectionType,
};

fn build_constraints_section(parsed_name: &ParsedDopName) -> DetailSectionData {
    let mut constraints_rows = Vec::new();

    if let Some(ref min) = parsed_name.range_min {
        constraints_rows.push(DetailRow {
            cells: vec!["Range Min (from name)".to_owned(), min.clone()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    if let Some(ref max) = parsed_name.range_max {
        constraints_rows.push(DetailRow {
            cells: vec!["Range Max (from name)".to_owned(), max.clone()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    let constraints_header = DetailRow {
        cells: vec!["Property".to_owned(), "Value".to_owned()],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    DetailSectionData {
        title: "Internal-Constr".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header: constraints_header,
            rows: constraints_rows,
            constraints: vec![
                ColumnConstraint::Percentage(40),
                ColumnConstraint::Percentage(60),
            ],
            use_row_selection: true,
        },
    }
}

fn build_compu_internal_to_phys_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let mut compu_i2p_rows = Vec::new();

    if let Some(compu_method) = normal_dop.compu_method() {
        compu_i2p_rows.push(DetailRow {
            cells: vec![
                "Category".to_owned(),
                format!("{:?}", compu_method.category()),
            ],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });

        if let Some(internal_to_phys) = compu_method.internal_to_phys()
            && let Some(scales) = internal_to_phys.compu_scales()
        {
            compu_i2p_rows.push(DetailRow {
                cells: vec!["Scales Count".to_owned(), scales.len().to_string()],
                cell_types: vec![CellType::Text, CellType::NumericValue],
                indent: 0,
                row_type: DetailRowType::Normal,
                metadata: None,
            });
        }
    }

    let compu_i2p_header = DetailRow {
        cells: vec!["Property".to_owned(), "Value".to_owned()],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    DetailSectionData {
        title: "Compu-Internal-To-Phys".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header: compu_i2p_header,
            rows: compu_i2p_rows,
            constraints: vec![
                ColumnConstraint::Percentage(40),
                ColumnConstraint::Percentage(60),
            ],
            use_row_selection: true,
        },
    }
}

fn build_compu_phys_to_internal_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let mut compu_p2i_rows = Vec::new();

    if let Some(compu_method) = normal_dop.compu_method()
        && let Some(phys_to_internal) = compu_method.phys_to_internal()
        && let Some(scales) = phys_to_internal.compu_scales()
    {
        compu_p2i_rows.push(DetailRow {
            cells: vec!["Scales Count".to_owned(), scales.len().to_string()],
            cell_types: vec![CellType::Text, CellType::NumericValue],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    let compu_p2i_header = DetailRow {
        cells: vec!["Property".to_owned(), "Value".to_owned()],
        cell_types: vec![CellType::Text, CellType::Text],
        indent: 0,
        ..Default::default()
    };

    DetailSectionData {
        title: "Compu-Phys-To-Internal".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header: compu_p2i_header,
            rows: compu_p2i_rows,
            constraints: vec![
                ColumnConstraint::Percentage(40),
                ColumnConstraint::Percentage(60),
            ],
            use_row_selection: true,
        },
    }
}

/// Build tabbed sections for `NormalDOP` with Types, Constraints, and Compu tabs
pub(super) fn build_normal_dop_tabs(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
    parsed_name: &ParsedDopName,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Ok(coded_type) = normal_dop.diag_coded_type() {
        types_rows.push(DetailRow {
            cells: vec![
                "Diag Coded Type".to_owned(),
                format!("{:?}", coded_type.base_datatype()),
            ],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });

        if let Some(bit_len) = coded_type.bit_len() {
            types_rows.push(DetailRow {
                cells: vec!["Bit Length".to_owned(), bit_len.to_string()],
                cell_types: vec![CellType::Text, CellType::NumericValue],
                indent: 0,
                row_type: DetailRowType::Normal,
                metadata: None,
            });
        }
    }

    if let Some(ref data_type) = parsed_name.data_type {
        types_rows.push(DetailRow {
            cells: vec!["Data Type (from name)".to_owned(), data_type.clone()],
            cell_types: vec![CellType::Text, CellType::Text],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    push_types_section(std::mem::take(types_rows), sections);

    sections.push(build_constraints_section(parsed_name));
    sections.push(build_compu_internal_to_phys_section(normal_dop));
    sections.push(build_compu_phys_to_internal_section(normal_dop));
}
