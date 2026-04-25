/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;

/// Build decorated tab strings from titles: `" Title "`.
pub(in crate::app) fn build_tab_strings(tab_titles: &[String]) -> Vec<String> {
    tab_titles
        .iter()
        .map(|title| format!(" {title} "))
        .collect()
}

/// Compute the line-grouped tab layout for the given available width.
/// Returns a `Vec` of lines, where each line is a `Vec` of tab indices.
pub(in crate::app) fn compute_tab_lines(
    tab_strings: &[String],
    available_width: usize,
) -> Vec<Vec<usize>> {
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current_line: Vec<usize> = Vec::new();
    let mut current_width: usize = 0;

    for (idx, tab_str) in tab_strings.iter().enumerate() {
        let tab_width = tab_str.len().saturating_add(1); // +1 for separator

        if current_width.saturating_add(tab_width) > available_width && !current_line.is_empty() {
            lines.push(std::mem::take(&mut current_line));
            current_width = 0;
        }

        current_line.push(idx);
        current_width = current_width.saturating_add(tab_width);
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

impl App {
    /// Calculate how many lines are needed to display tabs given available width
    pub(super) fn calculate_tab_lines(tab_titles: &[String], available_width: usize) -> usize {
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
    pub(super) fn render_wrapped_tabs(
        &self,
        frame: &mut Frame,
        area: Rect,
        tab_titles: &[String],
        selected: usize,
    ) {
        // No block needed - tabs are rendered directly in the provided area
        // Calculate how to distribute tabs across lines
        let available_width = usize::from(area.width);
        if available_width < 5 {
            return; // Too narrow to render anything meaningful
        }

        let tab_strings = build_tab_strings(tab_titles);
        let lines = compute_tab_lines(&tab_strings, available_width);

        // Render each line of tabs
        let num_tab_lines = lines.len();
        for (line_idx, line_tabs) in lines.iter().enumerate() {
            if line_idx >= usize::from(area.height.saturating_sub(1)) {
                break; // Reserve space for separator line
            }

            let y = area
                .y
                .saturating_add(u16::try_from(line_idx).unwrap_or(u16::MAX));
            let mut x = area.x;

            for (i, &tab_idx) in line_tabs.iter().enumerate() {
                let Some(tab_str) = tab_strings.get(tab_idx) else {
                    continue;
                };
                let tab_str = tab_str.as_str();
                let is_selected = tab_idx == selected;
                let style = if is_selected {
                    Style::default()
                        .fg(self.theme.tab_active_fg)
                        .bg(self.theme.tab_active_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(self.theme.tab_inactive_fg)
                        .bg(self.theme.tab_inactive_bg)
                };

                // Add separator before tab (except first on line)
                if i > 0 {
                    let sep_span = Span::styled("│", Style::default().fg(self.theme.separator));
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
                let span = Span::styled(tab_str, style);
                let line = Line::from(span);

                let tab_width = u16::try_from(tab_str.len()).unwrap_or(u16::MAX);
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

        let num_tab_lines_u16 = u16::try_from(num_tab_lines).unwrap_or(u16::MAX);
        if num_tab_lines > 0 && area.height > num_tab_lines_u16 {
            let separator_y = area.y.saturating_add(num_tab_lines_u16);
            let separator_line = "─".repeat(available_width);
            let sep_style = Style::default().fg(self.theme.separator);

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
