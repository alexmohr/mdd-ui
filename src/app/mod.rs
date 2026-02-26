/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

mod column_widths;
mod cursor;
mod history;
mod input;
mod mouse;
mod navigation;
mod render;
mod search;
mod sort;
mod visibility;

use std::{collections::HashMap, io, time::Instant};

use crossterm::event::{self, Event, KeyEventKind, KeyModifiers};
use input::Action;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::tree::{DetailRow, DetailSectionType, NodeType, TreeNode};

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
    /// Optional secondary column for tie-breaking (e.g., Bit position after Byte)
    pub secondary_column: Option<usize>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
enum DragState {
    None,
    Divider,
    TreeScrollbar,
    DetailScrollbar,
    DetailHScrollbar,
    ColumnBorder(usize),
}

/// Key for persisting column widths across element switches
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ColumnWidthCacheKey {
    section_type: DetailSectionType,
    title: String,
    column_count: usize,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum FocusState {
    Tree,
    Detail,
    HelpPopup,
}

pub struct App {
    all_nodes: Vec<TreeNode>,
    visible: Vec<usize>,
    cursor: usize,
    scroll_offset: usize,
    pub(crate) search: String,
    pub(crate) searching: bool,
    pub(crate) search_stack: Vec<(String, SearchScope)>, // Stack of (search_term, scope) pairs
    pub(crate) search_scope: SearchScope,
    pub(crate) search_matches: Vec<usize>,
    search_match_cursor: usize,
    pub(crate) status: String,
    focus_state: FocusState,
    pub(crate) selected_tab: usize, // Currently selected tab in detail pane
    pub(crate) focused_section: usize, // Which detail pane section is focused (0 = first)
    pub(crate) section_scrolls: Vec<usize>, // Scroll position for each section
    pub(crate) section_cursors: Vec<usize>, // Selected row in each section
    pub(crate) column_widths: Vec<Vec<u16>>, // Column widths for each section
    // Whether each section uses absolute (pixel) widths
    pub(crate) column_widths_absolute: Vec<bool>,
    pub(crate) horizontal_scroll: Vec<u16>, // Horizontal scroll offset (pixels) per section
    persisted_column_widths: HashMap<ColumnWidthCacheKey, Vec<u16>>, // Persistent absolute widths
    pub(crate) focused_column: usize,       // Currently focused column for resizing
    pub(crate) detail_popup: Option<PopupData>, // Generic popup state
    pub(crate) tree_width_percentage: u16,  // Tree pane width (0-100)
    pub(crate) diagcomm_sort_by_id: bool,   // true = sort by ID (default), false = sort by name
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
    drag_state: DragState,   // Drag state for dividers and scrollbars
    tree_scrollbar_area: Option<Rect>, // Cached tree scrollbar area for mouse handling
    detail_scrollbar_area: Option<Rect>, // Cached detail scrollbar area for mouse handling
    detail_hscrollbar_area: Option<Rect>, // Cached horizontal scrollbar area for mouse handling
    cached_total_table_width: u16, // Total table width for horizontal scrollbar drag
    last_diagcomm_tab: usize, // Last selected tab when viewing service/job nodes (for persistence)
    last_section_tabs: HashMap<DetailSectionType, usize>, // Last selected tab per section type
    last_selected_section_type: Option<DetailSectionType>,
    last_selected_section_title: Option<String>,
    jump_buffer: String, // Characters typed for type-to-jump in table views
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
            search: String::new(),
            searching: false,
            search_stack: Vec::new(),
            search_scope: SearchScope::All,
            search_matches: Vec::new(),
            search_match_cursor: 0,
            status: String::new(),
            focus_state: FocusState::Tree,
            selected_tab: 0,
            focused_section: 0,
            section_scrolls: Vec::new(),
            section_cursors: Vec::new(),
            column_widths: Vec::new(),
            column_widths_absolute: Vec::new(),
            horizontal_scroll: Vec::new(),
            persisted_column_widths: HashMap::new(),
            focused_column: 0,
            detail_popup: None,
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
            drag_state: DragState::None,
            tree_scrollbar_area: None,
            detail_scrollbar_area: None,
            detail_hscrollbar_area: None,
            cached_total_table_width: 0,
            last_diagcomm_tab: 0,
            last_section_tabs: HashMap::new(),
            last_selected_section_type: None,
            last_selected_section_title: None,
            jump_buffer: String::new(),
            jump_buffer_time: None,
        };
        // Apply initial sort order (default is by ID)
        app.sort_diagcomm_nodes_in_place();
        app.rebuild_visible();
        app
    }

    /// Helper: Check if a node is a service list section header
    fn is_service_list_section(node: &TreeNode) -> bool {
        node.service_list_type.is_some()
    }

    /// Helper: Check if a node is a specific service list type
    fn is_service_list_type(node: &TreeNode, list_type: crate::tree::ServiceListType) -> bool {
        matches!(&node.service_list_type, Some(t) if *t == list_type)
    }

    /// Check if a node is a DOP category node (child of DIAG-DATA-DICTIONARY-SPEC)
    fn is_dop_category_node(&self, node_idx: usize) -> bool {
        let Some(node) = self.all_nodes.get(node_idx) else {
            return false;
        };
        if !node.has_children || node.depth == 0 {
            return false;
        }
        // Walk backwards to find the parent (first node with depth - 1)
        for i in (0..node_idx).rev() {
            let Some(candidate) = self.all_nodes.get(i) else {
                continue;
            };
            if candidate.depth < node.depth {
                return matches!(candidate.node_type, NodeType::DOP);
            }
        }
        false
    }

    /// Check if a node is an individual DOP with children (e.g. a DTC-DOP under DTC-DOPS).
    /// These nodes should navigate to their children instead of showing a popup.
    fn is_individual_dop_node(&self, node_idx: usize) -> bool {
        let Some(node) = self.all_nodes.get(node_idx) else {
            return false;
        };
        if !node.has_children || node.depth < 2 {
            return false;
        }
        // Walk backwards to find the parent
        for i in (0..node_idx).rev() {
            let Some(candidate) = self.all_nodes.get(i) else {
                continue;
            };
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
            let Some(node) = self.all_nodes.get(idx) else {
                return self.selected_tab;
            };
            let sections = &node.detail_sections;
            if sections.len() > 1
                && let Some(first_section) = sections.first()
                && first_section.render_as_header
                && matches!(
                    &first_section.content,
                    crate::tree::DetailContent::PlainText(_)
                )
            {
                // Has header section, so selected_tab needs offset of 1
                return self.selected_tab.saturating_add(1);
            }
        }
        self.selected_tab
    }

    /// Get the section offset for rendering (0 or 1 if there's a header section)
    fn get_section_offset(&self) -> usize {
        if let Some(&idx) = self.visible.get(self.cursor) {
            let Some(node) = self.all_nodes.get(idx) else {
                return 0;
            };
            let sections = &node.detail_sections;
            if sections.len() > 1
                && let Some(first_section) = sections.first()
                && first_section.render_as_header
                && matches!(
                    &first_section.content,
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
        self.selected_tab.saturating_add(self.get_section_offset())
    }

    /// Update the selected tab and persist it generically for the current section type
    fn set_selected_tab(&mut self, new_tab: usize) {
        self.selected_tab = new_tab;
        self.jump_buffer.clear();
        self.jump_buffer_time = None;

        // Save tab selection for the current section type
        if self.cursor < self.visible.len()
            && let Some(&node_idx) = self.visible.get(self.cursor)
            && let Some(node) = self.all_nodes.get(node_idx)
        {
            // For backward compatibility, still save diagcomm tab
            if matches!(
                node.node_type,
                NodeType::Service | NodeType::ParentRefService | NodeType::Job
            ) {
                self.last_diagcomm_tab = new_tab;
            }

            // Save tab for any node with detail sections that have a section type
            if !node.detail_sections.is_empty() {
                let section_offset = self.get_section_offset();
                let section_idx = new_tab.saturating_add(section_offset);
                if let Some(section) = node.detail_sections.get(section_idx) {
                    self.last_section_tabs.insert(section.section_type, new_tab);
                    self.last_selected_section_type = Some(section.section_type);
                    self.last_selected_section_title = Some(section.title.clone());
                }
            }
        }
    }

    /// Jump to the first table row whose first cell starts with the `jump_buffer` text
    fn jump_to_matching_row(&mut self) {
        if self.jump_buffer.is_empty() || self.cursor >= self.visible.len() {
            return;
        }

        let Some(&node_idx) = self.visible.get(self.cursor) else {
            return;
        };
        let Some(node) = self.all_nodes.get(node_idx) else {
            return;
        };
        let section_idx = self.get_section_index();

        let Some(section) = node.detail_sections.get(section_idx) else {
            return;
        };

        // Get table rows (apply sorting if active)
        let Some(rows) = section.content.table_rows() else {
            return;
        };
        let rows = self.apply_table_sort(rows, section_idx);

        let buffer_lower = self.jump_buffer.to_lowercase();

        // Find first row where the focused column starts with the buffer (case-insensitive)
        for (i, row) in rows.iter().enumerate() {
            if let Some(cell) = row.cells.get(self.focused_column)
                && cell.to_lowercase().starts_with(&buffer_lower)
            {
                if let Some(cursor) = self.section_cursors.get_mut(section_idx) {
                    *cursor = i;
                }
                self.status = format!("Jump: \"{}\"", self.jump_buffer);
                return;
            }
        }

        self.status = format!("Jump: \"{}\" (no match)", self.jump_buffer);
    }

    /// Apply sorting to rows if a sort state exists for the given section
    fn apply_table_sort(&self, rows: &[DetailRow], section_idx: usize) -> Vec<DetailRow> {
        let Some(sort_state) = self
            .table_sort_state
            .get(section_idx)
            .and_then(|s| s.as_ref())
        else {
            return rows.to_vec();
        };

        let mut sorted = rows.to_vec();
        let col = sort_state.column;
        let dir = sort_state.direction;
        let secondary = sort_state.secondary_column;
        sorted.sort_by(|a, b| {
            let cmp = Self::compare_cells_by_column(a, b, col);
            let cmp = match cmp {
                std::cmp::Ordering::Equal => secondary.map_or(std::cmp::Ordering::Equal, |sec| {
                    Self::compare_cells_by_column(a, b, sec)
                }),
                other => other,
            };

            match dir {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
        sorted
    }

    fn compare_cells_by_column(a: &DetailRow, b: &DetailRow, col: usize) -> std::cmp::Ordering {
        let a_cell = a.cells.get(col).map_or("", String::as_str);
        let b_cell = b.cells.get(col).map_or("", String::as_str);

        match (a_cell.parse::<f64>(), b_cell.parse::<f64>()) {
            (Ok(a_num), Ok(b_num)) => a_num
                .partial_cmp(&b_num)
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => a_cell.cmp(b_cell),
        }
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
                Constraint::Percentage(100u16.saturating_sub(self.tree_width_percentage)),
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
        if self.focus_state == FocusState::HelpPopup {
            Self::draw_help_popup(frame);
        }
    }
}
