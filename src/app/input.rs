/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
};

use super::{
    App, COMPOSITE_SCROLL_STEP, DIVIDER_MAX_PCT, DIVIDER_MIN_PCT, FocusState, PAGE_SIZE,
    TREE_WIDTH_STEP,
};

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
                self.search.active = false;
                self.search.query.clear();
                // Note: Don't clear search_stack, it persists
            }
            KeyCode::Enter => {
                // Finalize current search and add to stack
                self.search.active = false;
                self.update_search();
            }
            KeyCode::Backspace => {
                if self.search.query.is_empty() {
                    // If search input is empty, pop from search stack
                    if !self.search.stack.is_empty() {
                        self.search.stack.pop();
                        self.rebuild_visible();
                        let depth = self.search.stack.len();
                        if depth > 0 {
                            let stack_display: Vec<String> =
                                self.search.stack.iter().map(|e| e.query.clone()).collect();
                            self.status =
                                format!("Search depth: {} [{}]", depth, stack_display.join(" → "));
                        } else {
                            self.status = "All searches cleared".into();
                        }
                    }
                } else {
                    self.search.query.pop();
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
                self.page_up(PAGE_SIZE);
            }
            KeyCode::PageDown => {
                self.page_down(PAGE_SIZE);
            }
            KeyCode::Home => {
                self.home();
            }
            KeyCode::End => {
                self.end();
            }

            // Regular character input for search
            KeyCode::Char(c) => {
                self.search.query.push(c);
                // Don't call update_search() here - only update on Enter
            }

            _ => {}
        }
        Action::Continue
    }

    /// Handle a key press in normal (non-search) mode.
    pub(super) fn handle_normal_key(&mut self, code: KeyCode, ctrl: bool) -> Action {
        // Early return for help popup
        if self.focus_state == FocusState::HelpPopup {
            if matches!(code, KeyCode::Esc | KeyCode::Char('?')) {
                self.focus_state = FocusState::Tree;
            }
            return Action::Continue;
        }

        // Early return for detail popup
        if self.detail.popup.is_some() {
            if matches!(code, KeyCode::Esc) {
                self.detail.popup = None;
            }
            return Action::Continue;
        }

        // Clear jump buffer if timed out (>1 second since last character)
        if let Some(last_time) = self.table.jump_buffer_time
            && last_time.elapsed() > std::time::Duration::from_secs(1)
        {
            self.table.jump_buffer.clear();
            self.table.jump_buffer_time = None;
        }

        match code {
            KeyCode::Char('Q') | KeyCode::Esc => return Action::Quit,
            KeyCode::Char('c') if ctrl => return Action::Quit,

            KeyCode::Backspace => {
                self.navigate_to_previous_in_history();
            }

            KeyCode::Tab => {
                self.focus_state = if self.focus_state == FocusState::Detail {
                    FocusState::Tree
                } else {
                    FocusState::Detail
                };
                // Clear the jump buffer on focus switch so a buffer accumulated
                // in one mode doesn't carry over and trigger an unintended jump
                // in the other mode.
                self.table.jump_buffer.clear();
                self.table.jump_buffer_time = None;
            }

            // Pane resizing
            KeyCode::Char('+' | '=') => {
                self.layout.tree_width_percentage = self
                    .layout
                    .tree_width_percentage
                    .saturating_add(TREE_WIDTH_STEP)
                    .min(DIVIDER_MAX_PCT);
            }
            KeyCode::Char('-' | '_') => {
                self.layout.tree_width_percentage = (self
                    .layout
                    .tree_width_percentage
                    .saturating_sub(TREE_WIDTH_STEP))
                .max(DIVIDER_MIN_PCT);
            }

            // Arrow keys and uppercase vim keys navigate in all modes
            KeyCode::Up
            | KeyCode::Char('K' | 'J' | 'H' | 'L')
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Left
            | KeyCode::Right => {
                self.handle_navigation_key(code);
            }
            // Lowercase vim keys navigate only when NOT in detail pane
            // (in detail pane they go to type-to-jump)
            KeyCode::Char('k' | 'j' | 'h' | 'l') if self.focus_state != FocusState::Detail => {
                self.handle_navigation_key(code);
            }
            KeyCode::Enter => {
                if self.focus_state == FocusState::Detail {
                    self.handle_enter_in_detail_pane();
                } else if !self.try_navigate_parent_ref_from_tree() {
                    self.try_expand();
                }
            }
            KeyCode::Char(' ') if self.focus_state != FocusState::Detail => {
                self.toggle_expand();
            }

            KeyCode::Char('e') if self.focus_state != FocusState::Detail => self.expand_all(),
            KeyCode::Char('c') if self.focus_state != FocusState::Detail => self.collapse_all(),

            // Clear search stack ('x')
            KeyCode::Char('x') if self.focus_state != FocusState::Detail => {
                self.clear_search_stack();
            }

            // Cycle search scope (Shift+S)
            KeyCode::Char('S') if self.focus_state != FocusState::Detail => {
                self.cycle_search_scope();
            }

            // Set subtree search scope to current node ('t')
            KeyCode::Char('t') if self.focus_state != FocusState::Detail => {
                self.set_subtree_scope();
            }

            // Sort children of the current parent tree node
            KeyCode::Char('s') if self.focus_state != FocusState::Detail => {
                self.toggle_tree_sort();
            }

            // Toggle table column sorting (Shift+S when detail pane is focused)
            KeyCode::Char('S') if self.focus_state == FocusState::Detail => {
                self.toggle_table_column_sort();
            }

            // Copy table to clipboard as markdown (Shift+C when detail pane is focused)
            KeyCode::Char('C') if self.focus_state == FocusState::Detail => {
                self.status = self.copy_table_to_clipboard();
            }

            KeyCode::Char('/') => {
                self.search.active = true;
                self.search.query.clear();
                let depth = self.search.stack.len();
                if depth > 0 {
                    self.status = format!("Add search (depth {depth}+1): ");
                } else {
                    self.status = "Search: ".into();
                }
            }
            KeyCode::Char('n') => self.next_search_match(),
            KeyCode::Char('N') => self.prev_search_match(),

            // Column/scroll keys (only when detail pane is focused)
            KeyCode::Char('[' | ']' | ',' | '.' | '<' | '>')
                if self.focus_state == FocusState::Detail =>
            {
                self.handle_detail_column_key(code);
            }

            // Toggle mouse mode (works everywhere)
            KeyCode::Char('m') => {
                self.toggle_mouse_mode();
            }

            // Show help popup
            KeyCode::Char('?') => {
                self.focus_state = FocusState::HelpPopup;
            }

            // Type-to-jump: alphanumeric keys jump to matching row in detail pane
            KeyCode::Char(c) if self.focus_state == FocusState::Detail && c.is_alphanumeric() => {
                self.table.jump_buffer.push(c.to_ascii_lowercase());
                self.table.jump_buffer_time = Some(std::time::Instant::now());
                self.jump_to_matching_row();
            }

            // Type-to-jump: alphanumeric keys jump to matching tree node
            KeyCode::Char(c) if self.focus_state != FocusState::Detail && c.is_alphanumeric() => {
                self.table.jump_buffer.push(c.to_ascii_lowercase());
                self.table.jump_buffer_time = Some(std::time::Instant::now());
                self.jump_to_matching_tree_node();
            }

            _ => {}
        }
        Action::Continue
    }

    /// Handle column resize, focus, and horizontal scroll keys in the detail pane.
    fn handle_detail_column_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('[') => self.resize_column(-10),
            KeyCode::Char(']') => self.resize_column(10),
            KeyCode::Char(',') => {
                self.table.focused_column = self.table.focused_column.saturating_sub(1);
                let section_idx = self.get_section_index();
                self.ensure_focused_column_visible(section_idx);
            }
            KeyCode::Char('.') => {
                self.table.focused_column = self.table.focused_column.saturating_add(1);
                let section_idx = self.get_section_index();
                self.ensure_focused_column_visible(section_idx);
            }
            KeyCode::Char('<') => self.scroll_horizontal(-10),
            KeyCode::Char('>') => self.scroll_horizontal(10),
            _ => {}
        }
    }

    /// Handle navigation keys, dispatching to detail or tree navigation.
    fn handle_navigation_key(&mut self, code: KeyCode) {
        if self.focus_state == FocusState::Detail {
            self.handle_detail_navigation(code);
            return;
        }
        match code {
            KeyCode::Up | KeyCode::Char('K' | 'k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('J' | 'j') => self.move_down(),
            KeyCode::PageUp => self.page_up(PAGE_SIZE),
            KeyCode::PageDown => self.page_down(PAGE_SIZE),
            KeyCode::Home => self.home(),
            KeyCode::End => self.end(),
            KeyCode::Left | KeyCode::Char('H' | 'h') => self.try_collapse_or_parent(),
            KeyCode::Right | KeyCode::Char('L' | 'l') => self.try_expand(),
            _ => {}
        }
    }

    /// Apply `f` to the section cursor at `section_idx`, if it exists.
    fn adjust_section_cursor(&mut self, section_idx: usize, f: impl Fn(usize) -> usize) {
        if let Some(c) = self.detail.section_cursors.get_mut(section_idx) {
            *c = f(*c);
        }
    }

    /// Apply `f` to the composite scroll at `section_idx`, if it exists.
    fn adjust_composite_scroll(&mut self, section_idx: usize, f: impl Fn(usize) -> usize) {
        if let Some(s) = self.detail.composite_scroll.get_mut(section_idx) {
            *s = f(*s);
        }
    }

    /// Handle navigation keys when the detail pane is focused.
    fn handle_detail_navigation(&mut self, code: KeyCode) {
        let section_idx = self.get_section_index();

        // For Composite sections, scroll by block index
        if self.is_current_section_composite() {
            self.detail.ensure_composite_capacity(section_idx);
            match code {
                KeyCode::Up | KeyCode::Char('K') => {
                    self.adjust_composite_scroll(section_idx, |s| s.saturating_sub(1));
                }
                KeyCode::Down | KeyCode::Char('J') => {
                    self.adjust_composite_scroll(section_idx, |s| s.saturating_add(1));
                }
                KeyCode::PageUp => {
                    self.adjust_composite_scroll(section_idx, |s| {
                        s.saturating_sub(COMPOSITE_SCROLL_STEP)
                    });
                }
                KeyCode::PageDown => {
                    self.adjust_composite_scroll(section_idx, |s| {
                        s.saturating_add(COMPOSITE_SCROLL_STEP)
                    });
                }
                KeyCode::Home => {
                    self.adjust_composite_scroll(section_idx, |_| 0);
                }
                KeyCode::End => {
                    let max = self.detail.composite_max_scroll;
                    self.adjust_composite_scroll(section_idx, |_| max);
                }
                KeyCode::Left | KeyCode::Char('H') => {
                    let new_tab = self.detail.selected_tab.saturating_sub(1);
                    self.set_selected_tab(new_tab);
                    self.table.focused_column = 0;
                }
                KeyCode::Right | KeyCode::Char('L') => {
                    let new_tab = self.detail.selected_tab.saturating_add(1);
                    self.set_selected_tab(new_tab);
                    self.table.focused_column = 0;
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Up | KeyCode::Char('K') => {
                self.adjust_section_cursor(section_idx, |c| c.saturating_sub(1));
            }
            KeyCode::Down | KeyCode::Char('J') => {
                self.adjust_section_cursor(section_idx, |c| c.saturating_add(1));
            }
            KeyCode::PageUp => {
                self.adjust_section_cursor(section_idx, |c| c.saturating_sub(PAGE_SIZE));
            }
            KeyCode::PageDown => {
                self.adjust_section_cursor(section_idx, |c| c.saturating_add(PAGE_SIZE));
            }
            KeyCode::Home => {
                self.adjust_section_cursor(section_idx, |_| 0);
            }
            KeyCode::End => {
                let row_count = self
                    .tree
                    .visible
                    .get(self.tree.cursor)
                    .and_then(|&node_idx| self.tree.all_nodes.get(node_idx))
                    .and_then(|node| node.detail_sections.get(section_idx))
                    .and_then(|s| s.content.table_rows())
                    .map_or(0, <[_]>::len);
                self.adjust_section_cursor(section_idx, |_| row_count.saturating_sub(1));
            }
            KeyCode::Left | KeyCode::Char('H') => {
                let new_tab = self.detail.selected_tab.saturating_sub(1);
                self.set_selected_tab(new_tab);
                self.table.focused_column = 0;
            }
            KeyCode::Right | KeyCode::Char('L') => {
                let new_tab = self.detail.selected_tab.saturating_add(1);
                self.set_selected_tab(new_tab);
                self.table.focused_column = 0;
            }
            _ => {}
        }
    }

    pub(crate) fn toggle_mouse_mode(&mut self) {
        let desired = !self.mouse.enabled;
        let result = if desired {
            execute!(std::io::stdout(), EnableMouseCapture)
        } else {
            execute!(std::io::stdout(), DisableMouseCapture)
        };

        if result.is_ok() {
            self.mouse.enabled = desired;
            self.status = format!("Mouse: {}", if desired { "enabled" } else { "disabled" });
        } else {
            self.status = "Failed to toggle mouse mode".into();
        }
    }
}
