/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};

use super::{App, SearchScope};

impl App {
    pub(crate) fn update_search(&mut self) {
        if self.search.is_empty() {
            // If search is empty, don't add to stack
            self.status.clear();
        } else {
            // Add current search with its scope to stack
            self.search_stack
                .push((self.search.clone(), self.search_scope.clone()));
            self.search.clear(); // Clear for next search

            let depth = self.search_stack.len();
            let stack_display: Vec<String> = self
                .search_stack
                .iter()
                .map(|(term, scope)| format!("{}{}", term, scope.abbrev()))
                .collect();
            self.status = format!("Search depth: {} [{}]", depth, stack_display.join(" → "));
        }

        // Rebuild visible list with the search stack
        self.rebuild_visible();
        self.cursor = 0;
        self.reset_detail_state();
        self.populate_search_matches();
        self.search_match_cursor = 0;
    }

    /// Populate `search_matches` with visible indices of nodes that directly match
    /// the most recent search query, enabling n/N navigation between matches.
    fn populate_search_matches(&mut self) {
        self.search_matches.clear();

        let Some((query, scope)) = self.search_stack.last().cloned() else {
            return;
        };
        let q = query.to_lowercase();

        self.search_matches = self
            .visible
            .iter()
            .enumerate()
            .filter(|&(_, &node_idx)| {
                self.all_nodes.get(node_idx).is_some_and(|node| {
                    self.node_matches_scope_and_query(node, node_idx, &scope, &q)
                })
            })
            .map(|(visible_idx, _)| visible_idx)
            .collect();
    }

    pub(crate) fn clear_search_stack(&mut self) {
        self.search_stack.clear();
        self.search.clear();
        "Search cleared".clone_into(&mut self.status);
        self.rebuild_visible();
        self.cursor = 0;
        self.reset_detail_state();
    }

    pub(crate) fn cycle_search_scope(&mut self) {
        self.search_scope = match self.search_scope {
            SearchScope::All => SearchScope::Variants,
            SearchScope::Variants => SearchScope::FunctionalGroups,
            SearchScope::FunctionalGroups => SearchScope::EcuSharedData,
            SearchScope::EcuSharedData => SearchScope::Services,
            SearchScope::Services => SearchScope::DiagComms,
            SearchScope::DiagComms => SearchScope::Requests,
            SearchScope::Requests => SearchScope::Responses,
            // Subtree is contextual, cycling from it resets to All
            SearchScope::Responses | SearchScope::Subtree { .. } => SearchScope::All,
        };

        self.status = format!("Search scope: {}", self.search_scope);
    }

    /// Set search scope to the subtree rooted at the current cursor position
    pub(crate) fn set_subtree_scope(&mut self) {
        let Some(&start_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(root_node) = self.all_nodes.get(start_idx) else {
            return;
        };
        let root_depth = root_node.depth;
        let root_name = root_node.text.clone();

        // Walk forward to find the last descendant
        let end_idx =
            if let Some(children_slice) = self.all_nodes.get(start_idx.saturating_add(1)..) {
                children_slice
                    .iter()
                    .position(|n| n.depth <= root_depth)
                    .map_or(self.all_nodes.len().saturating_sub(1), |offset| {
                        start_idx.saturating_add(offset)
                    })
            } else {
                self.all_nodes.len().saturating_sub(1)
            };

        self.search_scope = SearchScope::Subtree {
            start_idx,
            end_idx,
            root_name: root_name.clone(),
        };
        self.status = format!("Search scope: subtree '{root_name}'");
    }

    pub(crate) fn toggle_mouse_mode(&mut self) {
        self.mouse_enabled = !self.mouse_enabled;

        // Actually enable/disable mouse capture in the terminal
        let result = if self.mouse_enabled {
            execute!(std::io::stdout(), EnableMouseCapture)
        } else {
            execute!(std::io::stdout(), DisableMouseCapture)
        };

        if result.is_ok() {
            self.status = format!(
                "Mouse: {}",
                if self.mouse_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        } else {
            "Failed to toggle mouse mode".clone_into(&mut self.status);
        }
    }

    pub(crate) fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_cursor = self
            .search_match_cursor
            .saturating_add(1)
            .rem_euclid(self.search_matches.len());
        if let Some(&match_idx) = self.search_matches.get(self.search_match_cursor) {
            self.cursor = match_idx;
            self.reset_detail_state();
            self.status = format!(
                "Match {}/{}",
                self.search_match_cursor.saturating_add(1),
                self.search_matches.len()
            );
        }
    }

    pub(crate) fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_cursor = if self.search_match_cursor == 0 {
            self.search_matches.len().saturating_sub(1)
        } else {
            self.search_match_cursor.saturating_sub(1)
        };
        if let Some(&match_idx) = self.search_matches.get(self.search_match_cursor) {
            self.cursor = match_idx;
            self.reset_detail_state();
            self.status = format!(
                "Match {}/{}",
                self.search_match_cursor.saturating_add(1),
                self.search_matches.len()
            );
        }
    }
}
