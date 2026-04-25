/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellJumpTarget, CellJumpTargetType, CellType, ColumnConstraint, DetailCell, DetailContent,
        DetailRow, DetailSectionData, DetailSectionType, NodeType,
    },
};

fn make_index_jump(
    short_name: &str,
    node_indices: &std::collections::HashMap<String, usize>,
) -> CellJumpTarget {
    let index = node_indices.get(short_name).copied().unwrap_or(usize::MAX);
    CellJumpTarget::new(CellJumpTargetType::TreeNodeByIndex {
        index,
        short_name: short_name.to_owned(),
    })
}

/// Add state charts section to the tree
pub fn add_state_charts(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    let Some(charts) = layer.state_charts() else {
        return;
    };
    if charts.is_empty() {
        return;
    }

    // Push header first (empty details — patched below with indices).
    let header_idx = b.next_index();
    b.push_service_list_header(
        depth,
        format!("State Charts ({})", charts.len()),
        false,
        true,
        vec![],
        crate::tree::ServiceListType::StateCharts,
    );

    // Sort state charts alphabetically by name
    let mut sorted_charts: Vec<_> = charts.iter().collect();
    sorted_charts.sort_by_cached_key(|chart| chart.short_name().unwrap_or("").to_lowercase());

    // Push children, collecting (name → tree index).
    let mut node_indices = std::collections::HashMap::new();
    for chart in &sorted_charts {
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
                DetailRow::normal(
                    vec![
                        DetailCell::text(name),
                        DetailCell::text(src),
                        DetailCell::text(tgt),
                    ],
                    0,
                )
            })
            .collect();

        let state_rows: Vec<_> = chart
            .states()
            .into_iter()
            .flatten()
            .map(|state| {
                let sn = state.short_name().unwrap_or("?");
                DetailRow::normal(vec![DetailCell::text(sn)], 0)
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

        node_indices.insert(chart_name.to_owned(), b.next_index());
        b.push_details_structured(
            depth.saturating_add(1),
            chart_name.to_owned(),
            false,
            false,
            sections,
            NodeType::Default,
        );
    }

    // Collect chart summaries for the overview table.
    let chart_summaries: Vec<_> = sorted_charts
        .iter()
        .map(|chart| {
            let name = chart.short_name().unwrap_or("unnamed").to_owned();
            let states = chart.states().map_or(0, |s| s.len());
            let transitions = chart.state_transitions().map_or(0, |t| t.len());
            (name, states, transitions)
        })
        .collect();

    // Build overview with tree-node indices and patch header.
    let overview = build_state_charts_overview_table(&chart_summaries, &node_indices);
    b.set_detail_sections(header_idx, overview);
}

fn build_transitions_section(mut transitions: Vec<DetailRow>) -> DetailSectionData {
    transitions.sort_by_cached_key(|row| row.cells.first().map(|c| c.text.to_lowercase()));

    DetailSectionData {
        title: "State Transitions".to_string(),
        render_as_header: false,
        section_type: DetailSectionType::States,
        content: if transitions.is_empty() {
            DetailContent::PlainText(vec!["No state transitions".to_string()])
        } else {
            DetailContent::Table {
                header: DetailRow::header(vec![
                    DetailCell::text("Name"),
                    DetailCell::text("Source"),
                    DetailCell::text("Target"),
                ]),
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
    states.sort_by_cached_key(|row| row.cells.first().map(|c| c.text.to_lowercase()));

    DetailSectionData {
        title: "States".to_string(),
        render_as_header: false,
        section_type: DetailSectionType::States,
        content: if states.is_empty() {
            DetailContent::PlainText(vec!["No states".to_string()])
        } else {
            DetailContent::Table {
                header: DetailRow::header(vec![DetailCell::text("Name")]),
                rows: states,
                constraints: vec![ColumnConstraint::Percentage(100)],
                use_row_selection: false,
            }
        },
    }
}

/// Build an overview table listing all state chart short names for the section header
fn build_state_charts_overview_table(
    chart_summaries: &[(String, usize, usize)],
    node_indices: &std::collections::HashMap<String, usize>,
) -> Vec<DetailSectionData> {
    let header = DetailRow::header(vec![
        DetailCell::text("Name"),
        DetailCell::text("States"),
        DetailCell::text("Transitions"),
    ]);

    let rows: Vec<DetailRow> = chart_summaries
        .iter()
        .map(|(name, state_count, transition_count)| {
            let jump = make_index_jump(name, node_indices);
            DetailRow::normal(
                vec![
                    DetailCell::new(name.clone(), CellType::ParameterName).with_jump(jump),
                    DetailCell::new(state_count.to_string(), CellType::NumericValue),
                    DetailCell::new(transition_count.to_string(), CellType::NumericValue),
                ],
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
