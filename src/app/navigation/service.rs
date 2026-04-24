/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crate::{
    app::{App, FocusState, SCROLL_CONTEXT_LINES},
    tree::{DetailSectionType, NodeTextPrefix, NodeType, TreeNode},
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

        // Get service name from table or return early
        let Some(service_name) = self.extract_service_name_from_table(node_idx) else {
            return;
        };

        // Expand service list section if collapsed
        if let Some(node_at_idx) = self.tree.all_nodes.get(node_idx)
            && !node_at_idx.expanded
        {
            self.expand_and_update_cursor(node_idx);
        }

        // Find and navigate to service
        self.find_and_navigate_to_service(&service_name, node_idx);
    }

    /// Extract service name from the current table row
    pub(super) fn extract_service_name_from_table(&mut self, node_idx: usize) -> Option<String> {
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

        // Determine name column index based on node type
        let is_functional_class =
            Self::is_service_list_type(node, crate::tree::ServiceListType::FunctionalClasses);
        let name_column_index = usize::from(!is_functional_class);

        selected_row.cells.get(name_column_index).cloned()
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
            node.node_type == NodeType::FunctionalClass && node.text == target_name
        } else {
            if !node.node_type.is_diagcomm() {
                return false;
            }

            // Prefer the structured field; fall back to text matching
            if let Some(sn) = node.service_short_name() {
                sn == target_name
            } else if node.node_type == NodeType::Job {
                let job_name = node
                    .text
                    .strip_prefix(NodeTextPrefix::Job.as_str())
                    .unwrap_or(&node.text);
                job_name == target_name
            } else {
                node.text.contains(target_name)
            }
        }
    }

    /// Check if node is a service-related node
    pub(super) fn is_service_node(node: &TreeNode) -> bool {
        node.node_type.is_service()
    }

    /// Extract service name from node text.
    ///
    /// Prefers the pre-computed `service_short_name` field, falling back to
    /// parsing the display text for nodes that lack it.
    pub(super) fn extract_service_name_from_node(node: &TreeNode) -> String {
        if let Some(sn) = node.service_short_name() {
            return sn.to_owned();
        }
        node.text.find(" - ").map_or_else(
            || node.text.clone(),
            |dash_idx| node.text[dash_idx.saturating_add(3)..].to_string(),
        )
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
                .find(|(_, node)| node.text.contains(service_name))
                .map(|(offset, _)| diagcomm_idx.saturating_add(1).saturating_add(offset))
        } else {
            None
        }
    }

    /// Helper function to navigate to a service or job by name.
    /// Searches within the current container's hierarchy first, then globally.
    pub(super) fn navigate_to_service_or_job(&mut self, target_short_name: &str) {
        let matches_service = |n: &TreeNode| -> bool {
            if !n.node_type.is_diagcomm() {
                return false;
            }
            // Prefer the structured field; fall back to text parsing
            if let Some(sn) = n.service_short_name() {
                sn == target_short_name
            } else if matches!(n.node_type, NodeType::Service | NodeType::ParentRefService) {
                let service_name = n
                    .text
                    .find(" - ")
                    .map_or(n.text.as_str(), |idx| &n.text[idx.saturating_add(3)..]);
                service_name == target_short_name
            } else if n.node_type == NodeType::Job {
                let job_name = n
                    .text
                    .strip_prefix(NodeTextPrefix::Job.as_str())
                    .unwrap_or(&n.text);
                job_name == target_short_name
            } else {
                false
            }
        };

        let Some(service_node_idx) = self.find_in_hierarchy(matches_service) else {
            self.status = format!("Service/Job '{target_short_name}' not found in tree");
            return;
        };

        self.ensure_node_visible(service_node_idx);

        if let Some(new_cursor) = self
            .tree
            .visible
            .iter()
            .position(|&idx| idx == service_node_idx)
        {
            self.push_to_history();
            self.focus_state = FocusState::Tree;
            self.tree.cursor = new_cursor;
            self.reset_detail_state();
            self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
            self.status = format!("Navigated to: {target_short_name}");
        }
    }
}
