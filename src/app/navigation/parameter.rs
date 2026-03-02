/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crate::{
    app::{App, FocusState, SCROLL_CONTEXT_LINES},
    tree::{CellJumpTarget, CellType, NodeTextPrefix, ServiceListType},
};

impl App {
    /// Navigate from a parameter table based on the focused cell.
    /// Uses the active tab's section type to determine the target section
    /// in the tree, then navigates via path: container → section → service.
    /// Works uniformly for all service-related node types.
    pub(crate) fn try_navigate_from_param_table(&mut self) {
        let (node_idx, section_type, cell_type, cell_value, jump_target, service_name) = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let focused_col = self.get_focused_column(&selected_row.cell_types);
            let cell_type = selected_row
                .cell_types
                .get(focused_col)
                .map_or(CellType::Text, |&ct| ct);
            let cell_value = selected_row
                .cells
                .get(focused_col)
                .map_or_else(Default::default, Clone::clone);
            let jump_target = selected_row
                .cell_jump_targets
                .get(focused_col)
                .cloned()
                .flatten();
            let section_type = ctx.section.section_type;
            let service_name = ctx
                .node
                .text
                .strip_prefix(NodeTextPrefix::Service.as_str())
                .or_else(|| ctx.node.text.strip_prefix(NodeTextPrefix::Job.as_str()))
                .unwrap_or(&ctx.node.text)
                .to_owned();
            (
                ctx.node_idx,
                section_type,
                cell_type,
                cell_value,
                jump_target,
                service_name,
            )
        };

        if cell_value.is_empty() {
            self.status = "Empty cell".into();
            return;
        }

        let target_list_type = Self::section_type_to_list_type(section_type);

        // Parameter jump: navigate via section path
        // (container → section → service → param)
        if let Some(CellJumpTarget::Parameter { .. }) = &jump_target
            && let Some(list_type) = target_list_type
        {
            self.navigate_to_param_in_section(node_idx, list_type, &service_name, &cell_value);
            return;
        }

        // ParameterName without jump target: navigate to the service
        // counterpart in the target section
        if matches!(cell_type, CellType::ParameterName)
            && jump_target.is_none()
            && let Some(list_type) = target_list_type
        {
            self.navigate_to_counterpart_in_section(node_idx, list_type, &service_name);
            return;
        }

        // All other cases: use the per-cell jump target
        self.execute_cell_jump(jump_target, &cell_value);
    }

    /// Navigate to the counterpart service node in a target section.
    /// Uses path-based lookup: container → section header (by list type) →
    /// service (by name), so duplicate names in different sections are never
    /// confused.
    fn navigate_to_counterpart_in_section(
        &mut self,
        node_idx: usize,
        target_list_type: ServiceListType,
        service_name: &str,
    ) {
        let Some(container_idx) = self.find_current_container(node_idx) else {
            self.status = "Container not found".into();
            return;
        };

        if let Some(idx) = self.find_by_section_path(container_idx, target_list_type, service_name)
        {
            self.navigate_to_node(idx);
        } else {
            self.status = format!("No matching node found for {service_name}");
        }
    }

    /// Navigate to a parameter via section path.
    /// Builds: container → section header (by `ServiceListType`) →
    /// service (by name) → param (by name).
    fn navigate_to_param_in_section(
        &mut self,
        node_idx: usize,
        target_list_type: ServiceListType,
        service_name: &str,
        param_name: &str,
    ) {
        let Some(container_idx) = self.find_current_container(node_idx) else {
            self.status = "Container not found".into();
            return;
        };

        let param_idx = self
            .find_by_section_path(container_idx, target_list_type, service_name)
            .and_then(|svc_idx| self.find_param_in_subtree_by_name(svc_idx, param_name));

        let Some(param_idx) = param_idx else {
            self.status = format!("Parameter '{param_name}' not found");
            return;
        };

        self.ensure_node_visible(param_idx);

        let Some(new_cursor) = self.tree.visible.iter().position(|&idx| idx == param_idx) else {
            self.status = format!("Parameter '{param_name}' found but not visible");
            return;
        };

        self.push_to_history();
        self.focus_state = FocusState::Tree;
        self.tree.cursor = new_cursor;
        self.reset_detail_state();
        self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
        self.status = format!("Navigated to parameter: {param_name}");
    }

    /// Determine which column is focused, clamped to the available cell count
    pub(super) fn get_focused_column(&self, cell_types: &[CellType]) -> usize {
        self.table
            .focused_column
            .min(cell_types.len().saturating_sub(1))
    }

    /// Navigate to a DOP node by name.
    /// Uses the database hierarchy via `find_in_hierarchy` — current
    /// container subtree first, then parent-ref chain.
    pub(super) fn navigate_to_dop(&mut self, dop_name: &str) {
        if let Some(dop_idx) = self.find_in_hierarchy(|node| node.text == dop_name) {
            self.navigate_to_node(dop_idx);
            self.status = format!("Navigated to DOP: {dop_name}");
        } else {
            self.status = format!("DOP '{dop_name}' not found in tree");
        }
    }

    /// Navigate to a parameter node by name.
    /// Prefers name-based matching within the current subtree, then falls
    /// back to `find_in_hierarchy` which searches the enclosing section,
    /// container, and parent-ref chain.
    pub(super) fn navigate_to_parameter(&mut self, param_name: &str) {
        let param_idx = self
            .tree
            .visible
            .get(self.tree.cursor)
            .copied()
            .and_then(|node_idx| self.find_param_in_subtree_by_name(node_idx, param_name))
            .or_else(|| self.find_in_hierarchy(|n| n.param_id.is_some() && n.text == param_name));

        let Some(param_idx) = param_idx else {
            self.status = format!("Parameter '{param_name}' not found");
            return;
        };

        self.ensure_node_visible(param_idx);

        let Some(new_cursor) = self.tree.visible.iter().position(|&idx| idx == param_idx) else {
            self.status = format!("Parameter '{param_name}' found but not visible");
            return;
        };

        self.push_to_history();
        self.focus_state = FocusState::Tree;
        self.tree.cursor = new_cursor;
        self.reset_detail_state();
        self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
        self.status = format!("Navigated to parameter: {param_name}");
    }

    /// Find parameter node by name within a node's subtree.
    fn find_param_in_subtree_by_name(&self, parent_idx: usize, param_name: &str) -> Option<usize> {
        let parent = self.tree.all_nodes.get(parent_idx)?;
        let parent_depth = parent.depth;

        self.tree
            .all_nodes
            .iter()
            .enumerate()
            .skip(parent_idx.saturating_add(1))
            .take_while(|(_, node)| node.depth > parent_depth)
            .find(|(_, node)| node.param_id.is_some() && node.text == param_name)
            .map(|(idx, _)| idx)
    }

    /// Navigate from DIAG-DATA-DICTIONARY-SPEC or DOP category overview to a child node.
    pub(crate) fn try_navigate_to_dop_child(&mut self) {
        let (node_idx, current_depth, dop_ref, target_name) = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };

            let dop_ref = selected_row
                .cell_types
                .iter()
                .zip(selected_row.cells.iter())
                .find(|(ct, _)| **ct == CellType::DopReference)
                .map(|(_, cell)| cell.clone());

            let target_name = selected_row
                .cells
                .first()
                .map_or_else(Default::default, Clone::clone);

            (ctx.node_idx, ctx.node.depth, dop_ref, target_name)
        };

        if let Some(dop_name) = dop_ref {
            self.navigate_to_dop(&dop_name);
            return;
        }

        if target_name.is_empty() {
            return;
        }

        let target_depth = current_depth.saturating_add(1);
        let target_idx = self
            .tree
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
}
