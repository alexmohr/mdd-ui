/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType,
        DetailSectionData, DetailSectionType, NodeType,
    },
};

/// Add state charts section to the tree
pub fn add_state_charts(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    let Some(charts) = layer.state_charts() else {
        return;
    };
    if charts.is_empty() {
        return;
    }

    // Build overview table for the section header
    let overview = build_state_charts_overview_table(layer);

    b.push_details_structured(
        depth,
        format!("State Charts ({})", charts.len()),
        false,
        true,
        overview,
        NodeType::SectionHeader,
    );

    // Sort state charts alphabetically by name
    let mut sorted_charts: Vec<_> = charts.iter().collect();
    sorted_charts.sort_by_cached_key(|chart| chart.short_name().unwrap_or("").to_lowercase());

    for chart in sorted_charts {
        let chart_name = chart.short_name().unwrap_or("unnamed");
        let semantic = chart.semantic().unwrap_or("");

        let transition_rows: Vec<_> = chart
            .state_transitions()
            .into_iter()
            .flatten()
            .map(|tr| {
                let name = tr.short_name().unwrap_or("?");
                let src = tr.source_short_name_ref().unwrap_or("?");
                let tgt = tr.target_short_name_ref().unwrap_or("?");
                DetailRow {
                    row_type: DetailRowType::Normal,
                    metadata: None,
                    cells: vec![name.to_string(), src.to_string(), tgt.to_string()],
                    cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                    cell_jump_targets: vec![None; 3],
                    indent: 0,
                }
            })
            .collect();

        let state_rows: Vec<_> = chart
            .states()
            .into_iter()
            .flatten()
            .map(|state| {
                let sn = state.short_name().unwrap_or("?");
                DetailRow {
                    row_type: DetailRowType::Normal,
                    metadata: None,
                    cells: vec![sn.to_string()],
                    cell_types: vec![CellType::Text],
                    cell_jump_targets: vec![None; 1],
                    indent: 0,
                }
            })
            .collect();

        let sections = vec![
            DetailSectionData {
                title: format!("State Chart - {chart_name}"),
                render_as_header: true,
                section_type: DetailSectionType::Header,
                content: DetailContent::PlainText(vec![format!("Semantic: {semantic}")]),
            },
            build_transitions_section(transition_rows),
            build_states_section(state_rows),
        ];

        b.push_details_structured(
            depth.saturating_add(1),
            chart_name.to_owned(),
            false,
            false,
            sections,
            NodeType::Default,
        );
    }
}

fn build_transitions_section(mut transitions: Vec<DetailRow>) -> DetailSectionData {
    transitions.sort_by(|a, b| {
        a.cells
            .first()
            .map(|s| s.to_lowercase())
            .cmp(&b.cells.first().map(|s| s.to_lowercase()))
    });

    DetailSectionData {
        title: "State Transitions".to_string(),
        render_as_header: false,
        section_type: DetailSectionType::States,
        content: if transitions.is_empty() {
            DetailContent::PlainText(vec!["No state transitions".to_string()])
        } else {
            DetailContent::Table {
                header: DetailRow {
                    row_type: DetailRowType::Normal,
                    metadata: None,
                    cells: vec![
                        "Name".to_string(),
                        "Source".to_string(),
                        "Target".to_string(),
                    ],
                    cell_types: vec![CellType::Text, CellType::Text, CellType::Text],
                    cell_jump_targets: vec![None; 3],
                    indent: 0,
                },
                rows: transitions,
                constraints: vec![
                    ColumnConstraint::Percentage(34),
                    ColumnConstraint::Percentage(33),
                    ColumnConstraint::Percentage(33),
                ],
                use_row_selection: false,
            }
        },
    }
}

fn build_states_section(mut states: Vec<DetailRow>) -> DetailSectionData {
    states.sort_by(|a, b| {
        a.cells
            .first()
            .map(|s| s.to_lowercase())
            .cmp(&b.cells.first().map(|s| s.to_lowercase()))
    });

    DetailSectionData {
        title: "States".to_string(),
        render_as_header: false,
        section_type: DetailSectionType::States,
        content: if states.is_empty() {
            DetailContent::PlainText(vec!["No states".to_string()])
        } else {
            DetailContent::Table {
                header: DetailRow {
                    row_type: DetailRowType::Normal,
                    metadata: None,
                    cells: vec!["Name".to_string()],
                    cell_types: vec![CellType::Text],
                    cell_jump_targets: vec![None; 1],
                    indent: 0,
                },
                rows: states,
                constraints: vec![ColumnConstraint::Percentage(100)],
                use_row_selection: false,
            }
        },
    }
}

/// Build an overview table listing all state chart short names for the section header
fn build_state_charts_overview_table(layer: &DiagLayer<'_>) -> Vec<DetailSectionData> {
    let Some(charts) = layer.state_charts() else {
        return vec![];
    };

    let header = DetailRow::header(
        vec![
            "Name".to_owned(),
            "States".to_owned(),
            "Transitions".to_owned(),
        ],
        vec![CellType::Text, CellType::Text, CellType::Text],
    );

    let mut sorted_charts: Vec<_> = charts.iter().collect();
    sorted_charts.sort_by_cached_key(|chart| chart.short_name().unwrap_or("").to_lowercase());

    let rows: Vec<DetailRow> = sorted_charts
        .iter()
        .map(|chart| {
            let name = chart.short_name().unwrap_or("unnamed").to_owned();
            let state_count = chart.states().map_or(0, |s| s.len());
            let transition_count = chart.state_transitions().map_or(0, |t| t.len());
            DetailRow::with_jump_targets(
                vec![name, state_count.to_string(), transition_count.to_string()],
                vec![
                    CellType::ParameterName,
                    CellType::NumericValue,
                    CellType::NumericValue,
                ],
                vec![Some(CellJumpTarget::TreeNodeByName), None, None],
                0,
            )
        })
        .collect();

    vec![
        DetailSectionData::new(
            "State Charts Overview".to_owned(),
            DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    ColumnConstraint::Percentage(60),
                    ColumnConstraint::Percentage(20),
                    ColumnConstraint::Percentage(20),
                ],
                use_row_selection: true,
            },
            false,
        )
        .with_type(DetailSectionType::Overview),
    ]
}
