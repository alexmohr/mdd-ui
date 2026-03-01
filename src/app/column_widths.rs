/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, COLUMN_SPACING, ColumnWidthCacheKey};
use crate::tree::DetailSectionType;

impl App {
    /// Build a cache key for persisting column widths by section identity
    pub(crate) fn make_column_width_key(
        &self,
        section_idx: usize,
        column_count: usize,
    ) -> ColumnWidthCacheKey {
        let (section_type, title) = self
            .tree
            .visible
            .get(self.tree.cursor)
            .and_then(|&node_idx| self.tree.all_nodes.get(node_idx))
            .and_then(|node| node.detail_sections.get(section_idx))
            .map_or((DetailSectionType::Custom, String::new()), |s| {
                (s.section_type, s.title.clone())
            });

        ColumnWidthCacheKey {
            section_type,
            title,
            column_count,
        }
    }

    /// Convert percentage-based column widths to absolute pixel widths
    pub(crate) fn convert_to_absolute_widths(&mut self, section_idx: usize) {
        let table_width = self.layout.table_content_area.map_or(100, |a| a.width);
        let column_spacing = COLUMN_SPACING;

        let Some(widths) = self.table.column_widths.get(section_idx) else {
            return;
        };
        let num_cols = widths.len();
        let num_gaps = u16::try_from(num_cols.saturating_sub(1)).unwrap_or(0);
        let spacing_total = column_spacing.saturating_mul(num_gaps);
        let available = table_width.saturating_sub(spacing_total);

        let pixel_widths: Vec<u16> = widths
            .iter()
            .map(|&pct| {
                let px = u32::from(pct)
                    .saturating_mul(u32::from(available))
                    .saturating_div(100);
                u16::try_from(px.clamp(3, u32::from(available))).unwrap_or(3)
            })
            .collect();

        if let Some(w) = self.table.column_widths.get_mut(section_idx) {
            *w = pixel_widths;
        }

        self.table.ensure_column_width_capacity(section_idx);
        if let Some(abs) = self.table.column_widths_absolute.get_mut(section_idx) {
            *abs = true;
        }
    }

    /// Save current column widths to the persistent store
    pub(crate) fn save_column_widths_to_persistent(&mut self, section_idx: usize) {
        let Some(widths) = self.table.column_widths.get(section_idx) else {
            return;
        };
        if widths.is_empty() {
            return;
        }
        let key = self.make_column_width_key(section_idx, widths.len());
        self.table
            .persisted_column_widths
            .insert(key, widths.clone());
    }

    /// Scroll the table horizontally by the given pixel delta
    pub(crate) fn scroll_horizontal(&mut self, delta: i16) {
        let section_idx = self.get_section_index();
        self.table.ensure_horizontal_scroll_capacity(section_idx);

        let current = self
            .table
            .horizontal_scroll
            .get(section_idx)
            .copied()
            .unwrap_or(0);

        let new_offset = if delta < 0 {
            let abs_delta = u16::try_from(0i16.saturating_sub(delta)).unwrap_or(0);
            current.saturating_sub(abs_delta)
        } else {
            let abs_delta = u16::try_from(delta).unwrap_or(0);
            current.saturating_add(abs_delta)
        };

        if let Some(hs) = self.table.horizontal_scroll.get_mut(section_idx) {
            *hs = new_offset;
        }
    }

    /// Ensure the focused column is visible by adjusting horizontal scroll
    pub(crate) fn ensure_focused_column_visible(&mut self, section_idx: usize) {
        let column_spacing = COLUMN_SPACING;

        let Some(widths) = self.table.column_widths.get(section_idx) else {
            return;
        };
        if !self
            .table
            .column_widths_absolute
            .get(section_idx)
            .copied()
            .unwrap_or(false)
        {
            return;
        }

        let viewport_width = self.layout.table_content_area.map_or(100, |a| a.width);

        // Calculate start/end pixel of focused column
        let mut col_start = 0u16;
        for (i, &w) in widths.iter().enumerate() {
            if i == self.table.focused_column {
                let col_end = col_start.saturating_add(w);

                self.table.ensure_horizontal_scroll_capacity(section_idx);
                let h_scroll = self
                    .table
                    .horizontal_scroll
                    .get(section_idx)
                    .copied()
                    .unwrap_or(0);

                // Scroll right if column end is past viewport
                if col_end > h_scroll.saturating_add(viewport_width)
                    && let Some(hs) = self.table.horizontal_scroll.get_mut(section_idx)
                {
                    *hs = col_end.saturating_sub(viewport_width);
                }
                // Scroll left if column start is before viewport
                if col_start < h_scroll
                    && let Some(hs) = self.table.horizontal_scroll.get_mut(section_idx)
                {
                    *hs = col_start;
                }
                return;
            }
            col_start = col_start.saturating_add(w).saturating_add(column_spacing);
        }
    }

    fn initialize_column_widths(
        &mut self,
        constraints: &[crate::tree::ColumnConstraint],
        section_idx: usize,
    ) {
        let mut widths: Vec<u16> = constraints
            .iter()
            .map(|c| match c {
                crate::tree::ColumnConstraint::Fixed(w) => {
                    w.saturating_mul(3).saturating_div(2).clamp(3, 15)
                }
                crate::tree::ColumnConstraint::Percentage(p) => *p,
            })
            .collect();

        Self::normalize_column_widths(&mut widths);

        let Some(section_widths) = self.table.column_widths.get_mut(section_idx) else {
            return;
        };
        *section_widths = widths;
    }

    pub(crate) fn normalize_column_widths(widths: &mut [u16]) {
        let total: u16 = widths.iter().sum();
        if total == 0 || total == 100 {
            return;
        }
        let total_32 = u32::from(total);
        for w in widths.iter_mut() {
            *w = u16::try_from(
                u32::from(*w)
                    .saturating_mul(100)
                    .saturating_add(total_32.saturating_div(2))
                    .checked_div(total_32)
                    .unwrap_or(0)
                    .min(100),
            )
            .unwrap_or(100);
        }

        let new_total: u16 = widths.iter().sum();
        if new_total != 100 && !widths.is_empty() {
            let max_idx = widths
                .iter()
                .enumerate()
                .max_by_key(|(_, w)| *w)
                .map_or(0, |(idx, _)| idx);
            let Some(max_width) = widths.get_mut(max_idx) else {
                return;
            };
            *max_width = max_width.saturating_add(100u16.saturating_sub(new_total));
        }
    }

    pub(crate) fn resize_column(&mut self, delta: i16) {
        let section_idx = self.get_section_index();

        self.table.ensure_column_width_capacity(section_idx);

        if self.tree.cursor >= self.tree.visible.len() {
            return;
        }
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(node_idx) else {
            return;
        };
        if section_idx >= node.detail_sections.len() {
            return;
        }
        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };
        let Some(constraints) = section.content.table_constraints() else {
            self.status = "Column resizing only available in tables".into();
            return;
        };

        let Some(current_widths) = self.table.column_widths.get(section_idx) else {
            return;
        };
        if current_widths.is_empty() {
            let constraints = constraints.to_vec();
            self.initialize_column_widths(&constraints, section_idx);
        }

        // Switch to absolute pixel widths on first resize
        let is_absolute = self
            .table
            .column_widths_absolute
            .get(section_idx)
            .copied()
            .unwrap_or(false);
        if !is_absolute {
            self.convert_to_absolute_widths(section_idx);
        }

        let Some(section_widths) = self.table.column_widths.get(section_idx) else {
            return;
        };
        let num_cols = section_widths.len();
        if num_cols == 0 || self.table.focused_column >= num_cols {
            return;
        }

        let Some(&focused_w) = section_widths.get(self.table.focused_column) else {
            return;
        };

        let focused_i = i16::try_from(focused_w).unwrap_or(0);
        let new_width = focused_i.saturating_add(delta).max(3);
        let new_width_u16 = u16::try_from(new_width).unwrap_or(3);

        if new_width_u16 == focused_w {
            self.status = "Cannot resize: at min width".into();
            return;
        }

        if let Some(widths) = self.table.column_widths.get_mut(section_idx)
            && let Some(fw) = widths.get_mut(self.table.focused_column)
        {
            *fw = new_width_u16;
        }

        self.save_column_widths_to_persistent(section_idx);
        self.ensure_focused_column_visible(section_idx);

        self.status = format!(
            "Column {} width: {}px",
            self.table.focused_column, new_width_u16,
        );
    }
}
