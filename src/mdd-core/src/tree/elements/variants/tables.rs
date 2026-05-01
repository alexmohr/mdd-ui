/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::ParentRef;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailCell, DetailContent, DetailRow, DetailSectionData,
        DetailSectionType, NodeType,
    },
};

/// Add Tables section by collecting `TableDops` from parent refs
pub fn add_tables<'a>(
    b: &mut TreeBuilder,
    depth: usize,
    variant_parent_refs: Option<impl Iterator<Item = ParentRef<'a>> + 'a>,
) {
    let Some(parent_refs) = variant_parent_refs else {
        return;
    };

    let mut seen = std::collections::HashSet::new();

    let tables: Vec<TableData> = parent_refs
        .filter_map(|pr| {
            let table_dop = pr.ref__as_table_dop()?;
            let name = table_dop.short_name().unwrap_or("?").to_owned();
            if !seen.insert(name.clone()) {
                return None;
            }

            let semantic = table_dop.semantic().unwrap_or("-").to_owned();
            let key_label = table_dop.key_label().unwrap_or("-").to_owned();
            let struct_label = table_dop.struct_label().unwrap_or("-").to_owned();
            let key_dop_name = table_dop
                .key_dop()
                .and_then(|d| d.short_name())
                .map(str::to_owned);
            let row_count = table_dop.rows().map_or(0, |r| r.len());

            let rows: Vec<TableRowData> = table_dop
                .rows()
                .into_iter()
                .flatten()
                .map(|row| TableRowData {
                    short_name: row.short_name().unwrap_or("?").to_owned(),
                    key: row.key().unwrap_or("-").to_owned(),
                    structure: row
                        .structure()
                        .and_then(|s| s.short_name())
                        .map(str::to_owned),
                })
                .collect();

            Some(TableData {
                short_name: name,
                semantic,
                key_label,
                struct_label,
                key_dop_name,
                row_count,
                rows,
            })
        })
        .collect();

    if tables.is_empty() {
        return;
    }

    let overview = build_tables_overview(&tables);

    b.push_details_structured(
        depth,
        format!("Tables ({})", tables.len()),
        false,
        true,
        vec![overview],
        NodeType::SectionHeader,
    );

    for table in &tables {
        let detail = build_table_detail(table);
        b.push_details_structured(
            depth.saturating_add(1),
            table.short_name.clone(),
            false,
            false,
            detail,
            NodeType::Default,
        );
    }
}

struct TableData {
    short_name: String,
    semantic: String,
    key_label: String,
    struct_label: String,
    key_dop_name: Option<String>,
    row_count: usize,
    rows: Vec<TableRowData>,
}

struct TableRowData {
    short_name: String,
    key: String,
    structure: Option<String>,
}

fn build_tables_overview(tables: &[TableData]) -> DetailSectionData {
    let header = DetailRow::header(vec![DetailCell::text("Short Name")]);

    let rows: Vec<DetailRow> = tables
        .iter()
        .map(|t| DetailRow::normal(vec![DetailCell::text(t.short_name.clone())], 0))
        .collect();

    DetailSectionData {
        title: "Overview".to_owned(),
        render_as_header: false,
        section_type: DetailSectionType::Overview,
        content: DetailContent::Table {
            header,
            rows,
            constraints: vec![ColumnConstraint::Percentage(100)],
            use_row_selection: true,
        },
        byte_pattern_rows: None,
    }
}

fn build_table_detail(table: &TableData) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    let header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Value"),
    ]);

    let mut overview_rows = vec![
        DetailRow::normal(
            vec![
                DetailCell::text("Short Name"),
                DetailCell::text(table.short_name.clone()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Semantic"),
                DetailCell::text(table.semantic.clone()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Key Label"),
                DetailCell::text(table.key_label.clone()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Struct Label"),
                DetailCell::text(table.struct_label.clone()),
            ],
            0,
        ),
    ];

    let key_cell_type = if table.key_dop_name.is_some() {
        CellType::DopReference
    } else {
        CellType::Text
    };
    overview_rows.push(DetailRow::normal(
        vec![
            DetailCell::text("Key DOP"),
            DetailCell::new(
                table.key_dop_name.clone().unwrap_or_else(|| "-".to_owned()),
                key_cell_type,
            ),
        ],
        0,
    ));
    overview_rows.push(DetailRow::normal(
        vec![
            DetailCell::text("Row Count"),
            DetailCell::new(table.row_count.to_string(), CellType::NumericValue),
        ],
        0,
    ));

    sections.push(
        DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header,
                rows: overview_rows,
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

    if !table.rows.is_empty() {
        let rows_header = DetailRow::header(vec![
            DetailCell::text("Table Row"),
            DetailCell::text("Key"),
            DetailCell::new("Struct Ref", CellType::DopReference),
        ]);

        let rows: Vec<DetailRow> = table
            .rows
            .iter()
            .map(|r| {
                let struct_type = if r.structure.is_some() {
                    CellType::DopReference
                } else {
                    CellType::Text
                };
                DetailRow::normal(
                    vec![
                        DetailCell::text(r.short_name.clone()),
                        DetailCell::text(r.key.clone()),
                        DetailCell::new(
                            r.structure.clone().unwrap_or_else(|| "-".to_owned()),
                            struct_type,
                        ),
                    ],
                    0,
                )
            })
            .collect();

        sections.push(DetailSectionData {
            title: format!("Rows ({})", table.rows.len()),
            render_as_header: false,
            section_type: DetailSectionType::Custom,
            content: DetailContent::Table {
                header: rows_header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(30),
                    ColumnConstraint::Percentage(30),
                ],
                use_row_selection: false,
            },
            byte_pattern_rows: None,
        });
    }

    sections
}
