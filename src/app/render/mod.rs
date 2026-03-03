/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod composite;
mod detail;
mod popup;
mod table;
mod tabs;
mod tree;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use super::{App, BreadcrumbSegment, FocusState};
use crate::{
    app::config::ResolvedTheme,
    tree::{DiffStatus, NodeType, TreeNode},
};

const BREADCRUMB_SEPARATOR: &str = " > ";
const BREADCRUMB_SEPARATOR_LEN: u16 = 3;

// -----------------------------------------------------------------------
// Colour theme helpers (use the configurable ResolvedTheme)
// -----------------------------------------------------------------------

fn node_style(node: &TreeNode, theme: &ResolvedTheme) -> Style {
    // Diff status takes priority over node type styling
    if let Some(status) = node.diff_status {
        return match status {
            DiffStatus::Added => Style::default().fg(theme.diff_added),
            DiffStatus::Removed => Style::default()
                .fg(theme.diff_removed)
                .add_modifier(Modifier::CROSSED_OUT),
            DiffStatus::Modified => Style::default().fg(theme.diff_modified),
            DiffStatus::Unchanged => Style::default().fg(theme.diff_unchanged),
        };
    }

    // Browse mode: style by node type
    match node.node_type {
        NodeType::Container => styled(theme.tree_container, true),
        NodeType::SectionHeader | NodeType::ParentRefs | NodeType::Dop => {
            styled(theme.tree_section_header, true)
        }
        // Gray for inherited services
        NodeType::ParentRefService => Style::default().fg(theme.tree_inherited_service),
        NodeType::Service
        | NodeType::Request
        | NodeType::PosResponse
        | NodeType::NegResponse
        | NodeType::FunctionalClass
        | NodeType::Job
        | NodeType::Sdg
        | NodeType::Default => Style::default().fg(theme.tree_default_node),
    }
}

fn styled(fg: Color, bold: bool) -> Style {
    let s = Style::default().fg(fg);
    if bold {
        s.add_modifier(Modifier::BOLD)
    } else {
        s
    }
}

fn border_style(focused: bool, theme: &ResolvedTheme) -> Style {
    Style::default().fg(if focused {
        theme.border_focused
    } else {
        theme.border_unfocused
    })
}

fn row_style(node: &TreeNode, is_cursor: bool, theme: &ResolvedTheme) -> Style {
    if is_cursor {
        Style::default().bg(theme.cursor_bg).fg(theme.cursor_fg)
    } else {
        node_style(node, theme)
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
// Breadcrumb and status bar
// -----------------------------------------------------------------------

impl App {
    /// Build breadcrumb path for the currently selected node, including pixel-column
    /// positions for mouse hit-testing. `area_x` is the left edge of the breadcrumb bar.
    fn build_breadcrumb_segments(&self, area_x: u16) -> Vec<BreadcrumbSegment> {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return Vec::new();
        };
        let mut path_segments = Vec::new();
        let mut current_idx = node_idx;

        while let Some(node) = self.tree.all_nodes.get(current_idx) {
            path_segments.push((node.text.clone(), current_idx));

            let current_depth = node.depth;
            if current_depth == 0 {
                break;
            }

            let parent_idx = (0..current_idx).rev().find(|&i| {
                self.tree
                    .all_nodes
                    .get(i)
                    .is_some_and(|n| n.depth < current_depth)
            });

            let Some(idx) = parent_idx else {
                break;
            };
            current_idx = idx;
        }

        path_segments.reverse();

        let mut col_position = area_x;
        let last = path_segments.len().saturating_sub(1);
        path_segments
            .into_iter()
            .enumerate()
            .map(|(i, (text, node_idx))| {
                let start_col = col_position;
                let text_len = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
                let end_col = start_col.saturating_add(text_len);
                col_position = end_col;
                if i < last {
                    col_position = col_position.saturating_add(BREADCRUMB_SEPARATOR_LEN);
                }
                BreadcrumbSegment {
                    text,
                    node_idx,
                    start_col,
                    end_col,
                }
            })
            .collect()
    }

    pub(super) fn draw_breadcrumb(&mut self, frame: &mut Frame, area: Rect) {
        let cursor = self.tree.cursor;
        if self.layout.breadcrumb_cursor != Some(cursor) || self.layout.breadcrumb_area_x != area.x
        {
            self.layout.breadcrumb_segments = self.build_breadcrumb_segments(area.x);
            self.layout.breadcrumb_cursor = Some(cursor);
            self.layout.breadcrumb_area_x = area.x;
        }

        // Build display string from the (possibly cached) segments
        let breadcrumb_text: String = self
            .layout
            .breadcrumb_segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(BREADCRUMB_SEPARATOR);

        let paragraph = Paragraph::new(breadcrumb_text).style(
            Style::default()
                .fg(self.theme.breadcrumb_fg)
                .bg(self.theme.breadcrumb_bg),
        );
        frame.render_widget(paragraph, area);
    }

    pub(super) fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let (text, st) = if self.search.active {
            let current_search_info = if self.search.stack.is_empty() {
                String::new()
            } else {
                let stack_display: Vec<String> =
                    self.search.stack.iter().map(|e| e.query.clone()).collect();
                format!(" [active: {}]", stack_display.join(" → "))
            };

            (
                format!(
                    " /{}█{}{}  (scope: {} | Shift+S to change (leave search first) | Enter to \
                     add, Esc to cancel |  Backspace to undo last search)",
                    self.search.query,
                    self.search.scope.search_indicator(),
                    current_search_info,
                    self.search.scope
                ),
                Style::default()
                    .fg(self.theme.table_header)
                    .bg(self.theme.cursor_bg),
            )
        } else if !self.status.is_empty() {
            (
                format!(" {}", self.status),
                Style::default().fg(self.theme.status_fg),
            )
        } else {
            let focus = if self.focus_state == FocusState::Detail {
                "detail"
            } else {
                "tree"
            };

            let search_info = if self.search.stack.is_empty() {
                String::new()
            } else {
                let stack_display: Vec<String> = self
                    .search
                    .stack
                    .iter()
                    .map(|e| format!("{}{}", e.query, e.scope.abbrev()))
                    .collect();
                let joined = stack_display.join(" → ");
                format!(" | searches: {joined}")
            };

            (
                format!(
                    " {}/{} nodes | cursor: {} | focus: {focus}{}{}",
                    self.tree.visible.len(),
                    self.tree.all_nodes.len(),
                    self.tree.cursor.saturating_add(1),
                    self.search.scope.status_indicator(),
                    search_info,
                ),
                Style::default().fg(self.theme.status_fg),
            )
        };
        // Truncate to terminal width. char_indices().nth(n) short-circuits at the
        // n-th scalar boundary, avoiding a full O(len) scan on the common (fits) path.
        let display_width = usize::from(area.width);
        let text = match text.char_indices().nth(display_width) {
            Some((byte_idx, _)) => text[..byte_idx].to_string(),
            None => text,
        };
        frame.render_widget(Paragraph::new(text).style(st), area);
    }
}

// -----------------------------------------------------------------------
// Scrollbar helpers
// -----------------------------------------------------------------------

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

fn render_horizontal_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total_width: u16,
    scroll_position: u16,
    viewport_width: u16,
) {
    if total_width <= viewport_width {
        return;
    }
    let mut state = ScrollbarState::new(usize::from(total_width))
        .position(usize::from(scroll_position))
        .viewport_content_length(usize::from(viewport_width));
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .thumb_symbol("━")
            .track_symbol(Some("─"))
            .begin_symbol(Some("◂"))
            .end_symbol(Some("▸")),
        area,
        &mut state,
    );
}
