/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState};
use crate::tree::NodeType;

impl App {
    /// Reset detail pane state when changing nodes
    pub(crate) fn reset_detail_state(&mut self) {
        self.table.jump_buffer.clear();
        self.table.jump_buffer_time = None;

        // Determine if current node is a diagcomm node
        let is_diagcomm = self
            .tree
            .visible
            .get(self.tree.cursor)
            .and_then(|&node_idx| self.tree.all_nodes.get(node_idx))
            .is_some_and(|node| {
                matches!(
                    node.node_type,
                    NodeType::Service | NodeType::ParentRefService | NodeType::Job
                )
            });

        // Restore tab selection based on section type
        if is_diagcomm {
            self.detail.selected_tab = self.detail.last_diagcomm_tab;
        } else {
            self.restore_tab_from_section_type();
        }

        // Reset focus and clear per-section state
        self.table.focused_column = 0;
        self.detail.section_scrolls.clear();
        self.detail.section_cursors.clear();
        self.table.column_widths.clear();
        self.table.column_widths_absolute.clear();
        self.table.horizontal_scroll.clear();
        self.table.sort_state.clear();
    }

    /// Set the tree cursor to `new_cursor`, resetting detail state only when
    /// the cursor actually changes.
    pub(super) fn set_tree_cursor(&mut self, new_cursor: usize) {
        if self.tree.cursor != new_cursor {
            self.tree.cursor = new_cursor;
            self.reset_detail_state();
        }
    }

    /// Try to restore tab selection based on section type
    fn restore_tab_from_section_type(&mut self) {
        let restored = self
            .tree
            .visible
            .get(self.tree.cursor)
            .and_then(|&node_idx| self.tree.all_nodes.get(node_idx))
            .and_then(|node| {
                let section_offset = usize::from(
                    !node.detail_sections.is_empty()
                        && node.detail_sections.first()?.render_as_header,
                );

                let target_type = self.detail.last_selected_section_type;
                let target_title = self.detail.last_selected_section_title.as_deref();

                // First pass: match both title and type (precise match for Custom sections)
                let by_title = target_title.and_then(|title| {
                    node.detail_sections
                        .iter()
                        .enumerate()
                        .filter(|(idx, _)| *idx >= section_offset)
                        .find(|(_, s)| target_type == Some(s.section_type) && s.title == title)
                        .map(|(idx, _)| idx)
                });

                // Second pass: match type only (fallback)
                let by_type = node
                    .detail_sections
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx >= section_offset)
                    .find(|(_, s)| target_type == Some(s.section_type))
                    .map(|(idx, _)| idx);

                by_title.or(by_type).map(|idx| {
                    self.detail.selected_tab = idx.saturating_sub(section_offset);
                })
            })
            .is_some();

        if !restored {
            self.detail.selected_tab = 0;
        }
    }

    pub(crate) fn move_up(&mut self) {
        if self.focus_state == FocusState::Detail {
            // Move cursor up in the detail pane (typically a table row)
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_sub(1);
            }
        } else {
            self.set_tree_cursor(self.tree.cursor.saturating_sub(1));
        }
    }

    pub(crate) fn move_down(&mut self) {
        if self.focus_state == FocusState::Detail {
            // Move cursor down in the detail pane (typically a table row)
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_add(1);
            }
        } else if let Some(new_cursor) = self.tree.cursor.checked_add(1)
            && new_cursor < self.tree.visible.len()
        {
            self.set_tree_cursor(new_cursor);
        }
    }

    pub(crate) fn page_up(&mut self, n: usize) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_sub(n);
            }
        } else {
            self.set_tree_cursor(self.tree.cursor.saturating_sub(n));
        }
    }

    pub(crate) fn page_down(&mut self, n: usize) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_add(n);
            }
        } else {
            let new_cursor = self
                .tree
                .cursor
                .saturating_add(n)
                .min(self.tree.visible.len().saturating_sub(1));
            self.set_tree_cursor(new_cursor);
        }
    }

    pub(crate) fn home(&mut self) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                *cursor = 0;
            }
        } else {
            self.set_tree_cursor(0);
        }
    }

    pub(crate) fn end(&mut self) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            let row_count = self
                .tree
                .visible
                .get(self.tree.cursor)
                .and_then(|&node_idx| self.tree.all_nodes.get(node_idx))
                .and_then(|node| node.detail_sections.get(section_idx))
                .and_then(|s| s.content.table_rows())
                .map_or(0, <[_]>::len);
            if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                *cursor = row_count.saturating_sub(1);
            }
        } else {
            self.set_tree_cursor(self.tree.visible.len().saturating_sub(1));
        }
    }

    // -------------------------------------------------------------------
    // Scroll helpers
    // -------------------------------------------------------------------

    pub(crate) fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.tree.cursor < self.tree.scroll_offset {
            self.tree.scroll_offset = self.tree.cursor;
        } else if let Some(max_scroll) = self.tree.scroll_offset.checked_add(viewport_height)
            && self.tree.cursor >= max_scroll
        {
            self.tree.scroll_offset = self
                .tree
                .cursor
                .saturating_sub(viewport_height)
                .saturating_add(1);
        }
    }
}
