/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crate::{
    app::App,
    tree::{DetailRowType, DetailSectionType, NodeType, RowMetadata},
};

/// Type of not-inherited element, determines navigation strategy
enum NotInheritedElementType {
    /// `DiagComm` services — navigates to the service/job node
    DiagComm,
    /// Data Object Properties — navigates to the DOP node
    Dop,
    /// Table structures or `DiagVariables` — navigates to the matching tree node
    TreeNode,
}

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

        // Find parent container and navigate
        if let Some(container_idx) = self.find_container_by_name(&parent_layer_name) {
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

        // Check if the current node is a child of a ParentRefs node
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return false;
        };
        let node_depth = node.depth;

        if node_depth == 0 {
            return false;
        }

        // Walk backwards to find the parent node
        let parent_is_parent_refs = (0..node_idx).rev().any(|i| {
            self.tree.all_nodes.get(i).is_some_and(|n| {
                n.node_type == NodeType::ParentRefs && n.depth == node_depth.saturating_sub(1)
            })
        });

        if !parent_is_parent_refs {
            return false;
        }

        // The node text is the short name of the target
        let target_short_name = node.text.clone();
        self.navigate_to_container_by_name(&target_short_name);
        true
    }

    /// Navigate to a parent ref element from the Parent References table
    pub(crate) fn try_navigate_to_parent_ref(&mut self) {
        let target = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let Some(name_cell) = selected_row.cells.first() else {
                return;
            };
            name_cell.text.clone()
        };
        self.navigate_to_container_by_name(&target);
    }

    /// Navigate to a not-inherited element (`DiagComm`, `DiagVariable`, `Dop`, `Table`)
    pub(crate) fn try_navigate_to_not_inherited_element(&mut self) {
        let (element_type, target_short_name) = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };

            let element_type = match ctx.section.section_type {
                DetailSectionType::NotInheritedDiagComms => NotInheritedElementType::DiagComm,
                DetailSectionType::NotInheritedDops => NotInheritedElementType::Dop,
                DetailSectionType::NotInheritedTables
                | DetailSectionType::NotInheritedVariables => NotInheritedElementType::TreeNode,
                DetailSectionType::Header
                | DetailSectionType::Overview
                | DetailSectionType::Services
                | DetailSectionType::Requests
                | DetailSectionType::PosResponses
                | DetailSectionType::NegResponses
                | DetailSectionType::ComParams
                | DetailSectionType::States
                | DetailSectionType::RelatedRefs
                | DetailSectionType::FunctionalClass
                | DetailSectionType::Custom => return,
            };

            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let Some(name_cell) = selected_row.cells.first() else {
                return;
            };
            let name = name_cell.text.clone();
            if name.is_empty() {
                return;
            }
            (element_type, name)
        };

        match element_type {
            NotInheritedElementType::DiagComm => {
                self.navigate_to_service_or_job(&target_short_name);
            }
            NotInheritedElementType::Dop => {
                self.navigate_to_dop(&target_short_name);
            }
            NotInheritedElementType::TreeNode => {
                self.navigate_to_tree_node_by_text(&target_short_name);
            }
        }
    }
}
