/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crate::{
    app::App,
    tree::{CellJumpTarget, CellJumpTargetType, DetailSectionType, NodeType, TreeNode},
};

impl App {
    /// Handle Enter key for service nodes
    pub(super) fn handle_service_node_enter(&mut self) {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Check for parameter table (requests/responses)
        if matches!(
            section.section_type,
            DetailSectionType::Requests
                | DetailSectionType::PosResponses
                | DetailSectionType::NegResponses
        ) {
            self.try_navigate_from_param_table();
            return;
        }

        // Check for Overview section with "Inherited From" row
        if section.section_type == DetailSectionType::Overview
            && let Some(rows) = section.content.table_rows()
        {
            let row_cursor = self
                .detail
                .section_cursors
                .get(section_idx)
                .map_or(0, |&c| c);
            let sorted_rows = self.sort_rows(rows, section_idx);

            if let Some(selected_row) = sorted_rows.get(row_cursor)
                && selected_row.row_type == crate::tree::DetailRowType::InheritedFrom
            {
                self.try_navigate_to_inherited_parent();
                return;
            }
        }

        // Try to navigate based on the current cell content
        self.try_navigate_from_detail_row();
    }

    /// Navigate to a service in the tree from a service list table
    /// (Diag-Comms, Requests, Responses)
    pub(crate) fn try_navigate_to_service(&mut self) {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };

        if !Self::is_service_list_section(node) {
            self.status = "Not a service list section".into();
            return;
        }

        let Some((service_name, jump_target)) = self.extract_service_jump_from_table(node_idx)
        else {
            return;
        };

        // Prefer direct index navigation when available.
        if let Some(CellJumpTarget {
            target_type: CellJumpTargetType::TreeNodeByIndex { index, short_name },
        }) = &jump_target
            && self
                .tree
                .all_nodes
                .get(*index)
                .and_then(|n| n.service_short_name())
                .is_some_and(|sn| sn == short_name)
        {
            self.navigate_to_node(*index);
            return;
        }

        // Expand service list section if collapsed
        if let Some(node_at_idx) = self.tree.all_nodes.get(node_idx)
            && !node_at_idx.expanded
        {
            self.expand_and_update_cursor(node_idx);
        }

        // Fallback: name-based search
        self.find_and_navigate_to_service(&service_name, node_idx);
    }

    /// Extract the jump target and short name from the selected service-list
    /// table row (column 0).
    fn extract_service_jump_from_table(
        &mut self,
        node_idx: usize,
    ) -> Option<(String, Option<CellJumpTarget>)> {
        let node = self.tree.all_nodes.get(node_idx)?;
        let section = node.detail_sections.first()?;

        let Some(rows) = section.content.table_rows() else {
            self.status = "Details should be a table".into();
            return None;
        };

        let section_index = self.get_section_index();
        let row_cursor = *self.detail.section_cursors.get(section_index)?;
        let sorted_rows = self.sort_rows(rows, section_index);
        let selected_row = sorted_rows.get(row_cursor)?;

        // Column 0 is always the short name across all service-list tables.
        let cell = selected_row.cells.first()?;
        Some((cell.text.clone(), cell.jump_target.clone()))
    }

    /// Find and navigate to a service by name
    pub(super) fn find_and_navigate_to_service(
        &mut self,
        service_name: &str,
        parent_node_idx: usize,
    ) {
        let Some(parent_node) = self.tree.all_nodes.get(parent_node_idx) else {
            return;
        };
        let parent_depth = parent_node.depth;
        let is_functional_class = Self::is_service_list_type(
            parent_node,
            crate::tree::ServiceListType::FunctionalClasses,
        );

        // Search all_nodes in parent's subtree (not just visible) so collapsed
        // services are found and their ancestors expanded automatically.
        let (start, end) = self.subtree_range(parent_node_idx);
        let found_idx = self.find_at_depth(start, end, parent_depth.saturating_add(1), &|node| {
            Self::node_matches_service_name(node, service_name, is_functional_class)
        });

        if let Some(node_idx) = found_idx {
            self.navigate_to_node(node_idx);
        } else {
            let item_type = if is_functional_class {
                "Functional class"
            } else {
                "Service"
            };
            self.status = format!("{item_type} '{service_name}' not found in tree");
        }
    }

    /// Check if a node's name matches the target service name
    pub(super) fn node_matches_service_name(
        node: &TreeNode,
        target_name: &str,
        is_functional_class: bool,
    ) -> bool {
        if is_functional_class {
            node.node_type == NodeType::FunctionalClass
                && node.short_name().is_some_and(|sn| sn == target_name)
        } else {
            node.node_type.is_diagcomm()
                && node
                    .service_short_name()
                    .is_some_and(|sn| sn == target_name)
        }
    }

    /// Check if node is a service-related node
    pub(super) fn is_service_node(node: &TreeNode) -> bool {
        node.node_type.is_service()
    }

    /// Return the canonical short name for a service / job node.
    pub(super) fn extract_service_name_from_node(node: &TreeNode) -> String {
        node.service_short_name()
            .or_else(|| node.short_name())
            .unwrap_or_default()
            .to_owned()
    }

    /// Navigate to parent service in the container
    pub(super) fn navigate_to_parent_service(&mut self, container_idx: usize, service_name: &str) {
        // Expand ancestors and container
        self.expand_node_ancestors(container_idx);

        if let Some(node) = self.tree.all_nodes.get(container_idx)
            && node.has_children
            && let Some(node_mut) = self.tree.all_nodes.get_mut(container_idx)
        {
            node_mut.expanded = true;
        }

        // Find Diag-Comms section
        let Some(dc_idx) = self.find_diagcomm_section(container_idx) else {
            self.rebuild_visible();
            self.navigate_to_node(container_idx);
            return;
        };

        if let Some(node_mut) = self.tree.all_nodes.get_mut(dc_idx) {
            node_mut.expanded = true;
        }
        self.rebuild_visible();

        let target = self
            .find_service_in_diagcomm(dc_idx, service_name)
            .unwrap_or(container_idx);
        self.navigate_to_node(target);
    }

    /// Find the Diag-Comms section within a container
    pub(super) fn find_diagcomm_section(&self, container_idx: usize) -> Option<usize> {
        let container = self.tree.all_nodes.get(container_idx)?;
        let container_depth = container.depth;

        if let Some(children_slice) = self.tree.all_nodes.get(container_idx.saturating_add(1)..) {
            children_slice
                .iter()
                .enumerate()
                .take_while(|(_, child)| child.depth > container_depth)
                .find(|(_, child)| {
                    child.depth == container_depth.saturating_add(1)
                        && Self::is_service_list_type(
                            child,
                            crate::tree::ServiceListType::DiagComms,
                        )
                })
                .map(|(offset, _)| container_idx.saturating_add(1).saturating_add(offset))
        } else {
            None
        }
    }

    /// Find a service by name within a Diag-Comms section
    pub(super) fn find_service_in_diagcomm(
        &self,
        diagcomm_idx: usize,
        service_name: &str,
    ) -> Option<usize> {
        let diagcomm_node = self.tree.all_nodes.get(diagcomm_idx)?;
        let diagcomm_depth = diagcomm_node.depth;

        if let Some(children_slice) = self.tree.all_nodes.get(diagcomm_idx.saturating_add(1)..) {
            children_slice
                .iter()
                .enumerate()
                .take_while(|(_, node)| node.depth > diagcomm_depth)
                .filter(|(_, node)| node.depth == diagcomm_depth.saturating_add(1))
                .find(|(_, node)| {
                    node.service_short_name()
                        .is_some_and(|sn| sn == service_name)
                })
                .map(|(offset, _)| diagcomm_idx.saturating_add(1).saturating_add(offset))
        } else {
            None
        }
    }
}
