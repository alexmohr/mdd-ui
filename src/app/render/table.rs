/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::borrow::Cow;

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Text,
    widgets::{Cell, Row, Table},
};

use super::{render_horizontal_scrollbar, render_scrollbar};
use crate::{
    app::{App, COLUMN_SPACING, FocusState, SortDirection, TableSortState},
    tree::{CellType, DetailRow},
};

/// Describes how a particular cell should be highlighted.
#[derive(Clone, Copy)]
pub(super) enum CellHighlight {
    /// The cell is in the focused column of the selected row.
    FocusedCell,
    /// The cell is in the selected row (row selection mode).
    SelectedRow,
    /// Default state — not selected.
    Normal,
}

/// Parameters for rendering table content.
#[derive(Clone, Copy)]
pub(super) struct TableContentParams<'a> {
    pub inner: Rect,
    pub header: &'a DetailRow,
    pub rows: &'a [DetailRow],
    pub constraints: &'a [crate::tree::ColumnConstraint],
    pub section_idx: usize,
    pub use_row_selection: bool,
}

/// Parameters for rendering a horizontally scrolled table with scrollbar.
#[derive(Clone, Copy)]
struct HScrollTableParams<'a> {
    inner: Rect,
    table_area: Rect,
    header: &'a DetailRow,
    visible_rows: &'a [Row<'static>],
    column_widths: &'a [u16],
    column_spacing: u16,
    h_scroll: u16,
    total_table_width: u16,
    section_idx: usize,
}

/// Parameters for applying horizontal scroll to determine visible columns.
#[derive(Clone, Copy)]
struct HScrollParams<'a> {
    column_widths: &'a [u16],
    column_spacing: u16,
    h_scroll: u16,
    viewport_width: u16,
    header: &'a DetailRow,
    visible_rows: &'a [Row<'static>],
    sort_state: Option<TableSortState>,
}

impl App {
    pub(crate) fn sort_rows<'a>(
        &self,
        rows: &'a [DetailRow],
        section_idx: usize,
    ) -> Cow<'a, [DetailRow]> {
        let Some(sort_state) = self
            .table
            .sort_state
            .get(section_idx)
            .and_then(|s| s.as_ref())
        else {
            return Cow::Borrowed(rows);
        };

        let mut sorted = rows.to_vec();
        let col = sort_state.column;
        let dir = sort_state.direction;
        let secondary = sort_state.secondary_column;
        sorted.sort_by(|a, b| {
            let cmp = Self::compare_cells(a, b, col);
            let cmp = match cmp {
                std::cmp::Ordering::Equal => secondary.map_or(std::cmp::Ordering::Equal, |sec| {
                    Self::compare_cells(a, b, sec)
                }),
                other => other,
            };

            match dir {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
        Cow::Owned(sorted)
    }

    pub(crate) fn compare_cells(a: &DetailRow, b: &DetailRow, col: usize) -> std::cmp::Ordering {
        let a_cell = a.cells.get(col).map_or("", String::as_str);
        let b_cell = b.cells.get(col).map_or("", String::as_str);

        // Try integer comparison first to avoid f64 precision loss for large integers
        // (e.g. CAN IDs, large hex addresses that exceed 2^53).
        if let (Ok(ai), Ok(bi)) = (a_cell.parse::<i64>(), b_cell.parse::<i64>()) {
            return ai.cmp(&bi);
        }
        match (a_cell.parse::<f64>(), b_cell.parse::<f64>()) {
            (Ok(a_num), Ok(b_num)) => a_num.total_cmp(&b_num),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a_cell.cmp(b_cell),
        }
    }

    pub(super) fn clamp_section_cursor_and_scroll(
        &mut self,
        section_idx: usize,
        row_count: usize,
        viewport_height: usize,
    ) {
        let Some(cursor) = self.detail.section_cursors.get_mut(section_idx) else {
            return;
        };
        let Some(scroll) = self.detail.section_scrolls.get_mut(section_idx) else {
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

    pub(super) fn render_table_content(
        &mut self,
        frame: &mut Frame,
        TableContentParams {
            inner,
            header,
            rows,
            constraints,
            section_idx,
            use_row_selection,
        }: TableContentParams<'_>,
    ) {
        // Account for header height (3 lines) when calculating viewport
        let header_height = 3u16;
        let viewport_height = usize::from((inner.height.saturating_sub(header_height)).max(1));

        // Apply sorting based on table_sort_state if set
        let sorted_rows = self.sort_rows(rows, section_idx);

        let max_columns = sorted_rows
            .iter()
            .map(|r| r.cells.len())
            .max()
            .unwrap_or(header.cells.len());

        let rows_refs: Vec<&DetailRow> = sorted_rows.iter().collect();

        let row_count = rows_refs.len();
        self.clamp_section_cursor_and_scroll(section_idx, row_count, viewport_height);

        let focused_col = if self.table.focused_column >= max_columns {
            max_columns.saturating_sub(1)
        } else {
            self.table.focused_column
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

        let is_absolute = self
            .table
            .column_widths_absolute
            .get(section_idx)
            .copied()
            .unwrap_or(false);

        // Calculate total table width and determine if horizontal scrolling is needed
        let column_spacing = COLUMN_SPACING;
        let num_gaps = max_columns.saturating_sub(1);
        let total_table_width: u16 = if is_absolute {
            let cols_total: u16 = column_widths.iter().sum();
            let gaps = u16::try_from(num_gaps).unwrap_or(0);
            cols_total.saturating_add(column_spacing.saturating_mul(gaps))
        } else {
            inner.width
        };

        let needs_hscroll = is_absolute && total_table_width > inner.width;

        // Reserve bottom row for horizontal scrollbar if needed
        let table_area = if needs_hscroll {
            Rect {
                height: inner.height.saturating_sub(1),
                ..inner
            }
        } else {
            inner
        };

        // Determine horizontal scroll offset
        self.table.ensure_horizontal_scroll_capacity(section_idx);
        let h_scroll = self
            .table
            .horizontal_scroll
            .get(section_idx)
            .copied()
            .unwrap_or(0);

        // Clamp horizontal scroll
        let max_h_scroll = total_table_width.saturating_sub(inner.width);
        if let Some(hs) = self.table.horizontal_scroll.get_mut(section_idx) {
            *hs = h_scroll.min(max_h_scroll);
        }
        let h_scroll = self
            .table
            .horizontal_scroll
            .get(section_idx)
            .copied()
            .unwrap_or(0);

        self.table.cached_total_table_width = total_table_width;

        if needs_hscroll {
            self.render_hscrolled_table(
                frame,
                HScrollTableParams {
                    inner,
                    table_area,
                    header,
                    visible_rows: &visible_rows,
                    column_widths: &column_widths,
                    column_spacing,
                    h_scroll,
                    total_table_width,
                    section_idx,
                },
            );
        } else {
            self.layout.detail_hscrollbar_area = None;
            // Standard rendering (no horizontal scroll needed)
            let ratatui_constraints: Vec<Constraint> = column_widths
                .iter()
                .map(|&w| {
                    if is_absolute {
                        Constraint::Length(w)
                    } else {
                        Constraint::Percentage(w)
                    }
                })
                .collect();

            self.table
                .cached_ratatui_constraints
                .clone_from(&ratatui_constraints);

            let header_row = self.build_header_row(header, section_idx, max_columns);

            let table = Table::new(visible_rows, ratatui_constraints)
                .column_spacing(column_spacing)
                .header(header_row);
            frame.render_widget(table, table_area);
        }

        // Vertical scrollbar
        let vscroll_height = if needs_hscroll {
            table_area.height
        } else {
            inner.height
        };
        if row_count > viewport_height {
            let scrollbar_area = Rect {
                x: inner.x,
                y: inner.y.saturating_add(header_height),
                width: inner.width,
                height: vscroll_height.saturating_sub(header_height),
            };
            self.layout.detail_scrollbar_area = render_scrollbar(
                frame,
                scrollbar_area,
                row_count,
                *self.detail.section_cursors.get(section_idx).unwrap_or(&0),
                viewport_height,
            );
        } else {
            self.layout.detail_scrollbar_area = None;
        }
    }

    /// Render table with horizontal scrolling and scrollbar.
    fn render_hscrolled_table(
        &mut self,
        frame: &mut Frame,
        HScrollTableParams {
            inner,
            table_area,
            header,
            visible_rows,
            column_widths,
            column_spacing,
            h_scroll,
            total_table_width,
            section_idx,
        }: HScrollTableParams<'_>,
    ) {
        let (vis_constraints, vis_header, vis_rows) = self.apply_horizontal_scroll(HScrollParams {
            column_widths,
            column_spacing,
            h_scroll,
            viewport_width: table_area.width,
            header,
            visible_rows,
            sort_state: self.table.sort_state.get(section_idx).and_then(|s| *s),
        });

        self.table
            .cached_ratatui_constraints
            .clone_from(&vis_constraints);

        let table = Table::new(vis_rows, vis_constraints)
            .column_spacing(column_spacing)
            .header(vis_header);
        frame.render_widget(table, table_area);

        let hscroll_area = Rect {
            x: inner.x,
            y: inner.y.saturating_add(inner.height.saturating_sub(1)),
            width: inner.width,
            height: 1,
        };
        render_horizontal_scrollbar(
            frame,
            hscroll_area,
            total_table_width,
            h_scroll,
            inner.width,
        );
        self.layout.detail_hscrollbar_area = Some(hscroll_area);
    }

    /// Apply horizontal scroll to determine visible columns and build clipped constraints/rows.
    fn apply_horizontal_scroll(
        &self,
        HScrollParams {
            column_widths,
            column_spacing,
            h_scroll,
            viewport_width,
            header,
            visible_rows,
            sort_state,
        }: HScrollParams<'_>,
    ) -> (Vec<Constraint>, Row<'static>, Vec<Row<'static>>) {
        // Calculate cumulative positions: (start_px, end_px) for each column
        let mut col_positions: Vec<(u16, u16)> = Vec::with_capacity(column_widths.len());
        let mut x = 0u16;
        for (i, &w) in column_widths.iter().enumerate() {
            col_positions.push((x, x.saturating_add(w)));
            if i < column_widths.len().saturating_sub(1) {
                x = x.saturating_add(w).saturating_add(column_spacing);
            } else {
                x = x.saturating_add(w);
            }
        }

        let scroll_end = h_scroll.saturating_add(viewport_width);

        // Find columns that overlap with the visible window [h_scroll, scroll_end)
        let (vis_col_indices, vis_widths): (Vec<usize>, Vec<u16>) = col_positions
            .iter()
            .enumerate()
            .filter_map(|(i, &(start, end))| {
                if end <= h_scroll || start >= scroll_end {
                    return None;
                }
                let vis_start = start.max(h_scroll);
                let vis_end = end.min(scroll_end);
                let vis_width = vis_end.saturating_sub(vis_start);
                (vis_width > 0).then_some((i, vis_width))
            })
            .unzip();

        // Build constraints
        let constraints: Vec<Constraint> =
            vis_widths.iter().map(|&w| Constraint::Length(w)).collect();

        // Build header
        let header_row = self.build_header_row_impl(header, &vis_col_indices, sort_state);

        // Build data rows by extracting visible columns
        let data_rows: Vec<Row<'static>> = visible_rows.to_vec();

        // For data rows, we need to rebuild them with only visible columns.
        // Since ratatui Row doesn't let us extract cells after construction,
        // we return the original rows and let the Constraint::Length handle clipping.
        // Columns outside the viewport simply won't have space allocated.

        (constraints, header_row, data_rows)
    }

    /// Build a header row showing only the columns in `col_indices`.
    /// Shared implementation used by both the scrolled and non-scrolled paths.
    fn build_header_row_impl(
        &self,
        header: &DetailRow,
        col_indices: &[usize],
        sort_state: Option<TableSortState>,
    ) -> Row<'static> {
        let header_cells: Vec<Cell> = col_indices
            .iter()
            .map(|&idx| {
                let c = header.cells.get(idx).cloned().unwrap_or_default();

                let sort_indicator =
                    sort_state
                        .filter(|state| state.column == idx)
                        .map_or("", |state| match state.direction {
                            SortDirection::Ascending => "▲",
                            SortDirection::Descending => "▼",
                        });

                let text = if sort_indicator.is_empty() {
                    format!("\n{c}")
                } else {
                    format!("{sort_indicator}\n{c}")
                };

                let style = Style::default()
                    .fg(self.theme.table_header)
                    .add_modifier(Modifier::BOLD);
                Cell::from(Text::from(text)).style(style)
            })
            .collect();

        Row::new(header_cells).height(3)
    }

    pub(super) fn build_visible_rows(
        &self,
        rows_refs: &[&DetailRow],
        section_idx: usize,
        viewport_height: usize,
        max_columns: usize,
        focused_col: usize,
        use_row_selection: bool,
    ) -> Vec<Row<'static>> {
        let scroll_offset = self
            .detail
            .section_scrolls
            .get(section_idx)
            .copied()
            .unwrap_or(0);
        let cursor_pos = self
            .detail
            .section_cursors
            .get(section_idx)
            .copied()
            .unwrap_or(0);

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

                let is_child_element =
                    matches!(row_data.row_type, crate::tree::DetailRowType::ChildElement);
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

                        let has_jump = row_data
                            .cell_jump_targets
                            .get(col_idx)
                            .is_some_and(Option::is_some)
                            || is_child_element;

                        let highlight = if is_selected_row && col_idx == focused_col {
                            CellHighlight::FocusedCell
                        } else if is_selected_row && use_row_selection {
                            CellHighlight::SelectedRow
                        } else {
                            CellHighlight::Normal
                        };
                        let style = self.cell_style(highlight, cell_type, has_jump);

                        Cell::from(text).style(style)
                    })
                    .collect();

                while cells.len() < max_columns {
                    cells.push(Cell::from(""));
                }
                let row = Row::new(cells);
                // Apply background to the entire row (including column gaps)
                // so the selected row has a uniform highlight
                if is_selected_row && use_row_selection {
                    row.style(Style::default().bg(self.theme.cursor_bg))
                } else {
                    row
                }
            })
            .collect()
    }

    pub(super) fn cell_style(
        &self,
        highlight: CellHighlight,
        cell_type: CellType,
        has_jump: bool,
    ) -> Style {
        match highlight {
            CellHighlight::FocusedCell => Style::default()
                .fg(self.theme.focused_cell_fg)
                .bg(self.theme.focused_cell_bg)
                .add_modifier(Modifier::BOLD),
            CellHighlight::SelectedRow => Style::default()
                .fg(self.theme.table_cell)
                .bg(self.theme.cursor_bg)
                .add_modifier(Modifier::BOLD),
            CellHighlight::Normal => self.jump_target_style(cell_type, has_jump),
        }
    }

    pub(super) fn jump_target_style(&self, cell_type: CellType, has_jump: bool) -> Style {
        if has_jump && matches!(cell_type, CellType::DopReference | CellType::ParameterName) {
            Style::default().fg(self.theme.table_jump_cell)
        } else {
            Style::default().fg(self.theme.table_cell)
        }
    }

    pub(super) fn build_header_row(
        &self,
        header: &DetailRow,
        section_idx: usize,
        max_columns: usize,
    ) -> Row<'static> {
        let sort_state = self.table.sort_state.get(section_idx).and_then(|s| *s);

        // Build cells for all columns (pad to max_columns)
        let col_count = header.cells.len().max(max_columns);
        let all_col_indices: Vec<usize> = (0..col_count).collect();
        self.build_header_row_impl(header, &all_col_indices, sort_state)
    }

    pub(super) fn get_column_widths(
        &mut self,
        section_idx: usize,
        constraints: &[crate::tree::ColumnConstraint],
    ) -> Vec<u16> {
        self.table.ensure_column_width_capacity(section_idx);

        // If we don't have custom widths for this section, try persistent store or init defaults
        if self
            .table
            .column_widths
            .get(section_idx)
            .is_none_or(Vec::is_empty)
        {
            let key = self.make_column_width_key(section_idx, constraints.len());
            if let Some(persisted) = self.table.persisted_column_widths.get(&key).cloned() {
                // Restore persisted absolute widths
                if let Some(col_widths) = self.table.column_widths.get_mut(section_idx) {
                    *col_widths = persisted;
                }
                if let Some(abs) = self.table.column_widths_absolute.get_mut(section_idx) {
                    *abs = true;
                }
            } else {
                // Initialize from constraints as percentages
                let mut widths: Vec<u16> = constraints
                    .iter()
                    .map(|c| match c {
                        crate::tree::ColumnConstraint::Fixed(w) => {
                            // Convert fixed width to a reasonable percentage
                            w.saturating_mul(3).saturating_div(2).clamp(3, 15)
                        }
                        crate::tree::ColumnConstraint::Percentage(p) => *p,
                    })
                    .collect();

                // Normalize to ensure total is exactly 100%
                App::normalize_column_widths(&mut widths);

                if let Some(col_widths) = self.table.column_widths.get_mut(section_idx) {
                    *col_widths = widths;
                }
                // column_widths_absolute stays false (percentage mode)
            }
        }

        self.table
            .column_widths
            .get(section_idx)
            .map_or_else(Vec::new, Clone::clone)
    }
}
