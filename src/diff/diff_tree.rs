// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

//! Converts a [`DiffResult`] into a flat `Vec<TreeNode>` for the TUI tree view.
//!
//! The resulting tree mirrors the structure produced by `tree::build_tree` for
//! browse mode, but every node carries a [`DiffStatus`] annotation so the
//! renderer can colour-code additions, removals, modifications, and unchanged
//! elements.

use std::rc::Rc;

use crate::{
    diff::compare::{DiffResult, ElementDiff, PropertyDiff},
    tree::{
        CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DiffStatus,
        NodeType, SectionType, TreeNode,
    },
};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Build a flat tree-node list from a [`DiffResult`].
///
/// Returns `(nodes, ecu_label)` where `ecu_label` is formatted as
/// `"old_name vs new_name"` for display in the title bar.
pub fn build_diff_tree(diff: &DiffResult) -> (Vec<TreeNode>, String) {
    let mut nodes = Vec::new();

    // -- General section (depth 0) ------------------------------------------
    let general_status = if diff.ecu_diffs.is_empty() {
        None
    } else {
        Some(DiffStatus::Modified)
    };

    let general_sections = if diff.ecu_diffs.is_empty() {
        Vec::new()
    } else {
        vec![build_property_diff_section(
            "ECU Properties",
            &diff.ecu_diffs,
        )]
    };

    nodes.push(TreeNode {
        depth: 0,
        text: "General".to_owned(),
        expanded: true,
        has_children: true,
        detail_sections: Rc::from(general_sections),
        node_type: NodeType::SectionHeader,
        section_type: Some(SectionType::General),
        service_list_type: None,
        param_id: None,
        parent_ref_names: Vec::new(),
        diff_status: general_status,
    });

    // Summary child node (depth 1) under General
    let summary_text = format_summary(&diff.summary);
    nodes.push(TreeNode {
        depth: 1,
        text: summary_text,
        expanded: false,
        has_children: false,
        detail_sections: Rc::from([]),
        node_type: NodeType::Default,
        section_type: None,
        service_list_type: None,
        param_id: None,
        parent_ref_names: Vec::new(),
        diff_status: None,
    });

    // -- Variants section (depth 0) -----------------------------------------
    if !diff.variants.is_empty() {
        push_section_header(&mut nodes, "Variants", Some(SectionType::Variants), true);
        for elem in &diff.variants {
            add_element_diff_nodes(&mut nodes, elem, 1);
        }
    }

    // -- Functional Groups section (depth 0) --------------------------------
    if !diff.functional_groups.is_empty() {
        push_section_header(
            &mut nodes,
            "Functional Groups",
            Some(SectionType::FunctionalGroups),
            true,
        );
        for elem in &diff.functional_groups {
            add_element_diff_nodes(&mut nodes, elem, 1);
        }
    }

    // -- DTCs section (depth 0) ---------------------------------------------
    if !diff.dtcs.is_empty() {
        push_section_header(&mut nodes, "DTCs", None, true);
        for elem in &diff.dtcs {
            add_element_diff_nodes(&mut nodes, elem, 1);
        }
    }

    let ecu_label = format!("{} vs {}", diff.old_name, diff.new_name);
    (nodes, ecu_label)
}

// ---------------------------------------------------------------------------
// Recursive element-diff expansion
// ---------------------------------------------------------------------------

/// Recursively expand an [`ElementDiff`] into one or more `TreeNode`s starting
/// at the given `depth`.
fn add_element_diff_nodes(nodes: &mut Vec<TreeNode>, diff: &ElementDiff, depth: usize) {
    let has_children = !diff.children.is_empty();

    // Determine display text.
    // For modified leaf nodes with exactly one property diff, inline the change.
    let text = if let (DiffStatus::Modified, true, Some(p)) = (
        diff.status,
        diff.children.is_empty() && diff.property_diffs.len() == 1,
        diff.property_diffs.first(),
    ) {
        format!("{}: {} -> {}", diff.name, p.old_value, p.new_value)
    } else {
        diff.name.clone()
    };

    // Build detail sections for property diffs (if more than one).
    let sections: Vec<DetailSectionData> = if diff.property_diffs.len() > 1 {
        vec![build_property_diff_section("Changes", &diff.property_diffs)]
    } else {
        Vec::new()
    };

    nodes.push(TreeNode {
        depth,
        text,
        expanded: true,
        has_children,
        detail_sections: Rc::from(sections),
        node_type: NodeType::Default,
        section_type: None,
        service_list_type: None,
        param_id: None,
        parent_ref_names: Vec::new(),
        diff_status: Some(diff.status),
    });

    // Recurse into children at depth + 1.
    let child_depth = depth.saturating_add(1);
    for child in &diff.children {
        add_element_diff_nodes(nodes, child, child_depth);
    }
}

// ---------------------------------------------------------------------------
// Detail section builder
// ---------------------------------------------------------------------------

/// Create a detail section containing a table of property diffs.
///
/// The table has three columns: Property, Old, New.
fn build_property_diff_section(title: &str, diffs: &[PropertyDiff]) -> DetailSectionData {
    let header = DetailRow::header(
        vec!["Property".to_owned(), "Old".to_owned(), "New".to_owned()],
        vec![CellType::Text, CellType::Text, CellType::Text],
    );

    let rows: Vec<DetailRow> = diffs
        .iter()
        .map(|p| {
            DetailRow::normal(
                vec![p.name.clone(), p.old_value.clone(), p.new_value.clone()],
                vec![CellType::Text, CellType::Text, CellType::Text],
                0,
            )
        })
        .collect();

    let constraints = vec![
        ColumnConstraint::Percentage(33),
        ColumnConstraint::Percentage(33),
        ColumnConstraint::Percentage(34),
    ];

    let content = DetailContent::Table {
        header,
        rows,
        constraints,
        use_row_selection: false,
    };

    DetailSectionData::new(title.to_owned(), content, false)
}

// ---------------------------------------------------------------------------
// Section header helper
// ---------------------------------------------------------------------------

/// Push a depth-0 section header node with no diff status.
fn push_section_header(
    nodes: &mut Vec<TreeNode>,
    text: &str,
    section_type: Option<SectionType>,
    has_children: bool,
) {
    nodes.push(TreeNode {
        depth: 0,
        text: text.to_owned(),
        expanded: true,
        has_children,
        detail_sections: Rc::from([]),
        node_type: NodeType::SectionHeader,
        section_type,
        service_list_type: None,
        param_id: None,
        parent_ref_names: Vec::new(),
        diff_status: None,
    });
}

// ---------------------------------------------------------------------------
// Summary formatting
// ---------------------------------------------------------------------------

/// Format the diff summary counts into a human-readable string.
fn format_summary(summary: &crate::diff::compare::DiffSummary) -> String {
    format!(
        "+{} added, -{} removed, ~{} modified, {} unchanged",
        summary.added, summary.removed, summary.modified, summary.unchanged,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::compare::{DiffSummary, ElementDiff, PropertyDiff};

    fn empty_summary() -> DiffSummary {
        DiffSummary {
            added: 0,
            removed: 0,
            modified: 0,
            unchanged: 0,
        }
    }

    #[test]
    fn empty_diff_produces_general_section_only() {
        let diff = DiffResult {
            old_name: "ECU_A".to_owned(),
            new_name: "ECU_B".to_owned(),
            ecu_diffs: Vec::new(),
            variants: Vec::new(),
            functional_groups: Vec::new(),
            dtcs: Vec::new(),
            summary: empty_summary(),
        };

        let (nodes, label) = build_diff_tree(&diff);

        assert_eq!(label, "ECU_A vs ECU_B");
        // General header + summary line
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes.first().map(|n| n.text.as_str()), Some("General"));
        assert_eq!(nodes.first().map(|n| n.diff_status), Some(None));
    }

    #[test]
    fn ecu_property_diffs_shown_in_general_section() {
        let diff = DiffResult {
            old_name: "A".to_owned(),
            new_name: "B".to_owned(),
            ecu_diffs: vec![PropertyDiff {
                name: "version".to_owned(),
                old_value: "1.0".to_owned(),
                new_value: "2.0".to_owned(),
            }],
            variants: Vec::new(),
            functional_groups: Vec::new(),
            dtcs: Vec::new(),
            summary: empty_summary(),
        };

        let (nodes, _) = build_diff_tree(&diff);
        let general = nodes.first().expect("should have General node");
        assert_eq!(general.diff_status, Some(DiffStatus::Modified));
        assert!(!general.detail_sections.is_empty());
    }

    #[test]
    fn section_headers_have_no_diff_status() {
        let diff = DiffResult {
            old_name: "A".to_owned(),
            new_name: "B".to_owned(),
            ecu_diffs: Vec::new(),
            variants: vec![ElementDiff {
                name: "V1".to_owned(),
                status: DiffStatus::Added,
                property_diffs: Vec::new(),
                children: Vec::new(),
            }],
            functional_groups: Vec::new(),
            dtcs: Vec::new(),
            summary: DiffSummary {
                added: 1,
                removed: 0,
                modified: 0,
                unchanged: 0,
            },
        };

        let (nodes, _) = build_diff_tree(&diff);
        // Find the "Variants" section header
        let variants_header = nodes.iter().find(|n| n.text == "Variants");
        assert!(variants_header.is_some());
        let hdr = variants_header.expect("checked above");
        assert_eq!(hdr.diff_status, None);
        assert_eq!(hdr.node_type, NodeType::SectionHeader);
    }

    #[test]
    fn element_diff_nodes_carry_status() {
        let diff = DiffResult {
            old_name: "A".to_owned(),
            new_name: "B".to_owned(),
            ecu_diffs: Vec::new(),
            variants: Vec::new(),
            functional_groups: Vec::new(),
            dtcs: vec![
                ElementDiff {
                    name: "DTC_0x01".to_owned(),
                    status: DiffStatus::Added,
                    property_diffs: Vec::new(),
                    children: Vec::new(),
                },
                ElementDiff {
                    name: "DTC_0x02".to_owned(),
                    status: DiffStatus::Removed,
                    property_diffs: Vec::new(),
                    children: Vec::new(),
                },
            ],
            summary: DiffSummary {
                added: 1,
                removed: 1,
                modified: 0,
                unchanged: 0,
            },
        };

        let (nodes, _) = build_diff_tree(&diff);

        let dtc_added = nodes.iter().find(|n| n.text == "DTC_0x01");
        assert_eq!(
            dtc_added.map(|n| n.diff_status),
            Some(Some(DiffStatus::Added))
        );

        let dtc_removed = nodes.iter().find(|n| n.text == "DTC_0x02");
        assert_eq!(
            dtc_removed.map(|n| n.diff_status),
            Some(Some(DiffStatus::Removed))
        );
    }

    #[test]
    fn single_property_diff_inlined_in_text() {
        let diff = DiffResult {
            old_name: "A".to_owned(),
            new_name: "B".to_owned(),
            ecu_diffs: Vec::new(),
            variants: Vec::new(),
            functional_groups: Vec::new(),
            dtcs: vec![ElementDiff {
                name: "DTC_0x10".to_owned(),
                status: DiffStatus::Modified,
                property_diffs: vec![PropertyDiff {
                    name: "text".to_owned(),
                    old_value: "old desc".to_owned(),
                    new_value: "new desc".to_owned(),
                }],
                children: Vec::new(),
            }],
            summary: DiffSummary {
                added: 0,
                removed: 0,
                modified: 1,
                unchanged: 0,
            },
        };

        let (nodes, _) = build_diff_tree(&diff);
        let dtc_node = nodes
            .iter()
            .find(|n| n.text.contains("DTC_0x10"))
            .expect("should find DTC node");
        assert_eq!(dtc_node.text, "DTC_0x10: old desc -> new desc");
        // Single-property inline means no detail sections needed
        assert!(dtc_node.detail_sections.is_empty());
    }

    #[test]
    fn multiple_property_diffs_go_to_detail_pane() {
        let diff = DiffResult {
            old_name: "A".to_owned(),
            new_name: "B".to_owned(),
            ecu_diffs: Vec::new(),
            variants: Vec::new(),
            functional_groups: Vec::new(),
            dtcs: vec![ElementDiff {
                name: "DTC_0x20".to_owned(),
                status: DiffStatus::Modified,
                property_diffs: vec![
                    PropertyDiff {
                        name: "text".to_owned(),
                        old_value: "a".to_owned(),
                        new_value: "b".to_owned(),
                    },
                    PropertyDiff {
                        name: "level".to_owned(),
                        old_value: "1".to_owned(),
                        new_value: "2".to_owned(),
                    },
                ],
                children: Vec::new(),
            }],
            summary: DiffSummary {
                added: 0,
                removed: 0,
                modified: 1,
                unchanged: 0,
            },
        };

        let (nodes, _) = build_diff_tree(&diff);
        let dtc_node = nodes
            .iter()
            .find(|n| n.text == "DTC_0x20")
            .expect("should find DTC node");
        // Name only (not inlined)
        assert_eq!(dtc_node.text, "DTC_0x20");
        assert_eq!(dtc_node.detail_sections.len(), 1);
    }

    #[test]
    fn children_expand_recursively() {
        let diff = DiffResult {
            old_name: "A".to_owned(),
            new_name: "B".to_owned(),
            ecu_diffs: Vec::new(),
            variants: vec![ElementDiff {
                name: "V1".to_owned(),
                status: DiffStatus::Modified,
                property_diffs: Vec::new(),
                children: vec![ElementDiff {
                    name: "Services".to_owned(),
                    status: DiffStatus::Modified,
                    property_diffs: Vec::new(),
                    children: vec![ElementDiff {
                        name: "SvcA".to_owned(),
                        status: DiffStatus::Added,
                        property_diffs: Vec::new(),
                        children: Vec::new(),
                    }],
                }],
            }],
            functional_groups: Vec::new(),
            dtcs: Vec::new(),
            summary: DiffSummary {
                added: 0,
                removed: 0,
                modified: 1,
                unchanged: 0,
            },
        };

        let (nodes, _) = build_diff_tree(&diff);

        // Variants header(0), V1(1), Services(2), SvcA(3), General(0), summary(1)
        let svc_a = nodes.iter().find(|n| n.text == "SvcA");
        assert!(svc_a.is_some());
        let svc_a = svc_a.expect("checked above");
        assert_eq!(svc_a.depth, 3);
        assert_eq!(svc_a.diff_status, Some(DiffStatus::Added));
    }

    #[test]
    fn summary_line_format() {
        let summary = DiffSummary {
            added: 3,
            removed: 1,
            modified: 5,
            unchanged: 10,
        };
        let text = format_summary(&summary);
        assert_eq!(text, "+3 added, -1 removed, ~5 modified, 10 unchanged");
    }
}
