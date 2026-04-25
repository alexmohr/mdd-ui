/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use crate::{
    app::App,
    tree::{DetailSectionType, RowMetadata},
};

impl App {
    /// Navigate to a variant from the Variants overview table
    pub(crate) fn try_navigate_to_variant(&mut self) {
        let target = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let Some(name_cell) = selected_row.cells.first() else {
                return;
            };
            name_cell.text.clone()
        };
        self.navigate_to_container_by_name(&target);
    }

    /// Navigate from variant overview to a child element
    pub(crate) fn try_navigate_from_variant_overview(&mut self) {
        let (node_idx, depth, element_type) = {
            let Some(ctx) = self.resolve_selected_row() else {
                return;
            };
            if ctx.section.section_type != DetailSectionType::Overview {
                return;
            }
            let Some(selected_row) = ctx.selected_row() else {
                return;
            };
            let Some(RowMetadata::ChildElement { element_type }) = &selected_row.metadata else {
                return;
            };
            (ctx.node_idx, ctx.node.depth, element_type.clone())
        };
        self.navigate_to_child_element(node_idx, depth, &element_type);
    }
}
