/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod helpers;
mod parameter;
mod parent_ref;
mod service;
mod variant;

use crate::{
    app::App,
    tree::{CellJumpTarget, DetailSectionType, NodeType, RowMetadata, SectionType},
};

impl App {
    pub(crate) fn handle_enter_in_detail_pane(&mut self) {
        if self.tree.cursor >= self.tree.visible.len() {
            return;
        }

        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };

        // Early returns for different node types using functional matching
        if let Some(SectionType::Variants) = node.section_type {
            self.try_navigate_to_variant();
            return;
        }

        if matches!(node.node_type, NodeType::Container) {
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
        if matches!(node.node_type, NodeType::Dop)
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
        self.handle_generic_detail_enter(node_idx);
    }

    /// Handle Enter key for generic nodes with detail sections
    fn handle_generic_detail_enter(&mut self, node_idx: usize) {
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        let section = node
            .detail_sections
            .get(section_idx)
            .filter(|_| section_idx < node.detail_sections.len());

        if let Some(section) = section {
            if section.section_type == DetailSectionType::RelatedRefs {
                self.try_navigate_to_parent_ref();
            } else if matches!(
                section.section_type,
                DetailSectionType::NotInheritedDiagComms
                    | DetailSectionType::NotInheritedDops
                    | DetailSectionType::NotInheritedTables
                    | DetailSectionType::NotInheritedVariables
            ) {
                self.try_navigate_to_not_inherited_element();
            } else {
                self.try_navigate_from_detail_row();
            }
        } else {
            self.status = "No details available".into();
        }
    }

    /// Try to navigate to the item referenced by the currently focused cell.
    /// Falls back to searching the tree for a node matching the cell text.
    pub(crate) fn try_navigate_from_detail_row(&mut self) {
        let (node_idx, node_depth, element_type, cell_value, jump_target) = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let node_depth = ctx.node.depth;
            let node_idx = ctx.node_idx;

            // Check for ChildElement metadata first
            let element_type =
                if let Some(RowMetadata::ChildElement { element_type }) = &selected_row.metadata {
                    Some(element_type.clone())
                } else {
                    None
                };

            let focused_col = self.get_focused_column(&selected_row.cell_types);

            let nav_col = if ctx.use_row_selection
                && selected_row
                    .cell_jump_targets
                    .get(focused_col)
                    .and_then(|t| t.as_ref())
                    .is_none()
            {
                selected_row
                    .cell_jump_targets
                    .iter()
                    .position(Option::is_some)
                    .unwrap_or(focused_col)
            } else {
                focused_col
            };

            let cell_value = selected_row
                .cells
                .get(nav_col)
                .map_or_else(Default::default, Clone::clone);
            let jump_target = selected_row
                .cell_jump_targets
                .get(nav_col)
                .cloned()
                .flatten();

            (node_idx, node_depth, element_type, cell_value, jump_target)
        };

        if let Some(element_type) = element_type {
            self.navigate_to_child_element(node_idx, node_depth, &element_type);
            return;
        }

        if cell_value.is_empty() || cell_value == "-" {
            self.status = "Empty cell".into();
            return;
        }

        self.execute_cell_jump(jump_target, &cell_value);
    }

    /// Execute a cell jump based on the per-cell jump target metadata
    fn execute_cell_jump(&mut self, jump_target: Option<CellJumpTarget>, cell_value: &str) {
        let Some(target) = jump_target else {
            self.status = "This cell is not navigable".into();
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
                if let Some(idx) = self.find_in_hierarchy(|n| n.text == cell_value) {
                    self.navigate_to_node(idx);
                } else {
                    self.status = format!("Node \"{cell_value}\" not found in tree");
                }
            }
            CellJumpTarget::ContainerByName => {
                self.navigate_to_container_by_name(cell_value);
            }
            CellJumpTarget::ServiceOrJobByName => {
                self.navigate_to_service_or_job(cell_value);
            }
        }
    }
}
