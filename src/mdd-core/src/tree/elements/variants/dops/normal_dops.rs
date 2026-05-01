/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{ParsedDopName, push_types_section};
use crate::tree::types::{
    CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow, DetailSectionData,
    DetailSectionType,
};

fn build_constraints_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let header = DetailRow::header(vec![
        DetailCell::text("Lower Type"),
        DetailCell::text("Lower Limit"),
        DetailCell::text("Upper Limit"),
        DetailCell::text("Upper Type"),
        DetailCell::text("Validity"),
    ]);

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
            vec![
                DetailCell::text(lower_type),
                DetailCell::text(lower_val),
                DetailCell::text(upper_val),
                DetailCell::text(upper_type),
                DetailCell::text("-"),
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
                        vec![
                            DetailCell::text(lower_type),
                            DetailCell::text(lower_val),
                            DetailCell::text(upper_val),
                            DetailCell::text(upper_type),
                            DetailCell::text(validity),
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
        byte_pattern_rows: None,
    }
}

/// Build a composite `DetailSectionData` for one direction of a Compu-Method.
/// `category` is the formatted "Category: ..." string to show when present;
/// `rows` are the pre-built scale rows.
fn build_compu_direction_section(
    title: &str,
    category: Option<String>,
    rows: Vec<DetailRow>,
) -> DetailSectionData {
    let header = DetailRow::header(vec![
        DetailCell::text("Lower Limit"),
        DetailCell::text("Upper Limit"),
        DetailCell::text("Compu Inverse Value"),
        DetailCell::text("Compu Const"),
    ]);

    let mut subsections = Vec::new();

    if let Some(cat) = category {
        subsections.push(DetailSectionData {
            title: String::new(),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::PlainText(vec![cat]),
            byte_pattern_rows: None,
        });
    }

    subsections.push(DetailSectionData {
        title: String::new(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(15),
                ColumnConstraint::Percentage(15),
                ColumnConstraint::Percentage(25),
                ColumnConstraint::Percentage(25),
            ],
            use_row_selection: true,
        },
        byte_pattern_rows: None,
    });

    DetailSectionData {
        title: title.to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Custom,
        content: DetailContent::Composite(subsections),
        byte_pattern_rows: None,
    }
}

fn build_compu_internal_to_phys_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let compu_method = normal_dop.compu_method();
    let category = compu_method
        .as_ref()
        .map(|cm| format!("Category: {:?}", cm.category()));

    let rows = compu_method
        .and_then(|cm| cm.internal_to_phys())
        .and_then(|i2p| i2p.compu_scales())
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
                vec![
                    DetailCell::text(lower),
                    DetailCell::text(upper),
                    DetailCell::text(inverse),
                    DetailCell::text(consts),
                ],
                0,
            )
        })
        .collect();

    build_compu_direction_section("Compu-Internal-To-Phys", category, rows)
}

fn build_compu_phys_to_internal_section(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
) -> DetailSectionData {
    let (category, rows) = normal_dop.compu_method().map_or((None, vec![]), |cm| {
        let p2i = cm.phys_to_internal();
        let category = p2i
            .as_ref()
            .map(|_| format!("Category: {:?}", cm.category()));
        let rows = p2i
            .and_then(|p2i| p2i.compu_scales())
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
                    vec![
                        DetailCell::text(lower),
                        DetailCell::text(upper),
                        DetailCell::text(inverse),
                        DetailCell::text(consts),
                    ],
                    0,
                )
            })
            .collect();
        (category, rows)
    });

    build_compu_direction_section("Compu-Phys-To-Internal", category, rows)
}

/// Build tabbed sections for `NormalDOP` with Types, Constraints, and Compu tabs
pub(super) fn build_normal_dop_tabs(
    normal_dop: &cda_database::datatypes::NormalDop<'_>,
    parsed_name: &ParsedDopName,
    types_rows: &mut Vec<DetailRow>,
    sections: &mut Vec<DetailSectionData>,
) {
    if let Ok(coded_type) = normal_dop.diag_coded_type() {
        types_rows.push(DetailRow::normal(
            vec![
                DetailCell::text("Diag Coded Type"),
                DetailCell::text(format!("{:?}", coded_type.base_datatype())),
            ],
            0,
        ));

        if let Some(bit_len) = coded_type.bit_len() {
            types_rows.push(DetailRow::normal(
                vec![
                    DetailCell::text("Bit Length"),
                    DetailCell::new(bit_len.to_string(), CellType::NumericValue),
                ],
                0,
            ));
        }
    }

    if let Some(phys_type) = normal_dop.physical_type() {
        types_rows.push(DetailRow::normal(
            vec![
                DetailCell::text("Physical Type"),
                DetailCell::text(format!("{:?}", phys_type.base_data_type())),
            ],
            0,
        ));

        if let Some(precision) = phys_type.precision() {
            types_rows.push(DetailRow::normal(
                vec![
                    DetailCell::text("Precision"),
                    DetailCell::new(precision.to_string(), CellType::NumericValue),
                ],
                0,
            ));
        }

        types_rows.push(DetailRow::normal(
            vec![
                DetailCell::text("Display Radix"),
                DetailCell::text(format!("{:?}", phys_type.display_radix())),
            ],
            0,
        ));
    }

    if let Some(unit) = normal_dop.unit_ref() {
        if let Some(short_name) = unit.short_name() {
            types_rows.push(DetailRow::normal(
                vec![DetailCell::text("Unit"), DetailCell::text(short_name)],
                0,
            ));
        }
        if let Some(display_name) = unit.display_name() {
            types_rows.push(DetailRow::normal(
                vec![
                    DetailCell::text("Unit Display"),
                    DetailCell::text(display_name),
                ],
                0,
            ));
        }
    }

    if let Some(ref data_type) = parsed_name.data_type {
        types_rows.push(DetailRow::normal(
            vec![
                DetailCell::text("Data Type (from name)"),
                DetailCell::text(data_type.clone()),
            ],
            0,
        ));
    }

    push_types_section(std::mem::take(types_rows), sections);

    sections.push(build_constraints_section(normal_dop));
    sections.push(build_compu_internal_to_phys_section(normal_dop));
    sections.push(build_compu_phys_to_internal_section(normal_dop));
}
