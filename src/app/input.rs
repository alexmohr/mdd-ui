// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use crossterm::event::KeyCode;

use super::App;

/// Result of processing a key press.
pub enum Action {
    Continue,
    Quit,
}

impl App {
    /// Handle a key press while the search prompt is active.
    pub(super) fn handle_search_key(&mut self, code: KeyCode) -> Action {
        match code {
            KeyCode::Esc => {
                // Clear current search input and exit search mode
                self.searching = false;
                self.search.clear();
                // Note: Don't clear search_stack, it persists
            }
            KeyCode::Enter => {
                // Finalize current search and add to stack
                self.searching = false;
                self.update_search();
            }
            KeyCode::Backspace => {
                if self.search.is_empty() {
                    // If search input is empty, pop from search stack
                    if !self.search_stack.is_empty() {
                        self.search_stack.pop();
                        self.rebuild_visible();
                        let depth = self.search_stack.len();
                        if depth > 0 {
                            let stack_display: Vec<String> = self
                                .search_stack
                                .iter()
                                .map(|(term, _scope)| term.clone())
                                .collect();
                            self.status =
                                format!("Search depth: {} [{}]", depth, stack_display.join(" → "));
                        } else {
                            self.status = "All searches cleared".to_owned();
                        }
                    }
                } else {
                    self.search.pop();
                }
            }

            // Navigation with arrow keys only (preserve letter keys for search input)
            KeyCode::Up => {
                self.move_up();
            }
            KeyCode::Down => {
                self.move_down();
            }
            KeyCode::Left => {
                self.try_collapse_or_parent();
            }
            KeyCode::Right => {
                self.try_expand();
            }
            KeyCode::PageUp => {
                self.page_up(20);
            }
            KeyCode::PageDown => {
                self.page_down(20);
            }
            KeyCode::Home => {
                self.home();
            }
            KeyCode::End => {
                self.end();
            }

            // Toggle mouse mode (works in search mode too)
            KeyCode::Char('m') => {
                self.toggle_mouse_mode();
            }

            // Regular character input for search
            KeyCode::Char(c) => {
                self.search.push(c);
                // Don't call update_search() here - only update on Enter
            }

            _ => {}
        }
        Action::Continue
    }

    /// Handle a key press in normal (non-search) mode.
    pub(super) fn handle_normal_key(&mut self, code: KeyCode, ctrl: bool, shift: bool) -> Action {
        // Early return for help popup
        if self.help_popup_visible {
            if matches!(code, KeyCode::Esc | KeyCode::Char('?')) {
                self.help_popup_visible = false;
            }
            return Action::Continue;
        }

        // Early return for detail popup
        if self.detail_popup.is_some() {
            if matches!(code, KeyCode::Esc) {
                self.detail_popup = None;
            }
            return Action::Continue;
        }

        // Clear jump buffer if timed out (>1 second since last character)
        if let Some(last_time) = self.jump_buffer_time
            && last_time.elapsed() > std::time::Duration::from_secs(1)
        {
            self.jump_buffer.clear();
            self.jump_buffer_time = None;
        }

        match code {
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            KeyCode::Char('c') if ctrl => return Action::Quit,

            KeyCode::Backspace => {
                // Navigate in tree (when not in search mode and not in detail pane)
                if !self.detail_focused {
                    if shift {
                        // Shift+Backspace: Navigate up one layer in hierarchy
                        self.navigate_up_one_layer();
                    } else {
                        // Backspace: Jump to last element in history
                        self.navigate_to_previous_in_history();
                    }
                }
            }

            KeyCode::Tab => {
                self.detail_focused = !self.detail_focused;
                if !self.detail_focused {
                    self.focused_section = 0; // Reset when returning to tree
                }
            }

            // Pane resizing
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.tree_width_percentage = (self.tree_width_percentage + 5).min(80);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.tree_width_percentage = (self.tree_width_percentage.saturating_sub(5)).max(20);
            }

            KeyCode::Up | KeyCode::Char('K') => {
                if self.detail_focused {
                    // Move cursor up in the selected tab
                    let section_idx = self.get_section_index();
                    if section_idx < self.section_cursors.len() {
                        self.section_cursors[section_idx] =
                            self.section_cursors[section_idx].saturating_sub(1);
                    }
                } else {
                    self.move_up();
                }
            }
            KeyCode::Char('k') if !self.detail_focused => {
                self.move_up();
            }
            KeyCode::Down | KeyCode::Char('J') => {
                if self.detail_focused {
                    // Move cursor down in the selected tab (will be clamped during render)
                    let section_idx = self.get_section_index();
                    if section_idx < self.section_cursors.len() {
                        self.section_cursors[section_idx] =
                            self.section_cursors[section_idx].saturating_add(1);
                    }
                } else {
                    self.move_down();
                }
            }
            KeyCode::Char('j') if !self.detail_focused => {
                self.move_down();
            }
            KeyCode::PageUp => {
                if self.detail_focused {
                    // Move cursor up by page in selected tab
                    let section_idx = self.get_section_index();
                    if section_idx < self.section_cursors.len() {
                        self.section_cursors[section_idx] =
                            self.section_cursors[section_idx].saturating_sub(20);
                    }
                } else {
                    self.page_up(20);
                }
            }
            KeyCode::PageDown => {
                if self.detail_focused {
                    // Move cursor down by page in selected tab
                    let section_idx = self.get_section_index();
                    if section_idx < self.section_cursors.len() {
                        self.section_cursors[section_idx] =
                            self.section_cursors[section_idx].saturating_add(20);
                    }
                } else {
                    self.page_down(20);
                }
            }
            KeyCode::Home => {
                if self.detail_focused {
                    // Move cursor to top of selected tab
                    let section_idx = self.get_section_index();
                    if section_idx < self.section_cursors.len() {
                        self.section_cursors[section_idx] = 0;
                    }
                } else {
                    self.home();
                }
            }
            KeyCode::End => {
                if self.detail_focused {
                    // Move cursor to bottom of selected tab (will be clamped during render)
                    let section_idx = self.get_section_index();
                    if section_idx < self.section_cursors.len() {
                        self.section_cursors[section_idx] = usize::MAX;
                    }
                } else {
                    self.end();
                }
            }

            KeyCode::Left | KeyCode::Char('H') => {
                if self.detail_focused {
                    // Navigate to previous tab
                    let new_tab = self.selected_tab.saturating_sub(1);
                    self.set_selected_tab(new_tab);
                    self.focused_column = 0; // Reset column focus when switching tabs
                } else {
                    self.try_collapse_or_parent();
                }
            }
            KeyCode::Char('h') if !self.detail_focused => {
                self.try_collapse_or_parent();
            }
            KeyCode::Right | KeyCode::Char('L') => {
                if self.detail_focused {
                    // Navigate to next tab (will be clamped during render)
                    let new_tab = self.selected_tab.saturating_add(1);
                    self.set_selected_tab(new_tab);
                    self.focused_column = 0; // Reset column focus when switching tabs
                } else {
                    self.try_expand();
                }
            }
            KeyCode::Char('l') if !self.detail_focused => {
                self.try_expand();
            }
            KeyCode::Enter => {
                if !self.detail_focused {
                    self.try_expand();
                } else {
                    self.handle_enter_in_detail_pane();
                }
            }
            KeyCode::Char(' ') if !self.detail_focused => {
                self.toggle_expand();
            }

            KeyCode::Char('e') if !self.detail_focused => self.expand_all(),
            KeyCode::Char('c') if !self.detail_focused => self.collapse_all(),

            // Clear search stack ('x')
            KeyCode::Char('x') if !self.detail_focused => {
                self.clear_search_stack();
            }

            // Cycle search scope (Shift+S)
            KeyCode::Char('S') if !self.detail_focused => {
                self.cycle_search_scope();
            }

            // Set subtree search scope to current node ('t')
            KeyCode::Char('t') if !self.detail_focused => {
                self.set_subtree_scope();
            }

            // Toggle DiagComm sorting (only when tree is focused)
            KeyCode::Char('s') if !self.detail_focused => {
                self.toggle_diagcomm_sort();
            }

            // Toggle table column sorting (Shift+S when detail pane is focused)
            KeyCode::Char('S') if self.detail_focused => {
                self.toggle_table_column_sort();
            }

            KeyCode::Char('/') => {
                self.searching = true;
                self.search.clear();
                let depth = self.search_stack.len();
                if depth > 0 {
                    self.status = format!("Add search (depth {}+1): ", depth);
                } else {
                    self.status = "Search: ".into();
                }
            }
            KeyCode::Char('n') => self.next_search_match(),
            KeyCode::Char('N') => self.prev_search_match(),

            // Column resizing (only when detail pane is focused)
            KeyCode::Char('[') if self.detail_focused => {
                self.resize_column(-10);
            }
            KeyCode::Char(']') if self.detail_focused => {
                self.resize_column(10);
            }
            KeyCode::Char(',') if self.detail_focused => {
                self.focused_column = self.focused_column.saturating_sub(1);
            }
            KeyCode::Char('.') if self.detail_focused => {
                self.focused_column = self.focused_column.saturating_add(1);
            }

            // Toggle mouse mode (works everywhere)
            KeyCode::Char('m') => {
                self.toggle_mouse_mode();
            }

            // Show help popup
            KeyCode::Char('?') => {
                self.help_popup_visible = true;
            }

            // Type-to-jump: alphanumeric keys jump to matching row when detail pane is focused
            KeyCode::Char(c) if self.detail_focused && c.is_alphanumeric() => {
                self.jump_buffer.push(c.to_ascii_lowercase());
                self.jump_buffer_time = Some(std::time::Instant::now());
                self.jump_to_matching_row();
            }

            _ => {}
        }
        Action::Continue
    }
}
