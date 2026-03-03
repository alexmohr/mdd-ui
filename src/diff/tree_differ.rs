/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

//! Tree differencing logic for comparing two MDD tree structures.
//!
//! This module performs exhaustive comparison of all node properties,
//! detail sections, table rows, cell types, jump targets, metadata,
//! and all other attributes - nothing is skipped.

use std::collections::{HashMap, HashSet};

use crate::tree::{
    CellJumpTarget, ColumnConstraint, DetailContent, DetailRow, DetailSectionData, NodeType,
    SectionType, ServiceListType, TreeNode,
};

/// Represents the result of comparing two trees.
#[derive(Debug)]
pub struct DiffResult {
    /// Nodes present only in the base tree.
    pub only_in_base: Vec<DiffNode>,
    /// Nodes present only in the compare tree.
    pub only_in_compare: Vec<DiffNode>,
    /// Nodes present in both trees but with differences.
    pub modified: Vec<ModifiedNode>,
    /// Summary statistics.
    pub stats: DiffStats,
}

/// Summary statistics for the diff.
#[derive(Debug, Default)]
pub struct DiffStats {
    pub total_base_nodes: usize,
    pub total_compare_nodes: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub unchanged_count: usize,
}

/// A node that exists in only one tree.
#[derive(Debug)]
pub struct DiffNode {
    /// The hierarchical path to this node.
    pub path: String,
    /// The node's display text.
    pub text: String,
    /// The node type.
    pub node_type: NodeType,
    /// Full details of this node for comprehensive output.
    pub details: NodeDetails,
}

/// Complete details of a node for diff output.
#[derive(Debug)]
pub struct NodeDetails {
    pub depth: usize,
    pub has_children: bool,
    pub section_type: Option<SectionType>,
    pub service_list_type: Option<ServiceListType>,
    pub param_id: Option<u32>,
    pub parent_ref_names: Vec<String>,
    pub detail_section_count: usize,
    pub detail_section_titles: Vec<String>,
}

/// A node that exists in both trees but has differences.
#[derive(Debug)]
pub struct ModifiedNode {
    /// The hierarchical path to this node.
    pub path: String,
    /// Changes detected in this node.
    pub changes: Vec<NodeChange>,
}

/// A specific change in a node's content.
#[derive(Debug, Clone)]
pub enum NodeChange {
    /// A property of the node itself changed.
    PropertyChanged {
        property: String,
        base: String,
        compare: String,
    },
    /// A detail section was added.
    SectionAdded {
        section: String,
        details: Vec<String>,
    },
    /// A detail section was removed.
    SectionRemoved {
        section: String,
        details: Vec<String>,
    },
    /// A detail section's content changed.
    SectionModified {
        section: String,
        changes: Vec<SectionChange>,
    },
}

/// A specific change within a detail section.
#[derive(Debug, Clone)]
pub enum SectionChange {
    /// Section property changed (e.g., `render_as_header`, `section_type`).
    PropertyChanged {
        property: String,
        base: String,
        compare: String,
    },
    /// A row was added.
    RowAdded { row_summary: String },
    /// A row was removed.
    RowRemoved { row_summary: String },
    /// A row was modified.
    RowModified {
        row_index: usize,
        changes: Vec<RowChange>,
    },
    /// Table header changed.
    HeaderChanged { changes: Vec<RowChange> },
    /// Table constraints changed.
    ConstraintsChanged { base: String, compare: String },
    /// Content type changed (e.g., `PlainText` to `Table`).
    ContentTypeChanged { base: String, compare: String },
    /// Plain text line added.
    LineAdded { line: String },
    /// Plain text line removed.
    LineRemoved { line: String },
    /// Subsection added (in Composite).
    SubsectionAdded { title: String },
    /// Subsection removed (in Composite).
    SubsectionRemoved { title: String },
    /// Subsection modified (in Composite).
    SubsectionModified {
        title: String,
        changes: Vec<SectionChange>,
    },
}

/// A specific change within a row.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum RowChange {
    /// Cell value changed.
    CellValueChanged {
        column: usize,
        base: String,
        compare: String,
    },
    /// Cell type changed.
    CellTypeChanged {
        column: usize,
        base: String,
        compare: String,
    },
    /// Cell jump target changed.
    CellJumpTargetChanged {
        column: usize,
        base: String,
        compare: String,
    },
    /// Row indent changed.
    IndentChanged { base: usize, compare: usize },
    /// Row type changed.
    RowTypeChanged { base: String, compare: String },
    /// Row metadata changed.
    MetadataChanged { base: String, compare: String },
    /// Column count changed.
    ColumnCountChanged { base: usize, compare: usize },
}

/// Compares two tree structures and computes their differences.
pub struct TreeDiffer<'a> {
    base_nodes: &'a [TreeNode],
    compare_nodes: &'a [TreeNode],
}

impl<'a> TreeDiffer<'a> {
    pub fn new(base_nodes: &'a [TreeNode], compare_nodes: &'a [TreeNode]) -> Self {
        Self {
            base_nodes,
            compare_nodes,
        }
    }

    /// Compute the diff between the two trees.
    pub fn compute_diff(&self) -> DiffResult {
        let base_map = Self::build_path_map(self.base_nodes);
        let compare_map = Self::build_path_map(self.compare_nodes);

        let base_paths: HashSet<_> = base_map.keys().cloned().collect();
        let compare_paths: HashSet<_> = compare_map.keys().cloned().collect();

        let only_in_base_paths: Vec<_> = base_paths.difference(&compare_paths).collect();
        let only_in_compare_paths: Vec<_> = compare_paths.difference(&base_paths).collect();
        let common_paths: Vec<_> = base_paths.intersection(&compare_paths).collect();

        let only_in_base: Vec<DiffNode> = only_in_base_paths
            .iter()
            .filter_map(|path| {
                base_map
                    .get(*path)
                    .map(|node| Self::node_to_diff_node(path, node))
            })
            .collect();

        let only_in_compare: Vec<DiffNode> = only_in_compare_paths
            .iter()
            .filter_map(|path| {
                compare_map
                    .get(*path)
                    .map(|node| Self::node_to_diff_node(path, node))
            })
            .collect();

        let mut modified = Vec::new();
        let mut unchanged_count = 0usize;

        for path in &common_paths {
            let Some(base_node) = base_map.get(*path) else {
                continue;
            };
            let Some(compare_node) = compare_map.get(*path) else {
                continue;
            };

            let changes = Self::compare_nodes(base_node, compare_node);
            if changes.is_empty() {
                unchanged_count = unchanged_count.saturating_add(1);
            } else {
                modified.push(ModifiedNode {
                    path: (*path).clone(),
                    changes,
                });
            }
        }

        let stats = DiffStats {
            total_base_nodes: self.base_nodes.len(),
            total_compare_nodes: self.compare_nodes.len(),
            added_count: only_in_compare.len(),
            removed_count: only_in_base.len(),
            modified_count: modified.len(),
            unchanged_count,
        };

        DiffResult {
            only_in_base,
            only_in_compare,
            modified,
            stats,
        }
    }

    fn node_to_diff_node(path: &str, node: &TreeNode) -> DiffNode {
        DiffNode {
            path: path.to_owned(),
            text: node.text.clone(),
            node_type: node.node_type,
            details: NodeDetails {
                depth: node.depth,
                has_children: node.has_children,
                section_type: node.section_type,
                service_list_type: node.service_list_type,
                param_id: node.param_id,
                parent_ref_names: node.parent_ref_names.clone(),
                detail_section_count: node.detail_sections.len(),
                detail_section_titles: node
                    .detail_sections
                    .iter()
                    .map(|s| s.title.clone())
                    .collect(),
            },
        }
    }

    /// Build a map from hierarchical path to node reference.
    fn build_path_map(nodes: &'a [TreeNode]) -> HashMap<String, &'a TreeNode> {
        let mut map = HashMap::new();
        let mut path_stack: Vec<String> = Vec::new();

        for node in nodes {
            // Adjust path stack based on depth
            while path_stack.len() > node.depth {
                path_stack.pop();
            }

            // Build the full path
            let full_path = if path_stack.is_empty() {
                node.text.clone()
            } else {
                format!("{} > {}", path_stack.join(" > "), node.text)
            };

            map.insert(full_path.clone(), node);

            // If this node has children, add it to the path stack
            if node.has_children {
                if path_stack.len() == node.depth {
                    path_stack.push(node.text.clone());
                } else {
                    while path_stack.len() > node.depth {
                        path_stack.pop();
                    }
                    path_stack.push(node.text.clone());
                }
            }
        }

        map
    }

    /// Compare two nodes exhaustively.
    fn compare_nodes(base: &TreeNode, compare: &TreeNode) -> Vec<NodeChange> {
        let mut changes = Vec::new();

        // Compare all node properties
        if base.depth != compare.depth {
            changes.push(NodeChange::PropertyChanged {
                property: "depth".to_owned(),
                base: base.depth.to_string(),
                compare: compare.depth.to_string(),
            });
        }

        if base.text != compare.text {
            changes.push(NodeChange::PropertyChanged {
                property: "text".to_owned(),
                base: base.text.clone(),
                compare: compare.text.clone(),
            });
        }

        if base.has_children != compare.has_children {
            changes.push(NodeChange::PropertyChanged {
                property: "has_children".to_owned(),
                base: base.has_children.to_string(),
                compare: compare.has_children.to_string(),
            });
        }

        if base.node_type != compare.node_type {
            changes.push(NodeChange::PropertyChanged {
                property: "node_type".to_owned(),
                base: format!("{:?}", base.node_type),
                compare: format!("{:?}", compare.node_type),
            });
        }

        if base.section_type != compare.section_type {
            changes.push(NodeChange::PropertyChanged {
                property: "section_type".to_owned(),
                base: format!("{:?}", base.section_type),
                compare: format!("{:?}", compare.section_type),
            });
        }

        if base.service_list_type != compare.service_list_type {
            changes.push(NodeChange::PropertyChanged {
                property: "service_list_type".to_owned(),
                base: format!("{:?}", base.service_list_type),
                compare: format!("{:?}", compare.service_list_type),
            });
        }

        if base.param_id != compare.param_id {
            changes.push(NodeChange::PropertyChanged {
                property: "param_id".to_owned(),
                base: format!("{:?}", base.param_id),
                compare: format!("{:?}", compare.param_id),
            });
        }

        if base.parent_ref_names != compare.parent_ref_names {
            changes.push(NodeChange::PropertyChanged {
                property: "parent_ref_names".to_owned(),
                base: format!("{:?}", base.parent_ref_names),
                compare: format!("{:?}", compare.parent_ref_names),
            });
        }

        // Compare detail sections exhaustively
        Self::compare_detail_sections(
            &base.detail_sections,
            &compare.detail_sections,
            &mut changes,
        );

        changes
    }

    /// Compare detail sections exhaustively.
    fn compare_detail_sections(
        base_sections: &[DetailSectionData],
        compare_sections: &[DetailSectionData],
        changes: &mut Vec<NodeChange>,
    ) {
        let base_section_map: HashMap<_, _> =
            base_sections.iter().map(|s| (s.title.clone(), s)).collect();
        let compare_section_map: HashMap<_, _> = compare_sections
            .iter()
            .map(|s| (s.title.clone(), s))
            .collect();

        let base_titles: HashSet<_> = base_section_map.keys().cloned().collect();
        let compare_titles: HashSet<_> = compare_section_map.keys().cloned().collect();

        // Sections only in base (removed)
        for title in base_titles.difference(&compare_titles) {
            let Some(section) = base_section_map.get(title) else {
                continue;
            };
            changes.push(NodeChange::SectionRemoved {
                section: title.clone(),
                details: Self::section_summary(section),
            });
        }

        // Sections only in compare (added)
        for title in compare_titles.difference(&base_titles) {
            let Some(section) = compare_section_map.get(title) else {
                continue;
            };
            changes.push(NodeChange::SectionAdded {
                section: title.clone(),
                details: Self::section_summary(section),
            });
        }

        // Sections in both - compare exhaustively
        for title in base_titles.intersection(&compare_titles) {
            let Some(base_section) = base_section_map.get(title) else {
                continue;
            };
            let Some(compare_section) = compare_section_map.get(title) else {
                continue;
            };

            let section_changes = Self::compare_section(base_section, compare_section);
            if !section_changes.is_empty() {
                changes.push(NodeChange::SectionModified {
                    section: title.clone(),
                    changes: section_changes,
                });
            }
        }
    }

    /// Generate a summary of a section's content.
    fn section_summary(section: &DetailSectionData) -> Vec<String> {
        let mut summary = vec![
            format!("  Title: {}", section.title),
            format!("  Type: {:?}", section.section_type),
            format!("  Render as header: {}", section.render_as_header),
        ];

        match &section.content {
            DetailContent::PlainText(lines) => {
                summary.push(format!("  Content: PlainText with {} lines", lines.len()));
                for line in lines.iter().take(5) {
                    summary.push(format!("    {line}"));
                }
                if lines.len() > 5 {
                    summary.push(format!(
                        "    ... and {} more lines",
                        lines.len().saturating_sub(5)
                    ));
                }
            }
            DetailContent::Table {
                header,
                rows,
                constraints,
                use_row_selection,
            } => {
                summary.push(format!(
                    "  Content: Table with {} rows, {} columns",
                    rows.len(),
                    header.cells.len()
                ));
                summary.push(format!("    Header: {}", header.cells.join(" | ")));
                summary.push(format!("    Constraints: {}", constraints.len()));
                summary.push(format!("    Row selection: {use_row_selection}"));
                for row in rows.iter().take(3) {
                    summary.push(format!("    Row: {}", row.cells.join(" | ")));
                }
                if rows.len() > 3 {
                    summary.push(format!(
                        "    ... and {} more rows",
                        rows.len().saturating_sub(3)
                    ));
                }
            }
            DetailContent::Composite(subs) => {
                summary.push(format!(
                    "  Content: Composite with {} subsections",
                    subs.len()
                ));
                for sub in subs {
                    summary.push(format!("    Subsection: {}", sub.title));
                }
            }
        }

        summary
    }

    /// Compare two sections exhaustively.
    fn compare_section(
        base: &DetailSectionData,
        compare: &DetailSectionData,
    ) -> Vec<SectionChange> {
        let mut changes = Vec::new();

        // Compare section properties
        if base.render_as_header != compare.render_as_header {
            changes.push(SectionChange::PropertyChanged {
                property: "render_as_header".to_owned(),
                base: base.render_as_header.to_string(),
                compare: compare.render_as_header.to_string(),
            });
        }

        if base.section_type != compare.section_type {
            changes.push(SectionChange::PropertyChanged {
                property: "section_type".to_owned(),
                base: format!("{:?}", base.section_type),
                compare: format!("{:?}", compare.section_type),
            });
        }

        // Compare content
        Self::compare_content(&base.content, &compare.content, &mut changes);

        changes
    }

    /// Compare content exhaustively.
    fn compare_content(
        base: &DetailContent,
        compare: &DetailContent,
        changes: &mut Vec<SectionChange>,
    ) {
        match (base, compare) {
            (DetailContent::PlainText(base_lines), DetailContent::PlainText(compare_lines)) => {
                Self::compare_plain_text(base_lines, compare_lines, changes);
            }
            (
                DetailContent::Table {
                    header: base_header,
                    rows: base_rows,
                    constraints: base_constraints,
                    use_row_selection: base_selection,
                },
                DetailContent::Table {
                    header: compare_header,
                    rows: compare_rows,
                    constraints: compare_constraints,
                    use_row_selection: compare_selection,
                },
            ) => {
                Self::compare_table(
                    base_header,
                    base_rows,
                    base_constraints,
                    *base_selection,
                    compare_header,
                    compare_rows,
                    compare_constraints,
                    *compare_selection,
                    changes,
                );
            }
            (DetailContent::Composite(base_subs), DetailContent::Composite(compare_subs)) => {
                Self::compare_composite(base_subs, compare_subs, changes);
            }
            _ => {
                changes.push(SectionChange::ContentTypeChanged {
                    base: Self::content_type_name(base),
                    compare: Self::content_type_name(compare),
                });
            }
        }
    }

    fn content_type_name(content: &DetailContent) -> String {
        match content {
            DetailContent::PlainText(lines) => format!("PlainText({} lines)", lines.len()),
            DetailContent::Table { rows, .. } => format!("Table({} rows)", rows.len()),
            DetailContent::Composite(subs) => format!("Composite({} subs)", subs.len()),
        }
    }

    /// Compare plain text content.
    fn compare_plain_text(
        base_lines: &[String],
        compare_lines: &[String],
        changes: &mut Vec<SectionChange>,
    ) {
        let base_set: HashSet<_> = base_lines.iter().collect();
        let compare_set: HashSet<_> = compare_lines.iter().collect();

        for line in base_set.difference(&compare_set) {
            changes.push(SectionChange::LineRemoved {
                line: (*line).clone(),
            });
        }

        for line in compare_set.difference(&base_set) {
            changes.push(SectionChange::LineAdded {
                line: (*line).clone(),
            });
        }
    }

    /// Compare table content exhaustively.
    #[allow(clippy::too_many_arguments)]
    fn compare_table(
        base_header: &DetailRow,
        base_rows: &[DetailRow],
        base_constraints: &[ColumnConstraint],
        base_selection: bool,
        compare_header: &DetailRow,
        compare_rows: &[DetailRow],
        compare_constraints: &[ColumnConstraint],
        compare_selection: bool,
        changes: &mut Vec<SectionChange>,
    ) {
        // Compare header
        let header_changes = Self::compare_rows(base_header, compare_header);
        if !header_changes.is_empty() {
            changes.push(SectionChange::HeaderChanged {
                changes: header_changes,
            });
        }

        // Compare constraints
        if !Self::constraints_equal(base_constraints, compare_constraints) {
            changes.push(SectionChange::ConstraintsChanged {
                base: Self::constraints_to_string(base_constraints),
                compare: Self::constraints_to_string(compare_constraints),
            });
        }

        // Compare use_row_selection
        if base_selection != compare_selection {
            changes.push(SectionChange::PropertyChanged {
                property: "use_row_selection".to_owned(),
                base: base_selection.to_string(),
                compare: compare_selection.to_string(),
            });
        }

        // Compare rows - build a map by row content for matching
        let base_row_map: HashMap<String, (usize, &DetailRow)> = base_rows
            .iter()
            .enumerate()
            .map(|(i, r)| (r.cells.join("\0"), (i, r)))
            .collect();

        let compare_row_map: HashMap<String, (usize, &DetailRow)> = compare_rows
            .iter()
            .enumerate()
            .map(|(i, r)| (r.cells.join("\0"), (i, r)))
            .collect();

        let base_keys: HashSet<_> = base_row_map.keys().cloned().collect();
        let compare_keys: HashSet<_> = compare_row_map.keys().cloned().collect();

        // Rows only in base (removed)
        for key in base_keys.difference(&compare_keys) {
            if let Some((_, row)) = base_row_map.get(key) {
                changes.push(SectionChange::RowRemoved {
                    row_summary: Self::row_summary(row),
                });
            }
        }

        // Rows only in compare (added)
        for key in compare_keys.difference(&base_keys) {
            if let Some((_, row)) = compare_row_map.get(key) {
                changes.push(SectionChange::RowAdded {
                    row_summary: Self::row_summary(row),
                });
            }
        }

        // Rows in both - compare exhaustively (even if cells match, other properties might differ)
        for key in base_keys.intersection(&compare_keys) {
            let Some((base_idx, base_row)) = base_row_map.get(key) else {
                continue;
            };
            let Some((_, compare_row)) = compare_row_map.get(key) else {
                continue;
            };

            let row_changes = Self::compare_rows(base_row, compare_row);
            if !row_changes.is_empty() {
                changes.push(SectionChange::RowModified {
                    row_index: *base_idx,
                    changes: row_changes,
                });
            }
        }
    }

    fn row_summary(row: &DetailRow) -> String {
        format!(
            "{} (type={:?}, indent={}, cells={}, metadata={:?})",
            row.cells.join(" | "),
            row.row_type,
            row.indent,
            row.cells.len(),
            row.metadata
        )
    }

    /// Compare two rows exhaustively.
    fn compare_rows(base: &DetailRow, compare: &DetailRow) -> Vec<RowChange> {
        let mut changes = Vec::new();

        // Compare column count
        if base.cells.len() != compare.cells.len() {
            changes.push(RowChange::ColumnCountChanged {
                base: base.cells.len(),
                compare: compare.cells.len(),
            });
        }

        // Compare cell values
        let max_cells = base.cells.len().max(compare.cells.len());
        for i in 0..max_cells {
            let base_cell = base.cells.get(i).map_or("", String::as_str);
            let compare_cell = compare.cells.get(i).map_or("", String::as_str);
            if base_cell != compare_cell {
                changes.push(RowChange::CellValueChanged {
                    column: i,
                    base: base_cell.to_owned(),
                    compare: compare_cell.to_owned(),
                });
            }
        }

        // Compare cell types
        let max_types = base.cell_types.len().max(compare.cell_types.len());
        for i in 0..max_types {
            let base_type = base.cell_types.get(i);
            let compare_type = compare.cell_types.get(i);
            if base_type != compare_type {
                changes.push(RowChange::CellTypeChanged {
                    column: i,
                    base: format!("{base_type:?}"),
                    compare: format!("{compare_type:?}"),
                });
            }
        }

        // Compare cell jump targets
        let max_targets = base
            .cell_jump_targets
            .len()
            .max(compare.cell_jump_targets.len());
        for i in 0..max_targets {
            let base_target = base.cell_jump_targets.get(i).and_then(|t| t.as_ref());
            let compare_target = compare.cell_jump_targets.get(i).and_then(|t| t.as_ref());
            if !Self::jump_targets_equal(base_target, compare_target) {
                changes.push(RowChange::CellJumpTargetChanged {
                    column: i,
                    base: Self::jump_target_to_string(base_target),
                    compare: Self::jump_target_to_string(compare_target),
                });
            }
        }

        // Compare indent
        if base.indent != compare.indent {
            changes.push(RowChange::IndentChanged {
                base: base.indent,
                compare: compare.indent,
            });
        }

        // Compare row type
        if base.row_type != compare.row_type {
            changes.push(RowChange::RowTypeChanged {
                base: format!("{:?}", base.row_type),
                compare: format!("{:?}", compare.row_type),
            });
        }

        // Compare metadata
        if base.metadata != compare.metadata {
            changes.push(RowChange::MetadataChanged {
                base: format!("{:?}", base.metadata),
                compare: format!("{:?}", compare.metadata),
            });
        }

        changes
    }

    fn jump_targets_equal(base: Option<&CellJumpTarget>, compare: Option<&CellJumpTarget>) -> bool {
        match (base, compare) {
            (None, None) => true,
            (Some(b), Some(c)) => b == c,
            _ => false,
        }
    }

    fn jump_target_to_string(target: Option<&CellJumpTarget>) -> String {
        target.map_or_else(|| "None".to_owned(), |t| format!("{t:?}"))
    }

    fn constraints_equal(base: &[ColumnConstraint], compare: &[ColumnConstraint]) -> bool {
        if base.len() != compare.len() {
            return false;
        }
        base.iter()
            .zip(compare.iter())
            .all(|(b, c)| Self::constraint_equal(b, c))
    }

    fn constraint_equal(base: &ColumnConstraint, compare: &ColumnConstraint) -> bool {
        match (base, compare) {
            (ColumnConstraint::Fixed(b), ColumnConstraint::Fixed(c))
            | (ColumnConstraint::Percentage(b), ColumnConstraint::Percentage(c)) => b == c,
            _ => false,
        }
    }

    fn constraints_to_string(constraints: &[ColumnConstraint]) -> String {
        constraints
            .iter()
            .map(|c| match c {
                ColumnConstraint::Fixed(w) => format!("Fixed({w})"),
                ColumnConstraint::Percentage(p) => format!("Pct({p})"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Compare composite content exhaustively.
    fn compare_composite(
        base_subs: &[DetailSectionData],
        compare_subs: &[DetailSectionData],
        changes: &mut Vec<SectionChange>,
    ) {
        let base_map: HashMap<_, _> = base_subs.iter().map(|s| (s.title.clone(), s)).collect();
        let compare_map: HashMap<_, _> =
            compare_subs.iter().map(|s| (s.title.clone(), s)).collect();

        let base_titles: HashSet<_> = base_map.keys().cloned().collect();
        let compare_titles: HashSet<_> = compare_map.keys().cloned().collect();

        // Subsections only in base
        for title in base_titles.difference(&compare_titles) {
            changes.push(SectionChange::SubsectionRemoved {
                title: title.clone(),
            });
        }

        // Subsections only in compare
        for title in compare_titles.difference(&base_titles) {
            changes.push(SectionChange::SubsectionAdded {
                title: title.clone(),
            });
        }

        // Subsections in both
        for title in base_titles.intersection(&compare_titles) {
            let Some(base_sub) = base_map.get(title) else {
                continue;
            };
            let Some(compare_sub) = compare_map.get(title) else {
                continue;
            };

            let sub_changes = Self::compare_section(base_sub, compare_sub);
            if !sub_changes.is_empty() {
                changes.push(SectionChange::SubsectionModified {
                    title: title.clone(),
                    changes: sub_changes,
                });
            }
        }
    }
}
