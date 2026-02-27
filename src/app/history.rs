/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState, HistoryEntry};

impl App {
    /// Build the path from root to the given node index as a list of
    /// `(depth, text)` pairs. Walking backwards from `node_idx`, we collect
    /// each ancestor (first node at each successively lower depth).
    fn build_node_path(&self, node_idx: usize) -> Vec<(usize, String)> {
        let Some(target) = self.all_nodes.get(node_idx) else {
            return vec![];
        };

        let mut path = vec![(target.depth, target.text.clone())];
        let mut current_depth = target.depth;

        // Walk backwards to collect ancestors
        for i in (0..node_idx).rev() {
            if current_depth == 0 {
                break;
            }
            let Some(node) = self.all_nodes.get(i) else {
                continue;
            };
            if node.depth < current_depth {
                path.push((node.depth, node.text.clone()));
                current_depth = node.depth;
            }
        }

        path.reverse();
        path
    }

    /// Resolve a stored path back to a `node_idx` by walking the tree and
    /// expanding ancestors as needed so the target becomes visible.
    fn resolve_path(&mut self, path: &[(usize, String)]) -> Option<usize> {
        // Walk through the path entries, ensuring each ancestor is expanded
        let mut last_found_idx: Option<usize> = None;

        for (target_depth, target_text) in path {
            let search_start = last_found_idx.map_or(0, |i| i.saturating_add(1));

            let found = self
                .all_nodes
                .iter()
                .enumerate()
                .skip(search_start)
                .find(|(_, node)| node.depth == *target_depth && node.text == *target_text)
                .map(|(i, _)| i);

            let Some(idx) = found else {
                // Path entry not found; return last match as best effort
                return last_found_idx;
            };

            // Expand this node so subsequent children are visible
            if let Some(node) = self.all_nodes.get_mut(idx)
                && node.has_children
            {
                node.expanded = true;
            }

            last_found_idx = Some(idx);
        }

        // Rebuild visible list with the expanded ancestors
        self.rebuild_visible();
        last_found_idx
    }

    /// Add current node to navigation history, storing the full path for
    /// robust lookup even after expand/collapse changes.
    pub(crate) fn push_to_history(&mut self) {
        const MAX_HISTORY: usize = 100;

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };

        let path = self.build_node_path(node_idx);

        // Don't store duplicate consecutive entries
        if self
            .navigation_history
            .last()
            .is_some_and(|e| e.node_path == path)
        {
            return;
        }

        // Truncate forward history if not at end
        if self.history_position < self.navigation_history.len() {
            self.navigation_history.truncate(self.history_position);
        }

        let entry = HistoryEntry {
            node_idx,
            node_path: path,
        };

        self.navigation_history.push(entry);
        self.history_position = self.navigation_history.len();
        if self.navigation_history.len() > MAX_HISTORY {
            self.navigation_history.remove(0);
            self.history_position = self.navigation_history.len();
        }
    }

    /// Navigate to the previous element in navigation history
    pub(crate) fn navigate_to_previous_in_history(&mut self) {
        if self.navigation_history.is_empty() {
            "No previous element in history".clone_into(&mut self.status);
            return;
        }

        if self.history_position == 0 {
            "Already at oldest element in history".clone_into(&mut self.status);
            return;
        }

        self.history_position = self.history_position.saturating_sub(1);
        let Some(entry) = self.navigation_history.get(self.history_position).cloned() else {
            "History access failed".clone_into(&mut self.status);
            return;
        };

        // First try the stored node_idx (fast path — still correct if tree
        // structure hasn't changed)
        let target_node_idx = if self
            .all_nodes
            .get(entry.node_idx)
            .is_some_and(|n| entry.node_path.last().is_some_and(|(_, t)| *t == n.text))
        {
            // Index still points to the same node; ensure it's visible
            self.ensure_node_visible(entry.node_idx);
            entry.node_idx
        } else {
            // Fall back to path-based resolution
            let Some(idx) = self.resolve_path(&entry.node_path) else {
                "Previous element no longer reachable".clone_into(&mut self.status);
                return;
            };
            idx
        };

        let Some(cursor_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) else {
            "Previous element no longer reachable".clone_into(&mut self.status);
            return;
        };

        self.cursor = cursor_pos;
        self.reset_detail_state();
        self.scroll_offset = self.cursor.saturating_sub(5);
        self.focus_state = FocusState::Tree;
        if let Some(node) = self.all_nodes.get(target_node_idx) {
            self.status = format!("Navigated to: {}", node.text);
        }
    }
}
