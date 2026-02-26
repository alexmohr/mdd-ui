/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState};
use crate::tree::{
    CellJumpTarget, CellType, DetailRow, DetailRowType, DetailSectionType, NodeType, RowMetadata,
    SectionType, TreeNode,
};

impl App {
    pub(crate) fn handle_enter_in_detail_pane(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Early returns for different node types using functional matching
        if let Some(SectionType::Variants) = node.section_type {
            self.try_navigate_to_variant();
            return;
        }

        if matches!(node.node_type, NodeType::Container) && node.depth == 1 {
            self.try_navigate_from_variant_overview();
            return;
        }

        if node.service_list_type.is_some() {
            self.try_navigate_to_service();
            return;
        }

        if matches!(node.node_type, NodeType::FunctionalClass) {
            self.handle_functional_class_enter();
            return;
        }

        // ParentRefs overview: navigate to the selected parent ref container
        if matches!(node.node_type, NodeType::ParentRefs) {
            self.try_navigate_to_parent_ref();
            return;
        }

        // DIAG-DATA-DICTIONARY-SPEC, DOP category, and individual DOP nodes with children:
        // navigate to child instead of popup
        if matches!(node.node_type, NodeType::DOP)
            || self.is_dop_category_node(node_idx)
            || self.is_individual_dop_node(node_idx)
        {
            self.try_navigate_to_dop_child();
            return;
        }

        if matches!(
            node.node_type,
            NodeType::Service
                | NodeType::ParentRefService
                | NodeType::Request
                | NodeType::PosResponse
                | NodeType::NegResponse
        ) {
            self.handle_service_node_enter();
            return;
        }

        // Handle other node types with detail sections
        self.handle_generic_detail_enter();
    }

    /// Handle Enter key for functional class nodes
    fn handle_functional_class_enter(&mut self) {
        match self.focused_column {
            0 => self.try_navigate_to_service_from_functional_class(),
            5 => self.try_navigate_to_layer_from_functional_class(),
            _ => {}
        }
    }

    /// Handle Enter key for service nodes
    fn handle_service_node_enter(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

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
            let row_cursor = self.section_cursors.get(section_idx).copied().unwrap_or(0);
            let sorted_rows = self.apply_table_sort(rows, section_idx);

            if let Some(selected_row) = sorted_rows.get(row_cursor)
                && selected_row.row_type == DetailRowType::InheritedFrom
            {
                self.try_navigate_to_inherited_parent();
                return;
            }
        }

        // Try to navigate based on the current cell content
        self.try_navigate_from_detail_row();
    }

    /// Handle Enter key for generic nodes with detail sections
    fn handle_generic_detail_enter(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        let section = node
            .detail_sections
            .get(section_idx)
            .filter(|_| section_idx < node.detail_sections.len());

        if let Some(section) = section {
            if section.section_type == DetailSectionType::RelatedRefs
                && section.title == "Parent References"
            {
                self.try_navigate_to_parent_ref();
            } else if section.title.starts_with("Not Inherited") {
                self.try_navigate_to_not_inherited_element();
            } else {
                self.try_navigate_from_detail_row();
            }
        } else {
            "No details available".clone_into(&mut self.status);
        }
    }

    /// Try to navigate to the item referenced by the currently focused cell.
    /// Falls back to searching the tree for a node matching the cell text.
    pub(crate) fn try_navigate_from_detail_row(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        let Some((rows, use_row_selection)) = Self::get_table_rows(node, section_idx) else {
            "No table data".clone_into(&mut self.status);
            return;
        };

        let row_cursor = self.section_cursors.get(section_idx).copied().unwrap_or(0);
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        let focused_col = self.get_focused_column(use_row_selection, &selected_row.cell_types);
        let cell_value = selected_row
            .cells
            .get(focused_col)
            .map_or("", std::string::String::as_str);

        if cell_value.is_empty() || cell_value == "-" {
            "Empty cell".clone_into(&mut self.status);
            return;
        }

        // Use per-cell jump target metadata
        let jump_target = selected_row
            .cell_jump_targets
            .get(focused_col)
            .cloned()
            .flatten();
        self.execute_cell_jump(jump_target, cell_value);
    }

    /// Execute a cell jump based on the per-cell jump target metadata
    fn execute_cell_jump(&mut self, jump_target: Option<CellJumpTarget>, cell_value: &str) {
        let Some(target) = jump_target else {
            "This cell is not navigable".clone_into(&mut self.status);
            return;
        };

        match target {
            CellJumpTarget::Parameter { param_id } => {
                self.navigate_to_parameter_by_id(param_id);
            }
            CellJumpTarget::Dop { ref name } => {
                self.navigate_to_dop(name);
            }
            CellJumpTarget::TreeNodeByName => {
                let found_idx = self
                    .find_in_hierarchy(|n| n.text == cell_value)
                    .or_else(|| self.all_nodes.iter().position(|n| n.text == cell_value));
                if let Some(idx) = found_idx {
                    self.navigate_to_node(idx);
                } else {
                    self.status = format!("Node \"{cell_value}\" not found in tree");
                }
            }
            CellJumpTarget::ContainerByName => {
                self.navigate_to_container_by_name(cell_value);
            }
        }
    }

    /// Navigate to a service in the tree from a service list table
    /// (Diag-Comms, Requests, Responses)
    pub(crate) fn try_navigate_to_service(&mut self) {
        // Early validation
        if self.cursor >= self.visible.len() {
            self.status = "Cursor out of bounds".to_string();
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        if !Self::is_service_list_section(node) {
            "Not a service list section".clone_into(&mut self.status);
            return;
        }

        // Get service name from table or return early
        let Some(service_name) = self.extract_service_name_from_table(node_idx) else {
            return;
        };

        // Expand service list section if collapsed
        if let Some(node_at_idx) = self.all_nodes.get(node_idx)
            && !node_at_idx.expanded
        {
            self.expand_and_update_cursor(node_idx);
        }

        // Find and navigate to service
        self.find_and_navigate_to_service(&service_name, node_idx);
    }

    /// Extract service name from the current table row
    fn extract_service_name_from_table(&mut self, node_idx: usize) -> Option<String> {
        let node = self.all_nodes.get(node_idx)?;
        let section = node.detail_sections.first()?;

        let Some(rows) = section.content.table_rows() else {
            "Details should be a table".clone_into(&mut self.status);
            return None;
        };

        let section_index = self.get_section_index();
        let row_cursor = *self.section_cursors.get(section_index)?;
        let sorted_rows = self.apply_table_sort(rows, section_index);
        let selected_row = sorted_rows.get(row_cursor)?;

        // Determine name column index based on node type
        let is_functional_class =
            Self::is_service_list_type(node, crate::tree::ServiceListType::FunctionalClasses);
        let name_column_index = usize::from(!is_functional_class);

        selected_row.cells.get(name_column_index).cloned()
    }

    /// Expand section and update cursor position
    fn expand_and_update_cursor(&mut self, node_idx: usize) {
        if let Some(node_mut) = self.all_nodes.get_mut(node_idx) {
            node_mut.expanded = true;
        }
        self.rebuild_visible();

        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == node_idx) {
            self.cursor = new_cursor;
        }
    }

    /// Find and navigate to a service by name
    fn find_and_navigate_to_service(&mut self, service_name: &str, parent_node_idx: usize) {
        let Some(parent_node) = self.all_nodes.get(parent_node_idx) else {
            return;
        };
        let parent_depth = parent_node.depth;
        let is_functional_class = Self::is_service_list_type(
            parent_node,
            crate::tree::ServiceListType::FunctionalClasses,
        );

        // Find service in visible nodes after parent
        let found_idx = self
            .visible
            .get(self.cursor.saturating_add(1)..)
            .and_then(|rest| {
                rest.iter()
                    .copied()
                    .take_while(|&vis_idx| {
                        self.all_nodes
                            .get(vis_idx)
                            .is_some_and(|n| n.depth > parent_depth)
                    })
                    .filter(|&vis_idx| {
                        self.all_nodes
                            .get(vis_idx)
                            .is_some_and(|n| n.depth == parent_depth.saturating_add(1))
                    })
                    .find(|&vis_idx| {
                        if let Some(node) = self.all_nodes.get(vis_idx) {
                            Self::node_matches_service_name(node, service_name, is_functional_class)
                        } else {
                            false
                        }
                    })
                    .and_then(|vis_idx| self.visible.iter().position(|&idx| idx == vis_idx))
            });

        if let Some(target_cursor) = found_idx {
            self.push_to_history();
            self.focus_state = FocusState::Tree;
            self.cursor = target_cursor;
            self.reset_detail_state();
            self.scroll_offset = self.cursor.saturating_sub(5);
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
    fn node_matches_service_name(
        node: &TreeNode,
        target_name: &str,
        is_functional_class: bool,
    ) -> bool {
        if is_functional_class {
            node.node_type == NodeType::FunctionalClass && node.text == target_name
        } else {
            let is_target_node = matches!(
                node.node_type,
                NodeType::Service
                    | NodeType::ParentRefService
                    | NodeType::Request
                    | NodeType::PosResponse
                    | NodeType::NegResponse
                    | NodeType::Job
            );

            if !is_target_node {
                return false;
            }

            if node.node_type == NodeType::Job {
                let job_name = node.text.strip_prefix("[Job] ").unwrap_or(&node.text);
                job_name == target_name
            } else {
                node.text.contains(target_name)
            }
        }
    }

    /// Navigate to an inherited parent layer in the tree
    pub(crate) fn try_navigate_to_inherited_parent(&mut self) {
        // Early validations
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        if !Self::is_service_node(node) {
            "Not a service node".clone_into(&mut self.status);
            return;
        }

        // Extract current service name and parent layer name
        let current_service_name = Self::extract_service_name_from_node(node);
        let Some(parent_layer_name) = self.get_parent_layer_name(node_idx) else {
            return;
        };

        // Find parent container and navigate
        if let Some(container_idx) = self.find_container_by_name(&parent_layer_name) {
            self.navigate_to_parent_service(container_idx, &current_service_name);
        } else {
            self.status = format!("Parent layer '{parent_layer_name}' not found in tree");
        }
    }

    /// Check if node is a service-related node
    fn is_service_node(node: &TreeNode) -> bool {
        matches!(
            node.node_type,
            NodeType::Service
                | NodeType::ParentRefService
                | NodeType::Request
                | NodeType::PosResponse
                | NodeType::NegResponse
        )
    }

    /// Extract service name from node text
    fn extract_service_name_from_node(node: &TreeNode) -> String {
        node.text.find(" - ").map_or_else(
            || node.text.clone(),
            |dash_idx| node.text[dash_idx.saturating_add(3)..].to_string(),
        )
    }

    /// Get parent layer name from the Overview section's "Inherited From" row
    fn get_parent_layer_name(&self, node_idx: usize) -> Option<String> {
        let node = self.all_nodes.get(node_idx)?;

        let overview_idx = usize::from(
            node.detail_sections.len() > 1
                && node
                    .detail_sections
                    .first()
                    .is_some_and(|s| s.render_as_header),
        );

        let overview_section = node.detail_sections.get(overview_idx)?;

        let rows = overview_section.content.table_rows()?;

        let row_cursor = self.section_cursors.get(overview_idx).copied().unwrap_or(0);
        let sorted_rows = self.apply_table_sort(rows, overview_idx);
        let selected_row = sorted_rows.get(row_cursor)?;

        if selected_row.row_type != DetailRowType::InheritedFrom {
            return None;
        }

        // Extract from metadata or fallback to cell data
        match &selected_row.metadata {
            Some(RowMetadata::InheritedFrom { layer_name }) => Some(layer_name.clone()),
            _ => selected_row.cells.get(1).cloned(),
        }
    }

    /// Navigate to parent service in the container
    fn navigate_to_parent_service(&mut self, container_idx: usize, service_name: &str) {
        // Expand ancestors and container
        self.expand_node_ancestors(container_idx);

        if let Some(node) = self.all_nodes.get(container_idx)
            && node.has_children
            && let Some(node_mut) = self.all_nodes.get_mut(container_idx)
        {
            node_mut.expanded = true;
        }

        // Find Diag-Comms section
        let Some(dc_idx) = self.find_diagcomm_section(container_idx) else {
            self.rebuild_visible();
            self.navigate_to_node_by_idx(container_idx);
            return;
        };

        if let Some(node_mut) = self.all_nodes.get_mut(dc_idx) {
            node_mut.expanded = true;
        }
        self.rebuild_visible();

        let target = self
            .find_service_in_diagcomm(dc_idx, service_name)
            .unwrap_or(container_idx);
        self.navigate_to_node_by_idx(target);
    }

    /// Expand all ancestors of a node
    fn expand_node_ancestors(&mut self, node_idx: usize) {
        let Some(target_node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let target_depth = target_node.depth;

        if target_depth == 0 {
            return;
        }

        for i in 0..node_idx {
            if let Some(node) = self.all_nodes.get(i)
                && node.depth < target_depth
                && node.has_children
                && let Some(node_mut) = self.all_nodes.get_mut(i)
            {
                node_mut.expanded = true;
            }
        }
    }

    /// Find the Diag-Comms section within a container
    fn find_diagcomm_section(&self, container_idx: usize) -> Option<usize> {
        let container = self.all_nodes.get(container_idx)?;
        let container_depth = container.depth;

        if let Some(children_slice) = self.all_nodes.get(container_idx.saturating_add(1)..) {
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
    fn find_service_in_diagcomm(&self, diagcomm_idx: usize, service_name: &str) -> Option<usize> {
        let diagcomm_node = self.all_nodes.get(diagcomm_idx)?;
        let diagcomm_depth = diagcomm_node.depth;

        if let Some(children_slice) = self.all_nodes.get(diagcomm_idx.saturating_add(1)..) {
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

    /// Navigate to a node by its index in `all_nodes`
    fn navigate_to_node_by_idx(&mut self, target_idx: usize) {
        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == target_idx) {
            self.push_to_history();
            self.focus_state = FocusState::Tree;
            self.cursor = new_cursor;
            self.reset_detail_state();
            self.scroll_offset = self.cursor.saturating_sub(5);
        }
    }

    /// Find a container (variant/functional group) by name
    fn find_container_by_name(&self, name: &str) -> Option<usize> {
        self.all_nodes.iter().position(|node| {
            if !matches!(node.node_type, NodeType::Container) {
                return false;
            }

            let node_name = node
                .text
                .find(" [")
                .map_or(node.text.as_str(), |idx| &node.text[..idx]);

            node_name == name
        })
    }

    /// Navigate from a parameter table (Request/Response) based on the focused cell.
    /// For `DiagComm` Service nodes: `ParameterName` navigates to the counterpart service.
    /// For Request/Response nodes: uses per-cell jump target metadata.
    pub(crate) fn try_navigate_from_param_table(&mut self) {
        // Early validation
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        // Get table data or return early
        let Some((rows, use_row_selection)) = Self::get_table_rows(node, section_idx) else {
            return;
        };

        let row_cursor = self.section_cursors.get(section_idx).copied().unwrap_or(0);
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // Determine focused column and cell
        let focused_col = self.get_focused_column(use_row_selection, &selected_row.cell_types);
        let cell_type = selected_row
            .cell_types
            .get(focused_col)
            .copied()
            .unwrap_or(CellType::Text);
        let cell_value = selected_row
            .cells
            .get(focused_col)
            .map_or("", std::string::String::as_str);

        if cell_value.is_empty() {
            "Empty cell".clone_into(&mut self.status);
            return;
        }

        // DiagComm Service view: ParameterName navigates to the counterpart
        if matches!(cell_type, CellType::ParameterName)
            && matches!(
                node.node_type,
                NodeType::Service | NodeType::ParentRefService
            )
        {
            self.navigate_to_request_response_counterpart(node_idx, section_idx);
            return;
        }

        // Use per-cell jump target metadata for everything else
        let jump_target = selected_row
            .cell_jump_targets
            .get(focused_col)
            .cloned()
            .flatten();
        self.execute_cell_jump(jump_target, cell_value);
    }

    /// Get table rows from section
    fn get_table_rows(node: &TreeNode, section_idx: usize) -> Option<(&Vec<DetailRow>, bool)> {
        let section = node.detail_sections.get(section_idx)?;
        let rows = section.content.table_rows()?;
        let use_row_selection = section.content.table_use_row_selection().unwrap_or(false);
        Some((rows, use_row_selection))
    }

    /// Navigate from a `DiagComm` service to the corresponding Request/Response node.
    /// The `DiagComm` node text has a `[Service] ` prefix that the counterpart lacks.
    fn navigate_to_request_response_counterpart(&mut self, node_idx: usize, section_idx: usize) {
        let Some(node) = self.all_nodes.get(node_idx) else {
            "Invalid node index".clone_into(&mut self.status);
            return;
        };

        let service_name = node
            .text
            .strip_prefix("[Service] ")
            .unwrap_or(&node.text)
            .to_owned();

        let section_type = node
            .detail_sections
            .get(section_idx)
            .map(|s| s.section_type);

        let Some(target_node_type) = section_type.and_then(|st| match st {
            DetailSectionType::Requests => Some(NodeType::Request),
            DetailSectionType::PosResponses => Some(NodeType::PosResponse),
            DetailSectionType::NegResponses => Some(NodeType::NegResponse),
            DetailSectionType::Header
            | DetailSectionType::Overview
            | DetailSectionType::Services
            | DetailSectionType::ComParams
            | DetailSectionType::States
            | DetailSectionType::RelatedRefs
            | DetailSectionType::FunctionalClass
            | DetailSectionType::Custom => None,
        }) else {
            "Cannot navigate from this section type".clone_into(&mut self.status);
            return;
        };

        let found = self
            .find_in_hierarchy(|n| n.node_type == target_node_type && n.text == service_name)
            .or_else(|| {
                self.all_nodes
                    .iter()
                    .position(|n| n.node_type == target_node_type && n.text == service_name)
            });

        match found {
            Some(idx) => self.navigate_to_node(idx),
            None => {
                self.status = format!("No matching node found for {service_name}");
            }
        }
    }

    /// Determine which column is focused based on selection mode
    fn get_focused_column(&self, _use_row_selection: bool, cell_types: &[CellType]) -> usize {
        self.focused_column.min(cell_types.len().saturating_sub(1))
    }

    /// Navigate to a DOP node by name.
    /// Scopes the search to the current container's subtree first, then
    /// walks up through parent ref containers before falling back to a
    /// global search.
    fn navigate_to_dop(&mut self, dop_name: &str) {
        let found_idx = self
            .find_in_hierarchy(|node| node.text == dop_name)
            .or_else(|| self.all_nodes.iter().position(|node| node.text == dop_name));

        match found_idx {
            Some(dop_idx) => {
                self.navigate_to_node(dop_idx);
                self.status = format!("Navigated to DOP: {dop_name}");
            }
            None => {
                self.status = format!("DOP '{dop_name}' not found in tree");
            }
        }
    }

    /// Navigate to a parameter node by its ID.
    /// Scopes the search to the current node's subtree first to avoid
    /// jumping to a same-named parameter in a different section.
    fn navigate_to_parameter_by_id(&mut self, param_id: u32) {
        // Get current node index to scope the search
        let current_node_idx = self.visible.get(self.cursor).copied();

        // Find parameter node by ID, preferring children of the current node
        let param_idx = current_node_idx
            .and_then(|node_idx| self.find_param_in_subtree(node_idx, param_id))
            .or_else(|| self.find_param_by_id(param_id));

        let Some(param_idx) = param_idx else {
            self.status = format!("Parameter with ID {param_id} not found");
            return;
        };

        // Expand ancestors to make parameter visible
        self.ensure_node_visible(param_idx);

        // Navigate to parameter
        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == param_idx) {
            self.push_to_history();
            self.focus_state = FocusState::Tree;
            self.cursor = new_cursor;
            self.reset_detail_state();
            self.scroll_offset = self.cursor.saturating_sub(5);
            self.status = format!("Navigated to parameter (ID: {param_id})");
        } else {
            self.status = format!("Parameter found but not visible (ID: {param_id})");
        }
    }

    /// Find parameter node by `param_id` within a node's subtree
    fn find_param_in_subtree(&self, parent_idx: usize, param_id: u32) -> Option<usize> {
        let parent = self.all_nodes.get(parent_idx)?;
        let parent_depth = parent.depth;

        self.all_nodes
            .iter()
            .enumerate()
            .skip(parent_idx.saturating_add(1))
            .take_while(|(_, node)| node.depth > parent_depth)
            .find(|(_, node)| node.param_id == Some(param_id))
            .map(|(idx, _)| idx)
    }

    /// Find parameter node by `param_id`
    fn find_param_by_id(&self, param_id: u32) -> Option<usize> {
        self.all_nodes
            .iter()
            .position(|node| node.param_id == Some(param_id))
    }

    /// Find the containing depth-1 Container node for a given node index
    /// by walking backwards from the node to its nearest Container ancestor.
    fn find_current_container(&self, from_node_idx: usize) -> Option<usize> {
        let from_node = self.all_nodes.get(from_node_idx)?;

        // If already a depth-1 container, return it
        if from_node.depth == 1 && matches!(from_node.node_type, NodeType::Container) {
            return Some(from_node_idx);
        }

        // Walk backwards to find the nearest depth-1 Container ancestor
        (0..from_node_idx).rev().find(|&i| {
            self.all_nodes
                .get(i)
                .is_some_and(|n| n.depth == 1 && matches!(n.node_type, NodeType::Container))
        })
    }

    /// Get the subtree range (`start_idx` inclusive, `end_idx` exclusive)
    /// for a given node. The subtree includes the node itself and all
    /// children with deeper depth.
    fn subtree_range(&self, node_idx: usize) -> (usize, usize) {
        let Some(node) = self.all_nodes.get(node_idx) else {
            return (node_idx, node_idx);
        };
        let node_depth = node.depth;

        let end = self
            .all_nodes
            .iter()
            .enumerate()
            .skip(node_idx.saturating_add(1))
            .find(|(_, n)| n.depth <= node_depth)
            .map_or(self.all_nodes.len(), |(i, _)| i);

        (node_idx, end)
    }

    /// Search for a node within a subtree using a predicate.
    fn find_in_subtree(
        &self,
        start: usize,
        end: usize,
        predicate: impl Fn(&TreeNode) -> bool,
    ) -> Option<usize> {
        self.all_nodes
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
            .find(|(_, node)| predicate(node))
            .map(|(idx, _)| idx)
    }

    /// Search for a node following the database hierarchy:
    /// 1. Search within the current container's subtree
    /// 2. Look for parent ref containers referenced by the current container
    ///    and search within each
    /// 3. Returns `None` if not found (caller should fall back to global search)
    fn find_in_hierarchy(&self, predicate: impl Fn(&TreeNode) -> bool) -> Option<usize> {
        let current_node_idx = self.visible.get(self.cursor).copied()?;
        let container_idx = self.find_current_container(current_node_idx)?;

        // 1. Search within current container's subtree
        let (start, end) = self.subtree_range(container_idx);
        if let Some(found) = self.find_in_subtree(start, end, &predicate) {
            return Some(found);
        }

        // 2. Walk parent ref containers
        // Find the "Parent Refs" child section of this container
        let parent_refs_names: Vec<String> = self
            .all_nodes
            .iter()
            .enumerate()
            .skip(start.saturating_add(1))
            .take(end.saturating_sub(start).saturating_sub(1))
            .filter(|(_, n)| n.node_type == NodeType::ParentRefs)
            .flat_map(|(pr_idx, pr_node)| {
                // Collect the names of parent ref children (depth = pr_node.depth + 1)
                let pr_depth = pr_node.depth;
                self.all_nodes
                    .iter()
                    .skip(pr_idx.saturating_add(1))
                    .take_while(move |n| n.depth > pr_depth)
                    .filter(move |n| n.depth == pr_depth.saturating_add(1))
                    .map(|n| {
                        n.text
                            .find(" [")
                            .map_or(n.text.clone(), |idx| n.text[..idx].to_string())
                    })
            })
            .collect();

        for parent_name in &parent_refs_names {
            // Find the container node for this parent ref
            let parent_container_idx = self.all_nodes.iter().enumerate().find(|(_, n)| {
                matches!(n.node_type, NodeType::Container) && {
                    let name = n
                        .text
                        .find(" [")
                        .map_or(n.text.as_str(), |idx| &n.text[..idx]);
                    name == parent_name
                }
            });

            if let Some((pc_idx, _)) = parent_container_idx {
                let (pc_start, pc_end) = self.subtree_range(pc_idx);
                if let Some(found) = self.find_in_subtree(pc_start, pc_end, &predicate) {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Navigate to a parent ref target when pressing Enter on a parent ref child
    /// in the tree pane. Returns `true` if navigation was attempted.
    pub(crate) fn try_navigate_parent_ref_from_tree(&mut self) -> bool {
        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return false;
        };

        // Check if the current node is a child of a ParentRefs node
        let Some(node) = self.all_nodes.get(node_idx) else {
            return false;
        };
        let node_depth = node.depth;

        if node_depth == 0 {
            return false;
        }

        // Walk backwards to find the parent node
        let parent_is_parent_refs = (0..node_idx).rev().any(|i| {
            self.all_nodes
                .get(i)
                .is_some_and(|n| n.depth < node_depth && n.node_type == NodeType::ParentRefs)
                && self
                    .all_nodes
                    .get(i)
                    .is_some_and(|n| n.depth == node_depth.saturating_sub(1))
        });

        if !parent_is_parent_refs {
            return false;
        }

        // The node text is the short name of the target
        let target_short_name = node.text.clone();
        self.navigate_to_container_by_name(&target_short_name);
        true
    }

    /// Navigate to a Container node matching the given short name
    fn navigate_to_container_by_name(&mut self, target_short_name: &str) {
        let found = self.all_nodes.iter().enumerate().find(|(_, n)| {
            matches!(n.node_type, NodeType::Container) && {
                let name = n
                    .text
                    .find(" [")
                    .map_or(n.text.as_str(), |idx| &n.text[..idx]);
                name == target_short_name
            }
        });

        let Some((container_node_idx, _)) = found else {
            self.status = format!("Element '{target_short_name}' not found in tree");
            return;
        };

        self.navigate_to_node(container_node_idx);
        self.status = format!("Navigated to: {target_short_name}");
    }

    /// Navigate to a parent ref element from the Parent References table
    pub(crate) fn try_navigate_to_parent_ref(&mut self) {
        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Get the actual section index
        let section_idx = self.get_section_index();

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let Some(&row_cursor) = self.section_cursors.get(section_idx) else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        if selected_row.cells.len() < 2 {
            "Invalid parent ref row".clone_into(&mut self.status);
            return;
        }

        let Some(target_short_name) = selected_row.cells.first().cloned() else {
            return;
        };

        self.navigate_to_container_by_name(&target_short_name);
    }

    /// Navigate to a not-inherited element (`DiagComm`, `DiagVariable`, `Dop`, `Table`)
    pub(crate) fn try_navigate_to_not_inherited_element(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Determine what type of element we're looking for based on the section title
        let element_type = if section.title.contains("DiagComms") {
            "service"
        } else {
            // For now, only services (DiagComms) are navigable
            // TODO: Add navigation for DiagVariables, DOPs,
            // and Tables when they're added to the tree
            "Navigation not yet supported for this element type".clone_into(&mut self.status);
            return;
        };

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            *self.section_cursors.get(section_idx).unwrap_or(&0)
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // Extract the element short name from the first column
        if selected_row.cells.is_empty() {
            "Invalid row".clone_into(&mut self.status);
            return;
        }

        let Some(target_short_name) = selected_row.cells.first().cloned() else {
            return;
        };

        // Search for the element in the tree based on type
        if element_type == "service" {
            self.navigate_to_service_or_job(&target_short_name);
        }
    }

    /// Navigate to a layer from functional class detail view
    /// The layer name is extracted from the "Layer" column of the selected row
    /// Navigate to a service or job from a functional class detail view
    /// Uses the `ShortName` column (column 0) to find the target
    pub(crate) fn try_navigate_to_service_from_functional_class(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Verify we're on a functional class node
        if !matches!(node.node_type, NodeType::FunctionalClass) {
            "Not a functional class node".clone_into(&mut self.status);
            return;
        }

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // We should be in a Services section
        if section.section_type != DetailSectionType::Services {
            "Not in a services section".clone_into(&mut self.status);
            return;
        }

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            *self.section_cursors.get(section_idx).unwrap_or(&0)
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // ShortName is in column 0
        if selected_row.cells.is_empty() {
            "Invalid row structure".clone_into(&mut self.status);
            return;
        }

        let Some(target_short_name) = selected_row.cells.first().cloned() else {
            return;
        };

        // Search for the service/job in the tree
        self.navigate_to_service_or_job(&target_short_name);
    }

    pub(crate) fn try_navigate_to_layer_from_functional_class(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Verify we're on a functional class node
        if !matches!(node.node_type, NodeType::FunctionalClass) {
            "Not a functional class node".clone_into(&mut self.status);
            return;
        }

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // We should be in a Services section (the table showing services for this functional class)
        if section.section_type != DetailSectionType::Services {
            "Not in a services section".clone_into(&mut self.status);
            return;
        }

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            *self.section_cursors.get(section_idx).unwrap_or(&0)
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // The table has columns: ShortName | Type | SID_RQ | Semantic | Addressing | Layer
        // Layer name is in column 5 (index 5)
        let layer_column_index = 5;

        if selected_row.cells.len() <= layer_column_index {
            "Invalid row structure".clone_into(&mut self.status);
            return;
        }

        let Some(layer_name) = selected_row.cells.get(layer_column_index).cloned() else {
            return;
        };

        // Navigate to the container using the shared helper
        self.navigate_to_container_by_name(&layer_name);
    }

    /// Navigate to a variant from the Variants overview table
    pub(crate) fn try_navigate_to_variant(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Get the actual section index
        let section_idx = self.get_section_index();

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let Some(&row_cursor) = self.section_cursors.get(section_idx) else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // Extract the variant name from the first column
        let Some(target_variant_name) = selected_row.cells.first().cloned() else {
            "Invalid variant row".clone_into(&mut self.status);
            return;
        };

        self.navigate_to_container_by_name(&target_variant_name);
    }

    /// Navigate from variant overview to a child element
    pub(crate) fn try_navigate_from_variant_overview(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Get the actual section index
        let section_idx = self.get_section_index();

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Only handle Overview section type
        if section.section_type != DetailSectionType::Overview {
            return;
        }

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let Some(&row_cursor) = self.section_cursors.get(section_idx) else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // Check if this row is a child element row with metadata
        if let Some(RowMetadata::ChildElement { element_type }) = &selected_row.metadata {
            // Find the child section node under the current variant
            // It should be a direct child (depth = current + 1) of the current node
            let current_depth = node.depth;
            let target_depth = current_depth.saturating_add(1);

            // Start searching after the current node
            let mut target_idx: Option<usize> = None;
            for i in node_idx.saturating_add(1)..self.all_nodes.len() {
                let Some(child_node) = self.all_nodes.get(i) else {
                    continue;
                };

                // Stop if we've moved past this variant's children
                if child_node.depth <= current_depth {
                    break;
                }

                // Check if this is the target child at the correct depth
                if child_node.depth == target_depth
                    && element_type.matches_node_text(&child_node.text)
                {
                    target_idx = Some(i);
                    break;
                }
            }

            if let Some(target_node_idx) = target_idx {
                // Ensure the target node is visible (expand if needed)
                self.ensure_node_visible(target_node_idx);

                // Find the target in the visible list and navigate to it
                if let Some(new_cursor) =
                    self.visible.iter().position(|&idx| idx == target_node_idx)
                {
                    self.push_to_history();
                    self.focus_state = FocusState::Tree;
                    self.cursor = new_cursor;
                    self.reset_detail_state();
                    self.scroll_offset = self.cursor.saturating_sub(5);
                    self.status = format!("Navigated to: {}", element_type.display_name());
                }
            } else {
                self.status = format!("Section '{}' not found", element_type.display_name());
            }
        }
    }

    /// Navigate from DIAG-DATA-DICTIONARY-SPEC or DOP category overview to a child node.
    /// For DIAG-DATA-DICTIONARY-SPEC: rows are categories
    /// like "DTC-DOPS", navigates to the category child node.
    /// For DOP category nodes: rows are individual DOPs, navigates to the DOP child node.
    pub(crate) fn try_navigate_to_dop_child(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Get table rows
        let Some(rows) = section.content.table_rows() else {
            return;
        };

        // Get the selected row cursor
        let Some(&row_cursor) = self.section_cursors.get(section_idx) else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        // Check if the selected row has a DopReference cell — navigate to that DOP
        let dop_ref = selected_row
            .cell_types
            .iter()
            .zip(selected_row.cells.iter())
            .find(|(ct, _)| **ct == CellType::DopReference)
            .map(|(_, cell)| cell.clone());

        if let Some(dop_name) = dop_ref {
            self.navigate_to_dop(&dop_name);
            return;
        }

        // Get the first cell text (category name or DOP name)
        let target_name = match selected_row.cells.first() {
            Some(name) if !name.is_empty() => name.clone(),
            _ => return,
        };

        // Find the child node that matches the target name
        let current_depth = node.depth;
        let target_depth = current_depth.saturating_add(1);

        let target_idx = self
            .all_nodes
            .iter()
            .enumerate()
            .skip(node_idx.saturating_add(1))
            .take_while(|(_, child)| child.depth > current_depth)
            .find(|(_, child)| child.depth == target_depth && child.text.starts_with(&target_name))
            .map(|(i, _)| i);

        if let Some(target_node_idx) = target_idx {
            self.navigate_to_node(target_node_idx);
            self.status = format!("Navigated to: {target_name}");
        } else {
            self.status = format!("'{target_name}' not found");
        }
    }

    /// Helper function to navigate to a service or job by name.
    /// Searches within the current container's hierarchy first, then globally.
    fn navigate_to_service_or_job(&mut self, target_short_name: &str) {
        let matches_service = |n: &TreeNode| -> bool {
            if matches!(n.node_type, NodeType::Service | NodeType::ParentRefService) {
                let service_name = n
                    .text
                    .find(" - ")
                    .map_or(n.text.as_str(), |idx| &n.text[idx.saturating_add(3)..]);
                service_name == target_short_name
            } else if n.node_type == NodeType::Job {
                let job_name = n.text.strip_prefix("[Job] ").unwrap_or(&n.text);
                job_name == target_short_name
            } else {
                false
            }
        };

        let found_service_idx = self
            .find_in_hierarchy(matches_service)
            .or_else(|| self.all_nodes.iter().position(matches_service));

        let Some(service_node_idx) = found_service_idx else {
            self.status = format!("Service/Job '{target_short_name}' not found in tree");
            return;
        };

        self.ensure_node_visible(service_node_idx);

        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == service_node_idx) {
            self.push_to_history();
            self.focus_state = FocusState::Tree;
            self.cursor = new_cursor;
            self.reset_detail_state();
            self.scroll_offset = self.cursor.saturating_sub(5);
            self.status = format!("Navigated to: {target_short_name}");
        }
    }
}
