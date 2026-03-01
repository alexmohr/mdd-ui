/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::render_scrollbar;
use crate::{
    app::{App, COLUMN_SPACING},
    tree::{DetailContent, DetailRow, DetailSectionData},
};

/// Shared table fields used by both `TitledTable` and `Table` composite blocks.
/// Extracting them into a struct means adding a new field (e.g., `use_row_selection`)
/// only requires one change instead of two.
pub(super) struct TableBlock<'a> {
    pub header: &'a DetailRow,
    pub rows: &'a [DetailRow],
    pub constraints: &'a [crate::tree::ColumnConstraint],
}

/// A visual block inside a Composite section — either a titled table
/// (`PlainText` label merged with following `Table`) or a standalone element.
pub(super) enum CompositeBlock<'a> {
    TitledTable {
        title: String,
        table: TableBlock<'a>,
    },
    Table(TableBlock<'a>),
    PlainText {
        lines: &'a [String],
    },
}

/// Group raw subsections into renderable blocks.
/// Consecutive `PlainText` + `Table` pairs become a single `TitledTable` block.
pub(super) fn build_composite_blocks(subsections: &[DetailSectionData]) -> Vec<CompositeBlock<'_>> {
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < subsections.len() {
        let Some(sub) = subsections.get(i) else {
            break;
        };
        match &sub.content {
            DetailContent::PlainText(lines) => {
                // Peek ahead: if next subsection is a Table, merge into TitledTable
                if let Some(next) = subsections.get(i.saturating_add(1))
                    && let DetailContent::Table {
                        header,
                        rows,
                        constraints,
                        ..
                    } = &next.content
                {
                    blocks.push(CompositeBlock::TitledTable {
                        title: lines.join(" "),
                        table: TableBlock {
                            header,
                            rows,
                            constraints,
                        },
                    });
                    i = i.saturating_add(2);
                    continue;
                }
                // Standalone plain text
                blocks.push(CompositeBlock::PlainText { lines });
            }
            DetailContent::Table {
                header,
                rows,
                constraints,
                ..
            } => {
                blocks.push(CompositeBlock::Table(TableBlock {
                    header,
                    rows,
                    constraints,
                }));
            }
            DetailContent::Composite(_) => {
                // Nested Composite is not produced by the builder; this arm is
                // unreachable in correct usage.
                unreachable!("nested Composite not supported");
            }
        }
        i = i.saturating_add(1);
    }
    blocks
}

impl App {
    pub(super) fn render_composite_content(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        subsections: &[crate::tree::DetailSectionData],
        section_idx: usize,
    ) {
        if subsections.is_empty() {
            return;
        }

        // Group subsections into visual blocks.
        // A PlainText immediately followed by a Table becomes a titled block;
        // standalone PlainText or standalone Tables are individual blocks.
        let blocks = build_composite_blocks(subsections);
        if blocks.is_empty() {
            return;
        }

        // Calculate each block's natural height (border + content)
        let block_heights: Vec<u16> = blocks
            .iter()
            .map(|b| {
                let content_h: u16 = match b {
                    CompositeBlock::TitledTable { table, .. } | CompositeBlock::Table(table) => {
                        // table header (3) + data rows
                        let row_h = u16::try_from(table.rows.len()).unwrap_or(u16::MAX);
                        3u16.saturating_add(row_h)
                    }
                    CompositeBlock::PlainText { lines } => {
                        u16::try_from(lines.len().max(1)).unwrap_or(1)
                    }
                };
                // +2 for border top/bottom
                content_h.saturating_add(2)
            })
            .collect();

        let block_count = blocks.len();
        let needs_scroll = block_count > 1;

        // Reserve 1 column for scrollbar when there are multiple blocks
        let content_width = if needs_scroll {
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        // Clamp scroll offset (block index of first visible block)
        self.detail.ensure_composite_capacity(section_idx);
        let max_first_block = block_count.saturating_sub(1);
        self.detail.composite_max_scroll = max_first_block;
        if let Some(scroll) = self.detail.composite_scroll.get_mut(section_idx) {
            *scroll = (*scroll).min(max_first_block);
        }
        let first_block = self
            .detail
            .composite_scroll
            .get(section_idx)
            .copied()
            .unwrap_or(0);

        // Render blocks starting from first_block, stacking downward
        let spacing = 1u16;
        let mut y = area.y;
        let viewport_bottom = area.y.saturating_add(area.height);

        for (i, block) in blocks.iter().enumerate().skip(first_block) {
            let remaining = viewport_bottom.saturating_sub(y);
            if remaining == 0 {
                break;
            }

            let Some(&natural_h) = block_heights.get(i) else {
                continue;
            };

            // Minimum height to render a block usefully
            let min_h: u16 = match block {
                CompositeBlock::TitledTable { .. } | CompositeBlock::Table(_) => 5,
                CompositeBlock::PlainText { .. } => 3,
            };
            if remaining < min_h {
                break;
            }

            // Render at most the natural height, or whatever remains
            let render_h = natural_h.min(remaining);
            let block_rect = Rect {
                x: area.x,
                y,
                width: content_width,
                height: render_h,
            };

            self.render_composite_block(frame, block_rect, block);

            y = y
                .saturating_add(render_h)
                .saturating_add(spacing)
                .min(viewport_bottom);
        }

        // Render scrollbar when there are blocks beyond the viewport
        if needs_scroll {
            let scrollbar_area = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height,
            };
            self.layout.detail_scrollbar_area =
                render_scrollbar(frame, scrollbar_area, block_count, first_block, 1);
        }
    }

    /// Render a single composite block (bordered box with static table inside).
    pub(super) fn render_composite_block(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        block: &CompositeBlock<'_>,
    ) {
        match block {
            CompositeBlock::TitledTable { title, table } => {
                let border = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.tab_inactive_fg))
                    .title(title.as_str());
                let inner = border.inner(area);
                frame.render_widget(border, area);
                if inner.height > 0 && inner.width > 0 {
                    self.render_static_table(
                        frame,
                        inner,
                        table.header,
                        table.rows,
                        table.constraints,
                    );
                }
            }
            CompositeBlock::Table(table) => {
                let border = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.tab_inactive_fg));
                let inner = border.inner(area);
                frame.render_widget(border, area);
                if inner.height > 0 && inner.width > 0 {
                    self.render_static_table(
                        frame,
                        inner,
                        table.header,
                        table.rows,
                        table.constraints,
                    );
                }
            }
            CompositeBlock::PlainText { lines } => {
                let border = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.tab_inactive_fg));
                let inner = border.inner(area);
                frame.render_widget(border, area);
                let text = lines.join("\n");
                let para = Paragraph::new(text).style(Style::default().fg(self.theme.table_cell));
                frame.render_widget(para, inner);
            }
        }
    }

    /// Render a simple static table (no cursor, no scrollbar, no per-table scroll).
    /// Used inside composite blocks where the composite section handles scrolling.
    pub(super) fn render_static_table(
        &self,
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

        // Build header
        let header_cells: Vec<Cell> = header
            .cells
            .iter()
            .map(|c| {
                Cell::from(c.clone()).style(
                    Style::default()
                        .fg(self.theme.table_header)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();
        let header_row = Row::new(header_cells).height(3);

        // Build data rows
        let data_rows: Vec<Row> = rows
            .iter()
            .map(|r| {
                let cells: Vec<Cell> = (0..max_columns)
                    .map(|col| {
                        let text = r.cells.get(col).map_or("", String::as_str);
                        Cell::from(text.to_owned())
                            .style(Style::default().fg(self.theme.table_cell))
                    })
                    .collect();
                Row::new(cells)
            })
            .collect();

        // Build column constraints
        let ratatui_constraints: Vec<Constraint> = constraints
            .iter()
            .map(|c| match c {
                crate::tree::ColumnConstraint::Percentage(p) => Constraint::Percentage(*p),
                crate::tree::ColumnConstraint::Fixed(a) => Constraint::Length(*a),
            })
            .collect();

        let table = Table::new(data_rows, ratatui_constraints)
            .column_spacing(COLUMN_SPACING)
            .header(header_row);
        frame.render_widget(table, area);
    }
}
