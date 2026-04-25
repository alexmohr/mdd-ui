// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

//! Builds a diff-annotated tree by running the browse-mode tree builder for
//! both databases, then merging the two trees with [`DiffStatus`] annotations.
//!
//! This approach reuses all existing detail-section logic (service overviews,
//! request/response parameters, com-params, etc.) so the diff view shows the
//! same rich information as browse mode, plus colour-coded change indicators.

use std::{collections::HashSet, sync::Arc};

use cda_database::datatypes::DiagnosticDatabase;

use crate::tree::{
    self, ColumnConstraint, DetailCell, DetailContent, DetailRow, DetailSectionData, DiffStatus,
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
    /// Whether this node's own content (text + detail sections) differs
    /// between the old and new trees.  Used to limit upward propagation of
    /// `DiffStatus::Modified`: a parent is only marked modified when a
    /// **direct** child has `own_changed == true` or is Added/Removed.
    own_changed: bool,
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

            // Check whether this node's own content actually differs.
            let content_differs = !node_content_equal(&old_node.tree_node, &new_node.tree_node);

            // Build the human-readable "Changes" summary tab when content
            // differs.  Even when the summary builder finds nothing to show
            // (e.g. only row-order changes), the node is still considered
            // modified because its content IS different.
            let changes_section = if content_differs {
                build_changes_section(&old_node.tree_node, &new_node.tree_node)
            } else {
                None
            };
            let own_changed = content_differs;

            // A node is marked Modified only when a direct child has its
            // own content change or was structurally added/removed.
            // Children reachable only through parent-ref inheritance are
            // excluded so that inherited-service changes do not bubble up.
            let children_changed = merged_children.iter().any(|c| {
                !matches!(
                    c.node.node_type,
                    NodeType::ParentRefService | NodeType::ParentRefs
                ) && (c.own_changed
                    || matches!(
                        c.node.diff_status,
                        Some(DiffStatus::Added | DiffStatus::Removed)
                    ))
            });

            let status = if own_changed || children_changed {
                DiffStatus::Modified
            } else {
                DiffStatus::Unchanged
            };

            let mut node = new_node.tree_node.clone();
            node.diff_status = Some(status);

            // Insert the "Changes" section right after any header section so
            // that `split_header_and_tabs` still recognises the header.
            if let Some(changes) = changes_section {
                let mut sections: Vec<DetailSectionData> =
                    Vec::with_capacity(node.detail_sections.len().saturating_add(1));
                let mut inserted = false;
                for s in node.detail_sections.iter() {
                    sections.push(s.clone());
                    if !inserted && s.render_as_header {
                        sections.push(changes.clone());
                        inserted = true;
                    }
                }
                if !inserted {
                    sections.insert(0, changes);
                }
                node.detail_sections = Arc::from(sections);
            }

            // Annotate individual table rows in matching sections with
            // per-row diff status so the detail tabs highlight changes.
            if content_differs {
                annotate_section_rows(&old_node.tree_node, &mut node);
            }

            result.push(MergedNode {
                node,
                children: merged_children,
                own_changed,
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
        // Added/Removed nodes are inherently different.
        own_changed: true,
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
    // Container nodes carry a canonical short_name — use it directly.
    if let Some(sn) = node.short_name() {
        return sn.to_owned();
    }

    // Diagcomm nodes carry a canonical service_short_name.
    if let Some(sn) = node.service_short_name() {
        return sn.to_owned();
    }

    let text = node.text.strip_suffix(" [base]").unwrap_or(&node.text);

    // Service nodes without service_short_name (shouldn't happen in normal
    // builds, but keep as fallback for diff-merge created nodes):
    // "0x2E01 - WriteDID" → "WriteDID"
    if node.node_type.is_diagcomm() {
        if let Some(pos) = text.find(" - ") {
            return text
                .get(pos.saturating_add(3)..)
                .unwrap_or_default()
                .to_owned();
        }
        return text.to_owned();
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
            // Order-independent comparison: match rows by their first cell
            // (key column) and compare cell content.  Row reordering alone
            // is not considered a change.
            old_rows.len() == new_rows.len()
                && old_rows.iter().all(|old_row| {
                    let Some(key) = old_row.cells.first() else {
                        return false;
                    };
                    new_rows.iter().any(|new_row| {
                        new_row.cells.first() == Some(key) && new_row.cells == old_row.cells
                    })
                })
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
// Per-row diff annotation for detail sections
// ---------------------------------------------------------------------------

/// Walk matching detail sections between `old` and `new` and set
/// [`DiffStatus`] on each table row so the table renderer can highlight
/// added, removed, and modified rows.
fn annotate_section_rows(old: &TreeNode, new: &mut TreeNode) {
    let old_sections = &old.detail_sections;
    let mut new_sections: Vec<DetailSectionData> = new.detail_sections.to_vec();

    for new_section in &mut new_sections {
        // Skip the "Changes" section we injected — it has its own styling.
        if new_section.title == "Changes" {
            continue;
        }

        // Match by section_type + title, then title only, then section_type
        // only (for sections with dynamic titles like "Diag-Comms (N services,
        // M jobs)" where the count changes between versions).
        let matching_old = old_sections
            .iter()
            .find(|s| s.section_type == new_section.section_type && s.title == new_section.title)
            .or_else(|| old_sections.iter().find(|s| s.title == new_section.title))
            .or_else(|| {
                // Type-only fallback for sections with dynamic titles (e.g.
                // "Diag-Comms (N services, M jobs)").  Only match when there
                // is exactly one section of that type to avoid ambiguity.
                let mut by_type = old_sections
                    .iter()
                    .filter(|s| s.section_type == new_section.section_type);
                let first = by_type.next();
                let second = by_type.next();
                (second.is_none()).then_some(first).flatten()
            });

        let Some(old_section) = matching_old else {
            // Entire section is new — mark all rows as Added.
            mark_all_content_rows(&mut new_section.content, DiffStatus::Added);
            continue;
        };

        annotate_content_rows(&old_section.content, &mut new_section.content);
    }

    // Append rows from removed sections (old-only) so they are visible in the
    // closest matching tab. For simplicity we do not create new tabs for
    // removed sections — the "Changes" summary already notes them.

    new.detail_sections = Arc::from(new_sections);
}

/// Set `diff_status` on every row in a `DetailContent`.
fn mark_all_content_rows(content: &mut DetailContent, status: DiffStatus) {
    match content {
        DetailContent::Table { rows, .. } => {
            for row in rows.iter_mut() {
                row.diff_status = Some(status);
            }
        }
        DetailContent::Composite(subs) => {
            for sub in subs.iter_mut() {
                mark_all_content_rows(&mut sub.content, status);
            }
        }
        DetailContent::PlainText(_) => {}
    }
}

/// Compare old and new `DetailContent` and annotate new rows with diff status.
///
/// For tables the first column (Short Name / Property key) is used as the
/// match key.  Rows present in both are compared cell-by-cell; rows only in
/// the new table are `Added`; rows only in the old table are appended as
/// `Removed`.
fn annotate_content_rows(old: &DetailContent, new: &mut DetailContent) {
    match (old, new) {
        (
            DetailContent::Table { rows: old_rows, .. },
            DetailContent::Table { rows: new_rows, .. },
        ) => {
            annotate_table_rows(old_rows, new_rows);
        }
        (DetailContent::Composite(old_subs), DetailContent::Composite(new_subs)) => {
            for (old_sub, new_sub) in old_subs.iter().zip(new_subs.iter_mut()) {
                annotate_content_rows(&old_sub.content, &mut new_sub.content);
            }
        }
        _ => {}
    }
}

/// Annotate table rows by matching on the first cell (key column).
///
/// For rows present in both old and new with differing cells, per-cell diff
/// statuses are set so only the actually changed cells are highlighted.
fn annotate_table_rows(old_rows: &[DetailRow], new_rows: &mut Vec<DetailRow>) {
    // Index old rows by first cell value.
    let old_by_key: std::collections::BTreeMap<&str, &DetailRow> = old_rows
        .iter()
        .filter_map(|r| r.cells.first().map(|k| (k.text.as_str(), r)))
        .collect();

    let mut matched_old_keys: HashSet<String> = HashSet::new();

    for new_row in new_rows.iter_mut() {
        let Some(key) = new_row.cells.first().map(|c| c.text.clone()) else {
            continue;
        };
        if let Some(old_row) = old_by_key.get(key.as_str()) {
            matched_old_keys.insert(key);
            if old_row.cells != new_row.cells {
                // Row has changes — mark at row level.
                new_row.diff_status = Some(DiffStatus::Modified);
            }
            // Otherwise leave diff_status as None (unchanged) — no highlight.
        } else {
            new_row.diff_status = Some(DiffStatus::Added);
        }
    }

    // Append old-only rows as Removed so they stay visible in the tab.
    let removed_rows: Vec<DetailRow> = old_rows
        .iter()
        .filter(|r| {
            r.cells
                .first()
                .is_some_and(|k| !matched_old_keys.contains(k.text.as_str()))
        })
        .map(|r| {
            let mut row = r.clone();
            row.diff_status = Some(DiffStatus::Removed);
            row
        })
        .collect();
    new_rows.extend(removed_rows);
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

    // Detect sections removed in new
    for old_section in old.detail_sections.iter() {
        if old_section.render_as_header {
            continue;
        }
        let found = new
            .detail_sections
            .iter()
            .any(|s| s.title == old_section.title);
        if !found {
            diffs.push(ChangedProperty {
                name: format!("Section removed: {}", old_section.title),
                old_value: "(present)".to_owned(),
                new_value: "(absent)".to_owned(),
            });
        }
    }

    // Detect sections added in new
    for new_section in new.detail_sections.iter() {
        if new_section.render_as_header {
            continue;
        }
        let found = old
            .detail_sections
            .iter()
            .any(|s| s.title == new_section.title);
        if !found {
            diffs.push(ChangedProperty {
                name: format!("Section added: {}", new_section.title),
                old_value: "(absent)".to_owned(),
                new_value: "(present)".to_owned(),
            });
        }
    }

    // Compare detail section content — extract row-level diffs from tables
    for old_section in old.detail_sections.iter() {
        // Find the matching section in new by type+title, then title only,
        // then type only (for sections with dynamic titles).
        let matching_new = new
            .detail_sections
            .iter()
            .find(|s| s.section_type == old_section.section_type && s.title == old_section.title)
            .or_else(|| {
                new.detail_sections
                    .iter()
                    .find(|s| s.title == old_section.title)
            })
            .or_else(|| {
                let mut by_type = new
                    .detail_sections
                    .iter()
                    .filter(|s| s.section_type == old_section.section_type);
                let first = by_type.next();
                let second = by_type.next();
                (second.is_none()).then_some(first).flatten()
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
            let old_keys: HashSet<&str> = old_rows
                .iter()
                .filter_map(|r| r.cells.first().map(|c| c.text.as_str()))
                .collect();
            let new_keys: HashSet<&str> = new_rows
                .iter()
                .filter_map(|r| r.cells.first().map(|c| c.text.as_str()))
                .collect();

            // Modified rows — present in both with differing cells.
            for old_row in old_rows {
                let Some(key) = old_row.cells.first() else {
                    continue;
                };
                let Some(new_row) = new_rows.iter().find(|r| r.cells.first() == Some(key)) else {
                    continue;
                };
                if old_row.cells != new_row.cells {
                    let changed_cols: Vec<String> = old_row
                        .cells
                        .iter()
                        .zip(new_row.cells.iter())
                        .skip(1)
                        .filter(|(o, n)| o != n)
                        .map(|(o, n)| format!("{} → {}", o.text, n.text))
                        .collect();
                    let prop_name = if section_title.is_empty() {
                        key.text.clone()
                    } else {
                        format!("{section_title}: {}", key.text)
                    };
                    diffs.push(ChangedProperty {
                        name: prop_name,
                        old_value: join_cell_texts(&old_row.cells, 1),
                        new_value: if changed_cols.is_empty() {
                            join_cell_texts(&new_row.cells, 1)
                        } else {
                            changed_cols.join("; ")
                        },
                    });
                }
            }

            // Removed rows — present in old but not in new.
            for old_row in old_rows {
                let Some(key) = old_row.cells.first() else {
                    continue;
                };
                if !new_keys.contains(key.text.as_str()) {
                    let prop_name = if section_title.is_empty() {
                        key.text.clone()
                    } else {
                        format!("{section_title}: {}", key.text)
                    };
                    diffs.push(ChangedProperty {
                        name: prop_name,
                        old_value: join_cell_texts(&old_row.cells, 1),
                        new_value: "(removed)".to_owned(),
                    });
                }
            }

            // Added rows — present in new but not in old.
            for new_row in new_rows {
                let Some(key) = new_row.cells.first() else {
                    continue;
                };
                if !old_keys.contains(key.text.as_str()) {
                    let prop_name = if section_title.is_empty() {
                        key.text.clone()
                    } else {
                        format!("{section_title}: {}", key.text)
                    };
                    diffs.push(ChangedProperty {
                        name: prop_name,
                        old_value: "(added)".to_owned(),
                        new_value: join_cell_texts(&new_row.cells, 1),
                    });
                }
            }
        }
        (DetailContent::PlainText(old_lines), DetailContent::PlainText(new_lines))
            if old_lines != new_lines =>
        {
            diffs.push(ChangedProperty {
                name: section_title.to_owned(),
                old_value: old_lines.join(", "),
                new_value: new_lines.join(", "),
            });
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
// Helpers
// ---------------------------------------------------------------------------

/// Join the text of cells starting at `skip` with `", "`.
fn join_cell_texts(cells: &[DetailCell], skip: usize) -> String {
    cells.get(skip..).map_or_else(String::new, |slice| {
        slice
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    })
}

// ---------------------------------------------------------------------------
// Property diff table builder
// ---------------------------------------------------------------------------

/// Create a detail section containing a table of property differences.
fn build_property_diff_section(title: &str, diffs: &[ChangedProperty]) -> DetailSectionData {
    let header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Old"),
        DetailCell::text("New"),
    ]);

    let rows: Vec<DetailRow> = diffs
        .iter()
        .map(|p| {
            DetailRow::normal(
                vec![
                    DetailCell::text(&p.name),
                    DetailCell::text(&p.old_value),
                    DetailCell::text(&p.new_value),
                ],
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

/// Compute diff summary by recursively counting services and jobs
/// (the most meaningful unit for diagnostic databases).
fn compute_summary(root_nodes: &[MergedNode]) -> DiffSummary {
    let mut summary = DiffSummary::default();
    for root in root_nodes {
        count_service_statuses(&root.children, &mut summary);
    }
    // If no services/jobs were found (unlikely but possible), fall back to
    // counting top-level section children so the summary is never empty.
    if summary.added == 0 && summary.removed == 0 && summary.modified == 0 && summary.unchanged == 0
    {
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
    }
    summary
}

/// Recursively walk merged nodes and tally [`DiffStatus`] for service and job
/// nodes only.  Services and jobs are the most meaningful unit for a
/// diagnostic-database diff summary (`Requests` / `PosResponses` /
/// `NegResponses` are counted separately by the tree but refer to the same
/// logical entity).
fn count_service_statuses(nodes: &[MergedNode], summary: &mut DiffSummary) {
    for node in nodes {
        if matches!(node.node.node_type, NodeType::Service | NodeType::Job) {
            match node.node.diff_status {
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
        } else {
            count_service_statuses(&node.children, summary);
        }
    }
}

/// Recursively count all nodes (every node type) by diff status.
fn count_all_statuses(nodes: &[MergedNode], summary: &mut DiffSummary) {
    for node in nodes {
        match node.node.diff_status {
            Some(DiffStatus::Added) => {
                summary.added = summary.added.saturating_add(1);
            }
            Some(DiffStatus::Removed) => {
                summary.removed = summary.removed.saturating_add(1);
            }
            Some(DiffStatus::Modified) => {
                summary.modified = summary.modified.saturating_add(1);
            }
            Some(DiffStatus::Unchanged) | None => {}
        }
        count_all_statuses(&node.children, summary);
    }
}

/// Add summary and file source info under the "General" section header.
fn add_summary_to_general(
    mut nodes: Vec<MergedNode>,
    summary: &DiffSummary,
    old_path: &str,
    new_path: &str,
) -> Vec<MergedNode> {
    let mut totals = DiffSummary::default();
    count_all_statuses(&nodes, &mut totals);

    let Some(general) = nodes.iter_mut().find(|n| n.node.text == "General") else {
        return nodes;
    };

    // Build a detail section showing file sources and summary
    let header = DetailRow::header(vec![
        DetailCell::text("Property"),
        DetailCell::text("Value"),
    ]);
    let rows = vec![
        DetailRow::normal(
            vec![
                DetailCell::text("Old file (removed)"),
                DetailCell::text(old_path),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("New file (added)"),
                DetailCell::text(new_path),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Added"),
                DetailCell::text(summary.added.to_string()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Removed"),
                DetailCell::text(summary.removed.to_string()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Modified"),
                DetailCell::text(summary.modified.to_string()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Unchanged"),
                DetailCell::text(summary.unchanged.to_string()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Total added"),
                DetailCell::text(totals.added.to_string()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Total changed"),
                DetailCell::text(totals.modified.to_string()),
            ],
            0,
        ),
        DetailRow::normal(
            vec![
                DetailCell::text("Total removed"),
                DetailCell::text(totals.removed.to_string()),
            ],
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
    general.node.detail_sections = Arc::from(sections);

    nodes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::default_trait_access)]
    fn make_node(text: &str, depth: usize, has_children: bool) -> TreeNode {
        TreeNode {
            depth,
            text: text.to_owned(),
            expanded: false,
            has_children,
            detail_sections: Arc::from([]),
            node_type: NodeType::Default,
            payload: Default::default(),
            parent_idx: None,
            diff_status: None,
        }
    }

    #[allow(clippy::default_trait_access)]
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
            detail_sections: Arc::from(sections),
            node_type: NodeType::Default,
            payload: Default::default(),
            parent_idx: None,
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
        let a = hier.first().expect("first element");
        let b = hier.get(1).expect("second element");
        assert_eq!(a.tree_node.text, "A");
        assert_eq!(a.children.len(), 2);
        assert_eq!(
            a.children.first().expect("first child").tree_node.text,
            "A1"
        );
        assert_eq!(
            a.children.get(1).expect("second child").tree_node.text,
            "A2"
        );
        assert_eq!(b.tree_node.text, "B");
        assert!(b.children.is_empty());
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
        let root = hier.first().expect("root");
        let l1 = root.children.first().expect("l1");
        let l2 = l1.children.first().expect("l2");
        let l3 = l2.children.first().expect("l3");
        assert_eq!(root.children.len(), 1);
        assert_eq!(l1.children.len(), 1);
        assert_eq!(l2.children.len(), 1);
        assert_eq!(l3.tree_node.text, "L3");
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
        let first = merged.first().expect("first merged");
        assert_eq!(first.node.diff_status, Some(DiffStatus::Unchanged));
        assert!(
            first
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

        let section = merged.first().expect("first merged");
        assert_eq!(section.node.diff_status, Some(DiffStatus::Modified));
        assert_eq!(section.children.len(), 2);
        assert_eq!(
            section
                .children
                .first()
                .expect("first child")
                .node
                .diff_status,
            Some(DiffStatus::Unchanged)
        );
        assert_eq!(
            section
                .children
                .get(1)
                .expect("second child")
                .node
                .diff_status,
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

        let section = merged.first().expect("first merged");
        assert_eq!(section.node.diff_status, Some(DiffStatus::Modified));
        // Child1 (Unchanged) + Child2 (Removed, appended at end)
        assert_eq!(section.children.len(), 2);
        assert_eq!(
            section
                .children
                .first()
                .expect("first child")
                .node
                .diff_status,
            Some(DiffStatus::Unchanged)
        );
        assert_eq!(
            section
                .children
                .get(1)
                .expect("second child")
                .node
                .diff_status,
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

        assert_eq!(
            merged.first().expect("first merged").node.diff_status,
            Some(DiffStatus::Modified)
        );
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

        let parent = merged.first().expect("first merged");
        assert_eq!(parent.node.diff_status, Some(DiffStatus::Modified));
        assert_eq!(
            parent
                .children
                .first()
                .expect("first child")
                .node
                .diff_status,
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
                    own_changed: true,
                },
                MergedNode {
                    node: {
                        let mut n = make_node("Removed", 1, false);
                        n.diff_status = Some(DiffStatus::Removed);
                        n
                    },
                    children: Vec::new(),
                    own_changed: true,
                },
                MergedNode {
                    node: {
                        let mut n = make_node("Unchanged", 1, false);
                        n.diff_status = Some(DiffStatus::Unchanged);
                        n
                    },
                    children: Vec::new(),
                    own_changed: false,
                },
            ],
            own_changed: false,
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
                    own_changed: false,
                }],
                own_changed: false,
            }],
            own_changed: false,
        }];

        let flat = flatten_merged(&merged, 0);
        assert_eq!(flat.len(), 3);
        let f0 = flat.first().expect("first flat");
        let f1 = flat.get(1).expect("second flat");
        let f2 = flat.get(2).expect("third flat");
        assert_eq!(f0.depth, 0);
        assert_eq!(f0.text, "Root");
        assert_eq!(f1.depth, 1);
        assert_eq!(f1.text, "Child");
        assert_eq!(f2.depth, 2);
        assert_eq!(f2.text, "Grandchild");
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
        let c0 = merged.children.first().expect("first child");
        let c1 = merged.children.get(1).expect("second child");
        assert_eq!(merged.node.diff_status, Some(DiffStatus::Added));
        assert_eq!(c0.node.diff_status, Some(DiffStatus::Added));
        assert_eq!(
            c0.children
                .first()
                .expect("first grandchild")
                .node
                .diff_status,
            Some(DiffStatus::Added)
        );
        assert_eq!(c1.node.diff_status, Some(DiffStatus::Added));
    }

    /// A deep change should only propagate one level: the leaf's parent is
    /// Modified, but the grandparent stays Unchanged because the parent has
    /// no own content change.
    #[test]
    fn deep_change_does_not_propagate_to_grandparent() {
        // Root > Mid > Leaf (Leaf has a section change)
        let old_tree = vec![
            make_node("Root", 0, true),
            make_node("Mid", 1, true),
            make_node("Leaf", 2, false),
        ];
        let new_tree = vec![
            make_node("Root", 0, true),
            make_node("Mid", 1, true),
            make_node_with_sections(
                "Leaf",
                2,
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

        let root = merged.first().expect("root");
        let mid = root.children.first().expect("mid");
        let leaf = mid.children.first().expect("leaf");
        // Leaf: own content changed → Modified
        assert_eq!(leaf.node.diff_status, Some(DiffStatus::Modified));
        // Mid: direct child (Leaf) has own_changed → Modified
        assert_eq!(mid.node.diff_status, Some(DiffStatus::Modified));
        // Root: direct child (Mid) has own_changed=false → Unchanged
        assert_eq!(root.node.diff_status, Some(DiffStatus::Unchanged));
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
        let diag_comms = merged.first().expect("first merged");
        assert_eq!(diag_comms.children.len(), 2);

        // ReadDID is Unchanged (same text)
        assert_eq!(
            diag_comms
                .children
                .first()
                .expect("first child")
                .node
                .diff_status,
            Some(DiffStatus::Unchanged)
        );
        // WriteDID is Modified (ID changed: 0x2E01 → 0x2E02)
        assert_eq!(
            diag_comms
                .children
                .get(1)
                .expect("second child")
                .node
                .diff_status,
            Some(DiffStatus::Modified)
        );
    }
}
