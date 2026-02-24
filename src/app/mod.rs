/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod input;
mod render;

use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
};
use input::Action;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::tree::{
    CellType, DetailContent, DetailRow, DetailRowType, DetailSectionType, NodeType, RowMetadata,
    SectionType, TreeNode,
};

// -----------------------------------------------------------------------
// Application state
// -----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchScope {
    All,              // Search everywhere
    Variants,         // Search only in variant names
    FunctionalGroups, // Search only in functional group names
    EcuSharedData,    // Search only in ECU shared data names
    Services,         // Search only in service names
    DiagComms,        // Search only in Diag-Comms sections
    Requests,         // Search only in Requests
    Responses,        // Search only in Responses (Pos and Neg)
    Subtree {
        start_idx: usize,
        end_idx: usize,
        root_name: String,
    },
}

impl std::fmt::Display for SearchScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchScope::All => write!(f, "All"),
            SearchScope::Variants => write!(f, "Variants"),
            SearchScope::FunctionalGroups => write!(f, "functional groups"),
            SearchScope::EcuSharedData => write!(f, "ECU shared data"),
            SearchScope::Services => write!(f, "Services"),
            SearchScope::DiagComms => write!(f, "Diag-Comms"),
            SearchScope::Requests => write!(f, "Requests"),
            SearchScope::Responses => write!(f, "Responses"),
            SearchScope::Subtree { root_name, .. } => write!(f, "in {root_name}"),
        }
    }
}

impl SearchScope {
    /// Returns the scope indicator for search mode (e.g., " [variants]")
    pub(crate) fn search_indicator(&self) -> &str {
        match self {
            SearchScope::All => "",
            SearchScope::Variants => " [variants]",
            SearchScope::FunctionalGroups => " [functional groups]",
            SearchScope::EcuSharedData => " [ECU shared data]",
            SearchScope::Services => " [services]",
            SearchScope::DiagComms => " [diag-comms]",
            SearchScope::Requests => " [requests]",
            SearchScope::Responses => " [responses]",
            SearchScope::Subtree { .. } => " [subtree]",
        }
    }

    /// Returns the scope indicator for status line (e.g., " | scope: variants")
    pub(crate) fn status_indicator(&self) -> &str {
        match self {
            SearchScope::All => "",
            SearchScope::Variants => " | scope: variants",
            SearchScope::FunctionalGroups => " | scope: functional groups",
            SearchScope::EcuSharedData => " | scope: ECU shared data",
            SearchScope::Services => " | scope: services",
            SearchScope::DiagComms => " | scope: diag-comms",
            SearchScope::Requests => " | scope: requests",
            SearchScope::Responses => " | scope: responses",
            SearchScope::Subtree { .. } => " | scope: subtree",
        }
    }

    /// Returns the abbreviated scope indicator (e.g., "[V]" for Variants)
    pub(crate) fn abbrev(&self) -> &str {
        match self {
            SearchScope::All => "",
            SearchScope::Variants => "[V]",
            SearchScope::FunctionalGroups => "[FG]",
            SearchScope::EcuSharedData => "[ESD]",
            SearchScope::Services => "[S]",
            SearchScope::DiagComms => "[D]",
            SearchScope::Requests => "[Rq]",
            SearchScope::Responses => "[Rs]",
            SearchScope::Subtree { .. } => "[ST]",
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub(crate) enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub(crate) struct TableSortState {
    pub column: usize,
    pub direction: SortDirection,
}

pub struct App {
    all_nodes: Vec<TreeNode>,
    visible: Vec<usize>,
    cursor: usize,
    scroll_offset: usize,
    detail_scroll: usize,
    pub(crate) search: String,
    pub(crate) searching: bool,
    pub(crate) search_stack: Vec<(String, SearchScope)>, // Stack of (search_term, scope) pairs
    pub(crate) search_scope: SearchScope,
    pub(crate) search_matches: Vec<usize>,
    search_match_cursor: usize,
    pub(crate) status: String,
    pub(crate) detail_focused: bool,
    pub(crate) selected_tab: usize, // Currently selected tab in detail pane
    pub(crate) focused_section: usize, // Which detail pane section is focused (0 = first)
    pub(crate) section_scrolls: Vec<usize>, // Scroll position for each section
    pub(crate) section_cursors: Vec<usize>, // Selected row in each section
    pub(crate) column_widths: Vec<Vec<u16>>, // Column widths for each section (percentages)
    pub(crate) focused_column: usize, // Currently focused column for resizing
    pub(crate) detail_popup: Option<PopupData>, // Generic popup state
    pub(crate) help_popup_visible: bool, // Help popup visibility
    pub(crate) tree_width_percentage: u16, // Tree pane width (0-100)
    pub(crate) diagcomm_sort_by_id: bool, // true = sort by ID (default), false = sort by name
    // Sort state for each table section (None = default order)
    pub(crate) table_sort_state: Vec<Option<TableSortState>>,
    tree_area: Rect,                    // Cached tree area for mouse handling
    detail_area: Rect,                  // Cached detail area for mouse handling
    pub(crate) tab_area: Option<Rect>,  // Cached tab area for mouse handling
    pub(crate) tab_titles: Vec<String>, // Cached tab titles for click detection
    pub(crate) table_content_area: Option<Rect>, // Cached table content area
    // Exact constraints used in Table
    pub(crate) cached_ratatui_constraints: Vec<ratatui::layout::Constraint>,
    last_click_time: Option<Instant>, // Time of last click for double-click detection
    last_click_pos: (u16, u16),       // Position of last click (column, row)
    pub(crate) mouse_enabled: bool,   // Whether mouse input is enabled
    // History of node indices in all_nodes (stable across expand/collapse)
    navigation_history: Vec<usize>,
    history_position: usize, // Current position in history (for potential forward navigation)
    breadcrumb_area: Rect,   // Cached breadcrumb area for mouse handling
    breadcrumb_segments: Vec<(String, usize, u16, u16)>, // (text, node_idx, start_col, end_col)
    dragging_divider: bool,  // Whether user is currently dragging the tree/detail divider
    tree_scrollbar_area: Option<Rect>, // Cached tree scrollbar area for mouse handling
    detail_scrollbar_area: Option<Rect>, // Cached detail scrollbar area for mouse handling
    dragging_scrollbar: bool, // Whether user is currently dragging a scrollbar
    // true = dragging tree scrollbar, false = detail scrollbar
    dragging_tree_scrollbar: bool,
    last_diagcomm_tab: usize, // Last selected tab when viewing service/job nodes (for persistence)
    last_section_tabs: HashMap<DetailSectionType, usize>, // Last selected tab per section type
    jump_buffer: String,      // Characters typed for type-to-jump in table views
    jump_buffer_time: Option<Instant>, // Timestamp of last type-to-jump character for auto-reset
}

#[derive(Clone)]
pub struct PopupData {
    pub title: String,
    pub content: Vec<String>,
}

impl App {
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        let mut app = Self {
            all_nodes: nodes,
            visible: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            detail_scroll: 0,
            search: String::new(),
            searching: false,
            search_stack: Vec::new(),
            search_scope: SearchScope::All,
            search_matches: Vec::new(),
            search_match_cursor: 0,
            status: String::new(),
            detail_focused: false,
            selected_tab: 0,
            focused_section: 0,
            section_scrolls: Vec::new(),
            section_cursors: Vec::new(),
            column_widths: Vec::new(),
            focused_column: 0,
            detail_popup: None,
            help_popup_visible: false,
            tree_width_percentage: 35,
            diagcomm_sort_by_id: true,    // Default: sort by ID
            table_sort_state: Vec::new(), // No sorting by default
            tree_area: Rect::default(),
            detail_area: Rect::default(),
            tab_area: None,
            tab_titles: Vec::new(),
            table_content_area: None,
            cached_ratatui_constraints: Vec::new(),
            last_click_time: None,
            last_click_pos: (0, 0),
            mouse_enabled: true, // Mouse enabled by default
            navigation_history: Vec::new(),
            history_position: 0,
            breadcrumb_area: Rect::default(),
            breadcrumb_segments: Vec::new(),
            dragging_divider: false,
            tree_scrollbar_area: None,
            detail_scrollbar_area: None,
            dragging_scrollbar: false,
            dragging_tree_scrollbar: false,
            last_diagcomm_tab: 0,
            last_section_tabs: HashMap::new(),
            jump_buffer: String::new(),
            jump_buffer_time: None,
        };
        // Apply initial sort order (default is by ID)
        app.sort_diagcomm_nodes_in_place();
        app.rebuild_visible();
        app
    }

    /// Helper: Check if a node is a service list section header
    fn is_service_list_section(&self, node: &TreeNode) -> bool {
        node.service_list_type.is_some()
    }

    /// Helper: Check if a node is a specific service list type
    fn is_service_list_type(
        &self,
        node: &TreeNode,
        list_type: crate::tree::ServiceListType,
    ) -> bool {
        matches!(&node.service_list_type, Some(t) if *t == list_type)
    }

    /// Check if a node is a DOP category node (child of DIAG-DATA-DICTIONARY-SPEC)
    fn is_dop_category_node(&self, node_idx: usize) -> bool {
        let node = &self.all_nodes[node_idx];
        if !node.has_children || node.depth == 0 {
            return false;
        }
        // Walk backwards to find the parent (first node with depth - 1)
        for i in (0..node_idx).rev() {
            let candidate = &self.all_nodes[i];
            if candidate.depth < node.depth {
                return matches!(candidate.node_type, NodeType::DOP);
            }
        }
        false
    }

    /// Check if a node is an individual DOP with children (e.g. a DTC-DOP under DTC-DOPS).
    /// These nodes should navigate to their children instead of showing a popup.
    fn is_individual_dop_node(&self, node_idx: usize) -> bool {
        let node = &self.all_nodes[node_idx];
        if !node.has_children || node.depth < 2 {
            return false;
        }
        // Walk backwards to find the parent
        for i in (0..node_idx).rev() {
            let candidate = &self.all_nodes[i];
            if candidate.depth < node.depth {
                return self.is_dop_category_node(i);
            }
        }
        false
    }

    /// Get the actual section index accounting for header section offset
    fn get_section_index(&self) -> usize {
        // Check if current node has a header section (rendered above tabs)
        if let Some(&idx) = self.visible.get(self.cursor) {
            let sections = &self.all_nodes[idx].detail_sections;
            if sections.len() > 1
                && sections[0].render_as_header
                && matches!(
                    &sections[0].content,
                    crate::tree::DetailContent::PlainText(_)
                )
            {
                // Has header section, so selected_tab needs offset of 1
                return self.selected_tab + 1;
            }
        }
        self.selected_tab
    }

    /// Get the section offset for rendering (0 or 1 if there's a header section)
    fn get_section_offset(&self) -> usize {
        if let Some(&idx) = self.visible.get(self.cursor) {
            let sections = &self.all_nodes[idx].detail_sections;
            if sections.len() > 1
                && sections[0].render_as_header
                && matches!(
                    &sections[0].content,
                    crate::tree::DetailContent::PlainText(_)
                )
            {
                return 1;
            }
        }
        0
    }

    /// Get the actual table section index for storing/retrieving sort state
    fn get_table_section_idx(&self) -> usize {
        self.selected_tab + self.get_section_offset()
    }

    /// Update the selected tab and persist it generically for the current section type
    fn set_selected_tab(&mut self, new_tab: usize) {
        self.selected_tab = new_tab;
        self.jump_buffer.clear();
        self.jump_buffer_time = None;

        // Save tab selection for the current section type
        if self.cursor < self.visible.len()
            && let Some(&node_idx) = self.visible.get(self.cursor)
            && node_idx < self.all_nodes.len()
        {
            let node = &self.all_nodes[node_idx];

            // For backward compatibility, still save diagcomm tab
            if matches!(
                node.node_type,
                NodeType::Service | NodeType::ParentRefService | NodeType::Job
            ) {
                self.last_diagcomm_tab = new_tab;
            }

            // Save tab for any node with detail sections that have a section type
            if !node.detail_sections.is_empty() {
                // Get the section type from the currently selected tab
                let section_offset = self.get_section_offset();
                let section_idx = new_tab + section_offset;
                if section_idx < node.detail_sections.len() {
                    let section_type = node.detail_sections[section_idx].section_type;
                    self.last_section_tabs.insert(section_type, new_tab);
                }
            }
        }
    }

    /// Jump to the first table row whose first cell starts with the jump_buffer text
    fn jump_to_matching_row(&mut self) {
        if self.jump_buffer.is_empty() || self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Get table rows (apply sorting if active)
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => self.apply_table_sort(rows, section_idx),
            _ => return,
        };

        let buffer_lower = self.jump_buffer.to_lowercase();

        // Find first row where the focused column starts with the buffer (case-insensitive)
        for (i, row) in rows.iter().enumerate() {
            if let Some(cell) = row.cells.get(self.focused_column)
                && cell.to_lowercase().starts_with(&buffer_lower)
            {
                if section_idx < self.section_cursors.len() {
                    self.section_cursors[section_idx] = i;
                }
                self.status = format!("Jump: \"{}\"", self.jump_buffer);
                return;
            }
        }

        self.status = format!("Jump: \"{}\" (no match)", self.jump_buffer);
    }

    /// Apply sorting to rows if a sort state exists for the given section
    fn apply_table_sort(&self, rows: &[DetailRow], section_idx: usize) -> Vec<DetailRow> {
        if let Some(sort_state) = self
            .table_sort_state
            .get(section_idx)
            .and_then(|s| s.as_ref())
        {
            let mut sorted = rows.to_vec();
            let col = sort_state.column;
            let dir = sort_state.direction;
            sorted.sort_by(|a, b| {
                let a_cell = a.cells.get(col).map(|s| s.as_str()).unwrap_or("");
                let b_cell = b.cells.get(col).map(|s| s.as_str()).unwrap_or("");

                // Try to parse as numbers first, fall back to string comparison
                let cmp = match (a_cell.parse::<f64>(), b_cell.parse::<f64>()) {
                    (Ok(a_num), Ok(b_num)) => a_num
                        .partial_cmp(&b_num)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => a_cell.cmp(b_cell),
                };

                // Apply direction
                match dir {
                    SortDirection::Ascending => cmp,
                    SortDirection::Descending => cmp.reverse(),
                }
            });
            return sorted;
        }
        rows.to_vec()
    }

    // -------------------------------------------------------------------
    // Event loop
    // -------------------------------------------------------------------

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let event = event::read()?;

            let action = match event {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                    if self.searching {
                        self.handle_search_key(key.code)
                    } else {
                        self.handle_normal_key(key.code, ctrl, shift)
                    }
                }
                Event::Mouse(mouse) => {
                    if self.mouse_enabled {
                        self.handle_mouse_event(mouse.kind, mouse.column, mouse.row)
                    } else {
                        Action::Continue
                    }
                }
                _ => Action::Continue,
            };

            if matches!(action, Action::Quit) {
                return Ok(());
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [main, breadcrumb_bar, status_bar] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(frame.area());

        let [tree_area, detail_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(self.tree_width_percentage),
                Constraint::Percentage(100 - self.tree_width_percentage),
            ])
            .areas(main);

        // Cache areas for mouse handling
        self.tree_area = tree_area;
        self.detail_area = detail_area;
        self.breadcrumb_area = breadcrumb_bar;

        self.draw_tree(frame, tree_area);
        self.draw_detail(frame, detail_area);
        self.draw_breadcrumb(frame, breadcrumb_bar);
        self.draw_status(frame, status_bar);

        // Draw popups if open (order matters - last drawn is on top)
        if self.detail_popup.is_some() {
            self.draw_detail_popup(frame);
        }
        if self.help_popup_visible {
            self.draw_help_popup(frame);
        }
    }

    // -------------------------------------------------------------------
    // Visibility
    // -------------------------------------------------------------------

    /// Check if a node at index i is under a specific section type
    fn is_under_section_type(
        &self,
        node_idx: usize,
        section_type: crate::tree::SectionType,
    ) -> bool {
        if node_idx == 0 {
            return false;
        }

        let node_depth = self.all_nodes[node_idx].depth;

        // Search backwards from node_idx to find parent section
        for i in (0..node_idx).rev() {
            let parent = &self.all_nodes[i];

            // Stop if we reach a node at the same or lower depth (not a parent)
            if parent.depth >= node_depth {
                continue;
            }

            // If this is a section header at depth 0, check if it matches
            if parent.depth == 0 && matches!(&parent.section_type, Some(st) if *st == section_type)
            {
                return true;
            }
        }

        false
    }

    fn rebuild_visible(&mut self) {
        self.visible.clear();

        if self.search_stack.is_empty() {
            self.rebuild_visible_no_search();
        } else {
            self.rebuild_visible_with_search();
        }
    }

    /// Rebuild visible list when no search is active
    fn rebuild_visible_no_search(&mut self) {
        let mut collapsed_below: Option<usize> = None;

        for (i, node) in self.all_nodes.iter().enumerate() {
            // Skip nodes under collapsed parent
            if let Some(cd) = collapsed_below {
                if node.depth > cd {
                    continue;
                }
                collapsed_below = None;
            }

            self.visible.push(i);

            // Mark as collapsed if node has unexpanded children
            if node.has_children && !node.expanded {
                collapsed_below = Some(node.depth);
            }
        }
    }

    /// Rebuild visible list with active search stack
    fn rebuild_visible_with_search(&mut self) {
        // Start with all nodes included, then filter by each search
        let mut include = vec![true; self.all_nodes.len()];

        // Apply each search filter cumulatively
        for (query, scope) in &self.search_stack {
            include = self.apply_search_filter(&include, query, scope);
        }

        // Build visible list from included nodes, respecting collapse state
        self.build_visible_from_filter(&include);
    }

    /// Apply a single search filter to the include vector
    fn apply_search_filter(&self, include: &[bool], query: &str, scope: &SearchScope) -> Vec<bool> {
        let q = query.to_lowercase();
        let mut new_include = vec![false; self.all_nodes.len()];

        for (i, &included) in include.iter().enumerate().take(self.all_nodes.len()) {
            if !included {
                continue;
            }

            let node = &self.all_nodes[i];
            if self.node_matches_scope_and_query(node, i, scope, &q) {
                self.include_node_and_hierarchy(i, &mut new_include);
            }
        }

        new_include
    }

    /// Check if a node matches the search scope and query
    fn node_matches_scope_and_query(
        &self,
        node: &TreeNode,
        node_idx: usize,
        scope: &SearchScope,
        query: &str,
    ) -> bool {
        let matches_scope = match scope {
            SearchScope::All => true,
            SearchScope::Variants => {
                matches!(node.section_type, Some(crate::tree::SectionType::Variants))
                    || (matches!(node.node_type, NodeType::Container)
                        && node_idx > 0
                        && self.is_under_section_type(node_idx, crate::tree::SectionType::Variants))
            }
            SearchScope::FunctionalGroups => {
                matches!(
                    node.section_type,
                    Some(crate::tree::SectionType::FunctionalGroups)
                ) || (matches!(node.node_type, NodeType::Container)
                    && node_idx > 0
                    && self.is_under_section_type(
                        node_idx,
                        crate::tree::SectionType::FunctionalGroups,
                    ))
            }
            SearchScope::EcuSharedData => {
                matches!(
                    node.section_type,
                    Some(crate::tree::SectionType::EcuSharedData)
                ) || (matches!(node.node_type, NodeType::Container)
                    && node_idx > 0
                    && self
                        .is_under_section_type(node_idx, crate::tree::SectionType::EcuSharedData))
            }
            SearchScope::Services => matches!(
                node.node_type,
                NodeType::Service
                    | NodeType::ParentRefService
                    | NodeType::Request
                    | NodeType::PosResponse
                    | NodeType::NegResponse
            ),
            SearchScope::DiagComms => {
                self.is_service_list_type(node, crate::tree::ServiceListType::DiagComms)
                    || matches!(
                        node.node_type,
                        NodeType::Service | NodeType::ParentRefService | NodeType::Job
                    )
            }
            SearchScope::Requests => {
                self.is_service_list_type(node, crate::tree::ServiceListType::Requests)
                    || matches!(node.node_type, NodeType::Request)
            }
            SearchScope::Responses => {
                self.is_service_list_type(node, crate::tree::ServiceListType::PosResponses)
                    || self.is_service_list_type(node, crate::tree::ServiceListType::NegResponses)
                    || matches!(
                        node.node_type,
                        NodeType::PosResponse | NodeType::NegResponse
                    )
            }
            SearchScope::Subtree {
                start_idx, end_idx, ..
            } => node_idx >= *start_idx && node_idx <= *end_idx,
        };

        matches_scope && node.text.to_lowercase().contains(query)
    }

    /// Include a node and its entire hierarchy (parents and children)
    fn include_node_and_hierarchy(&self, node_idx: usize, new_include: &mut [bool]) {
        let node = &self.all_nodes[node_idx];
        new_include[node_idx] = true;

        // Include all children
        let match_depth = node.depth;
        for (offset, child) in self.all_nodes[(node_idx + 1)..].iter().enumerate() {
            if child.depth <= match_depth {
                break;
            }
            new_include[node_idx + 1 + offset] = true;
        }

        // Include all parents
        if node.depth > 0 {
            self.include_all_parents(node_idx, node.depth, new_include);
        }
    }

    /// Include all parent nodes up to the root
    fn include_all_parents(&self, node_idx: usize, target_depth: usize, new_include: &mut [bool]) {
        let mut parent_depth = target_depth.saturating_sub(1);

        for j in (0..node_idx).rev() {
            if self.all_nodes[j].depth == parent_depth {
                new_include[j] = true;
                if parent_depth == 0 {
                    break;
                }
                parent_depth = parent_depth.saturating_sub(1);
            }
        }
    }

    /// Build visible list from include filter, respecting collapse state
    fn build_visible_from_filter(&mut self, include: &[bool]) {
        let mut collapsed_below: Option<usize> = None;

        for (i, &should_include) in include.iter().enumerate() {
            if !should_include {
                continue;
            }

            let node = &self.all_nodes[i];

            // Check if we're inside a collapsed section
            if let Some(cd) = collapsed_below {
                if node.depth > cd {
                    continue;
                }
                collapsed_below = None;
            }

            self.visible.push(i);

            // If this node is collapsed, hide its children
            if node.has_children && !node.expanded {
                collapsed_below = Some(node.depth);
            }
        }
    }

    // -------------------------------------------------------------------
    // Tree navigation
    // -------------------------------------------------------------------

    pub(crate) fn toggle_expand(&mut self) {
        let Some(&idx) = self.visible.get(self.cursor) else {
            return;
        };
        if !self.all_nodes[idx].has_children {
            return;
        }
        self.all_nodes[idx].expanded = !self.all_nodes[idx].expanded;
        let old = self.cursor;
        self.rebuild_visible();
        self.cursor = old.min(self.visible.len().saturating_sub(1));
    }

    pub(crate) fn try_expand(&mut self) {
        if self.detail_focused {
            return;
        }
        let Some(&idx) = self.visible.get(self.cursor) else {
            return;
        };
        if self.all_nodes[idx].has_children && !self.all_nodes[idx].expanded {
            self.toggle_expand();
        }
    }

    pub(crate) fn try_collapse_or_parent(&mut self) {
        if self.detail_focused {
            return;
        }
        let Some(&idx) = self.visible.get(self.cursor) else {
            return;
        };
        let node = &self.all_nodes[idx];

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
            if self.all_nodes[self.visible[i]].depth < my_depth {
                self.cursor = i;
                self.detail_scroll = 0;
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
        // Toggle between sorting by ID and by name
        self.diagcomm_sort_by_id = !self.diagcomm_sort_by_id;

        // Rebuild tree nodes with new sort order
        // We need to pass the database to rebuild, but we don't have access here
        // Instead, we'll need to sort the existing nodes in place
        self.sort_diagcomm_nodes_in_place();
        self.rebuild_visible();

        self.status = if self.diagcomm_sort_by_id {
            "DiagComm sort: by ID".to_owned()
        } else {
            "DiagComm sort: by Name".to_owned()
        };
    }

    fn sort_diagcomm_nodes_in_place(&mut self) {
        // Find all "Diag-Comms" section headers and sort their children
        let mut i = 0;
        while i < self.all_nodes.len() {
            let node = &self.all_nodes[i];

            // Skip non-Diag-Comms nodes early
            if !self.is_service_list_type(node, crate::tree::ServiceListType::DiagComms) {
                i += 1;
                continue;
            }

            let section_depth = node.depth;
            let section_start = i + 1;

            // Find all children (services) of this section
            let mut section_end = section_start;
            while section_end < self.all_nodes.len()
                && self.all_nodes[section_end].depth > section_depth
            {
                section_end += 1;
            }

            // Skip if no children to sort
            if section_end <= section_start {
                i += 1;
                continue;
            }

            // Extract and sort the service nodes
            let mut services: Vec<TreeNode> =
                self.all_nodes.drain(section_start..section_end).collect();

            // Sort services based on current sort order
            match self.diagcomm_sort_by_id {
                true => services.sort_by(|a, b| {
                    let a_id = extract_service_id(&a.text);
                    let b_id = extract_service_id(&b.text);
                    a_id.cmp(&b_id)
                }),
                false => services.sort_by(|a, b| {
                    let a_name = extract_service_name(&a.text);
                    let b_name = extract_service_name(&b.text);
                    a_name.cmp(b_name)
                }),
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
                    header_node.text = format!("Diag-Comms ({})", new_count);
                }
            }

            // Re-insert sorted and deduplicated services
            self.all_nodes
                .splice(section_start..section_start, services);

            // Skip past the sorted section
            i = section_start + (section_end - section_start);
        }
    }

    pub(crate) fn toggle_table_column_sort(&mut self) {
        // Only works when detail pane is focused
        if !self.detail_focused {
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
        self.table_sort_state[section_idx] = match self.table_sort_state[section_idx] {
            Some(state) if state.column == column => {
                // Same column clicked: toggle direction
                let new_direction = match state.direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
                Some(TableSortState {
                    column,
                    direction: new_direction,
                })
            }
            _ => {
                // Different column or no sort: sort ascending by this column
                Some(TableSortState {
                    column,
                    direction: SortDirection::Ascending,
                })
            }
        };

        // Update status message
        if let Some(state) = self.table_sort_state[section_idx] {
            let direction_str = match state.direction {
                SortDirection::Ascending => "▲",
                SortDirection::Descending => "▼",
            };
            self.status = format!("Sort by column {} {}", state.column, direction_str);
        }
    }

    // -------------------------------------------------------------------
    // Navigation history
    // -------------------------------------------------------------------

    /// Add current node to navigation history (stores all_nodes index for stability)
    fn push_to_history(&mut self) {
        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };

        // Don't store duplicate consecutive entries
        if self.navigation_history.last() == Some(&node_idx) {
            return;
        }

        // Truncate forward history if not at end
        if self.history_position < self.navigation_history.len() {
            self.navigation_history.truncate(self.history_position);
        }

        self.navigation_history.push(node_idx);
        self.history_position = self.navigation_history.len();

        // Limit history size to prevent unbounded growth
        const MAX_HISTORY: usize = 100;
        if self.navigation_history.len() > MAX_HISTORY {
            self.navigation_history.remove(0);
            self.history_position = self.navigation_history.len();
        }
    }

    /// Navigate to the previous element in navigation history
    pub(crate) fn navigate_to_previous_in_history(&mut self) {
        // Need at least 2 elements (current + previous)
        if self.navigation_history.len() < 2 {
            self.status = "No previous element in history".to_owned();
            return;
        }

        if self.history_position <= 1 {
            self.status = "Already at oldest element in history".to_owned();
            return;
        }

        self.history_position -= 1;
        let target_node_idx = self.navigation_history[self.history_position - 1];

        // Ensure ancestors are expanded so the target node becomes visible
        self.ensure_node_visible(target_node_idx);

        // Find the target node in the (now-updated) visible list
        let Some(cursor_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) else {
            self.status = "Previous element no longer reachable".to_owned();
            return;
        };

        self.cursor = cursor_pos;
        self.reset_detail_state();
        self.scroll_offset = self.cursor.saturating_sub(5);
        self.detail_focused = false;
        self.status = format!("Navigated to: {}", self.all_nodes[target_node_idx].text);
    }

    /// Navigate up one level in hierarchy (parent node)
    pub(crate) fn navigate_up_one_layer(&mut self) {
        // Get the current node
        if self.cursor >= self.visible.len() {
            self.status = "No parent to navigate to".to_owned();
            return;
        }

        let node_idx = self.visible[self.cursor];
        let current_node = &self.all_nodes[node_idx];
        let current_depth = current_node.depth;

        // If we're at the root level, can't go up
        if current_depth == 0 {
            self.status = "Already at root level".to_owned();
            return;
        }

        // Find parent by looking for previous node with lower depth
        let mut found_parent = false;
        for i in (0..node_idx).rev() {
            if self.all_nodes[i].depth < current_depth {
                // Found parent node, now find it in visible list
                if let Some(visible_pos) = self.visible.iter().position(|&idx| idx == i) {
                    self.cursor = visible_pos;
                    self.reset_detail_state();
                    self.scroll_offset = self.cursor.saturating_sub(5); // Center the view
                    self.detail_focused = false;
                    self.status = format!("Navigated up to: {}", self.all_nodes[i].text);
                    found_parent = true;
                }
                break;
            }
        }

        if !found_parent {
            self.status = "Parent not visible in tree".to_owned();
        }
    }

    // -------------------------------------------------------------------
    // Cursor movement
    // -------------------------------------------------------------------

    /// Reset detail pane state when changing nodes
    fn reset_detail_state(&mut self) {
        self.detail_scroll = 0;
        self.jump_buffer.clear();
        self.jump_buffer_time = None;

        // Determine if current node is a diagcomm node
        let is_diagcomm = self
            .visible
            .get(self.cursor)
            .and_then(|&node_idx| self.all_nodes.get(node_idx))
            .map(|node| {
                matches!(
                    node.node_type,
                    NodeType::Service | NodeType::ParentRefService | NodeType::Job
                )
            })
            .unwrap_or(false);

        // Restore tab selection based on section type
        if is_diagcomm {
            self.selected_tab = self.last_diagcomm_tab;
        } else {
            self.restore_tab_from_section_type();
        }

        // Reset focus and clear per-section state
        self.focused_section = 0;
        self.focused_column = 0;
        self.section_scrolls.clear();
        self.section_cursors.clear();
        self.column_widths.clear();
        self.table_sort_state.clear();
    }

    /// Try to restore tab selection based on section type
    fn restore_tab_from_section_type(&mut self) {
        let restored = self
            .visible
            .get(self.cursor)
            .and_then(|&node_idx| self.all_nodes.get(node_idx))
            .and_then(|node| {
                let section_offset = if !node.detail_sections.is_empty()
                    && node.detail_sections[0].render_as_header
                {
                    1
                } else {
                    0
                };

                // Find first section with saved tab preference
                node.detail_sections
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx >= section_offset)
                    .find_map(|(idx, section)| {
                        self.last_section_tabs.get(&section.section_type).map(|_| {
                            self.selected_tab = idx - section_offset;
                            true
                        })
                    })
            })
            .is_some();

        if !restored {
            self.selected_tab = 0;
        }
    }

    pub(crate) fn move_up(&mut self) {
        if self.detail_focused {
            // Move cursor up in the detail pane (typically a table row)
            let section_idx = self.get_section_index();
            if section_idx < self.section_cursors.len() {
                self.section_cursors[section_idx] =
                    self.section_cursors[section_idx].saturating_sub(1);
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self.cursor.saturating_sub(1);

            // Reset detail state when moving to a different node
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn move_down(&mut self) {
        if self.detail_focused {
            // Move cursor down in the detail pane (typically a table row)
            let section_idx = self.get_section_index();
            if section_idx < self.section_cursors.len() {
                self.section_cursors[section_idx] =
                    self.section_cursors[section_idx].saturating_add(1);
            }
        } else if self.cursor + 1 < self.visible.len() {
            let old_cursor = self.cursor;
            self.cursor += 1;

            // Reset detail state when moving to a different node
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn page_up(&mut self, n: usize) {
        if self.detail_focused {
            let section_idx = self.get_section_index();
            if section_idx < self.section_cursors.len() {
                self.section_cursors[section_idx] =
                    self.section_cursors[section_idx].saturating_sub(n);
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self.cursor.saturating_sub(n);
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn page_down(&mut self, n: usize) {
        if self.detail_focused {
            let section_idx = self.get_section_index();
            if section_idx < self.section_cursors.len() {
                self.section_cursors[section_idx] =
                    self.section_cursors[section_idx].saturating_add(n);
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = (self.cursor + n).min(self.visible.len().saturating_sub(1));
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn home(&mut self) {
        if self.detail_focused {
            let section_idx = self.get_section_index();
            if section_idx < self.section_cursors.len() {
                self.section_cursors[section_idx] = 0;
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = 0;
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    pub(crate) fn end(&mut self) {
        if self.detail_focused {
            let section_idx = self.get_section_index();
            if section_idx < self.section_cursors.len() {
                self.section_cursors[section_idx] = usize::MAX; // clamped during render
            }
        } else {
            let old_cursor = self.cursor;
            self.cursor = self.visible.len().saturating_sub(1);
            if old_cursor != self.cursor {
                self.reset_detail_state();
                self.push_to_history();
            }
        }
    }

    // -------------------------------------------------------------------
    // Scroll helpers
    // -------------------------------------------------------------------

    pub(crate) fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor - viewport_height + 1;
        }
    }

    // -------------------------------------------------------------------
    // Search
    // -------------------------------------------------------------------

    pub(crate) fn update_search(&mut self) {
        if self.search.is_empty() {
            // If search is empty, don't add to stack
            self.status.clear();
        } else {
            // Add current search with its scope to stack
            self.search_stack
                .push((self.search.clone(), self.search_scope.clone()));
            self.search.clear(); // Clear for next search

            let depth = self.search_stack.len();
            let stack_display: Vec<String> = self
                .search_stack
                .iter()
                .map(|(term, scope)| format!("{}{}", term, scope.abbrev()))
                .collect();
            self.status = format!("Search depth: {} [{}]", depth, stack_display.join(" → "));
        }

        // Rebuild visible list with the search stack
        self.rebuild_visible();
        self.cursor = 0;
        self.reset_detail_state();
        self.search_matches.clear();
        self.search_match_cursor = 0;
    }

    pub(crate) fn clear_search_stack(&mut self) {
        self.search_stack.clear();
        self.search.clear();
        self.status = "Search cleared".to_owned();
        self.rebuild_visible();
        self.cursor = 0;
        self.reset_detail_state();
    }

    pub(crate) fn cycle_search_scope(&mut self) {
        self.search_scope = match self.search_scope {
            SearchScope::All => SearchScope::Variants,
            SearchScope::Variants => SearchScope::FunctionalGroups,
            SearchScope::FunctionalGroups => SearchScope::EcuSharedData,
            SearchScope::EcuSharedData => SearchScope::Services,
            SearchScope::Services => SearchScope::DiagComms,
            SearchScope::DiagComms => SearchScope::Requests,
            SearchScope::Requests => SearchScope::Responses,
            SearchScope::Responses => SearchScope::All,
            // Subtree is contextual, cycling from it resets to All
            SearchScope::Subtree { .. } => SearchScope::All,
        };

        self.status = format!("Search scope: {}", self.search_scope);
    }

    /// Set search scope to the subtree rooted at the current cursor position
    pub(crate) fn set_subtree_scope(&mut self) {
        let Some(&start_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let root_depth = self.all_nodes[start_idx].depth;
        let root_name = self.all_nodes[start_idx].text.clone();

        // Walk forward to find the last descendant
        let end_idx = self.all_nodes[start_idx + 1..]
            .iter()
            .position(|n| n.depth <= root_depth)
            .map_or(self.all_nodes.len() - 1, |offset| start_idx + offset);

        self.search_scope = SearchScope::Subtree {
            start_idx,
            end_idx,
            root_name: root_name.clone(),
        };
        self.status = format!("Search scope: subtree '{root_name}'");
    }

    pub(crate) fn toggle_mouse_mode(&mut self) {
        self.mouse_enabled = !self.mouse_enabled;

        // Actually enable/disable mouse capture in the terminal
        let result = if self.mouse_enabled {
            execute!(std::io::stdout(), EnableMouseCapture)
        } else {
            execute!(std::io::stdout(), DisableMouseCapture)
        };

        if result.is_ok() {
            self.status = format!(
                "Mouse: {}",
                if self.mouse_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        } else {
            self.status = "Failed to toggle mouse mode".to_owned();
        }
    }

    pub(crate) fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_cursor = (self.search_match_cursor + 1) % self.search_matches.len();
        self.cursor = self.search_matches[self.search_match_cursor];
        self.reset_detail_state();
        self.status = format!(
            "Match {}/{}",
            self.search_match_cursor + 1,
            self.search_matches.len()
        );
    }

    pub(crate) fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_cursor = if self.search_match_cursor == 0 {
            self.search_matches.len() - 1
        } else {
            self.search_match_cursor - 1
        };
        self.cursor = self.search_matches[self.search_match_cursor];
        self.reset_detail_state();
        self.status = format!(
            "Match {}/{}",
            self.search_match_cursor + 1,
            self.search_matches.len()
        );
    }

    /// Handle Enter key press when detail pane is focused
    pub(crate) fn handle_enter_in_detail_pane(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Early returns for different node types using functional matching
        if let Some(SectionType::Variants) = node.section_type {
            self.try_navigate_to_variant();
            return;
        }

        if matches!(node.node_type, NodeType::Container) && node.depth == 1 {
            self.try_navigate_from_variant_overview();
            return;
        }

        if node.service_list_type.is_some() {
            self.try_navigate_to_service();
            return;
        }

        if matches!(node.node_type, NodeType::FunctionalClass) {
            self.handle_functional_class_enter();
            return;
        }

        // DIAG-DATA-DICTIONARY-SPEC, DOP category, and individual DOP nodes with children:
        // navigate to child instead of popup
        if matches!(node.node_type, NodeType::DOP)
            || self.is_dop_category_node(node_idx)
            || self.is_individual_dop_node(node_idx)
        {
            self.try_navigate_to_dop_child();
            return;
        }

        if matches!(
            node.node_type,
            NodeType::Service
                | NodeType::ParentRefService
                | NodeType::Request
                | NodeType::PosResponse
                | NodeType::NegResponse
        ) {
            self.handle_service_node_enter();
            return;
        }

        // Handle other node types with detail sections
        self.handle_generic_detail_enter();
    }

    /// Handle Enter key for functional class nodes
    fn handle_functional_class_enter(&mut self) {
        match self.focused_column {
            0 => self.try_navigate_to_service_from_functional_class(),
            5 => self.try_navigate_to_layer_from_functional_class(),
            _ => {}
        }
    }

    /// Handle Enter key for service nodes
    fn handle_service_node_enter(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Check for parameter table (requests/responses)
        if matches!(
            section.section_type,
            DetailSectionType::Requests
                | DetailSectionType::PosResponses
                | DetailSectionType::NegResponses
        ) {
            self.try_navigate_from_param_table();
            return;
        }

        // Check for Overview section with "Inherited From" row
        if section.section_type == DetailSectionType::Overview
            && let crate::tree::DetailContent::Table { rows, .. } = &section.content
        {
            let row_cursor = self.section_cursors.get(section_idx).copied().unwrap_or(0);
            let sorted_rows = self.apply_table_sort(rows, section_idx);

            if let Some(selected_row) = sorted_rows.get(row_cursor)
                && selected_row.row_type == DetailRowType::InheritedFrom
            {
                self.try_navigate_to_inherited_parent();
                return;
            }
        }

        // Try to navigate based on the current cell content
        self.try_navigate_from_detail_row();
    }

    /// Handle Enter key for generic nodes with detail sections
    fn handle_generic_detail_enter(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];
        let section_idx = self.get_section_index();

        let section = node
            .detail_sections
            .get(section_idx)
            .filter(|_| section_idx < node.detail_sections.len());

        if let Some(section) = section {
            if section.section_type == DetailSectionType::RelatedRefs
                && section.title == "Parent References"
            {
                self.try_navigate_to_parent_ref();
            } else if section.title.starts_with("Not Inherited") {
                self.try_navigate_to_not_inherited_element();
            } else {
                self.try_navigate_from_detail_row();
            }
        } else {
            self.status = "No details available".to_owned();
        }
    }

    /// Try to navigate to the item referenced by the currently focused cell.
    /// Falls back to searching the tree for a node matching the cell text.
    fn try_navigate_from_detail_row(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];
        let section_idx = self.get_section_index();

        let (rows, use_row_selection) = match self.get_table_rows(node, section_idx) {
            Some(data) => data,
            None => {
                self.status = "No table data".to_owned();
                return;
            }
        };

        let row_cursor = self.section_cursors.get(section_idx).copied().unwrap_or(0);
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let Some(selected_row) = sorted_rows.get(row_cursor) else {
            return;
        };

        let focused_col = self.get_focused_column(use_row_selection, &selected_row.cell_types);
        let cell_type = selected_row
            .cell_types
            .get(focused_col)
            .cloned()
            .unwrap_or(CellType::Text);
        let cell_value = selected_row
            .cells
            .get(focused_col)
            .map(|s| s.as_str())
            .unwrap_or("");

        if cell_value.is_empty() || cell_value == "-" {
            self.status = "Empty cell".to_owned();
            return;
        }

        // Handle typed cells
        match cell_type {
            CellType::DopReference => {
                self.navigate_to_dop(cell_value);
                return;
            }
            CellType::ParameterName => {
                self.navigate_to_parameter(selected_row);
                return;
            }
            CellType::Text | CellType::NumericValue => {}
        }

        // Try to find a matching tree node by text
        let found_idx = self.all_nodes.iter().position(|n| n.text == cell_value);
        if let Some(idx) = found_idx {
            self.navigate_to_node(idx);
        } else {
            self.status = "This cell is not navigable".to_owned();
        }
    }

    /// Navigate to a service in the tree from a service list table
    /// (Diag-Comms, Requests, Responses)
    pub(crate) fn try_navigate_to_service(&mut self) {
        // Early validation
        if self.cursor >= self.visible.len() {
            self.status = "Cursor out of bounds".to_string();
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        if !self.is_service_list_section(node) {
            self.status = "Not a service list section".to_owned();
            return;
        }

        // Get service name from table or return early
        let service_name = match self.extract_service_name_from_table(node_idx) {
            Some(name) => name,
            None => return,
        };

        // Expand service list section if collapsed
        if !self.all_nodes[node_idx].expanded {
            self.expand_and_update_cursor(node_idx);
        }

        // Find and navigate to service
        self.find_and_navigate_to_service(&service_name, node_idx);
    }

    /// Extract service name from the current table row
    fn extract_service_name_from_table(&mut self, node_idx: usize) -> Option<String> {
        let node = &self.all_nodes[node_idx];
        let section = node.detail_sections.first()?;

        let rows = match &section.content {
            crate::tree::DetailContent::Table { rows, .. } => rows,
            _ => {
                self.status = "Details should be a table".to_owned();
                return None;
            }
        };

        let section_index = self.get_section_index();
        let row_cursor = *self.section_cursors.get(section_index)?;
        let sorted_rows = self.apply_table_sort(rows, section_index);
        let selected_row = sorted_rows.get(row_cursor)?;

        // Determine name column index based on node type
        let is_functional_class =
            self.is_service_list_type(node, crate::tree::ServiceListType::FunctionalClasses);
        let name_column_index = if is_functional_class { 0 } else { 1 };

        selected_row.cells.get(name_column_index).cloned()
    }

    /// Expand section and update cursor position
    fn expand_and_update_cursor(&mut self, node_idx: usize) {
        self.all_nodes[node_idx].expanded = true;
        self.rebuild_visible();

        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == node_idx) {
            self.cursor = new_cursor;
        }
    }

    /// Find and navigate to a service by name
    fn find_and_navigate_to_service(&mut self, service_name: &str, parent_node_idx: usize) {
        let parent_depth = self.all_nodes[parent_node_idx].depth;
        let is_functional_class = self.is_service_list_type(
            &self.all_nodes[parent_node_idx],
            crate::tree::ServiceListType::FunctionalClasses,
        );

        // Find service in visible nodes after parent
        let found_idx = self.visible[self.cursor + 1..]
            .iter()
            .copied()
            .take_while(|&vis_idx| self.all_nodes[vis_idx].depth > parent_depth)
            .filter(|&vis_idx| self.all_nodes[vis_idx].depth == parent_depth + 1)
            .find(|&vis_idx| {
                self.node_matches_service_name(
                    &self.all_nodes[vis_idx],
                    service_name,
                    is_functional_class,
                )
            })
            .and_then(|vis_idx| self.visible.iter().position(|&idx| idx == vis_idx));

        match found_idx {
            Some(target_cursor) => {
                self.push_to_history();
                self.detail_focused = false;
                self.cursor = target_cursor;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5);
            }
            None => {
                let item_type = if is_functional_class {
                    "Functional class"
                } else {
                    "Service"
                };
                self.status = format!("{} '{}' not found in tree", item_type, service_name);
            }
        }
    }

    /// Check if a node's name matches the target service name
    fn node_matches_service_name(
        &self,
        node: &TreeNode,
        target_name: &str,
        is_functional_class: bool,
    ) -> bool {
        if is_functional_class {
            node.node_type == NodeType::FunctionalClass && node.text == target_name
        } else {
            let is_target_node = matches!(
                node.node_type,
                NodeType::Service
                    | NodeType::ParentRefService
                    | NodeType::Request
                    | NodeType::PosResponse
                    | NodeType::NegResponse
                    | NodeType::Job
            );

            if !is_target_node {
                return false;
            }

            if node.node_type == NodeType::Job {
                let job_name = node.text.strip_prefix("[Job] ").unwrap_or(&node.text);
                job_name == target_name
            } else {
                node.text.contains(target_name)
            }
        }
    }

    /// Navigate to an inherited parent layer in the tree
    pub(crate) fn try_navigate_to_inherited_parent(&mut self) {
        // Early validations
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        if !self.is_service_node(node) {
            self.status = "Not a service node".to_owned();
            return;
        }

        // Extract current service name and parent layer name
        let current_service_name = self.extract_service_name_from_node(node);
        let parent_layer_name = match self.get_parent_layer_name(node_idx) {
            Some(name) => name,
            None => return,
        };

        // Find parent container and navigate
        if let Some(container_idx) = self.find_container_by_name(&parent_layer_name) {
            self.navigate_to_parent_service(container_idx, &current_service_name);
        } else {
            self.status = format!("Parent layer '{}' not found in tree", parent_layer_name);
        }
    }

    /// Check if node is a service-related node
    fn is_service_node(&self, node: &TreeNode) -> bool {
        matches!(
            node.node_type,
            NodeType::Service
                | NodeType::ParentRefService
                | NodeType::Request
                | NodeType::PosResponse
                | NodeType::NegResponse
        )
    }

    /// Extract service name from node text
    fn extract_service_name_from_node(&self, node: &TreeNode) -> String {
        node.text
            .find(" - ")
            .map(|dash_idx| node.text[dash_idx + 3..].to_string())
            .unwrap_or_else(|| node.text.clone())
    }

    /// Get parent layer name from the Overview section's "Inherited From" row
    fn get_parent_layer_name(&self, node_idx: usize) -> Option<String> {
        let node = &self.all_nodes[node_idx];

        let overview_idx =
            if node.detail_sections.len() > 1 && node.detail_sections[0].render_as_header {
                1
            } else {
                0
            };

        let overview_section = node.detail_sections.get(overview_idx)?;

        let rows = match &overview_section.content {
            crate::tree::DetailContent::Table { rows, .. } => rows,
            _ => return None,
        };

        let row_cursor = self.section_cursors.get(overview_idx).copied().unwrap_or(0);
        let sorted_rows = self.apply_table_sort(rows, overview_idx);
        let selected_row = sorted_rows.get(row_cursor)?;

        if selected_row.row_type != DetailRowType::InheritedFrom {
            return None;
        }

        // Extract from metadata or fallback to cell data
        match &selected_row.metadata {
            Some(RowMetadata::InheritedFrom { layer_name }) => Some(layer_name.clone()),
            _ => selected_row.cells.get(1).cloned(),
        }
    }

    /// Navigate to parent service in the container
    fn navigate_to_parent_service(&mut self, container_idx: usize, service_name: &str) {
        // Expand ancestors and container
        self.expand_node_ancestors(container_idx);

        if self.all_nodes[container_idx].has_children {
            self.all_nodes[container_idx].expanded = true;
        }

        // Find Diag-Comms section
        let Some(dc_idx) = self.find_diagcomm_section(container_idx) else {
            self.rebuild_visible();
            self.navigate_to_node_by_idx(container_idx);
            return;
        };

        self.all_nodes[dc_idx].expanded = true;
        self.rebuild_visible();

        let target = self
            .find_service_in_diagcomm(dc_idx, service_name)
            .unwrap_or(container_idx);
        self.navigate_to_node_by_idx(target);
    }

    /// Expand all ancestors of a node
    fn expand_node_ancestors(&mut self, node_idx: usize) {
        let target_depth = self.all_nodes[node_idx].depth;

        if target_depth == 0 {
            return;
        }

        for i in 0..node_idx {
            if self.all_nodes[i].depth < target_depth && self.all_nodes[i].has_children {
                self.all_nodes[i].expanded = true;
            }
        }
    }

    /// Find the Diag-Comms section within a container
    fn find_diagcomm_section(&self, container_idx: usize) -> Option<usize> {
        let container_depth = self.all_nodes[container_idx].depth;

        self.all_nodes[(container_idx + 1)..]
            .iter()
            .enumerate()
            .take_while(|(_, child)| child.depth > container_depth)
            .find(|(_, child)| {
                child.depth == container_depth + 1
                    && self.is_service_list_type(child, crate::tree::ServiceListType::DiagComms)
            })
            .map(|(offset, _)| container_idx + 1 + offset)
    }

    /// Find a service by name within a Diag-Comms section
    fn find_service_in_diagcomm(&self, diagcomm_idx: usize, service_name: &str) -> Option<usize> {
        let diagcomm_depth = self.all_nodes[diagcomm_idx].depth;

        self.all_nodes[(diagcomm_idx + 1)..]
            .iter()
            .enumerate()
            .take_while(|(_, node)| node.depth > diagcomm_depth)
            .filter(|(_, node)| node.depth == diagcomm_depth + 1)
            .find(|(_, node)| node.text.contains(service_name))
            .map(|(offset, _)| diagcomm_idx + 1 + offset)
    }

    /// Navigate to a node by its index in all_nodes
    fn navigate_to_node_by_idx(&mut self, target_idx: usize) {
        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == target_idx) {
            self.push_to_history();
            self.detail_focused = false;
            self.cursor = new_cursor;
            self.reset_detail_state();
            self.scroll_offset = self.cursor.saturating_sub(5);
        }
    }

    /// Find a container (variant/functional group) by name
    fn find_container_by_name(&self, name: &str) -> Option<usize> {
        self.all_nodes.iter().position(|node| {
            if !matches!(node.node_type, NodeType::Container) {
                return false;
            }

            let node_name = node
                .text
                .find(" [")
                .map(|idx| &node.text[..idx])
                .unwrap_or(&node.text);

            node_name == name
        })
    }

    /// Navigate from a parameter table (Request/Response) based on the focused cell
    /// If focused on DOP column, show DOP details popup
    /// If focused on Parameter name column, navigate to that parameter node
    pub(crate) fn try_navigate_from_param_table(&mut self) {
        // Early validation
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];
        let section_idx = self.get_section_index();

        // Get table data or return early
        let (rows, use_row_selection) = match self.get_table_rows(node, section_idx) {
            Some(data) => data,
            None => return,
        };

        let row_cursor = self.section_cursors.get(section_idx).copied().unwrap_or(0);
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        let selected_row = match sorted_rows.get(row_cursor) {
            Some(row) => row,
            None => return,
        };

        // Determine focused column and cell
        let focused_col = self.get_focused_column(use_row_selection, &selected_row.cell_types);
        let cell_type = selected_row
            .cell_types
            .get(focused_col)
            .cloned()
            .unwrap_or(CellType::Text);
        let cell_value = selected_row
            .cells
            .get(focused_col)
            .map(|s| s.as_str())
            .unwrap_or("");

        if cell_value.is_empty() {
            self.status = "Empty cell".to_owned();
            return;
        }

        // Handle different cell types
        match cell_type {
            CellType::DopReference => self.navigate_to_dop(cell_value),
            CellType::ParameterName => {
                let node_type = self.all_nodes[node_idx].node_type.clone();
                match node_type {
                    // DiagComm view: navigate to the matching Request/Response service node
                    NodeType::Service | NodeType::ParentRefService => {
                        self.navigate_to_request_response_counterpart(node_idx, section_idx);
                    }
                    // Already on a Request/Response node: navigate to parameter child
                    NodeType::Request | NodeType::PosResponse | NodeType::NegResponse => {
                        self.navigate_to_parameter(selected_row);
                    }
                    NodeType::Container
                    | NodeType::SectionHeader
                    | NodeType::ParentRefs
                    | NodeType::FunctionalClass
                    | NodeType::Job
                    | NodeType::DOP
                    | NodeType::SDG
                    | NodeType::Default => {
                        self.status = "Parameter navigation not available here".to_owned();
                    }
                }
            }
            CellType::Text | CellType::NumericValue => {
                self.status = "This cell is not navigable".to_owned();
            }
        }
    }

    /// Get table rows from section
    fn get_table_rows<'a>(
        &'a self,
        node: &'a TreeNode,
        section_idx: usize,
    ) -> Option<(&'a Vec<DetailRow>, bool)> {
        let section = node.detail_sections.get(section_idx)?;

        match &section.content {
            DetailContent::Table {
                rows,
                use_row_selection,
                ..
            } => Some((rows, *use_row_selection)),
            _ => None,
        }
    }

    /// Navigate from a DiagComm service to the corresponding Request/Response node.
    /// The DiagComm node text has a `[Service] ` prefix that the counterpart lacks.
    fn navigate_to_request_response_counterpart(&mut self, node_idx: usize, section_idx: usize) {
        let service_name = self.all_nodes[node_idx]
            .text
            .strip_prefix("[Service] ")
            .unwrap_or(&self.all_nodes[node_idx].text)
            .to_owned();

        let section_type = self.all_nodes[node_idx]
            .detail_sections
            .get(section_idx)
            .map(|s| s.section_type);

        let Some(target_node_type) = section_type.and_then(|st| match st {
            DetailSectionType::Requests => Some(NodeType::Request),
            DetailSectionType::PosResponses => Some(NodeType::PosResponse),
            DetailSectionType::NegResponses => Some(NodeType::NegResponse),
            DetailSectionType::Header
            | DetailSectionType::Overview
            | DetailSectionType::Services
            | DetailSectionType::ComParams
            | DetailSectionType::States
            | DetailSectionType::RelatedRefs
            | DetailSectionType::FunctionalClass
            | DetailSectionType::Custom => None,
        }) else {
            self.status = "Cannot navigate from this section type".to_owned();
            return;
        };

        let found = self
            .all_nodes
            .iter()
            .position(|n| n.node_type == target_node_type && n.text == service_name);

        match found {
            Some(idx) => self.navigate_to_node(idx),
            None => {
                self.status = format!("No matching node found for {service_name}");
            }
        }
    }

    /// Determine which column is focused based on selection mode
    fn get_focused_column(&self, use_row_selection: bool, cell_types: &[CellType]) -> usize {
        if use_row_selection {
            // In row selection mode, prioritize DOP column (6), then param name (0)
            if cell_types.get(6) == Some(&CellType::DopReference) {
                6
            } else {
                0
            }
        } else {
            // In cell selection mode, use actual focused column
            self.focused_column.min(cell_types.len().saturating_sub(1))
        }
    }

    /// Navigate to a DOP node by name
    fn navigate_to_dop(&mut self, dop_name: &str) {
        let found_idx = self.all_nodes.iter().position(|node| node.text == dop_name);

        match found_idx {
            Some(dop_idx) => {
                self.navigate_to_node(dop_idx);
                self.status = format!("Navigated to DOP: {}", dop_name);
            }
            None => {
                self.status = format!("DOP '{}' not found in tree", dop_name);
            }
        }
    }

    /// Navigate to a parameter node using param ID from metadata
    fn navigate_to_parameter(&mut self, selected_row: &DetailRow) {
        // Extract param ID from metadata
        let param_id = match &selected_row.metadata {
            Some(crate::tree::RowMetadata::ParameterRow { param_id }) => *param_id,
            _ => {
                self.status = "No parameter ID in metadata".to_owned();
                return;
            }
        };

        // Find parameter node by ID
        let param_idx = match self.find_param_by_id(param_id) {
            Some(idx) => idx,
            None => {
                self.status = format!("Parameter with ID {} not found", param_id);
                return;
            }
        };

        // Expand ancestors to make parameter visible
        self.ensure_node_visible(param_idx);

        // Navigate to parameter
        if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == param_idx) {
            self.push_to_history();
            self.detail_focused = false;
            self.cursor = new_cursor;
            self.reset_detail_state();
            self.scroll_offset = self.cursor.saturating_sub(5);
            self.status = format!("Navigated to parameter (ID: {})", param_id);
        } else {
            self.status = format!("Parameter found but not visible (ID: {})", param_id);
        }
    }

    /// Find parameter node by param_id
    fn find_param_by_id(&self, param_id: u32) -> Option<usize> {
        self.all_nodes
            .iter()
            .position(|node| node.param_id == Some(param_id))
    }

    /// Navigate to a parent ref element from the Parent References table
    pub(crate) fn try_navigate_to_parent_ref(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // Extract the short name from the first column (was second before reordering)
        if selected_row.cells.len() < 2 {
            self.status = "Invalid parent ref row".to_owned();
            return;
        }

        let target_short_name = selected_row.cells[0].clone();

        // Search for a node in the tree that matches this short name
        // We need to find Containers (variants, functional groups, ECU shared data, protocols)
        let mut found_container_idx: Option<usize> = None;

        for (ni, n) in self.all_nodes.iter().enumerate() {
            // Check if this is a Container with matching name
            if matches!(n.node_type, NodeType::Container) {
                // Extract name from text (may include [base] suffix for variants)
                let name = if let Some(idx) = n.text.find(" [") {
                    &n.text[..idx]
                } else {
                    &n.text
                };

                if name == target_short_name {
                    found_container_idx = Some(ni);
                    break;
                }
            }
        }

        if let Some(container_node_idx) = found_container_idx {
            // Expand all parents of the target node to make it visible
            let container_depth = self.all_nodes[container_node_idx].depth;

            // Expand parent nodes
            if container_depth > 0 {
                for i in 0..container_node_idx {
                    let node = &mut self.all_nodes[i];
                    if node.depth < container_depth && node.has_children {
                        node.expanded = true;
                    }
                }
            }

            // Expand the container node itself if it has children
            if self.all_nodes[container_node_idx].has_children {
                self.all_nodes[container_node_idx].expanded = true;
            }

            // Rebuild visible list
            self.rebuild_visible();

            // Navigate to the container
            if let Some(new_cursor) = self
                .visible
                .iter()
                .position(|&idx| idx == container_node_idx)
            {
                self.push_to_history();
                self.detail_focused = false;
                self.cursor = new_cursor;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5);
                self.status = format!("Navigated to: {}", target_short_name);
            }
        } else {
            self.status = format!("Element '{}' not found in tree", target_short_name);
        }
    }

    /// Navigate to a not-inherited element (DiagComm, DiagVariable, Dop, Table)
    pub(crate) fn try_navigate_to_not_inherited_element(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Determine what type of element we're looking for based on the section title
        let element_type = if section.title.contains("DiagComms") {
            "service"
        } else {
            // For now, only services (DiagComms) are navigable
            // TODO: Add navigation for DiagVariables, DOPs,
            // and Tables when they're added to the tree
            self.status = "Navigation not yet supported for this element type".to_owned();
            return;
        };

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // Extract the element short name from the first column
        if selected_row.cells.is_empty() {
            self.status = "Invalid row".to_owned();
            return;
        }

        let target_short_name = selected_row.cells[0].clone();

        // Search for the element in the tree based on type
        if element_type == "service" {
            // Search for a Service or ParentRefService node with matching name
            let mut found_service_idx: Option<usize> = None;

            for (ni, n) in self.all_nodes.iter().enumerate() {
                if matches!(n.node_type, NodeType::Service | NodeType::ParentRefService) {
                    // Service nodes have format "0xXXXX - ShortName"
                    let service_name = if let Some(dash_idx) = n.text.find(" - ") {
                        &n.text[dash_idx + 3..]
                    } else {
                        &n.text
                    };

                    if service_name == target_short_name {
                        found_service_idx = Some(ni);
                        break;
                    }
                }
            }

            if let Some(service_node_idx) = found_service_idx {
                // Expand all parents of the target node to make it visible
                let service_depth = self.all_nodes[service_node_idx].depth;

                // Expand parent nodes
                if service_depth > 0 {
                    for i in 0..service_node_idx {
                        let node = &mut self.all_nodes[i];
                        if node.depth < service_depth && node.has_children {
                            node.expanded = true;
                        }
                    }
                }

                // Rebuild visible list
                self.rebuild_visible();

                // Navigate to the service
                if let Some(new_cursor) =
                    self.visible.iter().position(|&idx| idx == service_node_idx)
                {
                    self.push_to_history();
                    self.detail_focused = false;
                    self.cursor = new_cursor;
                    self.reset_detail_state();
                    self.scroll_offset = self.cursor.saturating_sub(5);
                    self.status = format!("Navigated to service: {}", target_short_name);
                }
            } else {
                self.status = format!("Service '{}' not found in tree", target_short_name);
            }
        }
    }

    /// Navigate to a layer from functional class detail view
    /// The layer name is extracted from the "Layer" column of the selected row
    /// Navigate to a service or job from a functional class detail view
    /// Uses the ShortName column (column 0) to find the target
    pub(crate) fn try_navigate_to_service_from_functional_class(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Verify we're on a functional class node
        if !matches!(node.node_type, NodeType::FunctionalClass) {
            self.status = "Not a functional class node".to_owned();
            return;
        }

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // We should be in a Services section
        if section.section_type != DetailSectionType::Services {
            self.status = "Not in a services section".to_owned();
            return;
        }

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // ShortName is in column 0
        if selected_row.cells.is_empty() {
            self.status = "Invalid row structure".to_owned();
            return;
        }

        let target_short_name = selected_row.cells[0].clone();

        // Search for the service/job in the tree
        self.navigate_to_service_or_job(&target_short_name);
    }

    pub(crate) fn try_navigate_to_layer_from_functional_class(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Verify we're on a functional class node
        if !matches!(node.node_type, NodeType::FunctionalClass) {
            self.status = "Not a functional class node".to_owned();
            return;
        }

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // We should be in a Services section (the table showing services for this functional class)
        if section.section_type != DetailSectionType::Services {
            self.status = "Not in a services section".to_owned();
            return;
        }

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // The table has columns: ShortName | Type | SID_RQ | Semantic | Addressing | Layer
        // Layer name is in column 5 (index 5)
        let layer_column_index = 5;

        if selected_row.cells.len() <= layer_column_index {
            self.status = "Invalid row structure".to_owned();
            return;
        }

        let layer_name = selected_row.cells[layer_column_index].clone();

        // Search for a layer node in the tree with matching name
        // The layer name in the table is just the short name (e.g., "IDC_GEN6_C_17.00.09")
        // In the tree, variant nodes are at depth 1 under "Variants" section
        // and have text format: "short_name" or "short_name [base]"
        let mut found_layer_idx: Option<usize> = None;

        for (ni, n) in self.all_nodes.iter().enumerate() {
            // Look for Container nodes (variants) that have the matching layer name
            // The node text is either exactly the layer_name or "layer_name [base]"
            if n.node_type == NodeType::Container {
                // Strip any suffix like " [base]" before comparing
                let node_name = if let Some(idx) = n.text.find(" [") {
                    &n.text[..idx]
                } else {
                    &n.text
                };

                if node_name == layer_name {
                    found_layer_idx = Some(ni);
                    break;
                }
            }
        }

        if let Some(layer_node_idx) = found_layer_idx {
            // Expand all parents of the target node to make it visible
            let layer_depth = self.all_nodes[layer_node_idx].depth;

            // Expand parent nodes
            if layer_depth > 0 {
                for i in 0..layer_node_idx {
                    let node = &mut self.all_nodes[i];
                    if node.depth < layer_depth && node.has_children {
                        node.expanded = true;
                    }
                }
            }

            // Rebuild visible list
            self.rebuild_visible();

            // Navigate to the layer
            if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == layer_node_idx) {
                self.push_to_history();
                self.detail_focused = false;
                self.cursor = new_cursor;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5);
                self.status = format!("Navigated to layer: {}", layer_name);
            }
        } else {
            self.status = format!("Layer '{}' not found in tree", layer_name);
        }
    }

    /// Navigate to a variant from the Variants overview table
    pub(crate) fn try_navigate_to_variant(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // Extract the variant name from the first column
        if selected_row.cells.is_empty() {
            self.status = "Invalid variant row".to_owned();
            return;
        }

        let target_variant_name = selected_row.cells[0].clone();

        // Search for a variant node in the tree that matches this name
        // Variant nodes are at depth 1 under "Variants" section
        // and have text format: "variant_name" or "variant_name [base]"
        let mut found_variant_idx: Option<usize> = None;

        for (ni, n) in self.all_nodes.iter().enumerate() {
            // Check if this is a variant Container node
            if matches!(n.node_type, NodeType::Container) && n.depth == 1 {
                // Extract name from text (may include [base] suffix)
                let name = if let Some(idx) = n.text.find(" [") {
                    &n.text[..idx]
                } else {
                    &n.text
                };

                if name == target_variant_name {
                    found_variant_idx = Some(ni);
                    break;
                }
            }
        }

        if let Some(variant_node_idx) = found_variant_idx {
            // Ensure the variant node is visible (expand parents if needed)
            self.ensure_node_visible(variant_node_idx);

            // Find the variant in the visible list and navigate to it
            if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == variant_node_idx) {
                self.push_to_history();
                self.detail_focused = false;
                self.cursor = new_cursor;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5);
                self.status = format!("Navigated to variant: {}", target_variant_name);
            }
        } else {
            self.status = format!("Variant '{}' not found in tree", target_variant_name);
        }
    }

    /// Navigate from variant overview to a child element
    pub(crate) fn try_navigate_from_variant_overview(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Only handle Overview section type
        if section.section_type != DetailSectionType::Overview {
            return;
        }

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // Check if this row is a child element row with metadata
        if let Some(RowMetadata::ChildElement { element_type }) = &selected_row.metadata {
            // Find the child section node under the current variant
            // It should be a direct child (depth = current + 1) of the current node
            let current_depth = node.depth;
            let target_depth = current_depth + 1;

            // Start searching after the current node
            let mut target_idx: Option<usize> = None;
            for i in (node_idx + 1)..self.all_nodes.len() {
                let child_node = &self.all_nodes[i];

                // Stop if we've moved past this variant's children
                if child_node.depth <= current_depth {
                    break;
                }

                // Check if this is the target child at the correct depth
                if child_node.depth == target_depth
                    && element_type.matches_node_text(&child_node.text)
                {
                    target_idx = Some(i);
                    break;
                }
            }

            if let Some(target_node_idx) = target_idx {
                // Ensure the target node is visible (expand if needed)
                self.ensure_node_visible(target_node_idx);

                // Find the target in the visible list and navigate to it
                if let Some(new_cursor) =
                    self.visible.iter().position(|&idx| idx == target_node_idx)
                {
                    self.push_to_history();
                    self.detail_focused = false;
                    self.cursor = new_cursor;
                    self.reset_detail_state();
                    self.scroll_offset = self.cursor.saturating_sub(5);
                    self.status = format!("Navigated to: {}", element_type.display_name());
                }
            } else {
                self.status = format!("Section '{}' not found", element_type.display_name());
            }
        }
    }

    /// Navigate from DIAG-DATA-DICTIONARY-SPEC or DOP category overview to a child node.
    /// For DIAG-DATA-DICTIONARY-SPEC: rows are categories
    /// like "DTC-DOPS", navigates to the category child node.
    /// For DOP category nodes: rows are individual DOPs, navigates to the DOP child node.
    fn try_navigate_to_dop_child(&mut self) {
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Get the actual section index
        let section_idx = self.get_section_index();

        if section_idx >= node.detail_sections.len() {
            return;
        }

        let section = &node.detail_sections[section_idx];

        // Only handle Overview section type
        if section.section_type != DetailSectionType::Overview {
            return;
        }

        // Get table rows
        use crate::tree::DetailContent;
        let rows = match &section.content {
            DetailContent::Table { rows, .. } => rows,
            _ => return,
        };

        // Get the selected row cursor
        let row_cursor = if section_idx < self.section_cursors.len() {
            self.section_cursors[section_idx]
        } else {
            return;
        };

        // Apply sorting if active for this section
        let sorted_rows = self.apply_table_sort(rows, section_idx);

        if row_cursor >= sorted_rows.len() {
            return;
        }

        let selected_row = &sorted_rows[row_cursor];

        // Get the first cell text (category name or DOP name)
        let target_name = match selected_row.cells.first() {
            Some(name) if !name.is_empty() => name.clone(),
            _ => return,
        };

        // Find the child node that matches the target name
        let current_depth = node.depth;
        let target_depth = current_depth + 1;

        let mut target_idx: Option<usize> = None;
        for i in (node_idx + 1)..self.all_nodes.len() {
            let child_node = &self.all_nodes[i];

            // Stop if we've moved past this node's children
            if child_node.depth <= current_depth {
                break;
            }

            // Check if this is a direct child that starts with the target name
            if child_node.depth == target_depth && child_node.text.starts_with(&target_name) {
                target_idx = Some(i);
                break;
            }
        }

        if let Some(target_node_idx) = target_idx {
            // Ensure the target node is visible (expand if needed)
            self.ensure_node_visible(target_node_idx);

            // Find the target in the visible list and navigate to it
            if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == target_node_idx) {
                self.push_to_history();
                self.detail_focused = false;
                self.cursor = new_cursor;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5);
                self.status = format!("Navigated to: {}", target_name);
            }
        } else {
            self.status = format!("'{}' not found", target_name);
        }
    }

    /// Helper function to navigate to a service or job by name
    fn navigate_to_service_or_job(&mut self, target_short_name: &str) {
        // Search for a Service, ParentRefService, or Job node with matching name
        let mut found_service_idx: Option<usize> = None;

        for (ni, n) in self.all_nodes.iter().enumerate() {
            let matches = if matches!(n.node_type, NodeType::Service | NodeType::ParentRefService) {
                // Service nodes have format "0xXXXX - ShortName"
                let service_name = if let Some(dash_idx) = n.text.find(" - ") {
                    &n.text[dash_idx + 3..]
                } else {
                    &n.text
                };
                service_name == target_short_name
            } else if n.node_type == NodeType::Job {
                // Job nodes have format "[Job] ShortName"
                let job_name = n.text.strip_prefix("[Job] ").unwrap_or(&n.text);
                job_name == target_short_name
            } else {
                false
            };

            if matches {
                found_service_idx = Some(ni);
                break;
            }
        }

        if let Some(service_node_idx) = found_service_idx {
            // Expand all parents of the target node to make it visible
            let service_depth = self.all_nodes[service_node_idx].depth;

            // Expand parent nodes
            if service_depth > 0 {
                for i in 0..service_node_idx {
                    let node = &mut self.all_nodes[i];
                    if node.depth < service_depth && node.has_children {
                        node.expanded = true;
                    }
                }
            }

            // Rebuild visible list
            self.rebuild_visible();

            // Navigate to the service/job
            if let Some(new_cursor) = self.visible.iter().position(|&idx| idx == service_node_idx) {
                self.push_to_history();
                self.detail_focused = false;
                self.cursor = new_cursor;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5);
                self.status = format!("Navigated to: {}", target_short_name);
            }
        } else {
            self.status = format!("Service/Job '{}' not found in tree", target_short_name);
        }
    }

    pub(crate) fn resize_column(&mut self, delta: i16) {
        // Get the actual section index accounting for header sections
        let section_idx = self.get_section_index();

        // Ensure we have column_widths entries for all sections
        while self.column_widths.len() <= section_idx {
            self.column_widths.push(Vec::new());
        }

        if self.cursor >= self.visible.len() {
            return;
        }
        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];
        if section_idx >= node.detail_sections.len() {
            return;
        }
        let section = &node.detail_sections[section_idx];
        use crate::tree::DetailContent;
        let constraints = match &section.content {
            DetailContent::Table { constraints, .. } => constraints,
            _ => {
                self.status = "Column resizing only available in tables".to_owned();
                return;
            }
        };

        // Initialize column widths from constraints if not already done
        if self.column_widths[section_idx].is_empty() {
            // First pass: convert to initial widths
            let mut widths: Vec<u16> = constraints
                .iter()
                .map(|c| match c {
                    crate::tree::ColumnConstraint::Fixed(w) => {
                        // Convert fixed width to a reasonable percentage (roughly 1.5% per char)
                        (*w * 3 / 2).clamp(3, 15)
                    }
                    crate::tree::ColumnConstraint::Percentage(p) => *p,
                })
                .collect();

            // Normalize to ensure total is exactly 100%
            let total: u16 = widths.iter().sum();
            if total > 0 && total != 100 {
                // Scale all widths proportionally to sum to 100
                widths = widths
                    .iter()
                    .map(|&w| ((w as f32 / total as f32) * 100.0).round() as u16)
                    .collect();

                // Handle rounding errors: adjust the largest column
                let new_total: u16 = widths.iter().sum();
                if new_total != 100 && !widths.is_empty() {
                    let max_idx = widths
                        .iter()
                        .enumerate()
                        .max_by_key(|(_, w)| *w)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    widths[max_idx] = widths[max_idx].saturating_add(100 - new_total);
                }
            }

            self.column_widths[section_idx] = widths;
        }

        let num_cols = self.column_widths[section_idx].len();
        if num_cols == 0 || self.focused_column >= num_cols {
            return;
        }

        // Calculate new width for focused column
        let current_width = self.column_widths[section_idx][self.focused_column] as i16;
        let new_current = (current_width + delta).clamp(3, 95) as u16; // Min 3%, Max 95%
        let actual_delta = new_current as i16 - current_width;

        if actual_delta == 0 {
            self.status = "Cannot resize: at min/max width".to_owned();
            return;
        }

        // Apply the change to the focused column
        self.column_widths[section_idx][self.focused_column] = new_current;

        // Distribute the delta across all other columns proportionally
        let num_other_cols = num_cols - 1;
        if num_other_cols > 0 {
            let total_other: u16 = self.column_widths[section_idx]
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != self.focused_column)
                .map(|(_, w)| *w)
                .sum();

            if total_other > 0 {
                // Distribute the negative delta proportionally across other columns
                for i in 0..num_cols {
                    if i != self.focused_column {
                        let old_width = self.column_widths[section_idx][i] as i16;
                        let proportion = old_width as f32 / total_other as f32;
                        let adjustment = (-actual_delta as f32 * proportion).round() as i16;
                        let new_width = (old_width + adjustment).max(3) as u16;
                        self.column_widths[section_idx][i] = new_width;
                    }
                }
            }
        }

        // Normalize to ensure total is exactly 100%
        let total: u16 = self.column_widths[section_idx].iter().sum();
        if total > 0 && total != 100 {
            // Scale all widths proportionally to sum to 100
            let normalized: Vec<u16> = self.column_widths[section_idx]
                .iter()
                .map(|&w| ((w as f32 / total as f32) * 100.0).round() as u16)
                .collect();

            self.column_widths[section_idx] = normalized;

            // Handle rounding errors: adjust the focused column to make total exactly 100
            let new_total: u16 = self.column_widths[section_idx].iter().sum();
            if new_total != 100 {
                let diff = 100i16 - new_total as i16;
                let focused_width = self.column_widths[section_idx][self.focused_column] as i16;
                self.column_widths[section_idx][self.focused_column] =
                    (focused_width + diff).max(1) as u16;
            }
        }

        self.status = format!(
            "Column {} width: {}% (total: {}%)",
            self.focused_column,
            self.column_widths[section_idx][self.focused_column],
            self.column_widths[section_idx].iter().sum::<u16>()
        );
    }

    // -------------------------------------------------------------------
    // Mouse handling
    // -------------------------------------------------------------------

    pub(super) fn handle_mouse_event(
        &mut self,
        kind: MouseEventKind,
        column: u16,
        row: u16,
    ) -> Action {
        // If popup is open, only close on click
        if self.detail_popup.is_some() {
            if matches!(kind, MouseEventKind::Down(_)) {
                self.detail_popup = None;
            }
            return Action::Continue;
        }

        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if clicking on a scrollbar to start drag
                if self.is_in_tree_scrollbar(column, row) {
                    self.dragging_scrollbar = true;
                    self.dragging_tree_scrollbar = true;
                    self.handle_scrollbar_drag(row);
                    return Action::Continue;
                } else if self.is_in_detail_scrollbar(column, row) {
                    self.dragging_scrollbar = true;
                    self.dragging_tree_scrollbar = false;
                    self.handle_scrollbar_drag(row);
                    return Action::Continue;
                }

                // Check if clicking near the divider to start drag
                if self.is_near_divider(column) {
                    self.dragging_divider = true;
                    return Action::Continue;
                }

                // Check for double-click (within 500ms and same position)
                let is_double_click = if let Some(last_time) = self.last_click_time {
                    let elapsed = last_time.elapsed();
                    elapsed < Duration::from_millis(500) && self.last_click_pos == (column, row)
                } else {
                    false
                };

                if is_double_click {
                    // First handle the click to update cursor position
                    self.handle_click(column, row);
                    // Then handle the double-click action
                    self.handle_double_click(column, row);
                    // Reset click tracking to avoid triple-click being detected
                    // as another double-click
                    self.last_click_time = None;
                } else {
                    self.handle_click(column, row);
                    // Track this click for double-click detection
                    self.last_click_time = Some(Instant::now());
                    self.last_click_pos = (column, row);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // Stop dragging when mouse button is released
                self.dragging_divider = false;
                self.dragging_scrollbar = false;
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Handle drag to scroll via scrollbar
                if self.dragging_scrollbar {
                    self.handle_scrollbar_drag(row);
                }
                // Handle drag to resize tree pane
                else if self.dragging_divider {
                    self.handle_divider_drag(column);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.is_in_tree_area(column, row) {
                    self.move_down();
                } else if self.is_in_detail_area(column, row) {
                    self.detail_focused = true;
                    self.move_down();
                }
            }
            MouseEventKind::ScrollUp => {
                if self.is_in_tree_area(column, row) {
                    self.move_up();
                } else if self.is_in_detail_area(column, row) {
                    self.detail_focused = true;
                    self.move_up();
                }
            }
            // Mouse back button not supported by crossterm 0.29 (only Left/Right/Middle)
            _ => {}
        }
        Action::Continue
    }

    fn handle_click(&mut self, column: u16, row: u16) {
        // Check if click is in breadcrumb area first
        if self.is_in_breadcrumb_area(column, row) {
            self.handle_breadcrumb_click(column);
        } else if self.is_in_tree_area(column, row) {
            // Click in tree area
            self.detail_focused = false;
            self.handle_tree_click(row);
        } else if self.is_in_detail_area(column, row) {
            self.detail_focused = true;
            // Check if click is on tab area
            if self.is_in_tab_area(column, row) {
                self.handle_tab_click(column, row);
            } else if self.is_in_table_content_area(column, row) {
                self.handle_table_click(column, row);
            }
        }
    }

    fn handle_double_click(&mut self, column: u16, row: u16) {
        // Double-click in table content area should trigger navigation or DOP popup
        if self.is_in_table_content_area(column, row) {
            self.detail_focused = true;

            // Check if double-click is on table header and ignore it
            if let Some(area) = self.table_content_area {
                let relative_row = (row - area.y) as usize;
                const HEADER_HEIGHT: usize = 3;
                if relative_row < HEADER_HEIGHT {
                    // Ignore double-clicks on header
                    return;
                }
            }

            // Check what type of node we're on
            if self.cursor < self.visible.len() {
                let node_idx = self.visible[self.cursor];
                let node = &self.all_nodes[node_idx];

                // Check if this is a service list header (generic check)
                let is_service_list = self.is_service_list_section(node);

                // Check if this is the Variants overview section
                let is_variants_section =
                    matches!(node.section_type, Some(crate::tree::SectionType::Variants))
                        && node
                            .detail_sections
                            .first()
                            .is_some_and(|s| matches!(&s.content, DetailContent::Table { .. }));

                // Check if this is any service-related node type (generic check)
                let is_service_node = matches!(
                    node.node_type,
                    NodeType::Service
                        | NodeType::ParentRefService
                        | NodeType::Request
                        | NodeType::PosResponse
                        | NodeType::NegResponse
                );

                // Check if this is a functional class node
                let is_functional_class = matches!(node.node_type, NodeType::FunctionalClass);

                // Check if this is a DOP node (DIAG-DATA-DICTIONARY-SPEC,
                // DOP category, or individual DOP with children)
                let is_dop_node = matches!(node.node_type, NodeType::DOP)
                    || self.is_dop_category_node(node_idx)
                    || self.is_individual_dop_node(node_idx);

                if is_variants_section {
                    // Navigate to selected variant from the Variants overview table
                    self.try_navigate_to_variant();
                } else if is_service_list {
                    // Navigate to selected service from service list table
                    self.try_navigate_to_service();
                } else if is_functional_class {
                    // For functional class nodes, check which column is focused
                    // Column 0 (ShortName): navigate to service/job
                    // Column 5 (Layer): navigate to variant/layer
                    if self.focused_column == 0 {
                        self.try_navigate_to_service_from_functional_class();
                    } else if self.focused_column == 5 {
                        self.try_navigate_to_layer_from_functional_class();
                    }
                } else if is_dop_node {
                    // Navigate to child DOP element instead of showing popup
                    self.try_navigate_to_dop_child();
                } else if is_service_node {
                    // Check if we're on the "Inherited From" row in Overview
                    let mut should_navigate_to_parent = false;
                    let mut should_navigate_from_param_table = false;

                    // Get the actual section index accounting for header section offset
                    let section_idx = self.get_section_index();

                    if section_idx < node.detail_sections.len() {
                        let section = &node.detail_sections[section_idx];

                        // Check if this is a request/response parameter table
                        if matches!(
                            section.section_type,
                            DetailSectionType::Requests
                                | DetailSectionType::PosResponses
                                | DetailSectionType::NegResponses
                        ) {
                            should_navigate_from_param_table = true;
                        } else if section.section_type == DetailSectionType::Overview
                            && let crate::tree::DetailContent::Table { rows, .. } = &section.content
                        {
                            let row_cursor = if section_idx < self.section_cursors.len() {
                                self.section_cursors[section_idx]
                            } else {
                                0
                            };

                            // Apply sorting if active for this section
                            let sorted_rows = self.apply_table_sort(rows, section_idx);

                            if row_cursor < sorted_rows.len() {
                                let selected_row = &sorted_rows[row_cursor];
                                if selected_row.row_type == DetailRowType::InheritedFrom {
                                    should_navigate_to_parent = true;
                                }
                            }
                        }
                    }

                    if should_navigate_from_param_table {
                        self.try_navigate_from_param_table();
                    } else if should_navigate_to_parent {
                        self.try_navigate_to_inherited_parent();
                    } else {
                        self.try_navigate_from_detail_row();
                    }
                } else {
                    // Check if we're in a Parent References section
                    let section_idx = self.get_section_index();
                    if section_idx < node.detail_sections.len() {
                        let section = &node.detail_sections[section_idx];
                        if section.section_type == DetailSectionType::RelatedRefs
                            && section.title == "Parent References"
                        {
                            self.try_navigate_to_parent_ref();
                            return;
                        } else if section.title.starts_with("Not Inherited") {
                            // Navigate to the selected not-inherited element
                            self.try_navigate_to_not_inherited_element();
                            return;
                        }
                    }

                    self.try_navigate_from_detail_row();
                }
            }
        }
    }

    fn is_in_tree_area(&self, column: u16, row: u16) -> bool {
        column >= self.tree_area.x
            && column < self.tree_area.x + self.tree_area.width
            && row >= self.tree_area.y
            && row < self.tree_area.y + self.tree_area.height
    }

    fn is_in_detail_area(&self, column: u16, row: u16) -> bool {
        column >= self.detail_area.x
            && column < self.detail_area.x + self.detail_area.width
            && row >= self.detail_area.y
            && row < self.detail_area.y + self.detail_area.height
    }

    fn is_in_tab_area(&self, column: u16, row: u16) -> bool {
        if let Some(tab_area) = self.tab_area {
            column >= tab_area.x
                && column < tab_area.x + tab_area.width
                && row >= tab_area.y
                && row < tab_area.y + tab_area.height
        } else {
            false
        }
    }

    fn is_in_table_content_area(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.table_content_area {
            column >= area.x
                && column < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
        } else {
            false
        }
    }

    /// Check if the mouse is near the divider between tree and detail panes
    /// The divider is considered to be within 1-2 columns of the tree pane's right edge
    fn is_near_divider(&self, column: u16) -> bool {
        let divider_col = self.tree_area.x + self.tree_area.width;
        // Allow clicking on the last column of tree or first column of detail
        column >= divider_col.saturating_sub(1) && column <= divider_col + 1
    }

    /// Handle dragging the divider to resize tree pane
    fn handle_divider_drag(&mut self, column: u16) {
        // Get the total width of main area (tree + detail)
        let total_width = self.tree_area.width + self.detail_area.width;
        if total_width == 0 {
            return;
        }

        // Calculate the new tree width based on mouse position
        // The column is relative to the start of tree_area
        let new_tree_width = column.saturating_sub(self.tree_area.x);

        // Calculate percentage (clamped between 20% and 80%)
        let new_percentage = ((new_tree_width as f32 / total_width as f32) * 100.0) as u16;
        self.tree_width_percentage = new_percentage.clamp(20, 80);
    }

    /// Check if the mouse is in the tree scrollbar area
    fn is_in_tree_scrollbar(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.tree_scrollbar_area {
            column >= area.x
                && column < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
        } else {
            false
        }
    }

    /// Check if the mouse is in the detail scrollbar area
    fn is_in_detail_scrollbar(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.detail_scrollbar_area {
            column >= area.x
                && column < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
        } else {
            false
        }
    }

    /// Handle dragging on a scrollbar to scroll
    fn handle_scrollbar_drag(&mut self, row: u16) {
        if !self.dragging_scrollbar {
            return;
        }

        if self.dragging_tree_scrollbar {
            // Dragging tree scrollbar
            if let Some(area) = self.tree_scrollbar_area {
                let visible_count = self.visible.len();
                let viewport_height = area.height as usize;

                if visible_count <= viewport_height {
                    return;
                }

                // Map mouse position to cursor position in the full list
                let relative_y = row.saturating_sub(area.y) as usize;
                let max_cursor = visible_count.saturating_sub(1);

                let new_cursor = if area.height > 1 {
                    (relative_y * max_cursor) / (area.height.saturating_sub(1) as usize)
                } else {
                    0
                };

                self.cursor = new_cursor.min(max_cursor);

                // Adjust scroll_offset to keep cursor centered in view
                let half_viewport = viewport_height / 2;
                self.scroll_offset = self.cursor.saturating_sub(half_viewport);
                let max_scroll = visible_count.saturating_sub(viewport_height);
                self.scroll_offset = self.scroll_offset.min(max_scroll);
            }
        } else {
            // Dragging detail scrollbar
            if let Some(area) = self.detail_scrollbar_area {
                if self.focused_section >= self.section_scrolls.len() {
                    return;
                }

                // Get the current section's details
                let node_idx = if let Some(&idx) = self.visible.get(self.cursor) {
                    idx
                } else {
                    return;
                };

                let sections = &self.all_nodes[node_idx].detail_sections;
                if self.focused_section >= sections.len() {
                    return;
                }

                let section = &sections[self.focused_section];
                let row_count = match &section.content {
                    DetailContent::Table { rows, .. } => rows.len(),
                    DetailContent::PlainText(lines) => lines.len(),
                    DetailContent::Composite(_sections) => {
                        return;
                    }
                };
                let viewport_height = area.height as usize;

                if row_count <= viewport_height {
                    return;
                }

                // Map mouse position to cursor position in the section
                let relative_y = row.saturating_sub(area.y) as usize;
                let max_cursor = row_count.saturating_sub(1);

                let new_cursor = if area.height > 1 {
                    (relative_y * max_cursor) / (area.height.saturating_sub(1) as usize)
                } else {
                    0
                };

                self.section_cursors[self.focused_section] = new_cursor.min(max_cursor);

                // Adjust scroll to keep cursor centered
                let half_viewport = viewport_height / 2;
                let new_scroll =
                    self.section_cursors[self.focused_section].saturating_sub(half_viewport);
                let max_scroll = row_count.saturating_sub(viewport_height);
                self.section_scrolls[self.focused_section] = new_scroll.min(max_scroll);
            }
        }
    }

    fn handle_tab_click(&mut self, column: u16, row: u16) {
        // Early exits for invalid states
        if self.tab_titles.is_empty() {
            return;
        }

        let Some(tab_area) = self.tab_area else {
            return;
        };

        // No borders on tab area - tabs render directly
        if column < tab_area.x || row < tab_area.y {
            return;
        }

        let relative_col = (column - tab_area.x) as usize;
        let relative_row = (row - tab_area.y) as usize;

        // Calculate available width for tabs (full width, no borders)
        let available_width = tab_area.width as usize;

        // Build tab strings with decorators to match rendering logic
        let tab_strings: Vec<String> = self
            .tab_titles
            .iter()
            .map(|title| format!(" {} ", title))
            .collect();

        // Simulate tab wrapping to determine which line each tab is on
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let mut current_line: Vec<usize> = Vec::new();
        let mut current_width = 0;

        for (idx, tab_str) in tab_strings.iter().enumerate() {
            let tab_width = tab_str.len() + 1; // +1 for separator

            if current_width + tab_width > available_width && !current_line.is_empty() {
                // Start a new line
                lines.push(current_line);
                current_line = Vec::new();
                current_width = 0;
            }

            current_line.push(idx);
            current_width += tab_width;
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Determine which line was clicked
        if relative_row >= lines.len() {
            return;
        }

        let clicked_line_tabs = &lines[relative_row];

        // Calculate tab positions on the clicked line
        let mut current_pos = 0;
        for (i, &tab_idx) in clicked_line_tabs.iter().enumerate() {
            let tab_str = &tab_strings[tab_idx];
            let separator_width = if i == 0 { 0 } else { 1 }; // "│" separator before tab

            // Check if click falls within this tab
            if relative_col >= current_pos + separator_width
                && relative_col < current_pos + separator_width + tab_str.len()
            {
                self.set_selected_tab(tab_idx);
                return;
            }

            // Move past this tab and its separator
            current_pos += separator_width + tab_str.len();
        }
    }

    fn handle_table_click(&mut self, column: u16, row: u16) {
        let Some(area) = self.table_content_area else {
            return;
        };

        // Validate cursor position
        if self.cursor >= self.visible.len() {
            return;
        }

        let node_idx = self.visible[self.cursor];
        let node = &self.all_nodes[node_idx];

        // Get the correct section index (accounting for header section offset)
        let section_idx = self.get_section_index();

        // Validate section index
        if section_idx >= node.detail_sections.len() {
            return;
        }

        // Extract table content
        use crate::tree::DetailContent;
        let (rows, use_row_selection) = match &node.detail_sections[section_idx].content {
            DetailContent::Table {
                rows,
                use_row_selection,
                ..
            } => (rows, *use_row_selection),
            _ => return,
        };

        // Validate table has content
        if rows.is_empty() {
            return;
        }

        // Ensure section cursors and scrolls are properly sized
        while self.section_scrolls.len() <= section_idx {
            self.section_scrolls.push(0);
        }
        while self.section_cursors.len() <= section_idx {
            self.section_cursors.push(0);
        }

        // Calculate clicked row (skip header which is 3 lines tall)
        let relative_row = (row - area.y) as usize;
        const HEADER_HEIGHT: usize = 3;

        if relative_row < HEADER_HEIGHT {
            // Clicked on header - trigger column sort
            let relative_col = column - area.x;
            if let Some(col_idx) = self.calculate_clicked_column(relative_col) {
                self.focused_column = col_idx;
                self.toggle_table_column_sort();
            }
            return;
        }

        let clicked_row_idx = (relative_row - HEADER_HEIGHT) + self.section_scrolls[section_idx];

        if clicked_row_idx >= rows.len() {
            return;
        }

        // Update the row cursor
        self.section_cursors[section_idx] = clicked_row_idx;

        // For tables with row selection mode, only select by row
        // For cell selection mode, also update the focused column
        if !use_row_selection {
            let relative_col = column - area.x;
            if let Some(col_idx) = self.calculate_clicked_column(relative_col) {
                self.focused_column = col_idx;
            }
        }
    }

    fn calculate_clicked_column(&self, relative_col: u16) -> Option<usize> {
        let area = self.table_content_area?;

        if self.cached_ratatui_constraints.is_empty() {
            return None;
        }

        // Use ratatui's Layout with the exact constraints used in rendering
        use ratatui::layout::{Direction, Layout};

        // Split the area using ratatui's layout - this matches what Table does internally
        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.cached_ratatui_constraints.clone())
            .spacing(3) // Match the column_spacing(3) from Table
            .split(area);

        // Find which column area contains the click
        for (idx, col_area) in column_areas.iter().enumerate() {
            let col_start = col_area.x - area.x;
            let col_end = col_start + col_area.width;

            if relative_col >= col_start && relative_col < col_end {
                return Some(idx);
            }
        }

        // If not in any column (in spacing), find closest
        column_areas
            .iter()
            .enumerate()
            .map(|(idx, col_area)| {
                let col_center = (col_area.x - area.x) + col_area.width / 2;
                let distance = relative_col.abs_diff(col_center);
                (idx, distance)
            })
            .min_by_key(|(_, dist)| *dist)
            .map(|(idx, _)| idx)
    }

    fn handle_tree_click(&mut self, row: u16) {
        // Calculate which tree item was clicked
        // Account for border (1 line) and title
        let inner_y = self.tree_area.y + 1;
        if row < inner_y || row >= self.tree_area.y + self.tree_area.height - 1 {
            return; // Clicked on border or help text area
        }

        let clicked_line = (row - inner_y) as usize;
        let target_cursor = self.scroll_offset + clicked_line;

        if target_cursor >= self.visible.len() {
            return;
        }

        // If clicking on the same item, toggle expand/collapse
        if target_cursor == self.cursor {
            self.toggle_expand();
        } else {
            self.push_to_history(); // Store old position before jumping
            self.cursor = target_cursor;
            self.reset_detail_state();
        }
    }

    fn is_in_breadcrumb_area(&self, column: u16, row: u16) -> bool {
        column >= self.breadcrumb_area.x
            && column < self.breadcrumb_area.x + self.breadcrumb_area.width
            && row >= self.breadcrumb_area.y
            && row < self.breadcrumb_area.y + self.breadcrumb_area.height
    }

    fn handle_breadcrumb_click(&mut self, column: u16) {
        // Find which breadcrumb segment was clicked
        // Clone the data we need to avoid borrow checker issues
        let clicked_segment = self
            .breadcrumb_segments
            .iter()
            .find(|(_, _, start_col, end_col)| column >= *start_col && column < *end_col)
            .map(|(text, node_idx, _, _)| (text.clone(), *node_idx));

        if let Some((text, node_idx)) = clicked_segment {
            // Navigate to this node
            self.navigate_to_node(node_idx);
            self.status = format!("Jumped to: {}", text);
        }
    }

    /// Navigate to a specific node by its index in all_nodes
    fn navigate_to_node(&mut self, target_node_idx: usize) {
        // Find the position of this node in visible
        if let Some(_visible_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) {
            // Ensure the target node is expanded if needed
            self.ensure_node_visible(target_node_idx);

            // Find the updated position after expanding
            if let Some(new_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) {
                self.push_to_history(); // Store old position before jumping
                self.detail_focused = false;
                self.cursor = new_pos;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5); // Center the view
            }
        } else {
            // Node is not currently visible (might be collapsed), try to make it visible
            self.ensure_node_visible(target_node_idx);

            // Try to find it again after expanding parents
            if let Some(visible_pos) = self.visible.iter().position(|&idx| idx == target_node_idx) {
                self.push_to_history(); // Store old position before jumping
                self.detail_focused = false;
                self.cursor = visible_pos;
                self.reset_detail_state();
                self.scroll_offset = self.cursor.saturating_sub(5); // Center the view
            }
        }
    }

    /// Ensure a node is visible by expanding only its direct ancestors
    fn ensure_node_visible(&mut self, target_node_idx: usize) {
        if target_node_idx >= self.all_nodes.len() {
            return;
        }

        let mut needed_depth = self.all_nodes[target_node_idx].depth;

        // Walk backwards, expanding only the direct ancestor at each level
        for i in (0..target_node_idx).rev() {
            let node_depth = self.all_nodes[i].depth;

            if node_depth < needed_depth {
                self.all_nodes[i].expanded = true;
                needed_depth = node_depth;

                if node_depth == 0 {
                    break;
                }
            }
        }

        // Rebuild visible list to reflect expansions
        self.rebuild_visible();
    }
}

// Helper functions for service sorting
fn extract_service_id(text: &str) -> u32 {
    // Extract ID from format like "0x10    - ServiceName" or "0x22F501 - ServiceName"
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
        return text[dash_pos + 3..].trim();
    }
    text
}
