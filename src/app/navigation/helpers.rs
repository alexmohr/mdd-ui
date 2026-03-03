/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::borrow::Cow;

use crate::{
    app::{App, FocusState, SCROLL_CONTEXT_LINES},
    tree::{
        ChildElementType, DetailRow, DetailSectionData, DetailSectionType, NodeType,
        ServiceListType, TreeNode,
    },
};

/// Resolved state of the currently selected detail-table row.
/// All references borrow from the owning [`App`], so the context must be
/// dropped before any `&mut self` call.
pub(super) struct SelectedRowContext<'a> {
    pub node_idx: usize,
    pub node: &'a TreeNode,
    pub section: &'a DetailSectionData,
    pub row_cursor: usize,
    pub sorted_rows: Cow<'a, [DetailRow]>,
    pub use_row_selection: bool,
}

impl SelectedRowContext<'_> {
    /// Access the selected row (bounds already verified by [`App::resolve_selected_row`]).
    pub fn selected_row(&self) -> Option<&DetailRow> {
        self.sorted_rows.get(self.row_cursor)
    }
}

impl App {
    /// Resolve the currently selected detail-table row in one step.
    ///
    /// Performs cursor bounds check, node lookup, section lookup,
    /// table-row extraction, row-cursor lookup, and sorting.
    /// Returns `None` when any step fails (cursor out of bounds, no
    /// table data, etc.).
    pub(super) fn resolve_selected_row(&self) -> Option<SelectedRowContext<'_>> {
        let &node_idx = self.tree.visible.get(self.tree.cursor)?;
        let node = self.tree.all_nodes.get(node_idx)?;
        let section_idx = self.get_section_index();
        let section = node.detail_sections.get(section_idx)?;
        let rows = section.content.table_rows()?;
        let use_row_selection = section.content.table_use_row_selection().unwrap_or(false);
        let &row_cursor = self.detail.section_cursors.get(section_idx)?;
        let sorted_rows = self.sort_rows(rows, section_idx);
        if row_cursor >= sorted_rows.len() {
            return None;
        }
        Some(SelectedRowContext {
            node_idx,
            node,
            section,
            row_cursor,
            sorted_rows,
            use_row_selection,
        })
    }

    /// Expand section and update cursor position
    pub(super) fn expand_and_update_cursor(&mut self, node_idx: usize) {
        if let Some(node_mut) = self.tree.all_nodes.get_mut(node_idx) {
            node_mut.expanded = true;
        }
        self.rebuild_visible();

        if let Some(new_cursor) = self.tree.visible.iter().position(|&idx| idx == node_idx) {
            self.tree.cursor = new_cursor;
        }
    }

    /// Expand all ancestors of a node
    pub(super) fn expand_node_ancestors(&mut self, node_idx: usize) {
        let Some(target_node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };
        let target_depth = target_node.depth;

        if target_depth == 0 {
            return;
        }

        let to_expand: Vec<usize> = (0..node_idx)
            .filter(|&i| {
                self.tree
                    .all_nodes
                    .get(i)
                    .is_some_and(|n| n.depth < target_depth && n.has_children)
            })
            .collect();
        for &i in &to_expand {
            if let Some(n) = self.tree.all_nodes.get_mut(i) {
                n.expanded = true;
            }
        }
    }

    /// Find a container (variant/functional group) by name
    pub(super) fn find_container_by_name(&self, name: &str) -> Option<usize> {
        self.tree.all_nodes.iter().position(|node| {
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

    /// Find the containing depth-1 Container node for a given node index
    /// by walking backwards from the node to its nearest Container ancestor.
    pub(super) fn find_current_container(&self, from_node_idx: usize) -> Option<usize> {
        let from_node = self.tree.all_nodes.get(from_node_idx)?;

        // If already a depth-1 container, return it
        if from_node.depth == 1 && matches!(from_node.node_type, NodeType::Container) {
            return Some(from_node_idx);
        }

        // Walk backwards to find the nearest depth-1 Container ancestor
        (0..from_node_idx).rev().find(|&i| {
            self.tree
                .all_nodes
                .get(i)
                .is_some_and(|n| n.depth == 1 && matches!(n.node_type, NodeType::Container))
        })
    }

    /// Find the enclosing top-level section header (depth 0) for a given
    /// node. Returns the node's own index when it is already at depth 0.
    pub(super) fn find_enclosing_section(&self, from_node_idx: usize) -> Option<usize> {
        let from_node = self.tree.all_nodes.get(from_node_idx)?;
        if from_node.depth == 0 {
            return Some(from_node_idx);
        }
        (0..from_node_idx)
            .rev()
            .find(|&i| self.tree.all_nodes.get(i).is_some_and(|n| n.depth == 0))
    }

    /// Get the subtree range (`start_idx` inclusive, `end_idx` exclusive)
    /// for a given node. The subtree includes the node itself and all
    /// children with deeper depth.
    pub(super) fn subtree_range(&self, node_idx: usize) -> (usize, usize) {
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return (node_idx, node_idx);
        };
        let node_depth = node.depth;

        let end = self
            .tree
            .all_nodes
            .iter()
            .enumerate()
            .skip(node_idx.saturating_add(1))
            .find(|(_, n)| n.depth <= node_depth)
            .map_or(self.tree.all_nodes.len(), |(i, _)| i);

        (node_idx, end)
    }

    /// Search for a node within a subtree using a predicate.
    pub(super) fn find_in_subtree(
        &self,
        start: usize,
        end: usize,
        predicate: impl Fn(&TreeNode) -> bool,
    ) -> Option<usize> {
        self.tree
            .all_nodes
            .iter()
            .enumerate()
            .skip(start)
            .take(end.saturating_sub(start))
            .find(|(_, node)| predicate(node))
            .map(|(idx, _)| idx)
    }

    /// Search for a node at exactly `target_depth` within a range.
    pub(super) fn find_at_depth(
        &self,
        start: usize,
        end: usize,
        target_depth: usize,
        predicate: &impl Fn(&TreeNode) -> bool,
    ) -> Option<usize> {
        self.tree
            .all_nodes
            .iter()
            .enumerate()
            .skip(start.saturating_add(1))
            .take(end.saturating_sub(start).saturating_sub(1))
            .filter(|(_, n)| n.depth == target_depth)
            .find(|(_, n)| predicate(n))
            .map(|(i, _)| i)
    }

    /// Walk the database inheritance chain starting from a container node.
    /// Uses the `parent_ref_names` stored on each Container node (populated
    /// at build time from the database) to look up matching containers by
    /// exact short name, then searches each container's subtree.
    ///
    /// `visited` prevents infinite loops when the parent ref graph has cycles.
    fn walk_parent_refs(
        &self,
        container_idx: usize,
        predicate: &impl Fn(&TreeNode) -> bool,
        visited: &mut Vec<usize>,
    ) -> Option<usize> {
        let container = self.tree.all_nodes.get(container_idx)?;
        let parent_names = &container.parent_ref_names;

        parent_names.iter().find_map(|parent_name| {
            let pc_idx = self.find_container_by_name(parent_name)?;
            if visited.contains(&pc_idx) {
                return None;
            }
            visited.push(pc_idx);

            let (pc_start, pc_end) = self.subtree_range(pc_idx);
            self.find_in_subtree(pc_start, pc_end, predicate)
                .or_else(|| self.walk_parent_refs(pc_idx, predicate, visited))
        })
    }

    /// Find the enclosing service-list section header for a given node.
    /// Walks backwards to find the nearest ancestor with a
    /// `service_list_type` (e.g. Diag-Comms, Pos-Responses).
    pub(super) fn find_enclosing_list_section(&self, from_node_idx: usize) -> Option<usize> {
        let from_node = self.tree.all_nodes.get(from_node_idx)?;
        if from_node.service_list_type.is_some() {
            return Some(from_node_idx);
        }
        let from_depth = from_node.depth;
        (0..from_node_idx).rev().find(|&i| {
            self.tree
                .all_nodes
                .get(i)
                .is_some_and(|n| n.depth < from_depth && n.service_list_type.is_some())
        })
    }

    /// Search for a node following the database hierarchy. The search order
    /// mirrors the database inheritance structure:
    ///
    /// 1. Direct children of the current node (depth + 1).
    /// 2. Full subtree of the current node (deeper descendants).
    /// 3. Enclosing service-list section's subtree (e.g. Pos-Responses).
    /// 4. Enclosing container's subtree (broader cross-section search).
    /// 5. Walk parent-ref containers using the DB-derived `parent_ref_names`
    ///    chain stored on the container node, recursively following each
    ///    parent's own parent refs.
    ///
    /// No global fallback — resolution is strictly scoped to the database
    /// hierarchy.
    pub(super) fn find_in_hierarchy(&self, predicate: impl Fn(&TreeNode) -> bool) -> Option<usize> {
        let current_node_idx = self.tree.visible.get(self.tree.cursor).copied()?;
        let current_node = self.tree.all_nodes.get(current_node_idx)?;
        let current_depth = current_node.depth;
        let (node_start, node_end) = self.subtree_range(current_node_idx);

        // 1. Direct children of the current node (depth + 1)
        if let Some(found) = self.find_at_depth(
            node_start,
            node_end,
            current_depth.saturating_add(1),
            &predicate,
        ) {
            return Some(found);
        }

        // 2. Full subtree of the current node (deeper descendants)
        if let Some(found) =
            self.find_in_subtree(node_start.saturating_add(1), node_end, &predicate)
        {
            return Some(found);
        }

        // 3. Enclosing service-list section's subtree (e.g. search within
        //    Pos-Responses before searching the entire container). This
        //    prevents matching a same-named node in a sibling section.
        if let Some(list_section_idx) = self.find_enclosing_list_section(current_node_idx)
            && list_section_idx != current_node_idx
        {
            let (ls_start, ls_end) = self.subtree_range(list_section_idx);
            if let Some(found) = self.find_in_subtree(ls_start, ls_end, &predicate) {
                return Some(found);
            }
        }

        // 4. Enclosing container's subtree (when current node is deeper
        //    than the container). Validate the container belongs to the
        //    same section — find_current_container walks backwards
        //    unconditionally and may return one from a preceding section.
        let section_idx = self.find_enclosing_section(current_node_idx)?;
        let (sec_start, sec_end) = self.subtree_range(section_idx);

        let c_idx = self.find_current_container(current_node_idx)?;
        if c_idx < sec_start || c_idx >= sec_end || c_idx == current_node_idx {
            return None;
        }

        let (c_start, c_end) = self.subtree_range(c_idx);
        if let Some(found) = self.find_in_subtree(c_start, c_end, &predicate) {
            return Some(found);
        }

        // 5. Walk parent-ref containers using the DB-derived hierarchy
        let mut visited = vec![c_idx];
        self.walk_parent_refs(c_idx, &predicate, &mut visited)
    }

    // ------------------------------------------------------------------
    // Path-based lookups: Container → Section (by ServiceListType) → Node
    // ------------------------------------------------------------------

    /// Map a `DetailSectionType` (tab) to the corresponding
    /// `ServiceListType` (tree section header).
    pub(super) fn section_type_to_list_type(
        section_type: DetailSectionType,
    ) -> Option<ServiceListType> {
        match section_type {
            DetailSectionType::Requests => Some(ServiceListType::Requests),
            DetailSectionType::PosResponses => Some(ServiceListType::PosResponses),
            DetailSectionType::NegResponses => Some(ServiceListType::NegResponses),
            DetailSectionType::Services => Some(ServiceListType::DiagComms),
            DetailSectionType::Header
            | DetailSectionType::Overview
            | DetailSectionType::ComParams
            | DetailSectionType::States
            | DetailSectionType::RelatedRefs
            | DetailSectionType::FunctionalClass
            | DetailSectionType::NotInheritedDiagComms
            | DetailSectionType::NotInheritedDops
            | DetailSectionType::NotInheritedTables
            | DetailSectionType::NotInheritedVariables
            | DetailSectionType::Custom => None,
        }
    }

    /// Find a section header by its `ServiceListType` within a container's
    /// direct children (depth = `container_depth + 1`).
    fn find_section_in_container(
        &self,
        container_idx: usize,
        target_list_type: ServiceListType,
    ) -> Option<usize> {
        let container = self.tree.all_nodes.get(container_idx)?;
        let target_depth = container.depth.saturating_add(1);
        let (c_start, c_end) = self.subtree_range(container_idx);
        self.find_at_depth(c_start, c_end, target_depth, &|n| {
            n.service_list_type == Some(target_list_type)
        })
    }

    /// Find a service/response/request node by name within a section
    /// header's direct children (depth = `section_depth + 1`).
    fn find_service_in_section_by_name(
        &self,
        section_idx: usize,
        service_name: &str,
    ) -> Option<usize> {
        let section = self.tree.all_nodes.get(section_idx)?;
        let target_depth = section.depth.saturating_add(1);
        let (s_start, s_end) = self.subtree_range(section_idx);
        self.find_at_depth(s_start, s_end, target_depth, &|n| n.text == service_name)
    }

    /// Walk a path: container → section header (by `ServiceListType`) →
    /// service (by name). Returns the service node index.
    /// Falls back to parent-ref containers when the target is not in the
    /// current container.
    pub(super) fn find_by_section_path(
        &self,
        container_idx: usize,
        target_list_type: ServiceListType,
        service_name: &str,
    ) -> Option<usize> {
        // Current container
        if let Some(idx) =
            self.find_service_in_container_path(container_idx, target_list_type, service_name)
        {
            return Some(idx);
        }

        // Walk parent-ref containers
        let mut visited = vec![container_idx];
        self.walk_parent_refs_by_path(container_idx, target_list_type, service_name, &mut visited)
    }

    /// Container → section → service (no fallback).
    fn find_service_in_container_path(
        &self,
        container_idx: usize,
        target_list_type: ServiceListType,
        service_name: &str,
    ) -> Option<usize> {
        let section_idx = self.find_section_in_container(container_idx, target_list_type)?;
        self.find_service_in_section_by_name(section_idx, service_name)
    }

    /// Recursively walk parent-ref containers looking for the path
    /// container → section → service.
    fn walk_parent_refs_by_path(
        &self,
        container_idx: usize,
        target_list_type: ServiceListType,
        service_name: &str,
        visited: &mut Vec<usize>,
    ) -> Option<usize> {
        let container = self.tree.all_nodes.get(container_idx)?;
        let parent_names = &container.parent_ref_names;

        parent_names.iter().find_map(|parent_name| {
            let pc_idx = self.find_container_by_name(parent_name)?;
            if visited.contains(&pc_idx) {
                return None;
            }
            visited.push(pc_idx);

            self.find_service_in_container_path(pc_idx, target_list_type, service_name)
                .or_else(|| {
                    self.walk_parent_refs_by_path(pc_idx, target_list_type, service_name, visited)
                })
        })
    }

    /// Navigate to a child tree node whose text matches the given `ChildElementType`.
    /// Searches direct children (depth + 1) of the node at `parent_node_idx`.
    pub(super) fn navigate_to_child_element(
        &mut self,
        parent_node_idx: usize,
        parent_depth: usize,
        element_type: &ChildElementType,
    ) {
        let target_depth = parent_depth.saturating_add(1);

        let target_idx = self
            .tree
            .all_nodes
            .iter()
            .enumerate()
            .skip(parent_node_idx.saturating_add(1))
            .take_while(|(_, n)| n.depth > parent_depth)
            .find(|(_, n)| n.depth == target_depth && element_type.matches_node_text(&n.text))
            .map(|(i, _)| i);

        let Some(target_node_idx) = target_idx else {
            self.status = format!("Section '{element_type}' not found");
            return;
        };

        self.ensure_node_visible(target_node_idx);

        let Some(new_cursor) = self
            .tree
            .visible
            .iter()
            .position(|&idx| idx == target_node_idx)
        else {
            return;
        };

        self.push_to_history();
        self.focus_state = FocusState::Tree;
        self.tree.cursor = new_cursor;
        self.reset_detail_state();
        self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
        self.status = format!("Navigated to: {element_type}");
    }

    /// Navigate to a Container node matching the given short name.
    ///
    /// Uses the database hierarchy via `parent_ref_names`:
    /// 1. Current container's subtree (children first).
    /// 2. Walk the DB parent-ref chain from the current container.
    /// 3. Global fallback by exact name (needed for cross-section parent-ref
    ///    navigation, e.g. variant → ECU Shared Data).
    pub(super) fn navigate_to_container_by_name(&mut self, target_short_name: &str) {
        let is_target = |n: &TreeNode| -> bool {
            matches!(n.node_type, NodeType::Container) && {
                let name = n
                    .text
                    .find(" [")
                    .map_or(n.text.as_str(), |idx| &n.text[..idx]);
                name == target_short_name
            }
        };

        // 1. Search the DB hierarchy (current container + parent refs)
        if let Some(idx) = self.find_in_hierarchy(is_target) {
            self.navigate_to_node(idx);
            self.status = format!("Navigated to: {target_short_name}");
            return;
        }

        // 2. Global fallback by exact name (for cross-section parent ref
        //    targets that live outside the current hierarchy)
        if let Some(idx) = self.find_container_by_name(target_short_name) {
            self.navigate_to_node(idx);
            self.status = format!("Navigated to: {target_short_name} (cross-section)");
            return;
        }

        self.status = format!("Element '{target_short_name}' not found in tree");
    }

    /// Navigate to a tree node whose text matches the given name.
    /// Scoped to the enclosing section via `find_in_hierarchy`.
    pub(super) fn navigate_to_tree_node_by_text(&mut self, target_name: &str) {
        if let Some(idx) = self.find_in_hierarchy(|node| node.text == target_name) {
            self.navigate_to_node(idx);
            self.status = format!("Navigated to: {target_name}");
        } else {
            self.status = format!("'{target_name}' not found in tree");
        }
    }

    /// Navigate to a specific node by its index in `all_nodes`.
    pub(crate) fn navigate_to_node(&mut self, target_node_idx: usize) {
        // Clear search stack so navigation target is always reachable
        if !self.search.stack.is_empty() {
            self.search.stack.clear();
            self.search.query.clear();
            self.status = "Search cleared for navigation".into();
        }

        self.ensure_node_visible(target_node_idx);

        let Some(visible_pos) = self
            .tree
            .visible
            .iter()
            .position(|&idx| idx == target_node_idx)
        else {
            return;
        };

        self.push_to_history();
        self.focus_state = FocusState::Tree;
        self.tree.cursor = visible_pos;
        self.reset_detail_state();
        self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
    }

    /// Ensure a node is visible by expanding only its direct ancestors.
    pub(crate) fn ensure_node_visible(&mut self, target_node_idx: usize) {
        let Some(target_node) = self.tree.all_nodes.get(target_node_idx) else {
            return;
        };

        let mut needed_depth = target_node.depth;

        // Walk backwards, expanding only the direct ancestor at each level
        for i in (0..target_node_idx).rev() {
            let Some(node) = self.tree.all_nodes.get_mut(i) else {
                continue;
            };
            let node_depth = node.depth;

            if node_depth < needed_depth {
                node.expanded = true;
                needed_depth = node_depth;

                if node_depth == 0 {
                    break;
                }
            }
        }

        // Rebuild visible list to reflect expansions
        self.rebuild_visible();
    }
}
