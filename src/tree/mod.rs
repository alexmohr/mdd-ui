/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod builder;
mod elements;
mod types;

use builder::TreeBuilder;
use cda_database::datatypes::DiagnosticDatabase;
use elements::{add_ecu_shared_data, add_functional_groups, add_protocols, add_variants};
// Re-export public types
pub use types::{
    CellJumpTarget, CellType, ChildElementType, ColumnConstraint, DetailContent, DetailRow,
    DetailRowType, DetailSectionData, DetailSectionType, NodeTextPrefix, NodeType, RowMetadata,
    SectionType, ServiceListType, TreeNode, lines_to_single_section,
};

use crate::database::{extract_data, get_ecu_summary};

/// Walk the entire database and produce a flat list of tree nodes ready for
/// the TUI to display, together with the ECU name.
pub fn build_tree(db: &DiagnosticDatabase, file_path: &str) -> (Vec<TreeNode>, String) {
    let mut b = TreeBuilder::new();

    // Extract database data
    let data = extract_data(db);
    let ecu_name = data.ecu_name.clone();

    // Add General section with ECU info
    if let Some(ref ecu) = data.ecu {
        let ecu_details = get_ecu_summary(db, &data.ecu_name, file_path);
        let ecu_section = lines_to_single_section("Summary", ecu_details);
        b.push_section_header(
            "General".to_string(),
            false,
            false,
            vec![ecu_section],
            SectionType::General,
        );

        add_variants(&mut b, ecu);
        add_functional_groups(&mut b, ecu);
        add_ecu_shared_data(&mut b, ecu);
        add_protocols(&mut b, ecu);
    }

    (b.finish(), ecu_name)
}
