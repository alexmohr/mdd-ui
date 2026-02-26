/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState};
use crate::tree::NodeType;

impl App {
    /// Reset detail pane state when changing nodes
    pub(crate) fn reset_detail_state(&mut self) {
        self.jump_buffer.clear();
        self.jump_buffer_time = None;

        // Determine if current node is a diagcomm node
        let is_diagcomm = self
            .visible
            .get(self.cursor)
            .and_then(|&node_idx| self.all_nodes.get(node_idx))
            .is_some_and(|node| {
                matches!(
                    node.node_type,
                    NodeType::Service | NodeType::ParentRefService | NodeType::Job
                )
            });

        // Restore tab selection based on section type
        if is_diagcomm {
            self.selected_tab = self.last_diagcomm_tab;
        } else {
            self.restore_tab_from_section_type();
        }

        // Reset focus and clear per-section state
        self.focused_section = 0;
        self.focused_column = 0;
        self.section_scrolls.clear();
        self.section_cursors.clear();
        self.column_widths.clear();
        self.column_widths_absolute.clear();
        self.horizontal_scroll.clear();
        self.table_sort_state.clear();
    }

    /// Try to restore tab selection based on section type
    fn restore_tab_from_section_type(&mut self) {
        let restored = self
            .visible
            .get(self.cursor)
            .and_then(|&node_idx| self.all_nodes.get(node_idx))
            .and_then(|node| {
                let section_offset = usize::from(
                    !node.detail_sections.is_empty()
                        && node.detail_sections.first()?.render_as_header,
                );

                let target_type = self.last_selected_section_type;
                let target_title = self.last_selected_section_title.as_deref();
                let mut sections = node
                    .detail_sections
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx >= section_offset);

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
                let by_type = sections
                    .find(|(_, s)| target_type == Some(s.section_type))
                    .map(|(idx, _)| idx);

                by_title.or(by_type).map(|idx| {
                    self.selected_tab = idx.saturating_sub(section_offset);
                })
            })
            .is_some();

        if !restored {
            self.selected_tab = 0;
        }
    }

    pub(crate) fn move_up(&mut self) {
        if self.focus_state == FocusState::Detail {
            // Move cursor up in the detail pane (typically a table row)
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_sub(1);
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self.cursor.saturating_sub(1);

            // Reset detail state when moving to a different node
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn move_down(&mut self) {
        if self.focus_state == FocusState::Detail {
            // Move cursor down in the detail pane (typically a table row)
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_add(1);
            }
        } else if let Some(new_cursor) = self.cursor.checked_add(1)
            && new_cursor < self.visible.len()
        {
            let old_cursor = self.cursor;
            self.cursor = new_cursor;

            // Reset detail state when moving to a different node
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn page_up(&mut self, n: usize) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_sub(n);
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self.cursor.saturating_sub(n);
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn page_down(&mut self, n: usize) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                *cursor = cursor.saturating_add(n);
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self
                .cursor
                .saturating_add(n)
                .min(self.visible.len().saturating_sub(1));
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn home(&mut self) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                *cursor = 0;
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = 0;
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn end(&mut self) {
        if self.focus_state == FocusState::Detail {
            let section_idx = self.get_section_index();
            if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                *cursor = usize::MAX; // clamped during render
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self.visible.len().saturating_sub(1);
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    // -------------------------------------------------------------------
    // Scroll helpers
    // -------------------------------------------------------------------

    pub(crate) fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if let Some(max_scroll) = self.scroll_offset.checked_add(viewport_height)
            && self.cursor >= max_scroll
        {
            self.scroll_offset = self
                .cursor
                .saturating_sub(viewport_height)
                .saturating_add(1);
        }
    }
}
