// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    builder::TreeBuilder,
    types::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailRowType, DetailSectionData,
        DetailSectionType, NodeType,
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

    b.push(
        depth,
        format!("State Charts ({})", charts.len()),
        false,
        true,
        NodeType::SectionHeader,
    );

    // Sort state charts alphabetically by name
    let mut sorted_charts: Vec<_> = charts.iter().collect();
    sorted_charts.sort_by_cached_key(|chart| chart.short_name().unwrap_or("").to_lowercase());

    for chart in sorted_charts {
        let chart_name = chart.short_name().unwrap_or("unnamed");

        // Get semantic description if available
        let semantic = chart.semantic().unwrap_or("");

        // Build State Transitions table
        let mut transitions: Vec<_> = chart
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
                    indent: 0,
                }
            })
            .collect();

        // Sort transitions alphabetically by name
        transitions.sort_by(|a, b| {
            a.cells
                .first()
                .map(|s| s.to_lowercase())
                .cmp(&b.cells.first().map(|s| s.to_lowercase()))
        });

        let transitions_section = DetailSectionData {
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
        };

        // Build States table
        let mut states: Vec<_> = chart
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
                    indent: 0,
                }
            })
            .collect();

        // Sort states alphabetically by name
        states.sort_by(|a, b| {
            a.cells
                .first()
                .map(|s| s.to_lowercase())
                .cmp(&b.cells.first().map(|s| s.to_lowercase()))
        });

        let states_section = DetailSectionData {
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
                        indent: 0,
                    },
                    rows: states,
                    constraints: vec![ColumnConstraint::Percentage(100)],
                    use_row_selection: false,
                }
            },
        };

        // Create sections list with semantic header (always, even if empty)
        let mut sections = vec![];

        // Add semantic information as first section (not a tab)
        sections.push(DetailSectionData {
            title: format!("State Chart - {chart_name}"),
            render_as_header: true,
            section_type: DetailSectionType::Header,
            content: DetailContent::PlainText(vec![format!("Semantic: {semantic}")]),
        });

        // Add the two tab sections
        sections.push(transitions_section);
        sections.push(states_section);

        // Push the state chart as a non-expandable node with detail sections
        b.push_details_structured(
            depth.saturating_add(1),
            chart_name.to_owned(),
            false,
            false, // Not expandable - no tree children
            sections,
            NodeType::Default,
        );
    }
}
