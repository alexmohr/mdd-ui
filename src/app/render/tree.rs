/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::{border_style, expand_icon, render_scrollbar, row_style};
use crate::{
    app::{App, FocusState},
    tree::DiffStatus,
};

impl App {
    pub(in crate::app) fn draw_tree(&mut self, frame: &mut Frame, area: Rect) {
        let ecu_name = self.ecu_name.as_str();

        let tree_block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style(
                self.focus_state != FocusState::Detail,
                &self.theme,
            ))
            .title(format!(" {ecu_name} "));

        let tree_inner = tree_block.inner(area);
        frame.render_widget(tree_block, area);

        // Draw tree content
        let viewport_height = usize::from(tree_inner.height);
        self.ensure_cursor_visible(viewport_height);

        let lines: Vec<Line> = self
            .tree
            .visible
            .iter()
            .enumerate()
            .skip(self.tree.scroll_offset)
            .take(viewport_height)
            .filter_map(|(vi, &node_idx)| {
                let node = self.tree.all_nodes.get(node_idx)?;
                let row_style = row_style(node, vi == self.tree.cursor, &self.theme);

                let indent = "  ".repeat(node.depth);
                let icon = expand_icon(node);
                let diff_prefix = match node.diff_status {
                    Some(DiffStatus::Added) => "[+] ",
                    Some(DiffStatus::Removed) => "[-] ",
                    Some(DiffStatus::Modified) => "[~] ",
                    Some(DiffStatus::Unchanged) | None => "",
                };

                Some(Line::from(vec![
                    Span::styled(indent, row_style),
                    Span::styled(icon, row_style),
                    Span::styled(diff_prefix, row_style),
                    Span::styled(&node.text, row_style),
                ]))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), tree_inner);
        self.layout.tree_scrollbar_area = render_scrollbar(
            frame,
            area,
            self.tree.visible.len(),
            self.tree.cursor,
            viewport_height,
        );
    }
}
