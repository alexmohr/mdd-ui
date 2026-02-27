/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::ParentRef;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DetailSectionType,
        NodeType,
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

    let mut tables: Vec<TableData> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for pr in parent_refs {
        let Some(table_dop) = pr.ref__as_table_dop() else {
            continue;
        };

        let name = table_dop.short_name().unwrap_or("?").to_owned();
        if !seen.insert(name.clone()) {
            continue;
        }

        let semantic = table_dop.semantic().unwrap_or("-").to_owned();
        let key_label = table_dop.key_label().unwrap_or("-").to_owned();
        let struct_label = table_dop.struct_label().unwrap_or("-").to_owned();
        let key_dop_name = table_dop
            .key_dop()
            .and_then(|d| d.short_name())
            .unwrap_or("-")
            .to_owned();
        let has_key_dop = key_dop_name != "-";
        let row_count = table_dop.rows().map_or(0, |r| r.len());

        let mut rows_data = Vec::new();
        if let Some(rows) = table_dop.rows() {
            for row in rows {
                let row_name = row.short_name().unwrap_or("?").to_owned();
                let row_key = row.key().unwrap_or("-").to_owned();
                let row_struct = row
                    .structure()
                    .and_then(|s| s.short_name())
                    .unwrap_or("-")
                    .to_owned();

                rows_data.push(TableRowData {
                    short_name: row_name,
                    key: row_key,
                    structure: row_struct,
                });
            }
        }

        tables.push(TableData {
            short_name: name,
            semantic,
            key_label,
            struct_label,
            key_dop_name,
            has_key_dop,
            row_count,
            rows: rows_data,
        });
    }

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
    key_dop_name: String,
    has_key_dop: bool,
    row_count: usize,
    rows: Vec<TableRowData>,
}

struct TableRowData {
    short_name: String,
    key: String,
    structure: String,
}

fn build_tables_overview(tables: &[TableData]) -> DetailSectionData {
    let header = DetailRow::header(vec!["Short Name".to_owned()], vec![CellType::Text]);

    let rows: Vec<DetailRow> = tables
        .iter()
        .map(|t| DetailRow::normal(vec![t.short_name.clone()], vec![CellType::Text], 0))
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
    }
}

fn build_table_detail(table: &TableData) -> Vec<DetailSectionData> {
    let mut sections = Vec::new();

    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );

    let mut overview_rows = vec![
        DetailRow::normal(
            vec!["Short Name".to_owned(), table.short_name.clone()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Semantic".to_owned(), table.semantic.clone()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Key Label".to_owned(), table.key_label.clone()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Struct Label".to_owned(), table.struct_label.clone()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
    ];

    let key_cell_type = if table.has_key_dop {
        CellType::DopReference
    } else {
        CellType::Text
    };
    overview_rows.push(DetailRow::normal(
        vec!["Key DOP".to_owned(), table.key_dop_name.clone()],
        vec![CellType::Text, key_cell_type],
        0,
    ));
    overview_rows.push(DetailRow::normal(
        vec!["Row Count".to_owned(), table.row_count.to_string()],
        vec![CellType::Text, CellType::NumericValue],
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
        let rows_header = DetailRow::header(
            vec![
                "Table Row".to_owned(),
                "Key".to_owned(),
                "Struct Ref".to_owned(),
            ],
            vec![CellType::Text, CellType::Text, CellType::DopReference],
        );

        let rows: Vec<DetailRow> = table
            .rows
            .iter()
            .map(|r| {
                let has_struct = r.structure != "-";
                DetailRow::normal(
                    vec![r.short_name.clone(), r.key.clone(), r.structure.clone()],
                    vec![
                        CellType::Text,
                        CellType::Text,
                        if has_struct {
                            CellType::DopReference
                        } else {
                            CellType::Text
                        },
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
        });
    }

    sections
}
