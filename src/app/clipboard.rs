/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use std::fmt::Write;

use super::App;
use crate::tree::{DetailContent, DetailRow, DetailSectionData};

impl App {
    /// Copy the current table in the detail pane to the clipboard as markdown.
    /// Returns a status message describing the result.
    pub(crate) fn copy_table_to_clipboard(&self) -> String {
        let Some(section) = self.get_current_detail_section() else {
            return "No table to copy".to_owned();
        };

        let markdown = self.section_to_markdown(section);
        let Some(markdown) = markdown else {
            return "No table data to copy".to_owned();
        };

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(&markdown) {
                Ok(()) => "Table copied to clipboard as markdown".to_owned(),
                Err(e) => format!("Failed to copy: {e}"),
            },
            Err(e) => format!("Clipboard error: {e}"),
        }
    }

    /// Get the currently selected detail section.
    fn get_current_detail_section(&self) -> Option<&DetailSectionData> {
        let node_idx = *self.tree.visible.get(self.tree.cursor)?;
        let node = self.tree.all_nodes.get(node_idx)?;
        let section_idx = self.get_section_index();
        node.detail_sections.get(section_idx)
    }

    /// Convert a detail section to markdown format.
    /// Returns `None` if the section has no table content.
    fn section_to_markdown(&self, section: &DetailSectionData) -> Option<String> {
        match &section.content {
            DetailContent::Table { header, rows, .. } => {
                let section_idx = self.get_section_index();
                let sorted_rows = self.sort_rows(rows, section_idx);
                Some(table_to_markdown(header, &sorted_rows))
            }
            DetailContent::Composite(subsections) => {
                // For composite sections, combine all tables
                let mut result = String::new();
                for (i, sub) in subsections.iter().enumerate() {
                    if let DetailContent::Table { header, rows, .. } = &sub.content {
                        if i > 0 {
                            result.push_str("\n\n");
                        }
                        let _ = write!(result, "### {}\n\n", sub.title);
                        result.push_str(&table_to_markdown(header, rows));
                    }
                }
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }
            DetailContent::PlainText(_) => None,
        }
    }
}

/// Convert a table header and rows to markdown format.
fn table_to_markdown(header: &DetailRow, rows: &[DetailRow]) -> String {
    let mut result = String::new();

    // Calculate column widths for alignment
    let col_count = header.cells.len();
    let mut widths: Vec<usize> = header.cells.iter().map(String::len).collect();

    for row in rows {
        for (i, cell) in row.cells.iter().enumerate() {
            if let Some(current_width) = widths.get_mut(i) {
                *current_width = (*current_width).max(cell.len());
            }
        }
    }

    // Ensure minimum width of 3 for the separator
    for width in &mut widths {
        *width = (*width).max(3);
    }

    // Header row
    result.push('|');
    for (i, cell) in header.cells.iter().enumerate() {
        let width = widths.get(i).copied().unwrap_or(3);
        let _ = write!(result, " {cell:<width$} |");
    }
    result.push('\n');

    // Separator row
    result.push('|');
    for width in &widths {
        let _ = write!(result, " {:-<width$} |", "");
    }
    result.push('\n');

    // Data rows
    for row in rows {
        result.push('|');
        for i in 0..col_count {
            let cell = row.cells.get(i).map_or("", String::as_str);
            let width = widths.get(i).copied().unwrap_or(3);
            let _ = write!(result, " {cell:<width$} |");
        }
        result.push('\n');
    }

    result
}
