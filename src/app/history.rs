/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState};

impl App {
    /// Add current node to navigation history (stores `all_nodes` index for stability)
    pub(crate) fn push_to_history(&mut self) {
        // Limit history size to prevent unbounded growth
        const MAX_HISTORY: usize = 100;

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };

        // Don't store duplicate consecutive entries
        if self.navigation_history.last() == Some(&node_idx) {
            return;
        }

        // Truncate forward history if not at end
        if self.history_position < self.navigation_history.len() {
            self.navigation_history.truncate(self.history_position);
        }

        self.navigation_history.push(node_idx);
        self.history_position = self.navigation_history.len();
        if self.navigation_history.len() > MAX_HISTORY {
            self.navigation_history.remove(0);
            self.history_position = self.navigation_history.len();
        }
    }

    /// Navigate to the previous element in navigation history
    pub(crate) fn navigate_to_previous_in_history(&mut self) {
        // Need at least 2 elements (current + previous)
        if self.navigation_history.len() < 2 {
            "No previous element in history".clone_into(&mut self.status);
            return;
        }

        if self.history_position <= 1 {
            "Already at oldest element in history".clone_into(&mut self.status);
            return;
        }

        self.history_position = self.history_position.saturating_sub(1);
        let Some(&target_node_idx) = self
            .navigation_history
            .get(self.history_position.saturating_sub(1))
        else {
            "History access failed".clone_into(&mut self.status);
            return;
        };

        // Ensure ancestors are expanded so the target node becomes visible
        self.ensure_node_visible(target_node_idx);

        // Find the target node in the (now-updated) visible list
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

    /// Navigate up one level in hierarchy (parent node)
    pub(crate) fn navigate_up_one_layer(&mut self) {
        // Get the current node
        if self.cursor >= self.visible.len() {
            "No parent to navigate to".clone_into(&mut self.status);
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(current_node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let current_depth = current_node.depth;

        // If we're at the root level, can't go up
        if current_depth == 0 {
            "Already at root level".clone_into(&mut self.status);
            return;
        }

        // Find parent by looking for previous node with lower depth
        let mut found_parent = false;
        for i in (0..node_idx).rev() {
            if let Some(node_at_i) = self.all_nodes.get(i)
                && node_at_i.depth < current_depth
            {
                // Found parent node, now find it in visible list
                if let Some(visible_pos) = self.visible.iter().position(|&idx| idx == i) {
                    self.cursor = visible_pos;
                    self.reset_detail_state();
                    self.scroll_offset = self.cursor.saturating_sub(5); // Center the view
                    self.focus_state = FocusState::Tree;
                    if let Some(parent_node) = self.all_nodes.get(i) {
                        self.status = format!("Navigated up to: {}", parent_node.text);
                    }
                    found_parent = true;
                }
                break;
            }
        }

        if !found_parent {
            "Parent not visible in tree".clone_into(&mut self.status);
        }
    }
}
