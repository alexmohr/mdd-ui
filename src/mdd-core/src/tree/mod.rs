/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod builder;
mod elements;
mod types;

use std::sync::Arc;

use builder::TreeBuilder;
use cda_database::datatypes::DiagnosticDatabase;
use elements::{add_ecu_shared_data, add_functional_groups, add_protocols, add_variants};
// Re-export public types
pub(crate) use types::BIT_POSITION_UNSET;
pub use types::{
    CellJumpTarget, CellJumpTargetType, CellType, ChildElementType, ColumnConstraint, DetailCell,
    DetailContent, DetailRow, DetailRowType, DetailSectionData, DetailSectionType, DiffStatus,
    NodePayload, NodeTextPrefix, NodeType, RowMetadata, SectionType, ServiceListType, TreeNode,
    lines_to_single_section, param_type_label,
};

use crate::database::{extract_data, get_ecu_summary};

/// Rebuild all stored tree indices from canonical names.
///
/// Must be called after any operation that rearranges nodes in `all_nodes`
/// (e.g. sorting). Re-resolves:
/// - `parent_ref_indices` on Container nodes
/// - Every `TreeNodeByIndex` index in jump targets
pub fn resolve_all_indices(nodes: &mut [TreeNode]) {
    // 1. Build name → index maps (owned keys to avoid borrow conflicts).
    let mut name_to_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut container_map: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for (i, n) in nodes.iter().enumerate() {
        if let Some(sn) = n.service_short_name() {
            name_to_idx.entry(sn.to_owned()).or_insert(i);
        }
        if let Some(sn) = n.short_name() {
            name_to_idx.entry(sn.to_owned()).or_insert(i);
            container_map.insert(sn.to_owned(), i);
        }
        name_to_idx.entry(n.text.clone()).or_insert(i);
    }

    // 2. Resolve parent_ref_indices on Container nodes.
    for node in nodes.iter_mut() {
        if let types::NodePayload::Container {
            parent_ref_names,
            parent_ref_indices,
            ..
        } = &mut node.payload
        {
            *parent_ref_indices = parent_ref_names
                .iter()
                .filter_map(|name| container_map.get(name).copied())
                .collect();
        }
    }

    // 3. Resolve all jump-target indices.
    for node in nodes.iter_mut() {
        let sections = Arc::make_mut(&mut node.detail_sections);
        resolve_sections(&name_to_idx, &container_map, sections);
    }

    // 4. Resolve ChildElement rows: link "Diag-Comms", "Functional Classes",
    //    etc. in variant overview tables to their corresponding tree nodes.
    resolve_child_element_rows(nodes);
}

/// Recursively resolve jump-target indices in a slice of sections.
fn resolve_sections(
    name_to_idx: &std::collections::HashMap<String, usize>,
    container_map: &std::collections::HashMap<String, usize>,
    sections: &mut [DetailSectionData],
) {
    for section in sections.iter_mut() {
        match &mut section.content {
            DetailContent::Table { rows, .. } => rows
                .iter_mut()
                .flat_map(|row| &mut row.cells)
                .filter_map(|cell| cell.jump_target.as_mut())
                .for_each(|jt| match &mut jt.target_type {
                    CellJumpTargetType::TreeNodeByIndex { index, short_name }
                    | CellJumpTargetType::Dop {
                        index,
                        name: short_name,
                    } => {
                        if let Some(&real) = name_to_idx.get(short_name.as_str()) {
                            *index = real;
                        }
                    }
                    CellJumpTargetType::Container { index, short_name } => {
                        if let Some(&real) = container_map.get(short_name.as_str()) {
                            *index = real;
                        }
                    }
                    CellJumpTargetType::Parameter { .. } => {}
                }),
            DetailContent::Composite(subs) => resolve_sections(name_to_idx, container_map, subs),
            DetailContent::PlainText(_) => {}
        }
    }
}

/// For each node, find `ChildElement` metadata rows in its detail sections and
/// set jump targets pointing to the matching `ServiceListHeader` child node.
fn resolve_child_element_rows(nodes: &mut [TreeNode]) {
    use std::collections::HashMap;

    use types::RowMetadata;

    // Build: parent_index → [(service_list_type, child_index, child_text)]
    let mut parent_children: HashMap<usize, Vec<(ServiceListType, usize, String)>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let Some(slt) = node.service_list_type() else {
            continue;
        };
        let Some(parent) = node.parent_idx else {
            continue;
        };
        parent_children
            .entry(parent)
            .or_default()
            .push((slt, i, node.text.clone()));
    }

    for node_idx in 0..nodes.len() {
        let Some(children) = parent_children.get(&node_idx) else {
            continue;
        };
        let children = children.clone();

        let Some(node) = nodes.get_mut(node_idx) else {
            continue;
        };
        let sections = Arc::make_mut(&mut node.detail_sections);
        for section in sections.iter_mut() {
            let DetailContent::Table { rows, .. } = &mut section.content else {
                continue;
            };
            for row in rows.iter_mut() {
                let Some(RowMetadata::ChildElement { element_type }) = &row.metadata else {
                    continue;
                };
                let target_slt = element_type.to_service_list_type();
                let Some((_, child_idx, child_text)) =
                    children.iter().find(|(slt, _, _)| *slt == target_slt)
                else {
                    continue;
                };
                if let Some(cell) = row.cells.first_mut() {
                    cell.jump_target =
                        Some(CellJumpTarget::new(CellJumpTargetType::TreeNodeByIndex {
                            index: *child_idx,
                            short_name: child_text.clone(),
                        }));
                }
            }
        }
    }
}

/// Walk the entire database and produce a flat list of tree nodes ready for
/// the TUI to display, together with the ECU name.
#[must_use]
pub fn build_tree(db: &DiagnosticDatabase, file_path: &str) -> (Vec<TreeNode>, String) {
    // Extract database data
    let data = extract_data(db);
    let ecu_name = data.ecu_name.clone();
    let mut b = TreeBuilder::new();

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
