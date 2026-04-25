/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Add Unit Spec section to the tree by collecting
/// units from `ComParamRef` -> `ProtStack` -> `ComParamSubSet`
pub fn add_unit_spec(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    let Some(cp_refs) = layer.com_param_refs() else {
        return;
    };

    let mut units: Vec<UnitData> = Vec::new();
    let mut unit_groups: Vec<UnitGroupData> = Vec::new();
    let mut seen_units = std::collections::HashSet::new();
    let mut seen_groups = std::collections::HashSet::new();

    for cpr in cp_refs {
        let Some(prot_stack) = cpr.prot_stack() else {
            continue;
        };
        let Some(subsets) = prot_stack.comparam_subset_refs() else {
            continue;
        };

        for subset in subsets {
            let Some(unit_spec) = subset.unit_spec() else {
                continue;
            };

            if let Some(unit_list) = unit_spec.units() {
                for unit in unit_list {
                    let name = unit.short_name().unwrap_or("?").to_owned();
                    if !seen_units.insert(name.clone()) {
                        continue;
                    }
                    let display_name = unit.display_name().unwrap_or("-").to_owned();
                    let factor = unit
                        .factorsitounit()
                        .map(|f| format!("{f}"))
                        .unwrap_or_default();
                    let offset = unit
                        .offsetitounit()
                        .map(|o| format!("{o}"))
                        .unwrap_or_default();

                    units.push(UnitData {
                        short_name: name,
                        display_name,
                        factor,
                        offset,
                    });
                }
            }

            if let Some(group_list) = unit_spec.unit_groups() {
                for group in group_list {
                    let name = group.short_name().unwrap_or("?").to_owned();
                    if !seen_groups.insert(name.clone()) {
                        continue;
                    }
                    let unit_count = group.unitrefs().map_or(0, |refs| refs.len());

                    unit_groups.push(UnitGroupData {
                        short_name: name,
                        unit_count,
                    });
                }
            }
        }
    }

    if units.is_empty() && unit_groups.is_empty() {
        return;
    }

    let overview = build_unit_spec_overview(&units, &unit_groups);

    b.push_details_structured(
        depth,
        format!(
            "Unit Spec ({} units, {} groups)",
            units.len(),
            unit_groups.len()
        ),
        false,
        true,
        overview,
        NodeType::SectionHeader,
    );

    for group in &unit_groups {
        let detail = build_unit_group_detail(group);
        b.push_details_structured(
            depth.saturating_add(1),
            group.short_name.clone(),
            false,
            false,
            detail,
            NodeType::Default,
        );
    }

    for unit in &units {
        let detail = build_unit_detail(unit);
        b.push_details_structured(
            depth.saturating_add(1),
            unit.short_name.clone(),
            false,
            false,
            detail,
            NodeType::Default,
        );
    }
}

struct UnitData {
    short_name: String,
    display_name: String,
    factor: String,
    offset: String,
}

struct UnitGroupData {
    short_name: String,
    unit_count: usize,
}

fn build_unit_spec_overview(
    units: &[UnitData],
    unit_groups: &[UnitGroupData],
) -> Vec<DetailSectionData> {
    let units_header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::text("Display"),
        DetailCell::new("Factor", CellType::NumericValue),
        DetailCell::new("Offset", CellType::NumericValue),
    ]);

    let unit_rows: Vec<DetailRow> = units
        .iter()
        .map(|u| {
            DetailRow::normal(
                vec![
                    DetailCell::text(u.short_name.clone()),
                    DetailCell::text(u.display_name.clone()),
                    DetailCell::new(u.factor.clone(), CellType::NumericValue),
                    DetailCell::new(u.offset.clone(), CellType::NumericValue),
                ],
                0,
            )
        })
        .collect();

    let mut sections = vec![DetailSectionData {
        title: format!("Units ({})", units.len()),
        render_as_header: false,
        section_type: DetailSectionType::Overview,
        content: DetailContent::Table {
            header: units_header,
            rows: unit_rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(20),
                ColumnConstraint::Percentage(20),
            ],
            use_row_selection: true,
        },
    }];

    if !unit_groups.is_empty() {
        let groups_header = DetailRow::header(vec![
            DetailCell::text("Short Name"),
            DetailCell::new("Unit Count", CellType::NumericValue),
        ]);

        let group_rows: Vec<DetailRow> = unit_groups
            .iter()
            .map(|g| {
                DetailRow::normal(
                    vec![
                        DetailCell::text(g.short_name.clone()),
                        DetailCell::new(g.unit_count.to_string(), CellType::NumericValue),
                    ],
                    0,
                )
            })
            .collect();

        sections.push(DetailSectionData {
            title: format!("Unit Groups ({})", unit_groups.len()),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::Table {
                header: groups_header,
                rows: group_rows,
                constraints: vec![
                    ColumnConstraint::Percentage(70),
                    ColumnConstraint::Percentage(30),
                ],
                use_row_selection: true,
            },
        });
    }

    sections
}

fn build_unit_detail(unit: &UnitData) -> Vec<DetailSectionData> {
    let header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Value"),
    ]);

    let rows = vec![
        DetailRow::normal(
            vec![
                DetailCell::text("Short Name"),
                DetailCell::text(unit.short_name.clone()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Display Name"),
                DetailCell::text(unit.display_name.clone()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Factor (SI→Unit)"),
                DetailCell::new(unit.factor.clone(), CellType::NumericValue),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Offset (SI→Unit)"),
                DetailCell::new(unit.offset.clone(), CellType::NumericValue),
            ],
            0,
        ),
    ];

    vec![
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}

fn build_unit_group_detail(group: &UnitGroupData) -> Vec<DetailSectionData> {
    let header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Value"),
    ]);

    let rows = vec![
        DetailRow::normal(
            vec![
                DetailCell::text("Short Name"),
                DetailCell::text(group.short_name.clone()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Unit Count"),
                DetailCell::new(group.unit_count.to_string(), CellType::NumericValue),
            ],
            0,
        ),
    ];

    vec![
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: false,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}
