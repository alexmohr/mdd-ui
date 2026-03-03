/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod areas;
mod clicks;
mod drag;

use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEventKind};

use super::{App, DOUBLE_CLICK_MS, DragState, FocusState, input::Action};

impl App {
    pub(super) fn handle_mouse_event(
        &mut self,
        kind: MouseEventKind,
        column: u16,
        row: u16,
    ) -> Action {
        // If popup is open, only close on click
        if self.detail.popup.is_some() {
            if matches!(kind, MouseEventKind::Down(_)) {
                self.detail.popup = None;
            }
            return Action::Continue;
        }

        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if clicking on a scrollbar to start drag
                if self.is_in_tree_scrollbar(column, row) {
                    self.mouse.drag_state = DragState::TreeScrollbar;
                    self.handle_scrollbar_drag(row);
                    return Action::Continue;
                } else if self.is_in_detail_scrollbar(column, row) {
                    self.mouse.drag_state = DragState::DetailScrollbar;
                    self.handle_scrollbar_drag(row);
                    return Action::Continue;
                } else if self.is_in_detail_hscrollbar(column, row) {
                    self.mouse.drag_state = DragState::DetailHScrollbar;
                    self.handle_hscrollbar_drag(column);
                    return Action::Continue;
                }

                // Check if clicking near the divider to start drag
                if self.is_near_divider(column) {
                    self.mouse.drag_state = DragState::Divider;
                    return Action::Continue;
                }

                // Check if clicking near a column border in the table header
                if let Some(col_idx) = self.find_column_border(column, row) {
                    self.mouse.drag_state = DragState::ColumnBorder(col_idx);
                    return Action::Continue;
                }

                // Check for double-click (within threshold and same position)
                let is_double_click = if let Some(last_time) = self.mouse.last_click_time {
                    let elapsed = last_time.elapsed();
                    elapsed < Duration::from_millis(DOUBLE_CLICK_MS)
                        && self.mouse.last_click_pos == (column, row)
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
                    self.mouse.last_click_time = None;
                } else {
                    self.handle_click(column, row);
                    // Track this click for double-click detection
                    self.mouse.last_click_time = Some(Instant::now());
                    self.mouse.last_click_pos = (column, row);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse.drag_state = DragState::None;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Handle drag to scroll via scrollbar
                if matches!(
                    self.mouse.drag_state,
                    DragState::TreeScrollbar | DragState::DetailScrollbar
                ) {
                    self.handle_scrollbar_drag(row);
                } else if self.mouse.drag_state == DragState::DetailHScrollbar {
                    self.handle_hscrollbar_drag(column);
                }
                // Handle drag to resize tree pane
                else if self.mouse.drag_state == DragState::Divider {
                    self.handle_divider_drag(column);
                }
                // Handle drag to resize table columns
                else if let DragState::ColumnBorder(col_idx) = self.mouse.drag_state {
                    self.handle_column_border_drag(col_idx, column);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.is_in_tree_area(column, row) {
                    self.move_down();
                } else if self.is_in_detail_area(column, row) {
                    self.focus_state = FocusState::Detail;
                    if self.is_current_section_composite() {
                        let section_idx = self.get_section_index();
                        self.detail.ensure_composite_capacity(section_idx);
                        if let Some(scroll) = self.detail.composite_scroll.get_mut(section_idx) {
                            *scroll = scroll.saturating_add(1);
                        }
                    } else {
                        let section_idx = self.get_section_index();
                        if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                            *cursor = cursor.saturating_add(3);
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                if self.is_in_tree_area(column, row) {
                    self.move_up();
                } else if self.is_in_detail_area(column, row) {
                    self.focus_state = FocusState::Detail;
                    if self.is_current_section_composite() {
                        let section_idx = self.get_section_index();
                        self.detail.ensure_composite_capacity(section_idx);
                        if let Some(scroll) = self.detail.composite_scroll.get_mut(section_idx) {
                            *scroll = scroll.saturating_sub(1);
                        }
                    } else {
                        let section_idx = self.get_section_index();
                        if let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) {
                            *cursor = cursor.saturating_sub(3);
                        }
                    }
                }
            }
            MouseEventKind::ScrollLeft if self.is_in_detail_area(column, row) => {
                self.focus_state = FocusState::Detail;
                self.scroll_horizontal(-5);
            }
            MouseEventKind::ScrollRight if self.is_in_detail_area(column, row) => {
                self.focus_state = FocusState::Detail;
                self.scroll_horizontal(5);
            }
            // Mouse back button not supported by crossterm 0.29 (only Left/Right/Middle)
            _ => {}
        }
        Action::Continue
    }

    fn handle_click(&mut self, column: u16, row: u16) {
        if self.is_in_breadcrumb_area(column, row) {
            self.handle_breadcrumb_click(column);
        } else if self.is_in_tree_area(column, row) {
            self.focus_state = FocusState::Tree;
            self.handle_tree_click(row);
        } else if self.is_in_detail_area(column, row) {
            self.focus_state = FocusState::Detail;
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
            if let Some(area) = self.layout.table_content_area {
                let relative_row = usize::from(row.saturating_sub(area.y));
                if relative_row < HEADER_HEIGHT {
                    return;
                }
            }

            // Delegate to the unified Enter-in-detail dispatch
            self.handle_enter_in_detail_pane();
        }
    }
}
