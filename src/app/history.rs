/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState, HistoryEntry, SCROLL_CONTEXT_LINES};

impl App {
    /// Add current node to navigation history.
    pub(crate) fn push_to_history(&mut self) {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        self.push_node_to_history(node_idx);
    }

    /// Capture the current cursor's node index for deferred history push.
    pub(crate) fn capture_current_node_idx(&self) -> Option<usize> {
        self.tree.visible.get(self.tree.cursor).copied()
    }

    /// Add a node index to navigation history.
    pub(crate) fn push_node_to_history(&mut self, node_idx: usize) {
        const MAX_HISTORY: usize = 100;

        if node_idx >= self.tree.all_nodes.len() {
            return;
        }

        // Don't store duplicate consecutive entries
        if self
            .history
            .entries
            .back()
            .is_some_and(|e| e.node_idx == node_idx)
        {
            return;
        }

        // Truncate forward history if not at end
        if self.history.position < self.history.entries.len() {
            self.history.entries.truncate(self.history.position);
        }

        self.history.entries.push_back(HistoryEntry { node_idx });
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
        let Some(entry) = self.history.entries.get(self.history.position) else {
            self.status = "History access failed".into();
            return;
        };
        let target_node_idx = entry.node_idx;

        if target_node_idx >= self.tree.all_nodes.len() {
            self.status = "Previous element no longer reachable".into();
            return;
        }

        self.ensure_node_visible(target_node_idx);

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
