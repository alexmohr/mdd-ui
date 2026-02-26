/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

use super::{App, FocusState, SortDirection, TableSortState};
use crate::tree::TreeNode;

impl App {
    pub(crate) fn toggle_expand(&mut self) {
        let Some(&idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(idx) else {
            return;
        };
        if !node.has_children {
            return;
        }
        if let Some(node_mut) = self.all_nodes.get_mut(idx) {
            node_mut.expanded = !node_mut.expanded;
        }
        let old = self.cursor;
        self.rebuild_visible();
        self.cursor = old.min(self.visible.len().saturating_sub(1));
    }

    pub(crate) fn try_expand(&mut self) {
        if self.focus_state == FocusState::Detail {
            return;
        }
        let Some(&idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(idx) else {
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
        let Some(&idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(idx) else {
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
        for i in (0..self.cursor).rev() {
            if let Some(&visible_idx) = self.visible.get(i)
                && let Some(visible_node) = self.all_nodes.get(visible_idx)
                && visible_node.depth < my_depth
            {
                self.cursor = i;
                break;
            }
        }
    }

    pub(crate) fn expand_all(&mut self) {
        for n in &mut self.all_nodes {
            if n.has_children {
                n.expanded = true;
            }
        }
        self.rebuild_visible();
    }

    pub(crate) fn collapse_all(&mut self) {
        for (i, n) in self.all_nodes.iter_mut().enumerate() {
            if n.has_children {
                n.expanded = i == 0;
            }
        }
        self.rebuild_visible();
        self.cursor = 0;
        self.scroll_offset = 0;
        self.reset_detail_state();
    }

    pub(crate) fn toggle_diagcomm_sort(&mut self) {
        // Get the currently selected node
        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };

        // Find the nearest parent that has children (a sortable section)
        let sort_idx = self.find_sortable_parent(node_idx);

        let Some(sort_node) = self.all_nodes.get(sort_idx) else {
            return;
        };

        let is_service_list = sort_node.service_list_type.is_some();

        if is_service_list {
            // DiagComm/Request/Response sections: cycle ID/Name sort
            self.diagcomm_sort_by_id = !self.diagcomm_sort_by_id;
            self.sort_diagcomm_nodes_in_place();
            self.rebuild_visible();
            if self.diagcomm_sort_by_id {
                "Sort: by ID".clone_into(&mut self.status);
            } else {
                "Sort: by Name".clone_into(&mut self.status);
            }
        } else if sort_node.has_children {
            // Generic sort: toggle name ascending/descending for children
            self.sort_children_by_name(sort_idx);
            self.rebuild_visible();
        } else {
            "No sortable section found".clone_into(&mut self.status);
        }
    }

    /// Find the nearest parent (or self) that is a sortable section header
    fn find_sortable_parent(&self, node_idx: usize) -> usize {
        // If the node itself has children, sort it
        if let Some(node) = self.all_nodes.get(node_idx) {
            if node.has_children {
                return node_idx;
            }
            // Walk up to find parent with children
            let current_depth = node.depth;
            for i in (0..node_idx).rev() {
                if let Some(parent) = self.all_nodes.get(i)
                    && parent.depth < current_depth
                    && parent.has_children
                {
                    return i;
                }
            }
        }
        node_idx
    }

    /// Sort children of a node by name (ascending/descending toggle)
    fn sort_children_by_name(&mut self, parent_idx: usize) {
        let Some(parent) = self.all_nodes.get(parent_idx) else {
            return;
        };
        let parent_depth = parent.depth;
        let children_start = parent_idx.saturating_add(1);

        // Find end of children
        let mut children_end = children_start;
        while children_end < self.all_nodes.len() {
            if let Some(node) = self.all_nodes.get(children_end) {
                if node.depth > parent_depth {
                    children_end = children_end.saturating_add(1);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if children_end <= children_start {
            "No children to sort".clone_into(&mut self.status);
            return;
        }

        // Extract only direct children (depth == parent_depth + 1) with their subtrees
        let direct_child_depth = parent_depth.saturating_add(1);
        let mut child_groups: Vec<Vec<TreeNode>> = Vec::new();
        let all_children: Vec<TreeNode> =
            self.all_nodes.drain(children_start..children_end).collect();

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

        // Check if already sorted ascending to decide direction
        let already_ascending = child_groups.windows(2).all(|w| {
            let a = w
                .first()
                .and_then(|g| g.first())
                .map(|n| n.text.to_lowercase());
            let b = w
                .get(1)
                .and_then(|g| g.first())
                .map(|n| n.text.to_lowercase());
            a <= b
        });

        if already_ascending {
            child_groups.sort_by(|a, b| {
                let a_text = a.first().map(|n| n.text.to_lowercase());
                let b_text = b.first().map(|n| n.text.to_lowercase());
                b_text.cmp(&a_text)
            });
            "Sort: by Name ▼".clone_into(&mut self.status);
        } else {
            child_groups.sort_by(|a, b| {
                let a_text = a.first().map(|n| n.text.to_lowercase());
                let b_text = b.first().map(|n| n.text.to_lowercase());
                a_text.cmp(&b_text)
            });
            "Sort: by Name ▲".clone_into(&mut self.status);
        }

        // Re-insert sorted children
        let sorted: Vec<TreeNode> = child_groups.into_iter().flatten().collect();
        self.all_nodes
            .splice(children_start..children_start, sorted);
    }

    pub(crate) fn sort_diagcomm_nodes_in_place(&mut self) {
        // Find all "Diag-Comms" section headers and sort their children
        let mut i = 0;
        while i < self.all_nodes.len() {
            let Some(node) = self.all_nodes.get(i) else {
                break;
            };

            // Skip non-Diag-Comms nodes early
            if !Self::is_service_list_type(node, crate::tree::ServiceListType::DiagComms) {
                i = i.saturating_add(1);
                continue;
            }

            let section_depth = node.depth;
            let section_start = i.saturating_add(1);

            // Find all children (services) of this section
            let mut section_end = section_start;
            while section_end < self.all_nodes.len() {
                if let Some(node_at_end) = self.all_nodes.get(section_end) {
                    if node_at_end.depth > section_depth {
                        section_end = section_end.saturating_add(1);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Skip if no children to sort
            if section_end <= section_start {
                i = i.saturating_add(1);
                continue;
            }

            // Extract and sort the service nodes
            let mut services: Vec<TreeNode> =
                self.all_nodes.drain(section_start..section_end).collect();

            // Sort services based on current sort order
            if self.diagcomm_sort_by_id {
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

            // Deduplicate by name - keep only first occurrence of each service name
            let mut seen_names = std::collections::HashSet::new();
            services.retain(|service| {
                let name = extract_service_name(&service.text);
                seen_names.insert(name.to_owned())
            });

            // Update the count in the section header
            let new_count = services.len();
            if let Some(header_node) = self.all_nodes.get_mut(i) {
                // Update "Diag-Comms (X)" to reflect filtered count
                if header_node.text.find('(').is_some() {
                    header_node.text = format!("Diag-Comms ({new_count})");
                }
            }

            // Re-insert sorted and deduplicated services
            self.all_nodes
                .splice(section_start..section_start, services);

            // Skip past the sorted section
            i = section_start.saturating_add(section_end.saturating_sub(section_start));
        }
    }

    pub(crate) fn toggle_table_column_sort(&mut self) {
        // Only works when detail pane is focused
        if self.focus_state != FocusState::Detail {
            return;
        }

        let section_idx = self.get_table_section_idx();

        // Ensure we have enough entries in table_sort_state
        while self.table_sort_state.len() <= section_idx {
            self.table_sort_state.push(None);
        }

        let column = self.focused_column;

        // Toggle sort state: if already sorting by this column, toggle direction,
        // otherwise sort ascending by this column
        if let Some(sort_state) = self.table_sort_state.get_mut(section_idx) {
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
                _ => Some(TableSortState {
                    column,
                    direction: SortDirection::Ascending,
                    secondary_column: None,
                }),
            };
        }

        // Update status message
        if let Some(&state) = self
            .table_sort_state
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
fn extract_service_id(text: &str) -> u32 {
    // Extract ID from format like "[Service] 0x10 - ServiceName" or "0x22F501 - ServiceName"
    let text = text.strip_prefix("[Service] ").unwrap_or(text);
    if let Some(hex_part) = text.strip_prefix("0x")
        && let Some(dash_pos) = hex_part.find(" - ")
    {
        let id_str = hex_part[..dash_pos].trim();
        return u32::from_str_radix(id_str, 16).unwrap_or(0);
    }
    0
}

fn extract_service_name(text: &str) -> &str {
    // Extract name from format like "0x10    - ServiceName"
    if let Some(dash_pos) = text.find(" - ") {
        let start = dash_pos.saturating_add(3);
        return text.get(start..).unwrap_or(text).trim();
    }
    text
}
