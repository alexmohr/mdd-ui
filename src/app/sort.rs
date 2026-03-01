/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState, SortDirection, TableSortState};
use crate::tree::{NodeTextPrefix, TreeNode};

impl App {
    pub(crate) fn toggle_expand(&mut self) {
        let Some(&idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        if let Some(node_mut) = self.tree.all_nodes.get_mut(idx)
            && node_mut.has_children
        {
            node_mut.expanded = !node_mut.expanded;
        }
        let old = self.tree.cursor;
        self.rebuild_visible();
        self.tree.cursor = old.min(self.tree.visible.len().saturating_sub(1));
    }

    pub(crate) fn try_expand(&mut self) {
        if self.focus_state == FocusState::Detail {
            return;
        }
        let Some(&idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(idx) else {
            return;
        };
        if node.has_children && !node.expanded {
            self.toggle_expand();
        }
    }

    pub(crate) fn try_collapse_or_parent(&mut self) {
        if self.focus_state == FocusState::Detail {
            return;
        }
        let Some(&idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };
        let Some(node) = self.tree.all_nodes.get(idx) else {
            return;
        };

        if node.has_children && node.expanded {
            self.toggle_expand();
            return;
        }
        // Navigate to parent
        let my_depth = node.depth;
        if my_depth == 0 {
            return;
        }
        if let Some(parent_i) = (0..self.tree.cursor).rev().find(|&i| {
            self.tree
                .visible
                .get(i)
                .and_then(|&visible_idx| self.tree.all_nodes.get(visible_idx))
                .is_some_and(|n| n.depth < my_depth)
        }) {
            self.tree.cursor = parent_i;
        }
    }

    pub(crate) fn expand_all(&mut self) {
        self.tree
            .all_nodes
            .iter_mut()
            .filter(|n| n.has_children)
            .for_each(|n| n.expanded = true);
        self.rebuild_visible();
    }

    pub(crate) fn collapse_all(&mut self) {
        self.tree
            .all_nodes
            .iter_mut()
            .filter(|n| n.has_children)
            .for_each(|n| n.expanded = n.depth == 0);
        self.rebuild_visible();
        self.tree.cursor = 0;
        self.tree.scroll_offset = 0;
        self.reset_detail_state();
    }

    pub(crate) fn toggle_tree_sort(&mut self) {
        let Some(&node_idx) = self.tree.visible.get(self.tree.cursor) else {
            return;
        };

        let sort_idx = self.find_sortable_parent(node_idx);

        let Some(sort_node) = self.tree.all_nodes.get(sort_idx) else {
            return;
        };

        let is_diagcomms =
            Self::is_service_list_type(sort_node, crate::tree::ServiceListType::DiagComms);

        if is_diagcomms {
            self.tree.diagcomm_sort_by_id = !self.tree.diagcomm_sort_by_id;
            self.sort_diagcomm_nodes_in_place();
            self.rebuild_visible();
            if self.tree.diagcomm_sort_by_id {
                self.status = "Sort: by ID".into();
            } else {
                self.status = "Sort: by Name".into();
            }
        } else if sort_node.has_children {
            self.sort_children_by_name(sort_idx);
            self.rebuild_visible();
        } else {
            self.status = "No sortable section found".into();
        }
    }

    /// Find the nearest parent (or self) that is a sortable section header
    fn find_sortable_parent(&self, node_idx: usize) -> usize {
        // If the node itself has children, sort it
        if let Some(node) = self.tree.all_nodes.get(node_idx) {
            if node.has_children {
                return node_idx;
            }
            // Walk up to find parent with children
            let current_depth = node.depth;
            if let Some(idx) = (0..node_idx).rev().find(|&i| {
                self.tree
                    .all_nodes
                    .get(i)
                    .is_some_and(|parent| parent.depth < current_depth && parent.has_children)
            }) {
                return idx;
            }
        }
        node_idx
    }

    /// Sort children of a node by name (ascending/descending toggle)
    fn sort_children_by_name(&mut self, parent_idx: usize) {
        let Some(parent) = self.tree.all_nodes.get(parent_idx) else {
            return;
        };
        let parent_depth = parent.depth;
        let children_start = parent_idx.saturating_add(1);

        let children_end = self
            .tree
            .all_nodes
            .iter()
            .skip(children_start)
            .position(|n| n.depth <= parent_depth)
            .map_or(self.tree.all_nodes.len(), |pos| {
                children_start.saturating_add(pos)
            });

        if children_end <= children_start {
            self.status = "No children to sort".into();
            return;
        }

        // Extract only direct children (depth == parent_depth + 1) with their subtrees
        let direct_child_depth = parent_depth.saturating_add(1);
        let mut child_groups: Vec<Vec<TreeNode>> = Vec::new();
        let all_children: Vec<TreeNode> = self
            .tree
            .all_nodes
            .drain(children_start..children_end)
            .collect();

        let mut current_group: Vec<TreeNode> = Vec::new();
        for node in all_children {
            if node.depth == direct_child_depth && !current_group.is_empty() {
                child_groups.push(std::mem::take(&mut current_group));
            }
            current_group.push(node);
        }
        if !current_group.is_empty() {
            child_groups.push(current_group);
        }

        // Determine next sort direction from the persisted map;
        // absent means first press → ascending.
        let next_dir = match self.tree.children_sort_dirs.get(&parent_idx) {
            Some(SortDirection::Ascending) => SortDirection::Descending,
            Some(SortDirection::Descending) | None => SortDirection::Ascending,
        };
        self.tree.children_sort_dirs.insert(parent_idx, next_dir);

        if next_dir == SortDirection::Descending {
            child_groups.sort_by(|a, b| {
                let a_text = a.first().map(|n| n.text.to_lowercase());
                let b_text = b.first().map(|n| n.text.to_lowercase());
                b_text.cmp(&a_text)
            });
            self.status = "Sort: by Name ▼".into();
        } else {
            child_groups.sort_by(|a, b| {
                let a_text = a.first().map(|n| n.text.to_lowercase());
                let b_text = b.first().map(|n| n.text.to_lowercase());
                a_text.cmp(&b_text)
            });
            self.status = "Sort: by Name ▲".into();
        }

        // Re-insert sorted children
        let sorted: Vec<TreeNode> = child_groups.into_iter().flatten().collect();
        self.tree
            .all_nodes
            .splice(children_start..children_start, sorted);
    }

    pub(crate) fn sort_diagcomm_nodes_in_place(&mut self) {
        // Collect (header_idx, children_start, children_end) for every Diag-Comms
        // section, then process them in *reverse* order so that splice/drain at
        // an earlier index does not shift the positions of later sections.
        let sections: Vec<(usize, usize, usize)> = self
            .tree
            .all_nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| Self::is_service_list_type(n, crate::tree::ServiceListType::DiagComms))
            .map(|(i, n)| {
                let section_depth = n.depth;
                let section_start = i.saturating_add(1);
                let section_end = self
                    .tree
                    .all_nodes
                    .iter()
                    .skip(section_start)
                    .position(|m| m.depth <= section_depth)
                    .map_or(self.tree.all_nodes.len(), |pos| {
                        section_start.saturating_add(pos)
                    });
                (i, section_start, section_end)
            })
            .collect();

        for (_, section_start, section_end) in sections.into_iter().rev() {
            if section_end <= section_start {
                continue;
            }

            let mut services: Vec<TreeNode> = self
                .tree
                .all_nodes
                .drain(section_start..section_end)
                .collect();

            if self.tree.diagcomm_sort_by_id {
                services.sort_by(|a, b| {
                    let a_id = extract_service_id(&a.text);
                    let b_id = extract_service_id(&b.text);
                    a_id.cmp(&b_id)
                });
            } else {
                services.sort_by(|a, b| {
                    let a_name = extract_service_name(&a.text);
                    let b_name = extract_service_name(&b.text);
                    a_name.cmp(b_name)
                });
            }

            self.tree
                .all_nodes
                .splice(section_start..section_start, services);
        }
    }

    pub(crate) fn toggle_table_column_sort(&mut self) {
        // Only works when detail pane is focused
        if self.focus_state != FocusState::Detail {
            return;
        }

        let section_idx = self.get_section_index();

        // Ensure we have enough entries in table_sort_state
        while self.table.sort_state.len() <= section_idx {
            self.table.sort_state.push(None);
        }

        let column = self.table.focused_column;

        // Toggle sort state: if already sorting by this column, toggle direction,
        // otherwise sort ascending by this column
        if let Some(sort_state) = self.table.sort_state.get_mut(section_idx) {
            *sort_state = match *sort_state {
                Some(state) if state.column == column => {
                    let new_direction = match state.direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                    Some(TableSortState {
                        column,
                        direction: new_direction,
                        secondary_column: None,
                    })
                }
                Some(_) | None => Some(TableSortState {
                    column,
                    direction: SortDirection::Ascending,
                    secondary_column: None,
                }),
            };
        }

        // Update status message
        if let Some(&state) = self
            .table
            .sort_state
            .get(section_idx)
            .and_then(|s| s.as_ref())
        {
            let direction_str = match state.direction {
                SortDirection::Ascending => "▲",
                SortDirection::Descending => "▼",
            };
            self.status = format!("Sort by column {} {}", state.column, direction_str);
        }
    }
}

// Helper functions for service sorting
fn extract_service_id(text: &str) -> Option<u32> {
    // Extract ID from format like "[Service] 0x10 - ServiceName" or "0x22F501 - ServiceName"
    let text = text
        .strip_prefix(NodeTextPrefix::Service.as_str())
        .unwrap_or(text);
    let hex_part = text.strip_prefix("0x")?;
    let dash_pos = hex_part.find(" - ")?;
    let id_str = hex_part[..dash_pos].trim();
    u32::from_str_radix(id_str, 16).ok()
}

fn extract_service_name(text: &str) -> &str {
    // Extract name from format like "0x10    - ServiceName"
    if let Some(dash_pos) = text.find(" - ") {
        let start = dash_pos.saturating_add(3);
        return text.get(start..).unwrap_or(text).trim();
    }
    text
}
