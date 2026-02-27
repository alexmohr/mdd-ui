/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use cda_database::datatypes::DiagLayer;

use crate::tree::{builder::TreeBuilder, types::NodeType};

/// Add placeholder sections that are not fully implemented yet.
/// These are kept for structure but may be expanded in the future.
pub fn add_additional_audiences(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) {
    // Check if layer has additional audiences
    if let Some(additional_audiences) = layer.additional_audiences() {
        if additional_audiences.is_empty() {
            return;
        }

        // Build table with additional audiences
        let mut rows = Vec::new();

        for audience in additional_audiences {
            let short_name = audience.short_name().unwrap_or("?").to_owned();
            let long_name = audience
                .long_name()
                .and_then(|ln| ln.value())
                .unwrap_or("")
                .to_owned();

            rows.push(crate::tree::types::DetailRow::normal(
                vec![short_name, long_name],
                vec![
                    crate::tree::types::CellType::Text,
                    crate::tree::types::CellType::Text,
                ],
                0,
            ));
        }

        let header = crate::tree::types::DetailRow::header(
            vec!["Short Name".to_owned(), "Long Name".to_owned()],
            vec![
                crate::tree::types::CellType::Text,
                crate::tree::types::CellType::Text,
            ],
        );

        let section = crate::tree::types::DetailSectionData::new(
            "Additional Audiences".to_owned(),
            crate::tree::DetailContent::Table {
                header,
                rows,
                constraints: vec![
                    crate::tree::types::ColumnConstraint::Percentage(40),
                    crate::tree::types::ColumnConstraint::Percentage(60),
                ],
                use_row_selection: true,
            },
            false,
        );

        b.push_details_structured(
            depth,
            format!("Additional Audiences ({})", additional_audiences.len()),
            false,
            false,
            vec![section],
            NodeType::Default,
        );
    }
}

// Sub-Components is not supported and has been removed
// pub fn add_sub_components(b: &mut TreeBuilder, layer: &DiagLayer<'_>, depth: usize) { ... }

// SDGs are now implemented in the sdgs module
// This re-export maintains backward compatibility
pub use super::sdgs::add_sdgs;
