// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

//! Builds a diff-annotated tree by running the browse-mode tree builder for
//! both databases, then merging the two trees with [`DiffStatus`] annotations.
//!
//! This approach reuses all existing detail-section logic (service overviews,
//! request/response parameters, com-params, etc.) so the diff view shows the
//! same rich information as browse mode, plus colour-coded change indicators.

use std::{collections::HashSet, rc::Rc};

use cda_database::datatypes::DiagnosticDatabase;

use crate::tree::{
    self, CellType, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, DiffStatus,
    NodeType, TreeNode,
};

// ---------------------------------------------------------------------------
// Intermediate hierarchical representation
// ---------------------------------------------------------------------------

/// A node in the hierarchical (non-flat) tree representation used during
/// the merge phase.
struct HierNode {
    tree_node: TreeNode,
    children: Vec<HierNode>,
}

/// A merged node carrying its diff status and merged children.
struct MergedNode {
    node: TreeNode,
    children: Vec<MergedNode>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Build a diff-annotated flat tree by merging browse-mode trees for both
/// databases.
///
/// Returns `(nodes, ecu_label)` where `ecu_label` is `"old_name vs new_name"`.
pub fn build_diff_tree(
    db_old: &DiagnosticDatabase,
    db_new: &DiagnosticDatabase,
    old_path: &str,
    new_path: &str,
) -> (Vec<TreeNode>, String) {
    // 1. Build full browse-mode trees for both databases, reusing all
    //    existing detail-section builders (services, params, DOPs, …).
    let (tree_old, name_old) = tree::build_tree(db_old, old_path);
    let (tree_new, name_new) = tree::build_tree(db_new, new_path);

    // 2. Convert flat depth-based lists to hierarchical trees
    let hier_old = flat_to_hier(&tree_old);
    let hier_new = flat_to_hier(&tree_new);

    // 3. Merge with diff annotations
    let merged = merge_children(&hier_old, &hier_new);

    // 4. Compute summary from top-level element statuses
    let summary = compute_summary(&merged);

    // 5. Add summary and file source info under General
    let merged = add_summary_to_general(merged, &summary, old_path, new_path);

    // 6. Flatten back to a flat depth-based list
    let nodes = flatten_merged(&merged, 0);

    let label = format!("{name_old} vs {name_new}");
    (nodes, label)
}

// ---------------------------------------------------------------------------
// Flat ↔ hierarchical conversion
// ---------------------------------------------------------------------------

/// Convert a flat depth-based tree into a hierarchical tree of [`HierNode`]s.
///
/// Groups consecutive nodes: each node at depth *d* claims all immediately
/// following nodes with depth > *d* as its descendants.
fn flat_to_hier(nodes: &[TreeNode]) -> Vec<HierNode> {
    let mut result = Vec::new();
    let mut i = 0;
    while let Some(current) = nodes.get(i) {
        let depth = current.depth;
        let tree_node = current.clone();
        i = i.saturating_add(1);

        // All subsequent nodes that are deeper belong to this subtree
        let children_start = i;
        while nodes.get(i).is_some_and(|n| n.depth > depth) {
            i = i.saturating_add(1);
        }

        let children = nodes
            .get(children_start..i)
            .map(flat_to_hier)
            .unwrap_or_default();
        result.push(HierNode {
            tree_node,
            children,
        });
    }
    result
}

/// Flatten a merged hierarchical tree back into a depth-based flat list.
///
/// Preserves the original `expanded` state from the browse-mode tree so nodes
/// start collapsed by default (matching browse mode behaviour).
fn flatten_merged(nodes: &[MergedNode], depth: usize) -> Vec<TreeNode> {
    let mut result = Vec::new();
    for node in nodes {
        let mut tree_node = node.node.clone();
        tree_node.depth = depth;
        tree_node.has_children = !node.children.is_empty();
        // Keep the original expanded state from browse-mode tree building
        result.push(tree_node);
        result.extend(flatten_merged(&node.children, depth.saturating_add(1)));
    }
    result
}

// ---------------------------------------------------------------------------
// Tree merging
// ---------------------------------------------------------------------------

/// Merge two lists of hierarchical sibling nodes, producing merged nodes with
/// [`DiffStatus`] annotations.
///
/// Children are matched by [`match_key`]. New-tree order is preserved, with
/// removed (old-only) nodes appended at the end.
fn merge_children(old_children: &[HierNode], new_children: &[HierNode]) -> Vec<MergedNode> {
    // Index old children by match key for lookup.
    // If two siblings share the same key only the last one is indexed; this is
    // acceptable because sibling names should be unique in practice.
    let old_by_key: std::collections::BTreeMap<String, &HierNode> = old_children
        .iter()
        .map(|n| (match_key(&n.tree_node), n))
        .collect();

    let mut result = Vec::new();
    let mut matched_keys: HashSet<String> = HashSet::new();

    // ── Items from the new tree (Added or Matched) ─────────────────────
    for new_node in new_children {
        let key = match_key(&new_node.tree_node);

        if let Some(old_node) = old_by_key.get(&key) {
            matched_keys.insert(key);

            // Node exists in both trees — recurse into children
            let merged_children = merge_children(&old_node.children, &new_node.children);

            let own_changed = !node_content_equal(&old_node.tree_node, &new_node.tree_node);
            let children_changed = merged_children.iter().any(|c| {
                matches!(
                    c.node.diff_status,
                    Some(DiffStatus::Added | DiffStatus::Removed | DiffStatus::Modified)
                )
            });

            let status = if own_changed || children_changed {
                DiffStatus::Modified
            } else {
                DiffStatus::Unchanged
            };

            let mut node = new_node.tree_node.clone();
            node.diff_status = Some(status);

            // Prepend a "Changes" section for nodes whose own content differs
            if own_changed
                && let Some(changes) =
                    build_changes_section(&old_node.tree_node, &new_node.tree_node)
            {
                let mut sections: Vec<DetailSectionData> = vec![changes];
                sections.extend(node.detail_sections.iter().cloned());
                node.detail_sections = Rc::from(sections);
            }

            result.push(MergedNode {
                node,
                children: merged_children,
            });
        } else {
            // Node exists only in new tree — Added
            result.push(mark_subtree(new_node, DiffStatus::Added));
        }
    }

    // ── Removed items (old-only) ───────────────────────────────────────
    for old_node in old_children {
        let key = match_key(&old_node.tree_node);
        if !matched_keys.contains(&key) {
            result.push(mark_subtree(old_node, DiffStatus::Removed));
        }
    }

    result
}

/// Recursively mark an entire subtree with the given [`DiffStatus`].
fn mark_subtree(node: &HierNode, status: DiffStatus) -> MergedNode {
    let mut tree_node = node.tree_node.clone();
    tree_node.diff_status = Some(status);

    let children: Vec<MergedNode> = node
        .children
        .iter()
        .map(|child| mark_subtree(child, status))
        .collect();

    MergedNode {
        node: tree_node,
        children,
    }
}

// ---------------------------------------------------------------------------
// Node matching
// ---------------------------------------------------------------------------

/// Extract a stable match key from a tree node's text.
///
/// Normalizes display text so that nodes representing the same logical element
/// match even when cosmetic parts of the text differ (e.g. service IDs change,
/// item counts change).
fn match_key(node: &TreeNode) -> String {
    let text = node.text.strip_suffix(" [base]").unwrap_or(&node.text);

    // Service nodes: "[Service] 0x2E01 - WriteDID" → "WriteDID"
    // Job nodes:     "[Job] MyJob"                 → "MyJob"
    for prefix in [
        tree::NodeTextPrefix::Service.as_str(),
        tree::NodeTextPrefix::Job.as_str(),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return rest.find(" - ").map_or_else(
                || rest.to_owned(),
                |pos| {
                    rest.get(pos.saturating_add(3)..)
                        .unwrap_or_default()
                        .to_owned()
                },
            );
        }
    }

    // List headers: "Diag-Comms (5 services, 2 jobs)" → "Diag-Comms"
    if text.ends_with(')')
        && let Some(pos) = text.rfind(" (")
    {
        return text.get(..pos).unwrap_or(text).to_owned();
    }

    text.to_owned()
}

// ---------------------------------------------------------------------------
// Content comparison
// ---------------------------------------------------------------------------

/// Check whether two tree nodes have equal content (text and detail sections).
fn node_content_equal(old: &TreeNode, new: &TreeNode) -> bool {
    old.text == new.text
        && old.detail_sections.len() == new.detail_sections.len()
        && old
            .detail_sections
            .iter()
            .zip(new.detail_sections.iter())
            .all(|(o, n)| section_content_equal(o, n))
}

fn section_content_equal(old: &DetailSectionData, new: &DetailSectionData) -> bool {
    old.title == new.title && detail_content_equal(&old.content, &new.content)
}

fn detail_content_equal(old: &DetailContent, new: &DetailContent) -> bool {
    match (old, new) {
        (DetailContent::PlainText(o), DetailContent::PlainText(n)) => o == n,
        (
            DetailContent::Table { rows: old_rows, .. },
            DetailContent::Table { rows: new_rows, .. },
        ) => {
            old_rows.len() == new_rows.len()
                && old_rows
                    .iter()
                    .zip(new_rows.iter())
                    .all(|(o, n)| o.cells == n.cells)
        }
        (DetailContent::Composite(o), DetailContent::Composite(n)) => {
            o.len() == n.len()
                && o.iter()
                    .zip(n.iter())
                    .all(|(o, n)| section_content_equal(o, n))
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Changes section builder
// ---------------------------------------------------------------------------

/// Represents a single changed property for the "Changes" detail pane.
struct ChangedProperty {
    name: String,
    old_value: String,
    new_value: String,
}

/// Build a "Changes" detail section by comparing old and new node content.
///
/// Returns `None` if no displayable changes are found.
fn build_changes_section(old: &TreeNode, new: &TreeNode) -> Option<DetailSectionData> {
    let mut diffs: Vec<ChangedProperty> = Vec::new();

    // Compare node display text
    if old.text != new.text {
        diffs.push(ChangedProperty {
            name: "Display Name".to_owned(),
            old_value: old.text.clone(),
            new_value: new.text.clone(),
        });
    }

    // Compare detail section content — extract row-level diffs from tables
    for old_section in old.detail_sections.iter() {
        // Find the matching section in new by type+title, falling back to title
        let matching_new = new
            .detail_sections
            .iter()
            .find(|s| s.section_type == old_section.section_type && s.title == old_section.title)
            .or_else(|| {
                new.detail_sections
                    .iter()
                    .find(|s| s.title == old_section.title)
            });

        let Some(new_section) = matching_new else {
            continue;
        };

        extract_table_diffs(
            &old_section.content,
            &new_section.content,
            &old_section.title,
            &mut diffs,
        );
    }

    if diffs.is_empty() {
        return None;
    }

    Some(build_property_diff_section("Changes", &diffs))
}

/// Extract row-level diffs between two matching detail content sections.
fn extract_table_diffs(
    old: &DetailContent,
    new: &DetailContent,
    section_title: &str,
    diffs: &mut Vec<ChangedProperty>,
) {
    match (old, new) {
        (
            DetailContent::Table { rows: old_rows, .. },
            DetailContent::Table { rows: new_rows, .. },
        ) => {
            // For key-value tables, compare by first column (key)
            for old_row in old_rows {
                let Some(key) = old_row.cells.first() else {
                    continue;
                };
                let Some(new_row) = new_rows.iter().find(|r| r.cells.first() == Some(key)) else {
                    continue;
                };
                if old_row.cells != new_row.cells {
                    let old_val = old_row.cells.get(1).cloned().unwrap_or_default();
                    let new_val = new_row.cells.get(1).cloned().unwrap_or_default();
                    let prop_name = if section_title.is_empty() {
                        key.clone()
                    } else {
                        format!("{section_title}: {key}")
                    };
                    diffs.push(ChangedProperty {
                        name: prop_name,
                        old_value: old_val,
                        new_value: new_val,
                    });
                }
            }
        }
        (DetailContent::Composite(old_subs), DetailContent::Composite(new_subs)) => {
            for (old_sub, new_sub) in old_subs.iter().zip(new_subs.iter()) {
                extract_table_diffs(&old_sub.content, &new_sub.content, &old_sub.title, diffs);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Property diff table builder
// ---------------------------------------------------------------------------

/// Create a detail section containing a table of property differences.
fn build_property_diff_section(title: &str, diffs: &[ChangedProperty]) -> DetailSectionData {
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

    DetailSectionData::new(
        title.to_owned(),
        DetailContent::Table {
            header,
            rows,
            constraints,
            use_row_selection: false,
        },
        false,
    )
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

/// Diff summary counts for the General section.
#[derive(Default)]
struct DiffSummary {
    added: usize,
    removed: usize,
    modified: usize,
    unchanged: usize,
}

/// Compute diff summary by counting statuses of section header children
/// (top-level elements like variants, functional groups, etc.).
fn compute_summary(root_nodes: &[MergedNode]) -> DiffSummary {
    let mut summary = DiffSummary::default();
    for section in root_nodes {
        for child in &section.children {
            match child.node.diff_status {
                Some(DiffStatus::Added) => {
                    summary.added = summary.added.saturating_add(1);
                }
                Some(DiffStatus::Removed) => {
                    summary.removed = summary.removed.saturating_add(1);
                }
                Some(DiffStatus::Modified) => {
                    summary.modified = summary.modified.saturating_add(1);
                }
                Some(DiffStatus::Unchanged) => {
                    summary.unchanged = summary.unchanged.saturating_add(1);
                }
                None => {}
            }
        }
    }
    summary
}

/// Add summary and file source info under the "General" section header.
fn add_summary_to_general(
    mut nodes: Vec<MergedNode>,
    summary: &DiffSummary,
    old_path: &str,
    new_path: &str,
) -> Vec<MergedNode> {
    let Some(general) = nodes.iter_mut().find(|n| n.node.text == "General") else {
        return nodes;
    };

    // Build a detail section showing file sources and summary
    let header = DetailRow::header(
        vec!["Property".to_owned(), "Value".to_owned()],
        vec![CellType::Text, CellType::Text],
    );
    let rows = vec![
        DetailRow::normal(
            vec!["Old file (removed)".to_owned(), old_path.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["New file (added)".to_owned(), new_path.to_owned()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Added".to_owned(), summary.added.to_string()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Removed".to_owned(), summary.removed.to_string()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Modified".to_owned(), summary.modified.to_string()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
        DetailRow::normal(
            vec!["Unchanged".to_owned(), summary.unchanged.to_string()],
            vec![CellType::Text, CellType::Text],
            0,
        ),
    ];
    let diff_overview = DetailSectionData::new(
        "Diff Overview".to_owned(),
        DetailContent::Table {
            header,
            rows,
            constraints: vec![
                ColumnConstraint::Percentage(30),
                ColumnConstraint::Percentage(70),
            ],
            use_row_selection: false,
        },
        false,
    );

    // Prepend the diff overview to the General node's existing sections
    let mut sections: Vec<DetailSectionData> = vec![diff_overview];
    sections.extend(general.node.detail_sections.iter().cloned());
    general.node.detail_sections = Rc::from(sections);
    general.node.expanded = true;

    // Add summary text as a child node
    let summary_text = format!(
        "+{} added, -{} removed, ~{} modified, {} unchanged",
        summary.added, summary.removed, summary.modified, summary.unchanged,
    );
    general.children.push(MergedNode {
        node: TreeNode {
            depth: 0, // set during flatten
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
        },
        children: Vec::new(),
    });

    nodes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(text: &str, depth: usize, has_children: bool) -> TreeNode {
        TreeNode {
            depth,
            text: text.to_owned(),
            expanded: false,
            has_children,
            detail_sections: Rc::from([]),
            node_type: NodeType::Default,
            section_type: None,
            service_list_type: None,
            param_id: None,
            parent_ref_names: Vec::new(),
            diff_status: None,
        }
    }

    fn make_node_with_sections(
        text: &str,
        depth: usize,
        sections: Vec<DetailSectionData>,
    ) -> TreeNode {
        TreeNode {
            depth,
            text: text.to_owned(),
            expanded: false,
            has_children: false,
            detail_sections: Rc::from(sections),
            node_type: NodeType::Default,
            section_type: None,
            service_list_type: None,
            param_id: None,
            parent_ref_names: Vec::new(),
            diff_status: None,
        }
    }

    #[test]
    fn flat_to_hier_preserves_structure() {
        let nodes = vec![
            make_node("A", 0, true),
            make_node("A1", 1, false),
            make_node("A2", 1, false),
            make_node("B", 0, false),
        ];

        let hier = flat_to_hier(&nodes);
        assert_eq!(hier.len(), 2);
        assert_eq!(hier[0].tree_node.text, "A");
        assert_eq!(hier[0].children.len(), 2);
        assert_eq!(hier[0].children[0].tree_node.text, "A1");
        assert_eq!(hier[0].children[1].tree_node.text, "A2");
        assert_eq!(hier[1].tree_node.text, "B");
        assert!(hier[1].children.is_empty());
    }

    #[test]
    fn flat_to_hier_handles_deep_nesting() {
        let nodes = vec![
            make_node("Root", 0, true),
            make_node("L1", 1, true),
            make_node("L2", 2, true),
            make_node("L3", 3, false),
        ];

        let hier = flat_to_hier(&nodes);
        assert_eq!(hier.len(), 1);
        assert_eq!(hier[0].children.len(), 1);
        assert_eq!(hier[0].children[0].children.len(), 1);
        assert_eq!(hier[0].children[0].children[0].children.len(), 1);
        assert_eq!(
            hier[0].children[0].children[0].children[0].tree_node.text,
            "L3"
        );
    }

    #[test]
    fn identical_trees_produce_unchanged() {
        let tree = vec![
            make_node("Section", 0, true),
            make_node("Child1", 1, false),
            make_node("Child2", 1, false),
        ];

        let hier_old = flat_to_hier(&tree);
        let hier_new = flat_to_hier(&tree);
        let merged = merge_children(&hier_old, &hier_new);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].node.diff_status, Some(DiffStatus::Unchanged));
        assert!(
            merged[0]
                .children
                .iter()
                .all(|c| c.node.diff_status == Some(DiffStatus::Unchanged))
        );
    }

    #[test]
    fn added_node_detected() {
        let old_tree = vec![make_node("Section", 0, true), make_node("Child1", 1, false)];
        let new_tree = vec![
            make_node("Section", 0, true),
            make_node("Child1", 1, false),
            make_node("Child2", 1, false),
        ];

        let hier_old = flat_to_hier(&old_tree);
        let hier_new = flat_to_hier(&new_tree);
        let merged = merge_children(&hier_old, &hier_new);

        let section = &merged[0];
        assert_eq!(section.node.diff_status, Some(DiffStatus::Modified));
        assert_eq!(section.children.len(), 2);
        assert_eq!(
            section.children[0].node.diff_status,
            Some(DiffStatus::Unchanged)
        );
        assert_eq!(
            section.children[1].node.diff_status,
            Some(DiffStatus::Added)
        );
    }

    #[test]
    fn removed_node_detected() {
        let old_tree = vec![
            make_node("Section", 0, true),
            make_node("Child1", 1, false),
            make_node("Child2", 1, false),
        ];
        let new_tree = vec![make_node("Section", 0, true), make_node("Child1", 1, false)];

        let hier_old = flat_to_hier(&old_tree);
        let hier_new = flat_to_hier(&new_tree);
        let merged = merge_children(&hier_old, &hier_new);

        let section = &merged[0];
        assert_eq!(section.node.diff_status, Some(DiffStatus::Modified));
        // Child1 (Unchanged) + Child2 (Removed, appended at end)
        assert_eq!(section.children.len(), 2);
        assert_eq!(
            section.children[0].node.diff_status,
            Some(DiffStatus::Unchanged)
        );
        assert_eq!(
            section.children[1].node.diff_status,
            Some(DiffStatus::Removed)
        );
    }

    #[test]
    fn modified_content_detected() {
        let section1 = DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::PlainText(vec!["old value".to_owned()]),
            false,
        );
        let section2 = DetailSectionData::new(
            "Overview".to_owned(),
            DetailContent::PlainText(vec!["new value".to_owned()]),
            false,
        );

        let old_tree = vec![make_node_with_sections("Item", 0, vec![section1])];
        let new_tree = vec![make_node_with_sections("Item", 0, vec![section2])];

        let hier_old = flat_to_hier(&old_tree);
        let hier_new = flat_to_hier(&new_tree);
        let merged = merge_children(&hier_old, &hier_new);

        assert_eq!(merged[0].node.diff_status, Some(DiffStatus::Modified));
    }

    #[test]
    fn parent_modified_when_child_changes() {
        let old_tree = vec![make_node("Parent", 0, true), make_node("Child", 1, false)];
        let new_tree = vec![
            make_node("Parent", 0, true),
            make_node_with_sections(
                "Child",
                1,
                vec![DetailSectionData::new(
                    "New".to_owned(),
                    DetailContent::PlainText(vec!["data".to_owned()]),
                    false,
                )],
            ),
        ];

        let hier_old = flat_to_hier(&old_tree);
        let hier_new = flat_to_hier(&new_tree);
        let merged = merge_children(&hier_old, &hier_new);

        assert_eq!(merged[0].node.diff_status, Some(DiffStatus::Modified));
        assert_eq!(
            merged[0].children[0].node.diff_status,
            Some(DiffStatus::Modified)
        );
    }

    #[test]
    fn match_key_strips_service_prefix() {
        let node = make_node("[Service] 0x2E01 - WriteDID", 0, false);
        assert_eq!(match_key(&node), "WriteDID");
    }

    #[test]
    fn match_key_strips_job_prefix() {
        let node = make_node("[Job] MyJob", 0, false);
        assert_eq!(match_key(&node), "MyJob");
    }

    #[test]
    fn match_key_strips_count_suffix() {
        let node = make_node("Diag-Comms (5 services, 2 jobs)", 0, false);
        assert_eq!(match_key(&node), "Diag-Comms");
    }

    #[test]
    fn match_key_preserves_plain_text() {
        let node = make_node("Variants", 0, false);
        assert_eq!(match_key(&node), "Variants");
    }

    #[test]
    fn match_key_strips_base_suffix() {
        let node = make_node("MyVariant [base]", 0, false);
        assert_eq!(match_key(&node), "MyVariant");
    }

    #[test]
    fn match_key_handles_service_without_id() {
        let node = make_node("[Service] ReadDID", 0, false);
        assert_eq!(match_key(&node), "ReadDID");
    }

    #[test]
    fn summary_counts_section_children() {
        let merged = vec![MergedNode {
            node: {
                let mut n = make_node("Section", 0, true);
                n.node_type = NodeType::SectionHeader;
                n.diff_status = Some(DiffStatus::Modified);
                n
            },
            children: vec![
                MergedNode {
                    node: {
                        let mut n = make_node("Added", 1, false);
                        n.diff_status = Some(DiffStatus::Added);
                        n
                    },
                    children: Vec::new(),
                },
                MergedNode {
                    node: {
                        let mut n = make_node("Removed", 1, false);
                        n.diff_status = Some(DiffStatus::Removed);
                        n
                    },
                    children: Vec::new(),
                },
                MergedNode {
                    node: {
                        let mut n = make_node("Unchanged", 1, false);
                        n.diff_status = Some(DiffStatus::Unchanged);
                        n
                    },
                    children: Vec::new(),
                },
            ],
        }];

        let summary = compute_summary(&merged);
        assert_eq!(summary.added, 1);
        assert_eq!(summary.removed, 1);
        assert_eq!(summary.modified, 0);
        assert_eq!(summary.unchanged, 1);
    }

    #[test]
    fn flatten_sets_correct_depths() {
        let merged = vec![MergedNode {
            node: make_node("Root", 99, true),
            children: vec![MergedNode {
                node: make_node("Child", 99, true),
                children: vec![MergedNode {
                    node: make_node("Grandchild", 99, false),
                    children: Vec::new(),
                }],
            }],
        }];

        let flat = flatten_merged(&merged, 0);
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[0].text, "Root");
        assert_eq!(flat[1].depth, 1);
        assert_eq!(flat[1].text, "Child");
        assert_eq!(flat[2].depth, 2);
        assert_eq!(flat[2].text, "Grandchild");
    }

    #[test]
    fn changes_section_built_for_modified_text() {
        let old_node = make_node("Version 1", 0, false);
        let new_node = make_node("Version 2", 0, false);

        let changes = build_changes_section(&old_node, &new_node);
        assert!(changes.is_some());
        let section = changes.expect("checked above");
        assert_eq!(section.title, "Changes");
    }

    #[test]
    fn no_changes_for_identical_nodes() {
        let node = make_node("Same", 0, false);
        let changes = build_changes_section(&node, &node);
        assert!(changes.is_none());
    }

    #[test]
    fn mark_subtree_sets_all_descendants() {
        let hier = HierNode {
            tree_node: make_node("Root", 0, true),
            children: vec![
                HierNode {
                    tree_node: make_node("A", 1, true),
                    children: vec![HierNode {
                        tree_node: make_node("A1", 2, false),
                        children: Vec::new(),
                    }],
                },
                HierNode {
                    tree_node: make_node("B", 1, false),
                    children: Vec::new(),
                },
            ],
        };

        let merged = mark_subtree(&hier, DiffStatus::Added);
        assert_eq!(merged.node.diff_status, Some(DiffStatus::Added));
        assert_eq!(merged.children[0].node.diff_status, Some(DiffStatus::Added));
        assert_eq!(
            merged.children[0].children[0].node.diff_status,
            Some(DiffStatus::Added)
        );
        assert_eq!(merged.children[1].node.diff_status, Some(DiffStatus::Added));
    }

    #[test]
    fn services_matched_by_short_name_not_id() {
        let old_tree = vec![
            make_node("Diag-Comms (2 services, 0 jobs)", 0, true),
            make_node("[Service] 0x22   - ReadDID", 1, false),
            make_node("[Service] 0x2E01 - WriteDID", 1, false),
        ];
        let new_tree = vec![
            make_node("Diag-Comms (2 services, 0 jobs)", 0, true),
            make_node("[Service] 0x22   - ReadDID", 1, false),
            make_node("[Service] 0x2E02 - WriteDID", 1, false),
        ];

        let hier_old = flat_to_hier(&old_tree);
        let hier_new = flat_to_hier(&new_tree);
        let merged = merge_children(&hier_old, &hier_new);

        // Both service containers match by name "Diag-Comms"
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].children.len(), 2);

        // ReadDID is Unchanged (same text)
        assert_eq!(
            merged[0].children[0].node.diff_status,
            Some(DiffStatus::Unchanged)
        );
        // WriteDID is Modified (ID changed: 0x2E01 → 0x2E02)
        assert_eq!(
            merged[0].children[1].node.diff_status,
            Some(DiffStatus::Modified)
        );
    }
}
