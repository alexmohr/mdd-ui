/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEventKind};

use super::{App, DragState, FocusState, input::Action};
use crate::tree::{DetailContent, DetailRowType, DetailSectionType, NodeType};

impl App {
    // -------------------------------------------------------------------
    // Mouse handling
    // -------------------------------------------------------------------

    pub(super) fn handle_mouse_event(
        &mut self,
        kind: MouseEventKind,
        column: u16,
        row: u16,
    ) -> Action {
        // If popup is open, only close on click
        if self.detail_popup.is_some() {
            if matches!(kind, MouseEventKind::Down(_)) {
                self.detail_popup = None;
            }
            return Action::Continue;
        }

        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if clicking on a scrollbar to start drag
                if self.is_in_tree_scrollbar(column, row) {
                    self.drag_state = DragState::TreeScrollbar;
                    self.handle_scrollbar_drag(row);
                    return Action::Continue;
                } else if self.is_in_detail_scrollbar(column, row) {
                    self.drag_state = DragState::DetailScrollbar;
                    self.handle_scrollbar_drag(row);
                    return Action::Continue;
                } else if self.is_in_detail_hscrollbar(column, row) {
                    self.drag_state = DragState::DetailHScrollbar;
                    self.handle_hscrollbar_drag(column);
                    return Action::Continue;
                }

                // Check if clicking near the divider to start drag
                if self.is_near_divider(column) {
                    self.drag_state = DragState::Divider;
                    return Action::Continue;
                }

                // Check if clicking near a column border in the table header
                if let Some(col_idx) = self.find_column_border(column, row) {
                    self.drag_state = DragState::ColumnBorder(col_idx);
                    return Action::Continue;
                }

                // Check for double-click (within 500ms and same position)
                let is_double_click = if let Some(last_time) = self.last_click_time {
                    let elapsed = last_time.elapsed();
                    elapsed < Duration::from_millis(500) && self.last_click_pos == (column, row)
                } else {
                    false
                };

                if is_double_click {
                    // First handle the click to update cursor position
                    self.handle_click(column, row);
                    // Then handle the double-click action
                    self.handle_double_click(column, row);
                    // Reset click tracking to avoid triple-click being detected
                    // as another double-click
                    self.last_click_time = None;
                } else {
                    self.handle_click(column, row);
                    // Track this click for double-click detection
                    self.last_click_time = Some(Instant::now());
                    self.last_click_pos = (column, row);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // Stop dragging when mouse button is released
                self.drag_state = DragState::None;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Handle drag to scroll via scrollbar
                if matches!(
                    self.drag_state,
                    DragState::TreeScrollbar | DragState::DetailScrollbar
                ) {
                    self.handle_scrollbar_drag(row);
                } else if self.drag_state == DragState::DetailHScrollbar {
                    self.handle_hscrollbar_drag(column);
                }
                // Handle drag to resize tree pane
                else if self.drag_state == DragState::Divider {
                    self.handle_divider_drag(column);
                }
                // Handle drag to resize table columns
                else if let DragState::ColumnBorder(col_idx) = self.drag_state {
                    self.handle_column_border_drag(col_idx, column);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.is_in_tree_area(column, row) {
                    self.move_down();
                } else if self.is_in_detail_area(column, row) {
                    self.focus_state = FocusState::Detail;
                    self.move_down();
                }
            }
            MouseEventKind::ScrollUp => {
                if self.is_in_tree_area(column, row) {
                    self.move_up();
                } else if self.is_in_detail_area(column, row) {
                    self.focus_state = FocusState::Detail;
                    self.move_up();
                }
            }
            MouseEventKind::ScrollLeft => {
                if self.is_in_detail_area(column, row) {
                    self.focus_state = FocusState::Detail;
                    self.scroll_horizontal(-5);
                }
            }
            MouseEventKind::ScrollRight => {
                if self.is_in_detail_area(column, row) {
                    self.focus_state = FocusState::Detail;
                    self.scroll_horizontal(5);
                }
            }
            // Mouse back button not supported by crossterm 0.29 (only Left/Right/Middle)
            _ => {}
        }
        Action::Continue
    }

    fn handle_click(&mut self, column: u16, row: u16) {
        // Check if click is in breadcrumb area first
        if self.is_in_breadcrumb_area(column, row) {
            self.handle_breadcrumb_click(column);
        } else if self.is_in_tree_area(column, row) {
            // Click in tree area
            self.focus_state = FocusState::Tree;
            self.handle_tree_click(row);
        } else if self.is_in_detail_area(column, row) {
            self.focus_state = FocusState::Detail;
            // Check if click is on tab area
            if self.is_in_tab_area(column, row) {
                self.handle_tab_click(column, row);
            } else if self.is_in_table_content_area(column, row) {
                self.handle_table_click(column, row);
            }
        }
    }

    fn handle_double_click(&mut self, column: u16, row: u16) {
        const HEADER_HEIGHT: usize = 3;
        // Double-click in table content area should trigger navigation or DOP popup
        if self.is_in_table_content_area(column, row) {
            self.focus_state = FocusState::Detail;

            // Check if double-click is on table header and ignore it
            if let Some(area) = self.table_content_area {
                let relative_row = (row.saturating_sub(area.y)) as usize;
                if relative_row < HEADER_HEIGHT {
                    // Ignore double-clicks on header
                    return;
                }
            }

            // Check what type of node we're on
            if self.cursor < self.visible.len() {
                let Some(&node_idx) = self.visible.get(self.cursor) else {
                    return;
                };
                let Some(node) = self.all_nodes.get(node_idx) else {
                    return;
                };

                // Check if this is a service list header (generic check)
                let is_service_list = Self::is_service_list_section(node);

                // Check if this is the Variants overview section
                let is_variants_section =
                    matches!(node.section_type, Some(crate::tree::SectionType::Variants))
                        && node
                            .detail_sections
                            .first()
                            .is_some_and(|s| s.content.has_table());

                // Check if this is any service-related node type (generic check)
                let is_service_node = matches!(
                    node.node_type,
                    NodeType::Service
                        | NodeType::ParentRefService
                        | NodeType::Request
                        | NodeType::PosResponse
                        | NodeType::NegResponse
                );

                // Check if this is a functional class node
                let is_functional_class = matches!(node.node_type, NodeType::FunctionalClass);

                // Check if this is a DOP node (DIAG-DATA-DICTIONARY-SPEC,
                // DOP category, or individual DOP with children)
                let is_dop_node = matches!(node.node_type, NodeType::DOP)
                    || self.is_dop_category_node(node_idx)
                    || self.is_individual_dop_node(node_idx);

                if is_variants_section {
                    // Navigate to selected variant from the Variants overview table
                    self.try_navigate_to_variant();
                } else if is_service_list {
                    // Navigate to selected service from service list table
                    self.try_navigate_to_service();
                } else if is_functional_class {
                    // For functional class nodes, check which column is focused
                    // Column 0 (ShortName): navigate to service/job
                    // Column 5 (Layer): navigate to variant/layer
                    if self.focused_column == 0 {
                        self.try_navigate_to_service_from_functional_class();
                    } else if self.focused_column == 5 {
                        self.try_navigate_to_layer_from_functional_class();
                    }
                } else if is_dop_node {
                    // Navigate to child DOP element instead of showing popup
                    self.try_navigate_to_dop_child();
                } else if matches!(node.node_type, NodeType::ParentRefs) {
                    // Navigate to parent ref target from overview
                    self.try_navigate_to_parent_ref();
                } else if is_service_node {
                    // Check if we're on the "Inherited From" row in Overview
                    let mut should_navigate_to_parent = false;
                    let mut should_navigate_from_param_table = false;

                    // Get the actual section index accounting for header section offset
                    let section_idx = self.get_section_index();

                    if section_idx < node.detail_sections.len() {
                        let Some(section) = node.detail_sections.get(section_idx) else {
                            return;
                        };

                        // Check if this is a request/response parameter table
                        if matches!(
                            section.section_type,
                            DetailSectionType::Requests
                                | DetailSectionType::PosResponses
                                | DetailSectionType::NegResponses
                        ) {
                            should_navigate_from_param_table = true;
                        } else if section.section_type == DetailSectionType::Overview
                            && let Some(rows) = section.content.table_rows()
                        {
                            let row_cursor =
                                self.section_cursors.get(section_idx).copied().unwrap_or(0);

                            // Apply sorting if active for this section
                            let sorted_rows = self.apply_table_sort(rows, section_idx);

                            if let Some(selected_row) = sorted_rows.get(row_cursor)
                                && selected_row.row_type == DetailRowType::InheritedFrom
                            {
                                should_navigate_to_parent = true;
                            }
                        }
                    }

                    if should_navigate_from_param_table {
                        self.try_navigate_from_param_table();
                    } else if should_navigate_to_parent {
                        self.try_navigate_to_inherited_parent();
                    } else {
                        self.try_navigate_from_detail_row();
                    }
                } else {
                    // Check if we're in a Parent References section
                    let section_idx = self.get_section_index();
                    if let Some(section) = node.detail_sections.get(section_idx) {
                        if section.section_type == DetailSectionType::RelatedRefs
                            && section.title == "Parent References"
                        {
                            self.try_navigate_to_parent_ref();
                            return;
                        } else if section.title.starts_with("Not Inherited") {
                            // Navigate to the selected not-inherited element
                            self.try_navigate_to_not_inherited_element();
                            return;
                        }
                    }

                    self.try_navigate_from_detail_row();
                }
            }
        }
    }

    fn is_in_tree_area(&self, column: u16, row: u16) -> bool {
        column >= self.tree_area.x
            && column < self.tree_area.x.saturating_add(self.tree_area.width)
            && row >= self.tree_area.y
            && row < self.tree_area.y.saturating_add(self.tree_area.height)
    }

    fn is_in_detail_area(&self, column: u16, row: u16) -> bool {
        column >= self.detail_area.x
            && column < self.detail_area.x.saturating_add(self.detail_area.width)
            && row >= self.detail_area.y
            && row < self.detail_area.y.saturating_add(self.detail_area.height)
    }

    fn is_in_tab_area(&self, column: u16, row: u16) -> bool {
        if let Some(tab_area) = self.tab_area {
            column >= tab_area.x
                && column < tab_area.x.saturating_add(tab_area.width)
                && row >= tab_area.y
                && row < tab_area.y.saturating_add(tab_area.height)
        } else {
            false
        }
    }

    fn is_in_table_content_area(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.table_content_area {
            column >= area.x
                && column < area.x.saturating_add(area.width)
                && row >= area.y
                && row < area.y.saturating_add(area.height)
        } else {
            false
        }
    }

    /// Check if the mouse is near the divider between tree and detail panes
    /// The divider is considered to be within 1-2 columns of the tree pane's right edge
    fn is_near_divider(&self, column: u16) -> bool {
        let divider_col = self.tree_area.x.saturating_add(self.tree_area.width);
        // Allow clicking on the last column of tree or first column of detail
        column >= divider_col.saturating_sub(1) && column <= divider_col.saturating_add(1)
    }

    /// Check if the mouse is near a column border in the table header area.
    /// Returns the index of the column to the left of the border.
    fn find_column_border(&self, column: u16, row: u16) -> Option<usize> {
        use ratatui::layout::{Direction, Layout};

        let area = self.table_content_area?;

        // Only detect on the header rows (first 3 rows of the table)
        if row < area.y || row >= area.y.saturating_add(3) {
            return None;
        }

        if self.cached_ratatui_constraints.len() < 2 {
            return None;
        }

        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.cached_ratatui_constraints.clone())
            .spacing(3)
            .split(area);

        // Check each border between adjacent columns (in the spacing gap)
        // A border exists at the right edge of each column (except the last)
        column_areas
            .iter()
            .enumerate()
            .take(column_areas.len().saturating_sub(1))
            .find(|(_, col_area)| {
                let border = col_area.x.saturating_add(col_area.width);
                // ±2 pixels of the border
                column >= border.saturating_sub(1) && column <= border.saturating_add(3)
            })
            .map(|(idx, _)| idx)
    }

    /// Handle dragging a column border to resize table columns
    fn handle_column_border_drag(&mut self, col_idx: usize, column: u16) {
        use ratatui::layout::{Direction, Layout};

        let Some(area) = self.table_content_area else {
            return;
        };

        if self.cached_ratatui_constraints.len() < 2 {
            return;
        }

        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.cached_ratatui_constraints.clone())
            .spacing(3)
            .split(area);

        let Some(left_area) = column_areas.get(col_idx) else {
            return;
        };

        let section_idx = self.get_section_index();

        // Ensure we're in absolute mode
        let is_absolute = self
            .column_widths_absolute
            .get(section_idx)
            .copied()
            .unwrap_or(false);
        if !is_absolute {
            self.convert_to_absolute_widths(section_idx);
        }

        // Calculate new width for the dragged column based on mouse position
        let new_width = column.saturating_sub(left_area.x).max(3);

        let current_width = left_area.width;
        if new_width == current_width {
            return;
        }

        if let Some(widths) = self.column_widths.get_mut(section_idx)
            && let Some(lw) = widths.get_mut(col_idx)
        {
            *lw = new_width;
        }

        self.focused_column = col_idx;
        self.save_column_widths_to_persistent(section_idx);
    }

    /// Handle dragging the divider to resize tree pane
    fn handle_divider_drag(&mut self, column: u16) {
        // Get the total width of main area (tree + detail)
        let total_width = self.tree_area.width.saturating_add(self.detail_area.width);
        if total_width == 0 {
            return;
        }

        // Calculate the new tree width based on mouse position
        // The column is relative to the start of tree_area
        let new_tree_width = column.saturating_sub(self.tree_area.x);

        // Calculate percentage (clamped between 20% and 80%)
        let percentage_f32 = (f32::from(new_tree_width) / f32::from(total_width)) * 100.0;
        let clamped = percentage_f32.clamp(20.0, 80.0);
        // clamped percentage (20..=80) always fits in u16
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let percentage = clamped.round().clamp(0.0, 100.0) as u16;
        self.tree_width_percentage = percentage;
    }

    /// Check if the mouse is in the tree scrollbar area
    fn is_in_tree_scrollbar(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.tree_scrollbar_area {
            column >= area.x
                && column < area.x.saturating_add(area.width)
                && row >= area.y
                && row < area.y.saturating_add(area.height)
        } else {
            false
        }
    }

    /// Check if the mouse is in the detail scrollbar area
    fn is_in_detail_scrollbar(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.detail_scrollbar_area {
            column >= area.x
                && column < area.x.saturating_add(area.width)
                && row >= area.y
                && row < area.y.saturating_add(area.height)
        } else {
            false
        }
    }

    /// Check if the mouse is in the horizontal scrollbar area
    fn is_in_detail_hscrollbar(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.detail_hscrollbar_area {
            column >= area.x
                && column < area.x.saturating_add(area.width)
                && row >= area.y
                && row < area.y.saturating_add(area.height)
        } else {
            false
        }
    }

    /// Handle dragging on the horizontal scrollbar
    fn handle_hscrollbar_drag(&mut self, column: u16) {
        let Some(area) = self.detail_hscrollbar_area else {
            return;
        };
        let total_width = self.cached_total_table_width;
        let viewport_width = area.width;
        if total_width <= viewport_width {
            return;
        }
        let max_scroll = total_width.saturating_sub(viewport_width);
        let relative_x = column.saturating_sub(area.x);
        let new_scroll = if area.width > 1 {
            let numerator = u32::from(relative_x).saturating_mul(u32::from(max_scroll));
            let divisor = u32::from(area.width.saturating_sub(1));
            #[allow(clippy::cast_possible_truncation)]
            let result = numerator.checked_div(divisor).unwrap_or(0) as u16;
            result.min(max_scroll)
        } else {
            0
        };
        let section_idx = self.get_section_index();
        while self.horizontal_scroll.len() <= section_idx {
            self.horizontal_scroll.push(0);
        }
        if let Some(hs) = self.horizontal_scroll.get_mut(section_idx) {
            *hs = new_scroll;
        }
    }

    /// Handle dragging on a scrollbar to scroll
    fn handle_scrollbar_drag(&mut self, row: u16) {
        if !matches!(
            self.drag_state,
            DragState::TreeScrollbar | DragState::DetailScrollbar
        ) {
            return;
        }

        if self.drag_state == DragState::TreeScrollbar {
            // Dragging tree scrollbar
            if let Some(area) = self.tree_scrollbar_area {
                let visible_count = self.visible.len();
                let viewport_height = area.height as usize;

                if visible_count <= viewport_height {
                    return;
                }

                // Map mouse position to cursor position in the full list
                let relative_y = row.saturating_sub(area.y) as usize;
                let max_cursor = visible_count.saturating_sub(1);

                let new_cursor = if area.height > 1 {
                    let divisor = usize::from(area.height.saturating_sub(1));
                    relative_y
                        .saturating_mul(max_cursor)
                        .checked_div(divisor)
                        .unwrap_or(0)
                } else {
                    0
                };

                self.cursor = new_cursor.min(max_cursor);

                // Adjust scroll_offset to keep cursor centered in view
                let half_viewport = viewport_height.saturating_div(2);
                self.scroll_offset = self.cursor.saturating_sub(half_viewport);
                let max_scroll = visible_count.saturating_sub(viewport_height);
                self.scroll_offset = self.scroll_offset.min(max_scroll);
            }
        } else {
            // Dragging detail scrollbar
            if let Some(area) = self.detail_scrollbar_area {
                if self.focused_section >= self.section_scrolls.len() {
                    return;
                }

                // Get the current section's details
                let Some(&node_idx) = self.visible.get(self.cursor) else {
                    return;
                };

                let Some(node) = self.all_nodes.get(node_idx) else {
                    return;
                };
                let sections = &node.detail_sections;
                if self.focused_section >= sections.len() {
                    return;
                }

                let Some(section) = sections.get(self.focused_section) else {
                    return;
                };
                let row_count = match &section.content {
                    DetailContent::Table { rows, .. } => rows.len(),
                    DetailContent::PlainText(lines) => lines.len(),
                    DetailContent::Composite(subs) => subs
                        .iter()
                        .find_map(|s| s.content.table_rows())
                        .map_or(0, Vec::len),
                };
                let viewport_height = area.height as usize;

                if row_count <= viewport_height {
                    return;
                }

                // Map mouse position to cursor position in the section
                let relative_y = row.saturating_sub(area.y) as usize;
                let max_cursor = row_count.saturating_sub(1);

                let new_cursor = if area.height > 1 {
                    let divisor = usize::from(area.height.saturating_sub(1));
                    relative_y
                        .saturating_mul(max_cursor)
                        .checked_div(divisor)
                        .unwrap_or(0)
                } else {
                    0
                };

                let Some(section_cursor) = self.section_cursors.get_mut(self.focused_section)
                else {
                    return;
                };
                *section_cursor = new_cursor.min(max_cursor);

                // Adjust scroll to keep cursor centered
                let half_viewport = viewport_height.saturating_div(2);
                let Some(&cursor_val) = self.section_cursors.get(self.focused_section) else {
                    return;
                };
                let new_scroll = cursor_val.saturating_sub(half_viewport);
                let max_scroll = row_count.saturating_sub(viewport_height);
                let Some(section_scroll) = self.section_scrolls.get_mut(self.focused_section)
                else {
                    return;
                };
                *section_scroll = new_scroll.min(max_scroll);
            }
        }
    }

    fn handle_tab_click(&mut self, column: u16, row: u16) {
        // Early exits for invalid states
        if self.tab_titles.is_empty() {
            return;
        }

        let Some(tab_area) = self.tab_area else {
            return;
        };

        // No borders on tab area - tabs render directly
        if column < tab_area.x || row < tab_area.y {
            return;
        }

        let relative_col = column.saturating_sub(tab_area.x) as usize;
        let relative_row = row.saturating_sub(tab_area.y) as usize;

        // Calculate available width for tabs (full width, no borders)
        let available_width = tab_area.width as usize;

        // Build tab strings with decorators to match rendering logic
        let tab_strings: Vec<String> = self
            .tab_titles
            .iter()
            .map(|title| format!(" {title} "))
            .collect();

        // Simulate tab wrapping to determine which line each tab is on
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let mut current_line: Vec<usize> = Vec::new();
        let mut current_width: usize = 0;

        for (idx, tab_str) in tab_strings.iter().enumerate() {
            let tab_width: usize = tab_str.len().saturating_add(1); // +1 for separator

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
                        .saturating_add(tab_str.len())
            {
                self.set_selected_tab(tab_idx);
                return;
            }

            // Move past this tab and its separator
            current_pos = current_pos
                .saturating_add(separator_width)
                .saturating_add(tab_str.len());
        }
    }

    fn handle_table_click(&mut self, column: u16, row: u16) {
        const HEADER_HEIGHT: usize = 3;

        let Some(area) = self.table_content_area else {
            return;
        };

        // Validate cursor position
        if self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
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
        while self.section_scrolls.len() <= section_idx {
            self.section_scrolls.push(0);
        }
        while self.section_cursors.len() <= section_idx {
            self.section_cursors.push(0);
        }

        // Calculate clicked row (skip header which is 3 lines tall)
        let relative_row = (row.saturating_sub(area.y)) as usize;

        if relative_row < HEADER_HEIGHT {
            // Clicked on header - trigger column sort
            let relative_col = column.saturating_sub(area.x);
            if let Some(col_idx) = self.calculate_clicked_column(relative_col) {
                self.focused_column = col_idx;
                self.toggle_table_column_sort();
            }
            return;
        }

        let Some(&section_scroll) = self.section_scrolls.get(section_idx) else {
            return;
        };
        let clicked_row_idx = relative_row
            .saturating_sub(HEADER_HEIGHT)
            .saturating_add(section_scroll);

        if clicked_row_idx >= rows.len() {
            return;
        }

        // Update the row cursor
        let Some(section_cursor) = self.section_cursors.get_mut(section_idx) else {
            return;
        };
        *section_cursor = clicked_row_idx;

        // For tables with row selection mode, only select by row
        // For cell selection mode, also update the focused column
        if !use_row_selection {
            let relative_col = column.saturating_sub(area.x);
            if let Some(col_idx) = self.calculate_clicked_column(relative_col) {
                self.focused_column = col_idx;
            }
        }
    }

    fn calculate_clicked_column(&self, relative_col: u16) -> Option<usize> {
        use ratatui::layout::{Direction, Layout};

        let area = self.table_content_area?;

        if self.cached_ratatui_constraints.is_empty() {
            return None;
        }

        // Use ratatui's Layout with the exact constraints used in rendering

        // Split the area using ratatui's layout - this matches what Table does internally
        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.cached_ratatui_constraints.clone())
            .spacing(3) // Match the column_spacing(3) from Table
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

    fn handle_tree_click(&mut self, row: u16) {
        // Calculate which tree item was clicked
        // Account for border (1 line) and title
        let inner_y = self.tree_area.y.saturating_add(1);
        if row < inner_y
            || row
                >= self
                    .tree_area
                    .y
                    .saturating_add(self.tree_area.height)
                    .saturating_sub(1)
        {
            return; // Clicked on border or help text area
        }

        let clicked_line = row.saturating_sub(inner_y) as usize;
        let target_cursor = self.scroll_offset.saturating_add(clicked_line);

        if target_cursor >= self.visible.len() {
            return;
        }

        // If clicking on the same item, toggle expand/collapse
        if target_cursor == self.cursor {
            self.toggle_expand();
        } else {
            self.push_to_history(); // Store old position before jumping
            self.cursor = target_cursor;
            self.reset_detail_state();
        }
    }

    fn is_in_breadcrumb_area(&self, column: u16, row: u16) -> bool {
        column >= self.breadcrumb_area.x
            && column
                < self
                    .breadcrumb_area
                    .x
                    .saturating_add(self.breadcrumb_area.width)
            && row >= self.breadcrumb_area.y
            && row
                < self
                    .breadcrumb_area
                    .y
                    .saturating_add(self.breadcrumb_area.height)
    }

    fn handle_breadcrumb_click(&mut self, column: u16) {
        // Find which breadcrumb segment was clicked
        // Clone the data we need to avoid borrow checker issues
        let clicked_segment = self
            .breadcrumb_segments
            .iter()
            .find(|(_, _, start_col, end_col)| column >= *start_col && column < *end_col)
            .map(|(text, node_idx, _, _)| (text.clone(), *node_idx));

        if let Some((text, node_idx)) = clicked_segment {
            // Navigate to this node
            self.navigate_to_node(node_idx);
            self.status = format!("Jumped to: {text}");
        }
    }

    /// Navigate to a specific node by its index in `all_nodes`
    pub(crate) fn navigate_to_node(&mut self, target_node_idx: usize) {
        // Find the position of this node in visible
        if let Some(_visible_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) {
            // Ensure the target node is expanded if needed
            self.ensure_node_visible(target_node_idx);

            // Find the updated position after expanding
            if let Some(new_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) {
                self.push_to_history(); // Store old position before jumping
                self.focus_state = FocusState::Tree;
                self.cursor = new_pos;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5); // Center the view
            }
        } else {
            // Node is not currently visible (might be collapsed), try to make it visible
            self.ensure_node_visible(target_node_idx);

            // Try to find it again after expanding parents
            if let Some(visible_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) {
                self.push_to_history(); // Store old position before jumping
                self.focus_state = FocusState::Tree;
                self.cursor = visible_pos;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5); // Center the view
            }
        }
    }

    /// Ensure a node is visible by expanding only its direct ancestors
    pub(crate) fn ensure_node_visible(&mut self, target_node_idx: usize) {
        let Some(target_node) = self.all_nodes.get(target_node_idx) else {
            return;
        };

        let mut needed_depth = target_node.depth;

        // Walk backwards, expanding only the direct ancestor at each level
        for i in (0..target_node_idx).rev() {
            let Some(node) = self.all_nodes.get_mut(i) else {
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
