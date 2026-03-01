/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState, HistoryEntry, SCROLL_CONTEXT_LINES};

impl App {
    /// Build the path from root to the given node index as a list of
    /// `(depth, text)` pairs. Walking backwards from `node_idx`, we collect
    /// each ancestor (first node at each successively lower depth).
    fn build_node_path(&self, node_idx: usize) -> Vec<(usize, String)> {
        let Some(target) = self.tree.all_nodes.get(node_idx) else {
            return vec![];
        };

        let mut path: Vec<(usize, String)> = (0..node_idx)
            .rev()
            .filter_map(|i| self.tree.all_nodes.get(i))
            .scan(target.depth, |cur_depth, node| {
                if *cur_depth == 0 {
                    None
                } else if node.depth < *cur_depth {
                    *cur_depth = node.depth;
                    Some(Some((node.depth, node.text.clone())))
                } else {
                    Some(None)
                }
            })
            .flatten()
            .collect();

        path.reverse();
        path.push((target.depth, target.text.clone()));
        path
    }

    /// Resolve a stored path back to a `node_idx` by walking the tree.
    /// Does not expand ancestors or rebuild visible — callers must call
    /// `ensure_node_visible` afterwards.
    fn resolve_path(&self, path: &[(usize, String)]) -> Option<usize> {
        let mut last_found_idx: Option<usize> = None;

        for (target_depth, target_text) in path {
            let search_start = last_found_idx.map_or(0, |i| i.saturating_add(1));

            let found = self
                .tree
                .all_nodes
                .iter()
                .enumerate()
                .skip(search_start)
                .find(|(_, node)| node.depth == *target_depth && node.text == *target_text)
                .map(|(i, _)| i);

            let Some(idx) = found else {
                return last_found_idx;
            };

            last_found_idx = Some(idx);
        }

        last_found_idx
    }

    /// Add current node to navigation history, storing the full path for
    /// robust lookup even after expand/collapse changes.
    pub(crate) fn push_to_history(&mut self) {
        const MAX_HISTORY: usize = 100;

        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };

        let path = self.build_node_path(node_idx);

        // Don't store duplicate consecutive entries
        if self
            .history
            .entries
            .back()
            .is_some_and(|e| e.node_path == path)
        {
            return;
        }

        // Truncate forward history if not at end
        if self.history.position < self.history.entries.len() {
            self.history.entries.truncate(self.history.position);
        }

        let entry = HistoryEntry {
            node_idx,
            node_path: path,
        };

        self.history.entries.push_back(entry);
        if self.history.entries.len() > MAX_HISTORY {
            self.history.entries.pop_front();
        }
        self.history.position = self.history.entries.len();
    }

    /// Navigate to the previous element in navigation history
    pub(crate) fn navigate_to_previous_in_history(&mut self) {
        if self.history.entries.is_empty() {
            self.status = "No previous element in history".into();
            return;
        }

        if self.history.position == 0 {
            self.status = "Already at oldest element in history".into();
            return;
        }

        self.history.position = self.history.position.saturating_sub(1);
        let Some(entry) = self.history.entries.get(self.history.position).cloned() else {
            self.status = "History access failed".into();
            return;
        };

        // First try the stored node_idx (fast path — still correct if tree
        // structure hasn't changed)
        let target_node_idx = if self
            .tree
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
                self.status = "Previous element no longer reachable".into();
                return;
            };
            self.ensure_node_visible(idx);
            idx
        };

        let Some(cursor_pos) = self
            .tree
            .visible
            .iter()
            .position(|&idx| idx == target_node_idx)
        else {
            self.status = "Previous element no longer reachable".into();
            return;
        };

        self.tree.cursor = cursor_pos;
        self.reset_detail_state();
        self.tree.scroll_offset = self.tree.cursor.saturating_sub(SCROLL_CONTEXT_LINES);
        self.focus_state = FocusState::Tree;
        if let Some(node) = self.tree.all_nodes.get(target_node_idx) {
            self.status = format!("Navigated to: {}", node.text);
        }
    }
}
