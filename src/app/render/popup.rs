/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::app::App;

/// Calculate a centered popup rectangle as a percentage of the given area.
fn centered_popup(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let w = area.width.saturating_mul(width_pct) / 100;
    let h = area.height.saturating_mul(height_pct) / 100;
    Rect {
        x: (area.width.saturating_sub(w)) / 2,
        y: (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

impl App {
    pub(in crate::app) fn draw_help_popup(&self, frame: &mut Frame) {
        let popup_rect = centered_popup(frame.area(), 70, 80);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_rect);

        // Draw the popup block
        let block = Block::default()
            .title(" Help - Press ? or Esc to close ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.help_border));

        let inner_area = block.inner(popup_rect);
        frame.render_widget(block, popup_rect);

        // Help content
        let mut help_text = vec![
            "NAVIGATION",
            "  ↑/↓ or k/j      Move cursor up/down",
            "  ←/→ or h/l      Collapse/expand node (or navigate tabs in detail)",
            "  PgUp/PgDn       Page up/down",
            "  Home/End        Jump to first/last",
            "  Space           Toggle expand/collapse current node",
            "  Tab             Switch focus between tree and detail pane",
            "  Backspace       Jump to last element in navigation history",
            "",
            "TREE OPERATIONS",
            "  e               Expand all nodes",
            "  c               Collapse all nodes",
            "  s               Toggle sort (by ID/name for services, by name for others)",
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
            "  Shift+C            Copy table to clipboard as markdown",
            "  [ / ]              Decrease/increase column width",
            "  , / .              Select previous/next column",
            "  < / >              Scroll table left/right",
            "  a-z, 0-9           Type-to-jump to matching row (resets after 1s)",
            "",
            "TYPE-TO-JUMP (tree)",
            "  a-z, 0-9           Jump to tree node matching typed text",
            "",
            "WINDOW",
            "  + / -           Increase/decrease tree pane width",
            "  Mouse drag      Drag the divider between tree and detail to resize",
            "  m               Toggle mouse mode (enable/disable terminal text selection)",
            "  ?               Show this help",
            "  Q or Esc        Quit application",
        ];

        if self.is_diff_mode {
            help_text.extend_from_slice(&[
                "",
                "DIFF MODE",
                "  u               Toggle show/hide unchanged nodes",
                "  n / N           Jump to next/previous change (when no search active)",
            ]);
        }

        let help_paragraph = Paragraph::new(help_text.join("\n"))
            .style(Style::default().fg(self.theme.help_text))
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        frame.render_widget(help_paragraph, inner_area);
    }

    pub(in crate::app) fn draw_detail_popup(&self, frame: &mut Frame) {
        let Some(popup_data) = &self.detail.popup else {
            return;
        };

        let popup_rect = centered_popup(frame.area(), 60, 50);

        // Clear the area first
        frame.render_widget(Clear, popup_rect);

        // Create the popup block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.detail_border))
            .title(format!(" {} ", popup_data.title))
            .title_alignment(Alignment::Center)
            .title_bottom(" Press Esc to close ")
            .style(Style::default().bg(self.theme.detail_bg));

        let inner = block.inner(popup_rect);
        frame.render_widget(block, popup_rect);

        // Render the content
        let content_text = popup_data.content.join("\n");
        let paragraph = Paragraph::new(content_text)
            .style(Style::default().fg(self.theme.detail_text))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, inner);
    }
}
