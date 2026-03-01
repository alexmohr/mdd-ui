/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use ratatui::layout::{Direction, Layout};

use crate::app::{App, COLUMN_SPACING};

impl App {
    pub(super) fn handle_tab_click(&mut self, column: u16, row: u16) {
        // Early exits for invalid states
        if self.layout.tab_titles.is_empty() {
            return;
        }

        let Some(tab_area) = self.layout.tab_area else {
            return;
        };

        // No borders on tab area - tabs render directly
        if column < tab_area.x || row < tab_area.y {
            return;
        }

        let relative_col = usize::from(column.saturating_sub(tab_area.x));
        let relative_row = usize::from(row.saturating_sub(tab_area.y));

        // Calculate available width for tabs (full width, no borders)
        let available_width = usize::from(tab_area.width);

        // Build tab strings with decorators to match rendering logic
        let tab_strings: Vec<String> = self
            .layout
            .tab_titles
            .iter()
            .map(|title| format!(" {title} "))
            .collect();

        // Simulate tab wrapping to determine which line each tab is on
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let mut current_line: Vec<usize> = Vec::new();
        let mut current_width: usize = 0;

        for (idx, tab_str) in tab_strings.iter().enumerate() {
            let tab_width: usize = tab_str.chars().count().saturating_add(1); // +1 for separator

            if current_width.saturating_add(tab_width) > available_width && !current_line.is_empty()
            {
                // Start a new line
                lines.push(current_line);
                current_line = Vec::new();
                current_width = 0;
            }

            current_line.push(idx);
            current_width = current_width.saturating_add(tab_width);
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Determine which line was clicked
        let Some(clicked_line_tabs) = lines.get(relative_row) else {
            return;
        };

        // Calculate tab positions on the clicked line
        let mut current_pos: usize = 0;
        for (i, &tab_idx) in clicked_line_tabs.iter().enumerate() {
            let Some(tab_str) = tab_strings.get(tab_idx) else {
                continue;
            };
            let separator_width: usize = usize::from(i != 0); // "│" separator before tab

            // Check if click falls within this tab
            if relative_col >= current_pos.saturating_add(separator_width)
                && relative_col
                    < current_pos
                        .saturating_add(separator_width)
                        .saturating_add(tab_str.chars().count())
            {
                self.set_selected_tab(tab_idx);
                return;
            }

            // Move past this tab and its separator
            current_pos = current_pos
                .saturating_add(separator_width)
                .saturating_add(tab_str.chars().count());
        }
    }

    pub(super) fn handle_table_click(&mut self, column: u16, row: u16) {
        const HEADER_HEIGHT: usize = 3;

        let Some(area) = self.layout.table_content_area else {
            return;
        };

        // Validate cursor position
        if self.tree.cursor >= self.tree.visible.len() {
            return;
        }

        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };

        // Get the correct section index (accounting for header section offset)
        let section_idx = self.get_section_index();

        // Validate section index
        if section_idx >= node.detail_sections.len() {
            return;
        }

        // Extract table content
        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };
        let Some(rows) = section.content.table_rows() else {
            return;
        };
        let use_row_selection = section.content.table_use_row_selection().unwrap_or(false);

        // Validate table has content
        if rows.is_empty() {
            return;
        }

        // Ensure section cursors and scrolls are properly sized
        self.detail.ensure_section_capacity(section_idx);

        // Calculate clicked row (skip header which is 3 lines tall)
        let relative_row = usize::from(row.saturating_sub(area.y));

        if relative_row < HEADER_HEIGHT {
            // Clicked on header - trigger column sort
            let relative_col = column.saturating_sub(area.x);
            if let Some(col_idx) = self.calculate_clicked_column(relative_col) {
                self.table.focused_column = col_idx;
                self.toggle_table_column_sort();
            }
            return;
        }

        let Some(&section_scroll) = self.detail.section_scrolls.get(section_idx) else {
            return;
        };
        let clicked_row_idx = relative_row
            .saturating_sub(HEADER_HEIGHT)
            .saturating_add(section_scroll);

        if clicked_row_idx >= rows.len() {
            return;
        }

        // Update the row cursor
        let Some(section_cursor) = self.detail.section_cursors.get_mut(section_idx) else {
            return;
        };
        *section_cursor = clicked_row_idx;

        // For tables with row selection mode, only select by row
        // For cell selection mode, also update the focused column
        if !use_row_selection {
            let relative_col = column.saturating_sub(area.x);
            if let Some(col_idx) = self.calculate_clicked_column(relative_col) {
                self.table.focused_column = col_idx;
            }
        }
    }

    pub(super) fn calculate_clicked_column(&self, relative_col: u16) -> Option<usize> {
        let area = self.layout.table_content_area?;

        if self.table.cached_ratatui_constraints.is_empty() {
            return None;
        }

        // Split the area using ratatui's layout - this matches what Table does internally
        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.table.cached_ratatui_constraints.clone())
            .spacing(COLUMN_SPACING)
            .split(area);

        // Find which column area contains the click
        for (idx, col_area) in column_areas.iter().enumerate() {
            let col_start = col_area.x.saturating_sub(area.x);
            let col_end = col_start.saturating_add(col_area.width);

            if relative_col >= col_start && relative_col < col_end {
                return Some(idx);
            }
        }

        // If not in any column (in spacing), find closest
        column_areas
            .iter()
            .enumerate()
            .map(|(idx, col_area)| {
                let col_center = col_area
                    .x
                    .saturating_sub(area.x)
                    .saturating_add(col_area.width.saturating_div(2));
                let distance = relative_col.abs_diff(col_center);
                (idx, distance)
            })
            .min_by_key(|(_, dist)| *dist)
            .map(|(idx, _)| idx)
    }

    pub(super) fn handle_tree_click(&mut self, row: u16) {
        // Calculate which tree item was clicked
        // Account for border (1 line) and title
        let inner_y = self.layout.tree_area.y.saturating_add(1);
        if row < inner_y
            || row
                >= self
                    .layout
                    .tree_area
                    .y
                    .saturating_add(self.layout.tree_area.height)
                    .saturating_sub(1)
        {
            return; // Clicked on border or help text area
        }

        let clicked_line = usize::from(row.saturating_sub(inner_y));
        let target_cursor = self.tree.scroll_offset.saturating_add(clicked_line);

        if target_cursor >= self.tree.visible.len() {
            return;
        }

        // If clicking on the same item, toggle expand/collapse
        if target_cursor == self.tree.cursor {
            self.toggle_expand();
        } else {
            self.push_to_history(); // Store old position before jumping
            self.tree.cursor = target_cursor;
            self.reset_detail_state();
        }
    }

    pub(super) fn handle_breadcrumb_click(&mut self, column: u16) {
        // Find which breadcrumb segment was clicked
        // Clone the data we need to avoid borrow checker issues
        let clicked_segment = self
            .layout
            .breadcrumb_segments
            .iter()
            .find(|seg| column >= seg.start_col && column < seg.end_col)
            .map(|seg| (seg.text.clone(), seg.node_idx));

        if let Some((text, node_idx)) = clicked_segment {
            // Navigate to this node
            self.navigate_to_node(node_idx);
            self.status = format!("Jumped to: {text}");
        }
    }
}
