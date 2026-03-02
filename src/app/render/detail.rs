/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

use super::{border_style, table::TableContentParams};
use crate::{
    app::{App, FocusState, TableSortState},
    tree::{DetailContent, DetailSectionData},
};

impl App {
    pub(in crate::app) fn draw_detail(&mut self, frame: &mut Frame, area: Rect) {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style(
                    self.focus_state == FocusState::Detail,
                    &self.theme,
                ))
                .title(" Details ");
            frame.render_widget(block, area);
            return;
        };

        let Some(selected_node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };
        let node_text = selected_node.text.clone();
        let detail_sections = Rc::clone(&selected_node.detail_sections);

        if detail_sections.is_empty() {
            // Draw a default/dummy pane with helpful information
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style(
                    self.focus_state == FocusState::Detail,
                    &self.theme,
                ))
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
                .style(Style::default().fg(self.theme.border_unfocused))
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: false });

            frame.render_widget(paragraph, inner);
        } else {
            self.draw_detail_panes(frame, area, &detail_sections, &node_text);
        }
    }

    pub(super) fn draw_detail_panes(
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
            .border_style(border_style(
                self.focus_state == FocusState::Detail,
                &self.theme,
            ))
            .title(detail_title);
        let outer_inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Clamp selected tab to valid range
        self.detail.selected_tab = self
            .detail
            .selected_tab
            .min(tab_sections.len().saturating_sub(1));

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
            self.render_header_section(frame, area, hdr);
        }

        // Render content area
        let Some(&content_area) = chunks.get(usize::from(header_height.is_some())) else {
            return;
        };

        self.render_content_area(frame, content_area, tab_sections, sections);
    }

    /// Split sections into header and tabs
    pub(super) fn split_header_and_tabs(
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
    pub(super) fn calculate_header_height(
        header: &DetailSectionData,
        outer_inner: Rect,
    ) -> Option<u16> {
        match &header.content {
            DetailContent::PlainText(lines) => {
                let cap = outer_inner.height / 4;
                // clamp min is 0 when the pane is too small to show any header
                let height =
                    u16::try_from(lines.len()).map_or(cap, |h| h.clamp(1u16.min(cap), cap));
                Some(height)
            }
            DetailContent::Table { .. } | DetailContent::Composite(_) => None,
        }
    }

    /// Build layout for detail pane (header + content)
    pub(super) fn build_detail_layout(outer_inner: Rect, header_height: Option<u16>) -> Rc<[Rect]> {
        let mut constraints = vec![];
        if let Some(h) = header_height {
            constraints.push(Constraint::Length(h));
        }
        constraints.push(Constraint::Min(0));

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(outer_inner)
    }

    /// Ensure section state vectors are properly sized
    pub(super) fn ensure_section_state_initialized(&mut self, sections: &[DetailSectionData]) {
        let last_idx = sections.len().saturating_sub(1);
        self.detail.ensure_section_capacity(last_idx);

        // Initialize table_sort_state and column_widths
        while self.table.sort_state.len() < sections.len() {
            let section_idx = self.table.sort_state.len();
            self.table
                .sort_state
                .push(Self::initialize_table_sort(sections.get(section_idx)));
        }

        self.table
            .ensure_column_width_capacity(sections.len().saturating_sub(1));

        self.table
            .ensure_horizontal_scroll_capacity(sections.len().saturating_sub(1));
    }

    /// Initialize table sort state for a section
    pub(super) fn initialize_table_sort(
        section: Option<&DetailSectionData>,
    ) -> Option<TableSortState> {
        let section = section?;
        let header = section.content.table_header()?;

        // Detect Byte and Bit columns for default positional sort
        let byte_col = header
            .cells
            .iter()
            .position(|c| c == "Byte" || c == "Byte Pos");
        let bit_col = header
            .cells
            .iter()
            .position(|c| c == "Bit" || c == "Bit Pos");

        match (byte_col, bit_col) {
            (Some(byte), Some(bit)) => Some(TableSortState {
                column: byte,
                direction: crate::app::SortDirection::Ascending,
                secondary_column: Some(bit),
            }),
            (Some(byte), None) => Some(TableSortState {
                column: byte,
                direction: crate::app::SortDirection::Ascending,
                secondary_column: None,
            }),
            (None, Some(_) | None) => Some(TableSortState {
                column: 0,
                direction: crate::app::SortDirection::Ascending,
                secondary_column: None,
            }),
        }
    }

    /// Render header section
    pub(super) fn render_header_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        header: &DetailSectionData,
    ) {
        if let DetailContent::PlainText(lines) = &header.content {
            let text = lines.join("\n");
            let para = Paragraph::new(text).style(Style::default().fg(self.theme.table_cell));
            frame.render_widget(para, area);
        }
    }

    /// Render the main content area (tabs + content)
    pub(super) fn render_content_area(
        &mut self,
        frame: &mut Frame,
        content_area: Rect,
        tab_sections: &[DetailSectionData],
        all_sections: &[DetailSectionData],
    ) {
        let show_tabs = tab_sections.len() > 1;
        let section_offset = usize::from(all_sections.len() > tab_sections.len());

        let Some(section) = tab_sections.get(self.detail.selected_tab) else {
            return;
        };
        let help_text = if self.focus_state == FocusState::Detail {
            " H/L:tabs  J/K:row  ,/.:col  [/]:resize  </>:scroll  S:sort  C:copy  a-z:jump"
        } else {
            ""
        };

        // Content block with borders
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style(
                self.focus_state == FocusState::Detail,
                &self.theme,
            ))
            .title_bottom(help_text);

        let block_inner = block.inner(content_area);
        frame.render_widget(block, content_area);

        // Render tabs if needed, then content
        let inner = if show_tabs {
            self.render_tabs_and_get_content_area(frame, block_inner, tab_sections)
        } else {
            self.layout.tab_area = None;
            self.layout.tab_titles.clear();
            block_inner
        };

        // Cache table content area
        self.layout.table_content_area = Some(inner);

        // Render section content
        self.render_section_content(
            frame,
            inner,
            section,
            self.detail.selected_tab.saturating_add(section_offset),
        );
    }

    /// Render tabs and return content area
    pub(super) fn render_tabs_and_get_content_area(
        &mut self,
        frame: &mut Frame,
        block_inner: Rect,
        tab_sections: &[DetailSectionData],
    ) -> Rect {
        let tab_titles: Vec<String> = tab_sections.iter().map(|s| s.title.clone()).collect();
        let tab_lines_needed =
            Self::calculate_tab_lines(&tab_titles, usize::from(block_inner.width));
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
        self.layout.tab_area = Some(tab_area);
        self.layout.tab_titles.clone_from(&tab_titles);

        // Render tabs
        self.render_wrapped_tabs(frame, tab_area, &tab_titles, self.detail.selected_tab);

        content_inner
    }

    /// Render section content based on type
    pub(super) fn render_section_content(
        &mut self,
        frame: &mut Frame,
        inner: Rect,
        section: &DetailSectionData,
        section_idx: usize,
    ) {
        match &section.content {
            DetailContent::PlainText(lines) => {
                let text = lines.join("\n");
                let para = Paragraph::new(text).style(Style::default().fg(self.theme.table_cell));
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
                    TableContentParams {
                        inner,
                        header,
                        rows,
                        constraints,
                        section_idx,
                        use_row_selection: *use_row_selection,
                    },
                );
            }
            DetailContent::Composite(subsections) => {
                self.render_composite_content(frame, inner, subsections, section_idx);
            }
        }
    }
}
