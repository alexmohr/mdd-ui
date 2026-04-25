/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{
    DetailContent,
    builder::TreeBuilder,
    types::{ColumnConstraint, DetailCell, DetailRow, DetailSectionData, NodeType},
};

/// Add additional audiences section to the tree with individual child nodes.
pub fn add_additional_audiences(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    let Some(additional_audiences) = layer.additional_audiences() else {
        return;
    };
    if additional_audiences.is_empty() {
        return;
    }

    let overview_rows: Vec<_> = additional_audiences
        .iter()
        .map(|audience| {
            let short_name = audience.short_name().unwrap_or("?").to_owned();
            let long_name = audience
                .long_name()
                .and_then(|ln| ln.value())
                .unwrap_or("")
                .to_owned();
            DetailRow::normal(
                vec![DetailCell::text(short_name), DetailCell::text(long_name)],
                0,
            )
        })
        .collect();

    let overview_header = DetailRow::header(vec![
        DetailCell::text("Short Name"),
        DetailCell::text("Long Name"),
    ]);

    let overview_section = DetailSectionData::new(
        "Additional Audiences".to_owned(),
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
    );

    b.push_details_structured(
        depth,
        format!("Additional Audiences ({})", additional_audiences.len()),
        false,
        true,
        vec![overview_section],
        NodeType::Default,
    );

    for audience in additional_audiences {
        let short_name = audience.short_name().unwrap_or("?").to_owned();
        let long_name = audience
            .long_name()
            .and_then(|ln| ln.value())
            .unwrap_or("")
            .to_owned();
        let display_name = format!("[Audience] {short_name}");

        let detail_section = DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::Table {
                header: DetailRow::header(vec![
                    DetailCell::text("Property"),
                    DetailCell::text("Value"),
                ]),
                rows: vec![
                    DetailRow::normal(
                        vec![DetailCell::text("Short Name"), DetailCell::text(short_name)],
                        0,
                    ),
                    DetailRow::normal(
                        vec![DetailCell::text("Long Name"), DetailCell::text(long_name)],
                        0,
                    ),
                ],
                constraints: vec![
                    ColumnConstraint::Percentage(40),
                    ColumnConstraint::Percentage(60),
                ],
                use_row_selection: false,
            },
            false,
        );

        b.push_details_structured(
            depth.saturating_add(1),
            display_name,
            false,
            false,
            vec![detail_section],
            NodeType::Default,
        );
    }
}

// Sub-Components is not supported and has been removed
// pub fn add_sub_components(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) { ... }

// SDGs are now implemented in the sdgs module
// This re-export maintains backward compatibility
pub use super::sdgs::add_sdgs;
