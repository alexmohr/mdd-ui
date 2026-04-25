/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crate::{
    app::App,
    tree::{CellJumpTargetType, DetailRowType, NodeType, RowMetadata},
};

impl App {
    /// Navigate to an inherited parent layer in the tree
    pub(crate) fn try_navigate_to_inherited_parent(&mut self) {
        // Early validations
        if self.tree.cursor >= self.tree.visible.len() {
            return;
        }

        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };

        if !Self::is_service_node(node) {
            self.status = "Not a service node".into();
            return;
        }

        // Extract current service name and parent layer name
        let current_service_name = Self::extract_service_name_from_node(node);
        let Some(parent_layer_name) = self.get_parent_layer_name(node_idx) else {
            return;
        };

        // Find parent container: prefer resolved index from jump target,
        // fall back to name lookup.
        let container_idx = self
            .get_inherited_from_container_index(node_idx)
            .or_else(|| self.find_container_by_name(&parent_layer_name));

        if let Some(container_idx) = container_idx {
            self.navigate_to_parent_service(container_idx, &current_service_name);
        } else {
            self.status = format!("Parent layer '{parent_layer_name}' not found in tree");
        }
    }

    /// Get parent layer name from the Overview section's "Inherited From" row
    pub(super) fn get_parent_layer_name(&self, node_idx: usize) -> Option<String> {
        let node = self.tree.all_nodes.get(node_idx)?;

        let overview_idx = usize::from(
            node.detail_sections.len() > 1
                && node
                    .detail_sections
                    .first()
                    .is_some_and(|s| s.render_as_header),
        );

        let overview_section = node.detail_sections.get(overview_idx)?;

        let rows = overview_section.content.table_rows()?;

        let row_cursor = self
            .detail
            .section_cursors
            .get(overview_idx)
            .map_or(0, |&c| c);
        let sorted_rows = self.sort_rows(rows, overview_idx);
        let selected_row = sorted_rows.get(row_cursor)?;

        if selected_row.row_type != DetailRowType::InheritedFrom {
            return None;
        }

        // Extract from metadata or fallback to cell data
        match &selected_row.metadata {
            Some(RowMetadata::InheritedFrom { layer_name }) => Some(layer_name.clone()),
            None | Some(RowMetadata::ChildElement { .. } | RowMetadata::ParameterRow { .. }) => {
                selected_row.cells.get(1).map(|c| c.text.clone())
            }
        }
    }

    /// Navigate to a parent ref target when pressing Enter on a parent ref child
    /// in the tree pane. Returns `true` if navigation was attempted.
    pub(crate) fn try_navigate_parent_ref_from_tree(&mut self) -> bool {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return false;
        };

        // Check if the current node's direct parent is a ParentRefs node.
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return false;
        };
        let Some(parent_idx) = node.parent_idx else {
            return false;
        };
        let parent_is_parent_refs = self
            .tree
            .all_nodes
            .get(parent_idx)
            .is_some_and(|p| p.node_type == NodeType::ParentRefs);

        if !parent_is_parent_refs {
            return false;
        }

        // The node text is the short name of the target
        let target_short_name = node.text.clone();
        self.navigate_to_container_by_name(&target_short_name);
        true
    }

    /// Navigate to a parent ref element from the Parent References table.
    /// Prefers the resolved index from the cell's jump target.
    pub(crate) fn try_navigate_to_parent_ref(&mut self) {
        let (cell_value, jump_index) = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let Some(name_cell) = selected_row.cells.first() else {
                return;
            };
            let idx = name_cell
                .jump_target
                .as_ref()
                .and_then(|jt| match &jt.target_type {
                    CellJumpTargetType::TreeNodeByIndex { index, .. } if *index != usize::MAX => {
                        Some(*index)
                    }
                    _ => None,
                });
            (name_cell.text.clone(), idx)
        };

        if let Some(idx) = jump_index {
            self.navigate_to_node(idx);
        } else {
            self.navigate_to_container_by_name(&cell_value);
        }
    }

    /// Extract the resolved container index from the "Inherited From" row's
    /// jump target, if it has been resolved (not `usize::MAX`).
    fn get_inherited_from_container_index(&self, node_idx: usize) -> Option<usize> {
        let node = self.tree.all_nodes.get(node_idx)?;
        let overview_idx = usize::from(
            node.detail_sections.len() > 1
                && node
                    .detail_sections
                    .first()
                    .is_some_and(|s| s.render_as_header),
        );
        let section = node.detail_sections.get(overview_idx)?;
        let rows = section.content.table_rows()?;
        let inherited_row = rows
            .iter()
            .find(|r| r.row_type == DetailRowType::InheritedFrom)?;
        let cell = inherited_row.cells.get(1)?;
        match cell.jump_target.as_ref()?.target_type {
            CellJumpTargetType::TreeNodeByIndex { index, .. } if index != usize::MAX => Some(index),
            _ => None,
        }
    }
}
