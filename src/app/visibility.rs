/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, SearchScope};
use crate::tree::{DiffStatus, NodeType, TreeNode};

impl App {
    /// Check if a node at index i is under a specific section type
    fn is_under_section_type(
        &self,
        node_idx: usize,
        section_type: crate::tree::SectionType,
    ) -> bool {
        if node_idx == 0 {
            return false;
        }

        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return false;
        };
        let node_depth = node.depth;

        (0..node_idx)
            .rev()
            .filter_map(|i| self.tree.all_nodes.get(i))
            .filter(|parent| parent.depth < node_depth)
            .any(|parent| {
                parent.depth == 0 && matches!(&parent.section_type, Some(st) if *st == section_type)
            })
    }

    /// Check if `node_idx` is within the subtree rooted at `root_idx`.
    ///
    /// Walks forward from `root_idx + 1` until a node at the same or lesser
    /// depth is found, determining the subtree boundary dynamically.
    fn is_in_subtree(&self, root_idx: usize, node_idx: usize) -> bool {
        if node_idx == root_idx {
            return true;
        }
        if node_idx < root_idx {
            return false;
        }
        let Some(root) = self.tree.all_nodes.get(root_idx) else {
            return false;
        };
        let root_depth = root.depth;

        // Every node between root_idx+1 and node_idx (inclusive) must have
        // depth > root_depth for node_idx to still be inside the subtree.
        self.tree
            .all_nodes
            .get(root_idx.saturating_add(1)..=node_idx)
            .is_some_and(|slice| slice.iter().all(|n| n.depth > root_depth))
    }

    pub(crate) fn rebuild_visible(&mut self) {
        self.tree.visible.clear();

        if self.search.stack.is_empty() {
            self.rebuild_visible_no_search();
        } else {
            self.rebuild_visible_with_search();
        }
    }

    /// Rebuild visible list when no search is active
    fn rebuild_visible_no_search(&mut self) {
        let mut collapsed_below: Option<usize> = None;

        for (i, node) in self.tree.all_nodes.iter().enumerate() {
            // Skip nodes under collapsed parent
            if let Some(cd) = collapsed_below {
                if node.depth > cd {
                    continue;
                }
                collapsed_below = None;
            }

            // Skip unchanged diff nodes when filter is active
            if self.hide_unchanged && matches!(node.diff_status, Some(DiffStatus::Unchanged)) {
                continue;
            }

            // Skip section headers whose children are all hidden (unchanged).
            // Section headers have diff_status: None and depth 0.
            if self.hide_unchanged
                && node.diff_status.is_none()
                && node.node_type == NodeType::SectionHeader
            {
                let all_children_unchanged = self
                    .tree
                    .all_nodes
                    .get(i.saturating_add(1)..)
                    .unwrap_or_default()
                    .iter()
                    .take_while(|ch| ch.depth > node.depth)
                    .all(|ch| matches!(ch.diff_status, Some(DiffStatus::Unchanged)));
                if all_children_unchanged {
                    // Also skip children under this header
                    collapsed_below = Some(node.depth);
                    continue;
                }
            }

            self.tree.visible.push(i);

            // Mark as collapsed if node has unexpanded children
            if node.has_children && !node.expanded {
                collapsed_below = Some(node.depth);
            }
        }
    }

    /// Rebuild visible list with active search stack
    fn rebuild_visible_with_search(&mut self) {
        // Start with all nodes included, then filter by each search
        let mut include = vec![true; self.tree.all_nodes.len()];

        // Apply each search filter cumulatively
        for entry in &self.search.stack {
            include = self.apply_search_filter(&include, &entry.query, &entry.scope);
        }

        // Build visible list from included nodes, respecting collapse state
        self.build_visible_from_filter(&include);
    }

    /// Apply a single search filter to the include vector
    fn apply_search_filter(&self, include: &[bool], query: &str, scope: &SearchScope) -> Vec<bool> {
        let q = query.to_lowercase();
        let len = self.tree.all_nodes.len();
        let mut new_include = vec![false; len];

        // Pass 1: Mark matching nodes and all their children
        let mut skip_below: Option<usize> = None; // depth of matched parent
        for (i, &included) in include.iter().enumerate().take(len) {
            let Some(node) = self.tree.all_nodes.get(i) else {
                continue;
            };

            // If inside a matched subtree, include if previously included
            if let Some(depth) = skip_below {
                if node.depth > depth {
                    if included && let Some(slot) = new_include.get_mut(i) {
                        *slot = true;
                    }
                    continue;
                }
                skip_below = None;
            }

            if !included {
                continue;
            }

            if self.node_matches_scope_and_query(node, i, scope, &q) {
                if let Some(slot) = new_include.get_mut(i) {
                    *slot = true;
                }
                skip_below = Some(node.depth);
            }
        }

        // unwrap_or(0): empty tree → no nodes → parent_at_depth is empty → the
        // subsequent depth walk is a no-op, so 0 is correct and safe.
        let max_depth = self
            .tree
            .all_nodes
            .iter()
            .map(|n| n.depth)
            .max()
            .unwrap_or(0);
        let mut parent_at_depth = vec![0usize; max_depth.saturating_add(1)];

        for (i, node) in self.tree.all_nodes.iter().enumerate() {
            if let Some(slot) = parent_at_depth.get_mut(node.depth) {
                *slot = i;
            }

            if new_include.get(i).copied().unwrap_or(false) && node.depth > 0 {
                for d in (0..node.depth).rev() {
                    let Some(&ancestor) = parent_at_depth.get(d) else {
                        break;
                    };
                    if new_include.get(ancestor).copied().unwrap_or(false) {
                        break;
                    }
                    if let Some(slot) = new_include.get_mut(ancestor) {
                        *slot = true;
                    }
                }
            }
        }

        new_include
    }

    /// Check if a node matches the search scope and query
    pub(crate) fn node_matches_scope_and_query(
        &self,
        node: &TreeNode,
        node_idx: usize,
        scope: &SearchScope,
        query: &str,
    ) -> bool {
        let matches_scope = match scope {
            SearchScope::All => true,
            SearchScope::Variants => {
                matches!(node.section_type, Some(crate::tree::SectionType::Variants))
                    || (matches!(node.node_type, NodeType::Container)
                        && node_idx > 0
                        && self.is_under_section_type(node_idx, crate::tree::SectionType::Variants))
            }
            SearchScope::FunctionalGroups => {
                matches!(
                    node.section_type,
                    Some(crate::tree::SectionType::FunctionalGroups)
                ) || (matches!(node.node_type, NodeType::Container)
                    && node_idx > 0
                    && self.is_under_section_type(
                        node_idx,
                        crate::tree::SectionType::FunctionalGroups,
                    ))
            }
            SearchScope::EcuSharedData => {
                matches!(
                    node.section_type,
                    Some(crate::tree::SectionType::EcuSharedData)
                ) || (matches!(node.node_type, NodeType::Container)
                    && node_idx > 0
                    && self
                        .is_under_section_type(node_idx, crate::tree::SectionType::EcuSharedData))
            }
            SearchScope::Services => matches!(
                node.node_type,
                NodeType::Service
                    | NodeType::ParentRefService
                    | NodeType::Request
                    | NodeType::PosResponse
                    | NodeType::NegResponse
            ),
            SearchScope::DiagComms => {
                Self::is_service_list_type(node, crate::tree::ServiceListType::DiagComms)
                    || matches!(
                        node.node_type,
                        NodeType::Service | NodeType::ParentRefService | NodeType::Job
                    )
            }
            SearchScope::Requests => {
                Self::is_service_list_type(node, crate::tree::ServiceListType::Requests)
                    || matches!(node.node_type, NodeType::Request)
            }
            SearchScope::Responses => {
                Self::is_service_list_type(node, crate::tree::ServiceListType::PosResponses)
                    || Self::is_service_list_type(node, crate::tree::ServiceListType::NegResponses)
                    || matches!(
                        node.node_type,
                        NodeType::PosResponse | NodeType::NegResponse
                    )
            }
            SearchScope::Subtree { root_idx, .. } => self.is_in_subtree(*root_idx, node_idx),
        };

        matches_scope && node.text.to_lowercase().contains(query)
    }

    /// Build visible list from include filter, respecting collapse state
    fn build_visible_from_filter(&mut self, include: &[bool]) {
        let mut collapsed_below: Option<usize> = None;

        for (i, &should_include) in include.iter().enumerate() {
            if !should_include {
                continue;
            }

            let Some(node) = self.tree.all_nodes.get(i) else {
                continue;
            };

            // Check if we're inside a collapsed section
            if let Some(cd) = collapsed_below {
                if node.depth > cd {
                    continue;
                }
                collapsed_below = None;
            }

            self.tree.visible.push(i);

            // If this node is collapsed, hide its children
            if node.has_children && !node.expanded {
                collapsed_below = Some(node.depth);
            }
        }
    }
}
