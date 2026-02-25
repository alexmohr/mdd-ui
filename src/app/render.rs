/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
};

use super::{App, FocusState, SortDirection};
use crate::tree::{CellType, DetailContent, DetailRow, DetailSectionData, NodeType, TreeNode};

// -----------------------------------------------------------------------
// Colour theme
// -----------------------------------------------------------------------

fn node_style(node: &TreeNode) -> Style {
    match node.node_type {
        NodeType::Container => style(Color::Blue, true),
        NodeType::SectionHeader | NodeType::ParentRefs | NodeType::DOP | NodeType::SDG => {
            style(Color::Yellow, true)
        }
        // Gray for inherited services
        NodeType::ParentRefService => Style::default().fg(Color::DarkGray),
        NodeType::Service
        | NodeType::Request
        | NodeType::PosResponse
        | NodeType::NegResponse
        | NodeType::FunctionalClass
        | NodeType::Job
        | NodeType::Default => Style::default().fg(Color::White),
    }
}

fn style(fg: Color, bold: bool) -> Style {
    let s = Style::default().fg(fg);
    if bold {
        s.add_modifier(Modifier::BOLD)
    } else {
        s
    }
}

fn border(focused: bool) -> Style {
    Style::default().fg(if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    })
}

fn row_style(node: &TreeNode, is_cursor: bool) -> Style {
    if is_cursor {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    } else {
        node_style(node)
    }
}

fn expand_icon(node: &TreeNode) -> &'static str {
    if !node.has_children {
        "  "
    } else if node.expanded {
        "▼ "
    } else {
        "▶ "
    }
}

// -----------------------------------------------------------------------
// Drawing
// -----------------------------------------------------------------------

impl App {
    /// Extract ECU name from the General node's detail sections
    fn get_ecu_name(&self) -> &str {
        self.all_nodes
            .first()
            .and_then(|node| {
                if node.text != "General" {
                    return None;
                }

                node.detail_sections.first().and_then(|sec| {
                    if let crate::tree::DetailContent::PlainText(lines) = &sec.content {
                        lines.first()?.strip_prefix("ECU Name: ")
                    } else {
                        None
                    }
                })
            })
            .unwrap_or("Tree")
    }

    pub(super) fn draw_tree(&mut self, frame: &mut Frame, area: Rect) {
        let ecu_name = self.get_ecu_name();

        let tree_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border(self.focus_state != FocusState::Detail))
            .title(format!(" {ecu_name} "));

        let tree_inner = tree_block.inner(area);
        frame.render_widget(tree_block, area);

        // Draw tree content
        let viewport_height = tree_inner.height as usize;
        self.ensure_cursor_visible(viewport_height);

        let lines: Vec<Line> = self
            .visible
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(viewport_height)
            .filter_map(|(vi, &node_idx)| {
                let node = self.all_nodes.get(node_idx)?;
                let row_style = row_style(node, vi == self.cursor);

                let indent = "  ".repeat(node.depth);
                let icon = expand_icon(node);

                Some(Line::from(vec![
                    Span::styled(indent, row_style),
                    Span::styled(icon, row_style),
                    Span::styled(&node.text, row_style),
                ]))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), tree_inner);
        self.tree_scrollbar_area = render_scrollbar(
            frame,
            area,
            self.visible.len(),
            self.cursor,
            viewport_height,
        );
    }

    pub(super) fn draw_detail(&mut self, frame: &mut Frame, area: Rect) {
        let Some(&node_idx) = self.visible.get(self.cursor) else {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border(self.focus_state == FocusState::Detail))
                .title(" Details ");
            frame.render_widget(block, area);
            return;
        };

        let Some(selected_node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let node_text = selected_node.text.clone();
        let detail_sections = selected_node.detail_sections.clone();

        if detail_sections.is_empty() {
            // Draw a default/dummy pane with helpful information
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border(self.focus_state == FocusState::Detail))
                .title(" Details ");

            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Add helpful message in the center
            let help_message = [
                "",
                "No detailed information available for this item.",
                "",
                "Navigate the tree to select items with more details.",
                "",
                "Press ? for help.",
            ];

            let paragraph = Paragraph::new(help_message.join("\n"))
                .style(Style::default().fg(Color::DarkGray))
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: false });

            frame.render_widget(paragraph, inner);
        } else {
            self.draw_detail_panes(frame, area, &detail_sections, &node_text);
        }
    }

    /// Build breadcrumb path for the currently selected node
    /// Returns a vector of (text, `node_idx`) pairs in root-to-leaf order
    fn build_breadcrumb_segments(&self) -> Vec<(String, usize)> {
        if let Some(&node_idx) = self.visible.get(self.cursor) {
            let mut path_segments = Vec::new();
            let mut current_idx = node_idx;

            // Walk up the tree to build the path
            while let Some(node) = self.all_nodes.get(current_idx) {
                path_segments.push((node.text.clone(), current_idx));

                // Find parent by looking for previous node with lower depth
                let current_depth = node.depth;
                if current_depth == 0 {
                    break;
                }

                let parent_idx = (0..current_idx).rev().find(|&i| {
                    self.all_nodes
                        .get(i)
                        .is_some_and(|n| n.depth < current_depth)
                });

                let Some(idx) = parent_idx else {
                    break;
                };
                current_idx = idx;
            }

            // Reverse to get root-to-leaf order
            path_segments.reverse();
            path_segments
        } else {
            Vec::new()
        }
    }

    pub(super) fn draw_breadcrumb(&mut self, frame: &mut Frame, area: Rect) {
        // Get breadcrumb segments with their node indices
        let segments = self.build_breadcrumb_segments();

        // Build the display text and track segment positions
        let mut breadcrumb_segments = Vec::new();
        let mut col_position: u16 = area.x;

        for (i, (text, node_idx)) in segments.iter().enumerate() {
            let start_col = col_position;
            let text_len = u16::try_from(text.len()).unwrap_or(u16::MAX);
            let end_col = start_col.saturating_add(text_len);

            breadcrumb_segments.push((text.clone(), *node_idx, start_col, end_col));
            col_position = end_col;

            // Add separator if not the last segment
            if i < segments.len().saturating_sub(1) {
                col_position = col_position.saturating_add(3); // " > " is 3 characters
            }
        }

        // Store segments for click handling
        self.breadcrumb_segments = breadcrumb_segments;

        // Build display string
        let breadcrumb_text: String = segments
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join(" > ");

        let paragraph = Paragraph::new(breadcrumb_text)
            .style(Style::default().fg(Color::Cyan).bg(Color::Black));
        frame.render_widget(paragraph, area);
    }

    pub(super) fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let (text, st) = if self.searching {
            let current_search_info = if self.search_stack.is_empty() {
                String::new()
            } else {
                let stack_display: Vec<String> = self
                    .search_stack
                    .iter()
                    .map(|(term, _scope)| term.clone())
                    .collect();
                format!(" [active: {}]", stack_display.join(" → "))
            };

            (
                format!(
                    " /{}█{}{}  (scope: {} | Shift+S to change (leave search first) | Enter to \
                     add, Esc to cancel |  Backspace to undo last search)",
                    self.search,
                    self.search_scope.search_indicator(),
                    current_search_info,
                    self.search_scope
                ),
                Style::default().fg(Color::Yellow).bg(Color::DarkGray),
            )
        } else if !self.status.is_empty() {
            (
                format!(" {}", self.status),
                Style::default().fg(Color::Gray),
            )
        } else {
            let focus = if self.focus_state == FocusState::Detail {
                "detail"
            } else {
                "tree"
            };

            let search_info = if self.search_stack.is_empty() {
                String::new()
            } else {
                let stack_display: Vec<String> = self
                    .search_stack
                    .iter()
                    .map(|(term, scope)| format!("{term}{}", scope.abbrev()))
                    .collect();
                let joined = stack_display.join(" → ");
                format!(" | searches: {joined}")
            };

            (
                format!(
                    " {}/{} nodes | cursor: {} | focus: {focus}{}{}",
                    self.visible.len(),
                    self.all_nodes.len(),
                    self.cursor.saturating_add(1),
                    self.search_scope.status_indicator(),
                    search_info,
                ),
                Style::default().fg(Color::Gray),
            )
        };
        frame.render_widget(Paragraph::new(text).style(st), area);
    }

    pub(super) fn draw_help_popup(frame: &mut Frame) {
        use ratatui::{
            layout::{Alignment, Rect},
            style::{Color, Style},
            widgets::{Block, Borders, Clear, Paragraph, Wrap},
        };

        // Calculate popup size and position (centered, 70% width, 80% height)
        let area = frame.area();
        let popup_width = area.width.saturating_mul(70) / 100;
        let popup_height = area.height.saturating_mul(80) / 100;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_rect = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_rect);

        // Draw the popup block
        let block = Block::default()
            .title(" Help - Press ? or Esc to close ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner_area = block.inner(popup_rect);
        frame.render_widget(block, popup_rect);

        // Help content
        let help_text = vec![
            "NAVIGATION",
            "  ↑/↓ or k/j      Move cursor up/down",
            "  ←/→ or h/l      Collapse/expand node (or navigate tabs in detail)",
            "  PgUp/PgDn       Page up/down",
            "  Home/End        Jump to first/last",
            "  Space           Toggle expand/collapse current node",
            "  Tab             Switch focus between tree and detail pane",
            "  Backspace       Jump to last element in navigation history",
            "  Shift+Backspace Navigate up one layer (parent node)",
            "",
            "TREE OPERATIONS",
            "  e               Expand all nodes",
            "  c               Collapse all nodes",
            "  s               Toggle DiagComm sort (by ID/name)",
            "",
            "SEARCH & FILTER",
            "  /               Start search (type, then Enter to add to stack)",
            "  Shift+S         Cycle search scope \
             (All/Variants/Services/Diag-Comms/Requests/Responses)",
            "  t               Scope search to subtree under cursor",
            "  Enter           Confirm search and add to stack",
            "  x               Clear all search filters",
            "  Backspace       Remove last search from stack (when search input empty)",
            "  Esc             Cancel current search input",
            "",
            "DETAIL PANE (when focused)",
            "  ↑/↓ or Shift+K/J  Navigate rows in table",
            "  ←/→ or Shift+H/L  Switch between tabs",
            "  Enter              Navigate to element (or show details popup)",
            "  Shift+S            Toggle sort on focused column",
            "  [ / ]              Decrease/increase column width",
            "  , / .              Select previous/next column",
            "  a-z, 0-9           Type-to-jump to matching row (resets after 1s)",
            "",
            "WINDOW",
            "  + / -           Increase/decrease tree pane width",
            "  Mouse drag      Drag the divider between tree and detail to resize",
            "  m               Toggle mouse mode (enable/disable terminal text selection)",
            "  ?               Show this help",
            "  q or Esc        Quit application",
        ];

        let help_paragraph = Paragraph::new(help_text.join("\n"))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        frame.render_widget(help_paragraph, inner_area);
    }

    pub(super) fn draw_detail_popup(&self, frame: &mut Frame) {
        use ratatui::{
            layout::{Alignment, Rect},
            style::{Color, Style},
            widgets::{Block, Borders, Clear, Paragraph, Wrap},
        };

        let Some(popup_data) = &self.detail_popup else {
            return;
        };

        // Calculate popup size and position (centered, 60% width, 50% height)
        let area = frame.area();
        let popup_width = area.width.saturating_mul(60) / 100;
        let popup_height = area.height.saturating_mul(50) / 100;
        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_rect = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area first
        frame.render_widget(Clear, popup_rect);

        // Create the popup block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(format!(" {} ", popup_data.title))
            .title_alignment(Alignment::Center)
            .title_bottom(" Press Esc to close ")
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(popup_rect);
        frame.render_widget(block, popup_rect);

        // Render the content
        let content_text = popup_data.content.join("\n");
        let paragraph = Paragraph::new(content_text)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, inner);
    }

    fn draw_detail_panes(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        sections: &[DetailSectionData],
        node_name: &str,
    ) {
        if sections.is_empty() {
            return;
        }

        // Separate header and tab sections
        let (header_section, tab_sections) = Self::split_header_and_tabs(sections);

        // Setup outer block and detail title
        let detail_title = format!(" {node_name} ");
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border(self.focus_state == FocusState::Detail))
            .title(detail_title);
        let outer_inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Clamp selected tab to valid range
        self.selected_tab = self.selected_tab.min(tab_sections.len().saturating_sub(1));

        // Initialize state vectors
        self.ensure_section_state_initialized(sections);

        // Build layout constraints
        let header_height =
            header_section.and_then(|h| Self::calculate_header_height(h, outer_inner));
        let chunks = Self::build_detail_layout(outer_inner, header_height);

        // Render header section if present
        if let (Some(hdr), Some(&area)) =
            (header_section, header_height.and_then(|_| chunks.first()))
        {
            Self::render_header_section(frame, area, hdr);
        }

        // Render content area
        let Some(&content_area) = chunks.get(usize::from(header_height.is_some())) else {
            return;
        };

        self.render_content_area(frame, content_area, tab_sections, sections, node_name);
    }

    /// Split sections into header and tabs
    fn split_header_and_tabs(
        sections: &[DetailSectionData],
    ) -> (Option<&DetailSectionData>, &[DetailSectionData]) {
        let Some((first, rest)) = sections.split_first() else {
            return (None, sections);
        };
        if sections.len() > 1
            && first.render_as_header
            && matches!(&first.content, DetailContent::PlainText(_))
        {
            (Some(first), rest)
        } else {
            (None, sections)
        }
    }

    /// Calculate header height for a section
    fn calculate_header_height(header: &DetailSectionData, outer_inner: Rect) -> Option<u16> {
        match &header.content {
            DetailContent::PlainText(lines) => {
                let height = u16::try_from(lines.len())
                    .unwrap_or(u16::MAX)
                    .max(1)
                    .min(outer_inner.height / 4);
                Some(height)
            }
            _ => None,
        }
    }

    /// Build layout for detail pane (header + content)
    fn build_detail_layout(outer_inner: Rect, header_height: Option<u16>) -> Vec<Rect> {
        use ratatui::layout::{Constraint, Direction, Layout};

        let mut constraints = vec![];
        if let Some(h) = header_height {
            constraints.push(Constraint::Length(h));
        }
        constraints.push(Constraint::Min(0));

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(outer_inner)
            .to_vec()
    }

    /// Ensure section state vectors are properly sized
    fn ensure_section_state_initialized(&mut self, sections: &[DetailSectionData]) {
        while self.section_scrolls.len() < sections.len() {
            self.section_scrolls.push(0);
        }
        while self.section_cursors.len() < sections.len() {
            self.section_cursors.push(0);
        }

        // Initialize table_sort_state and column_widths
        while self.table_sort_state.len() < sections.len() {
            let section_idx = self.table_sort_state.len();
            self.table_sort_state
                .push(Self::initialize_table_sort(sections.get(section_idx)));
        }

        while self.column_widths.len() < sections.len() {
            self.column_widths.push(Vec::new());
        }
    }

    /// Initialize table sort state for a section
    fn initialize_table_sort(section: Option<&DetailSectionData>) -> Option<super::TableSortState> {
        section
            .filter(|s| matches!(&s.content, DetailContent::Table { .. }))
            .map(|_| super::TableSortState {
                column: 0,
                direction: super::SortDirection::Ascending,
            })
    }

    /// Render header section
    fn render_header_section(frame: &mut Frame, area: Rect, header: &DetailSectionData) {
        if let DetailContent::PlainText(lines) = &header.content {
            let text = lines.join("\n");
            let para = Paragraph::new(text).style(Style::default().fg(Color::White));
            frame.render_widget(para, area);
        }
    }

    /// Render the main content area (tabs + content)
    fn render_content_area(
        &mut self,
        frame: &mut Frame,
        content_area: Rect,
        tab_sections: &[DetailSectionData],
        all_sections: &[DetailSectionData],
        _node_name: &str,
    ) {
        let show_tabs = tab_sections.len() > 1;
        let section_offset = usize::from(all_sections.len() > tab_sections.len());

        let Some(section) = tab_sections.get(self.selected_tab) else {
            return;
        };
        let help_text = if self.focus_state == FocusState::Detail {
            " H/L:tabs  J/K:row  ,/.:column  [/]:resize  Enter:Select  S:sort  a-z:jump"
        } else {
            ""
        };

        // Content block with borders
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border(self.focus_state == FocusState::Detail))
            .title_bottom(help_text);

        let block_inner = block.inner(content_area);
        frame.render_widget(block, content_area);

        // Render tabs if needed, then content
        let inner = if show_tabs {
            self.render_tabs_and_get_content_area(frame, block_inner, tab_sections)
        } else {
            self.tab_area = None;
            self.tab_titles.clear();
            block_inner
        };

        // Cache table content area
        self.table_content_area = Some(inner);

        // Render section content
        self.render_section_content(
            frame,
            inner,
            content_area,
            section,
            self.selected_tab.saturating_add(section_offset),
        );
    }

    /// Render tabs and return content area
    fn render_tabs_and_get_content_area(
        &mut self,
        frame: &mut Frame,
        block_inner: Rect,
        tab_sections: &[DetailSectionData],
    ) -> Rect {
        use ratatui::layout::{Constraint, Direction, Layout};

        let tab_titles: Vec<String> = tab_sections.iter().map(|s| s.title.clone()).collect();
        let tab_lines_needed = Self::calculate_tab_lines(&tab_titles, block_inner.width as usize);
        let tab_height = u16::try_from(tab_lines_needed).unwrap_or(u16::MAX).max(1);

        let tab_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(tab_height), Constraint::Min(0)])
            .split(block_inner);

        let Some(&tab_area) = tab_chunks.first() else {
            return block_inner;
        };
        let Some(&content_inner) = tab_chunks.get(1) else {
            return block_inner;
        };

        // Cache tab data
        self.tab_area = Some(tab_area);
        self.tab_titles.clone_from(&tab_titles);

        // Render tabs
        Self::render_wrapped_tabs(
            frame,
            tab_area,
            &tab_titles,
            self.selected_tab,
            self.focus_state == FocusState::Detail,
        );

        content_inner
    }

    /// Render section content based on type
    fn render_section_content(
        &mut self,
        frame: &mut Frame,
        inner: Rect,
        area: Rect,
        section: &DetailSectionData,
        section_idx: usize,
    ) {
        match &section.content {
            DetailContent::PlainText(lines) => {
                let text = lines.join("\n");
                let para = Paragraph::new(text).style(Style::default().fg(Color::White));
                frame.render_widget(para, inner);
            }
            DetailContent::Table {
                header,
                rows,
                constraints,
                use_row_selection,
            } => {
                self.render_table_content(
                    frame,
                    inner,
                    area,
                    header,
                    rows,
                    constraints,
                    section_idx,
                    *use_row_selection,
                );
            }
            DetailContent::Composite(subsections) => {
                Self::render_composite_content(frame, inner, subsections);
            }
        }
    }

    fn render_composite_content(
        frame: &mut Frame,
        area: Rect,
        subsections: &[crate::tree::DetailSectionData],
    ) {
        use ratatui::{
            layout::{Constraint, Direction, Layout},
            widgets::Block,
        };

        if subsections.is_empty() {
            return;
        }

        // Create vertical layout for subsections
        let subsection_count = subsections.len();
        let pct = 100u16.saturating_div(u16::try_from(subsection_count).unwrap_or(1).max(1));
        let constraints: Vec<Constraint> = (0..subsection_count)
            .map(|_| Constraint::Percentage(pct))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Render each subsection in its own box
        for (i, subsection) in subsections.iter().enumerate() {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(format!(" {} ", subsection.title));

            let Some(&chunk) = chunks.get(i) else {
                continue;
            };
            let inner = block.inner(chunk);
            frame.render_widget(block, chunk);

            // Render the content of the subsection
            match &subsection.content {
                crate::tree::DetailContent::PlainText(lines) => {
                    let text = lines.join("\n");
                    let para = Paragraph::new(text).style(Style::default().fg(Color::White));
                    frame.render_widget(para, inner);
                }
                crate::tree::DetailContent::Table {
                    header,
                    rows,
                    constraints,
                    ..
                } => {
                    // For composite tables, we don't track cursors/scrolling per subsection yet
                    // This is a simplified rendering
                    Self::render_simple_table(frame, inner, header, rows, constraints);
                }
                crate::tree::DetailContent::Composite(_) => {
                    // Nested composites not supported
                    let text = "(Nested composites not supported)";
                    let para = Paragraph::new(text).style(Style::default().fg(Color::Red));
                    frame.render_widget(para, inner);
                }
            }
        }
    }

    fn render_simple_table(
        frame: &mut Frame,
        area: Rect,
        header: &DetailRow,
        rows: &[DetailRow],
        constraints: &[crate::tree::ColumnConstraint],
    ) {
        let max_columns = rows
            .iter()
            .map(|r| r.cells.len())
            .max()
            .unwrap_or(header.cells.len());

        // Convert constraints to ratatui Constraints
        let mut ratatui_constraints: Vec<Constraint> = constraints
            .iter()
            .map(|c| match c {
                crate::tree::ColumnConstraint::Fixed(w) => Constraint::Length(*w),
                crate::tree::ColumnConstraint::Percentage(p) => Constraint::Percentage(*p),
            })
            .collect();

        // Ensure we have enough constraints
        while ratatui_constraints.len() < max_columns {
            ratatui_constraints.push(Constraint::Percentage(10));
        }

        // Create header
        let header_cells: Vec<Cell> = header
            .cells
            .iter()
            .map(|c| {
                Cell::from(c.as_str()).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();
        let header_row = Row::new(header_cells);

        // Create data rows
        let data_rows: Vec<Row> = rows
            .iter()
            .map(|row_data| {
                let indent_str = "  ".repeat(row_data.indent / 2);
                let mut cells: Vec<Cell> = row_data
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(col_idx, cell_text)| {
                        let text = if col_idx == 0 {
                            format!("{indent_str}{cell_text}")
                        } else {
                            cell_text.clone()
                        };
                        Cell::from(text).style(Style::default().fg(Color::White))
                    })
                    .collect();

                while cells.len() < max_columns {
                    cells.push(Cell::from(""));
                }
                Row::new(cells)
            })
            .collect();

        let table = Table::new(data_rows, ratatui_constraints)
            .column_spacing(1)
            .header(header_row);
        frame.render_widget(table, area);
    }

    fn sort_rows(&self, rows: &[DetailRow], section_idx: usize) -> Vec<DetailRow> {
        use crate::app::SortDirection;

        let Some(sort_state) = self
            .table_sort_state
            .get(section_idx)
            .and_then(|s| s.as_ref())
        else {
            return rows.to_vec();
        };

        let mut sorted = rows.to_vec();
        let col = sort_state.column;
        let dir = sort_state.direction;
        sorted.sort_by(|a, b| {
            let a_cell = a.cells.get(col).map_or("", String::as_str);
            let b_cell = b.cells.get(col).map_or("", String::as_str);

            let cmp = match (a_cell.parse::<f64>(), b_cell.parse::<f64>()) {
                (Ok(a_num), Ok(b_num)) => a_num
                    .partial_cmp(&b_num)
                    .unwrap_or(std::cmp::Ordering::Equal),
                _ => a_cell.cmp(b_cell),
            };

            match dir {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
        sorted
    }

    fn clamp_section_cursor_and_scroll(
        &mut self,
        section_idx: usize,
        row_count: usize,
        viewport_height: usize,
    ) {
        let Some(cursor) = self.section_cursors.get_mut(section_idx) else {
            return;
        };
        let Some(scroll) = self.section_scrolls.get_mut(section_idx) else {
            return;
        };

        if row_count > 0 && *cursor >= row_count {
            *cursor = row_count.saturating_sub(1);
        }

        if row_count > 0 {
            if *cursor < *scroll {
                *scroll = *cursor;
            } else if *cursor >= scroll.saturating_add(viewport_height) {
                *scroll = cursor.saturating_sub(viewport_height).saturating_add(1);
            }
        }

        if row_count > viewport_height && *scroll >= row_count.saturating_sub(viewport_height) {
            *scroll = row_count.saturating_sub(viewport_height);
        }
    }

    // render_table_content needs all params to draw sorted, scrollable, selectable tables
    #[allow(clippy::too_many_arguments)]
    fn render_table_content(
        &mut self,
        frame: &mut Frame,
        inner: Rect,
        _area: Rect,
        header: &DetailRow,
        rows: &[DetailRow],
        constraints: &[crate::tree::ColumnConstraint],
        section_idx: usize,
        use_row_selection: bool,
    ) {
        // Account for header height (3 lines) when calculating viewport
        let header_height = 3u16;
        let viewport_height = (inner.height.saturating_sub(header_height)).max(1) as usize;

        // Apply sorting based on table_sort_state if set
        let sorted_rows: Vec<DetailRow> = self.sort_rows(rows, section_idx);

        let max_columns = sorted_rows
            .iter()
            .map(|r| r.cells.len())
            .max()
            .unwrap_or(header.cells.len());

        let rows_refs: Vec<&DetailRow> = sorted_rows.iter().collect();

        let row_count = rows_refs.len();
        self.clamp_section_cursor_and_scroll(section_idx, row_count, viewport_height);

        let focused_col = if self.focused_column >= max_columns {
            max_columns.saturating_sub(1)
        } else {
            self.focused_column
        };

        // Build visible rows with column-specific or row-specific highlighting
        let visible_rows = self.build_visible_rows(
            &rows_refs,
            section_idx,
            viewport_height,
            max_columns,
            focused_col,
            use_row_selection,
        );

        // Get column widths for this section, or use defaults from constraints
        let column_widths = self.get_column_widths(section_idx, constraints);

        // Convert to ratatui Constraint using custom widths
        let ratatui_constraints: Vec<Constraint> = column_widths
            .iter()
            .map(|&w| Constraint::Percentage(w))
            .collect();

        // Cache the exact constraints for accurate click detection
        self.cached_ratatui_constraints
            .clone_from(&ratatui_constraints);

        // Build header row
        let header_row = self.build_header_row(header, section_idx, max_columns, focused_col);

        let table = Table::new(visible_rows, ratatui_constraints)
            .column_spacing(3)
            .header(header_row);
        frame.render_widget(table, inner);

        if row_count > viewport_height {
            // Render scrollbar below the header to avoid color overlap
            let scrollbar_area = Rect {
                x: inner.x,
                y: inner.y.saturating_add(header_height),
                width: inner.width,
                height: inner.height.saturating_sub(header_height),
            };
            self.detail_scrollbar_area = render_scrollbar(
                frame,
                scrollbar_area,
                row_count,
                *self.section_cursors.get(section_idx).unwrap_or(&0),
                viewport_height,
            );
        } else {
            self.detail_scrollbar_area = None;
        }
    }

    // build_visible_rows needs viewport, column, and selection state for row rendering
    #[allow(clippy::too_many_arguments)]
    fn build_visible_rows(
        &self,
        rows_refs: &[&DetailRow],
        section_idx: usize,
        viewport_height: usize,
        max_columns: usize,
        focused_col: usize,
        use_row_selection: bool,
    ) -> Vec<Row<'static>> {
        let scroll_offset = self.section_scrolls.get(section_idx).copied().unwrap_or(0);
        let cursor_pos = self.section_cursors.get(section_idx).copied().unwrap_or(0);

        rows_refs
            .iter()
            .skip(scroll_offset)
            .take(viewport_height)
            .enumerate()
            .map(|(idx, row_data)| {
                let indent_str = "  ".repeat(row_data.indent / 2);
                let absolute_row_idx = scroll_offset.saturating_add(idx);
                let is_selected_row =
                    (self.focus_state == FocusState::Detail) && absolute_row_idx == cursor_pos;

                let mut cells: Vec<Cell> = row_data
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(col_idx, cell_text)| {
                        let text = if col_idx == 0 {
                            format!("{indent_str}{cell_text}")
                        } else {
                            cell_text.clone()
                        };

                        let cell_type = row_data
                            .cell_types
                            .get(col_idx)
                            .copied()
                            .unwrap_or(CellType::Text);

                        let style = Self::cell_style(
                            is_selected_row,
                            use_row_selection,
                            col_idx == focused_col,
                            cell_type,
                        );

                        Cell::from(text).style(style)
                    })
                    .collect();

                while cells.len() < max_columns {
                    cells.push(Cell::from(""));
                }
                Row::new(cells)
            })
            .collect()
    }

    fn cell_style(
        is_selected_row: bool,
        use_row_selection: bool,
        is_focused_col: bool,
        cell_type: CellType,
    ) -> Style {
        if is_selected_row {
            if use_row_selection {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if is_focused_col {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Self::jump_target_style(cell_type)
            }
        } else {
            Self::jump_target_style(cell_type)
        }
    }

    fn jump_target_style(cell_type: CellType) -> Style {
        match cell_type {
            CellType::DopReference | CellType::ParameterName => Style::default().fg(Color::Blue),
            _ => Style::default().fg(Color::White),
        }
    }

    fn build_header_row(
        &self,
        header: &DetailRow,
        section_idx: usize,
        max_columns: usize,
        focused_col: usize,
    ) -> Row<'static> {
        use ratatui::text::Text;

        let sort_state = self.table_sort_state.get(section_idx).and_then(|s| *s);

        let header_cells: Vec<Cell> = header
            .cells
            .iter()
            .enumerate()
            .map(|(idx, c)| {
                let sort_indicator =
                    sort_state
                        .filter(|state| state.column == idx)
                        .map_or("", |state| match state.direction {
                            SortDirection::Ascending => "▲",
                            SortDirection::Descending => "▼",
                        });

                let underscore = if (self.focus_state == FocusState::Detail) && idx == focused_col {
                    "_"
                } else {
                    ""
                };

                let text = if sort_indicator.is_empty() && underscore.is_empty() {
                    c.clone()
                } else if sort_indicator.is_empty() {
                    format!("{underscore}\n{c}")
                } else if underscore.is_empty() {
                    format!("{sort_indicator}\n{c}")
                } else {
                    format!("{sort_indicator} {underscore}\n{c}")
                };

                let style = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);
                Cell::from(Text::from(text)).style(style)
            })
            .collect();

        // Pad header to match column count
        let mut all_cells = header_cells;
        while all_cells.len() < max_columns {
            all_cells.push(Cell::from(""));
        }

        Row::new(all_cells).height(3)
    }

    fn get_column_widths(
        &mut self,
        section_idx: usize,
        constraints: &[crate::tree::ColumnConstraint],
    ) -> Vec<u16> {
        // Ensure we have enough entries in column_widths
        while self.column_widths.len() <= section_idx {
            self.column_widths.push(Vec::new());
        }

        // If we don't have custom widths for this section, initialize from constraints
        if self
            .column_widths
            .get(section_idx)
            .is_none_or(Vec::is_empty)
        {
            // First pass: convert to initial widths
            let mut widths: Vec<u16> = constraints
                .iter()
                .map(|c| match c {
                    crate::tree::ColumnConstraint::Fixed(w) => {
                        // Convert fixed width to a reasonable percentage (roughly 1.5% per char)
                        w.saturating_mul(3).saturating_div(2).clamp(3, 15)
                    }
                    crate::tree::ColumnConstraint::Percentage(p) => *p,
                })
                .collect();

            // Normalize to ensure total is exactly 100%
            let total: u16 = widths.iter().sum();
            if total > 0 && total != 100 {
                // Scale all widths proportionally to sum to 100
                let scaled_widths = widths
                    .iter()
                    .map(|&w| {
                        // f64 percentage value always fits in u16 (0..=100)
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        {
                            ((f64::from(w) / f64::from(total)) * 100.0).round() as u16
                        }
                    })
                    .collect();
                widths = scaled_widths;

                // Handle rounding errors: adjust the largest column
                let new_total: u16 = widths.iter().sum();
                if new_total != 100 && !widths.is_empty() {
                    let max_idx = widths
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, w)| *w)
                        .map_or(0, |(idx, _)| idx);
                    if let Some(width) = widths.get_mut(max_idx) {
                        *width = width.saturating_add(u16::saturating_sub(100, new_total));
                    }
                }
            }

            if let Some(col_widths) = self.column_widths.get_mut(section_idx) {
                *col_widths = widths;
            }
        }

        self.column_widths
            .get(section_idx)
            .map_or_else(Vec::new, Clone::clone)
    }

    /// Calculate how many lines are needed to display tabs given available width
    fn calculate_tab_lines(tab_titles: &[String], available_width: usize) -> usize {
        if available_width < 5 || tab_titles.is_empty() {
            return 1;
        }

        let mut lines: usize = 1;
        let mut current_width: usize = 0;

        for title in tab_titles {
            // +3 for " title " padding, +1 for separator
            let tab_width = title.len().saturating_add(3).saturating_add(1);

            if current_width.saturating_add(tab_width) > available_width && current_width > 0 {
                // Need a new line
                lines = lines.saturating_add(1);
                current_width = tab_width;
            } else {
                current_width = current_width.saturating_add(tab_width);
            }
        }

        // Add 1 for the separator line below tabs
        lines.saturating_add(1)
    }

    /// Render tabs with wrapping support for narrow windows
    fn render_wrapped_tabs(
        frame: &mut Frame,
        area: Rect,
        tab_titles: &[String],
        selected: usize,
        _focused: bool,
    ) {
        // No block needed - tabs are rendered directly in the provided area
        // Calculate how to distribute tabs across lines
        let available_width = area.width as usize;
        if available_width < 5 {
            return; // Too narrow to render anything meaningful
        }

        // Build tab strings with decorators: " TabName "
        let tab_strings: Vec<String> = tab_titles
            .iter()
            .map(|title| format!(" {title} "))
            .collect();

        // Calculate positions and line breaks
        let mut lines: Vec<Vec<(usize, &String)>> = Vec::new();
        let mut current_line: Vec<(usize, &String)> = Vec::new();
        let mut current_width: usize = 0;

        for (idx, tab_str) in tab_strings.iter().enumerate() {
            let tab_width = tab_str.len().saturating_add(1); // +1 for separator

            if current_width.saturating_add(tab_width) > available_width && !current_line.is_empty()
            {
                // Start a new line
                lines.push(current_line);
                current_line = Vec::new();
                current_width = 0;
            }

            current_line.push((idx, tab_str));
            current_width = current_width.saturating_add(tab_width);
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Render each line of tabs
        let num_tab_lines = lines.len();
        for (line_idx, line_tabs) in lines.iter().enumerate() {
            if line_idx >= area.height.saturating_sub(1) as usize {
                break; // Reserve space for separator line
            }

            let y = area
                .y
                .saturating_add(u16::try_from(line_idx).unwrap_or(u16::MAX));
            let mut x = area.x;

            for (i, (tab_idx, tab_str)) in line_tabs.iter().enumerate() {
                let is_selected = *tab_idx == selected;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };

                // Add separator before tab (except first on line)
                if i > 0 {
                    let sep_span = Span::styled("│", Style::default().fg(Color::DarkGray));
                    frame.render_widget(
                        Paragraph::new(Line::from(sep_span)),
                        Rect {
                            x,
                            y,
                            width: 1,
                            height: 1,
                        },
                    );
                    x = x.saturating_add(1);
                }

                // Render the tab
                let span = Span::styled(tab_str.as_str(), style);
                let line = Line::from(span);

                // str::len() fits in u16 for any realistic tab label
                #[allow(clippy::cast_possible_truncation)]
                let tab_width = tab_str.len() as u16;
                frame.render_widget(
                    Paragraph::new(line),
                    Rect {
                        x,
                        y,
                        width: tab_width,
                        height: 1,
                    },
                );

                x = x.saturating_add(tab_width);
            }
        }

        // Draw a horizontal separator line below all tabs
        // tab line count is always a small number that fits in u16
        #[allow(clippy::cast_possible_truncation)]
        let num_tab_lines_u16 = num_tab_lines as u16;
        if num_tab_lines > 0 && area.height > num_tab_lines_u16 {
            let separator_y = area.y.saturating_add(num_tab_lines_u16);
            let separator_line = "─".repeat(available_width);
            let sep_style = Style::default().fg(Color::DarkGray);

            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(separator_line, sep_style))),
                Rect {
                    x: area.x,
                    y: separator_y,
                    width: area.width,
                    height: 1,
                },
            );
        }
    }
}

fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total: usize,
    position: usize,
    viewport_height: usize,
) -> Option<Rect> {
    if total <= viewport_height {
        return None;
    }
    let mut state = ScrollbarState::new(total)
        .position(position)
        .viewport_content_length(viewport_height);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        area,
        &mut state,
    );
    // The scrollbar is rendered in the rightmost column of the area
    Some(Rect {
        x: area.x.saturating_add(area.width.saturating_sub(1)),
        y: area.y,
        width: 1,
        height: area.height,
    })
}
