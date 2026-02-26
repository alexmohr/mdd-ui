// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use super::{ParsedDopName, push_types_section};
use crate::tree::types::{
    CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
    DetailSectionType,
};

fn build_constraints_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let header = DetailRow::header(
        vec![
            "Lower Type".to_owned(),
            "Lower Limit".to_owned(),
            "Upper Limit".to_owned(),
            "Upper Type".to_owned(),
            "Validity".to_owned(),
        ],
        vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
    );

    let mut rows = Vec::new();

    if let Some(constr) = normal_dop.internal_constr() {
        // Main constraint range
        let lower_type = constr
            .lower_limit()
            .map_or("-".to_owned(), |l| format!("{:?}", l.interval_type()));
        let lower_val = constr
            .lower_limit()
            .and_then(|l| l.value())
            .unwrap_or("-")
            .to_owned();
        let upper_type = constr
            .upper_limit()
            .map_or("-".to_owned(), |l| format!("{:?}", l.interval_type()));
        let upper_val = constr
            .upper_limit()
            .and_then(|l| l.value())
            .unwrap_or("-")
            .to_owned();

        rows.push(DetailRow::normal(
            vec![lower_type, lower_val, upper_val, upper_type, "-".to_owned()],
            vec![
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
                CellType::Text,
            ],
            0,
        ));

        // Scale constraints
        rows.extend(
            constr
                .scale_constr()
                .into_iter()
                .flat_map(|sc| sc.iter())
                .map(|sc| {
                    let lower_type = sc
                        .lower_limit()
                        .map_or("-".to_owned(), |l| format!("{:?}", l.interval_type()));
                    let lower_val = sc
                        .lower_limit()
                        .and_then(|l| l.value())
                        .unwrap_or("-")
                        .to_owned();
                    let upper_type = sc
                        .upper_limit()
                        .map_or("-".to_owned(), |l| format!("{:?}", l.interval_type()));
                    let upper_val = sc
                        .upper_limit()
                        .and_then(|l| l.value())
                        .unwrap_or("-")
                        .to_owned();
                    let validity = format!("{:?}", sc.validity());

                    DetailRow::normal(
                        vec![lower_type, lower_val, upper_val, upper_type, validity],
                        vec![
                            CellType::Text,
                            CellType::Text,
                            CellType::Text,
                            CellType::Text,
                            CellType::Text,
                        ],
                        0,
                    )
                }),
        );
    }

    DetailSectionData {
        title: "Internal-Constr".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }
}

fn build_compu_internal_to_phys_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let header = DetailRow::header(
        vec![
            "Lower Limit".to_owned(),
            "Upper Limit".to_owned(),
            "Compu Inverse Value".to_owned(),
            "Compu Const".to_owned(),
        ],
        vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
    );

    let mut rows = Vec::new();
    let mut subsections = Vec::new();

    if let Some(compu_method) = normal_dop.compu_method() {
        subsections.push(DetailSectionData {
            title: String::new(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::PlainText(vec![format!(
                "Category: {:?}",
                compu_method.category()
            )]),
        });

        if let Some(i2p) = compu_method.internal_to_phys() {
            rows.extend(
                i2p.compu_scales()
                    .into_iter()
                    .flat_map(|scales| scales.iter())
                    .map(|scale| {
                        let lower = scale
                            .lower_limit()
                            .and_then(|l| l.value())
                            .unwrap_or("-")
                            .to_owned();
                        let upper = scale
                            .upper_limit()
                            .and_then(|l| l.value())
                            .unwrap_or("-")
                            .to_owned();
                        let inverse = scale.inverse_values().map_or("-".to_owned(), |iv| {
                            iv.vt().map_or_else(
                                || iv.v().map_or("-".to_owned(), |v| v.to_string()),
                                str::to_owned,
                            )
                        });
                        let consts = scale.consts().map_or("-".to_owned(), |c| {
                            c.vt().map_or_else(
                                || c.v().map_or("-".to_owned(), |v| v.to_string()),
                                str::to_owned,
                            )
                        });

                        DetailRow::normal(
                            vec![lower, upper, inverse, consts],
                            vec![
                                CellType::Text,
                                CellType::Text,
                                CellType::Text,
                                CellType::Text,
                            ],
                            0,
                        )
                    }),
            );
        }
    }

    subsections.push(DetailSectionData {
        title: String::new(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
            ],
            use_row_selection: true,
        },
    });

    DetailSectionData {
        title: "Compu-Internal-To-Phys".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Composite(subsections),
    }
}

fn build_compu_phys_to_internal_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let header = DetailRow::header(
        vec![
            "Lower Limit".to_owned(),
            "Upper Limit".to_owned(),
            "Compu Inverse Value".to_owned(),
            "Compu Const".to_owned(),
        ],
        vec![
            CellType::Text,
            CellType::Text,
            CellType::Text,
            CellType::Text,
        ],
    );

    let mut rows = Vec::new();
    let mut subsections = Vec::new();

    if let Some(compu_method) = normal_dop.compu_method()
        && let Some(p2i) = compu_method.phys_to_internal()
    {
        subsections.push(DetailSectionData {
            title: String::new(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::PlainText(vec![format!(
                "Category: {:?}",
                compu_method.category()
            )]),
        });

        rows.extend(
            p2i.compu_scales()
                .into_iter()
                .flat_map(|scales| scales.iter())
                .map(|scale| {
                    let lower = scale
                        .lower_limit()
                        .and_then(|l| l.value())
                        .unwrap_or("-")
                        .to_owned();
                    let upper = scale
                        .upper_limit()
                        .and_then(|l| l.value())
                        .unwrap_or("-")
                        .to_owned();
                    let inverse = scale.inverse_values().map_or("-".to_owned(), |iv| {
                        iv.vt().map_or_else(
                            || iv.v().map_or("-".to_owned(), |v| v.to_string()),
                            str::to_owned,
                        )
                    });
                    let consts = scale.consts().map_or("-".to_owned(), |c| {
                        c.vt().map_or_else(
                            || c.v().map_or("-".to_owned(), |v| v.to_string()),
                            str::to_owned,
                        )
                    });

                    DetailRow::normal(
                        vec![lower, upper, inverse, consts],
                        vec![
                            CellType::Text,
                            CellType::Text,
                            CellType::Text,
                            CellType::Text,
                        ],
                        0,
                    )
                }),
        );
    }

    subsections.push(DetailSectionData {
        title: String::new(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
            ],
            use_row_selection: true,
        },
    });

    DetailSectionData {
        title: "Compu-Phys-To-Internal".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Composite(subsections),
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
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });

        if let Some(bit_len) = coded_type.bit_len() {
            types_rows.push(DetailRow {
                cells: vec!["Bit Length".to_owned(), bit_len.to_string()],
                cell_types: vec![CellType::Text, CellType::NumericValue],
                cell_jump_targets: vec![None; 2],
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
            cell_jump_targets: vec![None; 2],
            indent: 0,
            row_type: DetailRowType::Normal,
            metadata: None,
        });
    }

    push_types_section(std::mem::take(types_rows), sections);

    sections.push(build_constraints_section(normal_dop));
    sections.push(build_compu_internal_to_phys_section(normal_dop));
    sections.push(build_compu_phys_to_internal_section(normal_dop));
}
