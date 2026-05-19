// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

// Tauri commands require owned types for JSON deserialization and state injection.
#![allow(clippy::needless_pass_by_value)]

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use mdd_core::{
    tree::{
        CellJumpTargetType, DetailContent, DetailRow, DetailSectionData, DiffStatus, NodePayload,
        NodeType, ServiceListType, TreeNode,
    },
    uds::translator::{
        MatchedService, ServiceSchemaResult, UdsEncodeResult, UdsLookupResult, VariantInfo,
    },
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

// ---------------------------------------------------------------------------
// Lightweight DTOs sent to the Vue frontend
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct VisibleNode {
    pub index: usize,
    pub depth: usize,
    pub text: String,
    pub expanded: bool,
    pub has_children: bool,
    pub node_type: NodeType,
    pub diff_status: Option<DiffStatus>,
    pub is_sortable: bool,
    pub old_text: Option<String>,
}

#[derive(Serialize)]
pub struct LoadResult {
    pub tab_id: String,
    pub ecu_name: String,
    pub node_count: usize,
    pub visible: Vec<VisibleNode>,
    pub is_diff: bool,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub visible: Vec<VisibleNode>,
    pub match_count: usize,
    pub scope: String,
}

#[derive(Serialize)]
pub struct NavigateResult {
    pub visible: Vec<VisibleNode>,
    pub target_index: usize,
    pub detail: Vec<DetailSectionData>,
}

#[derive(Serialize)]
pub struct ToggleSortResult {
    pub nodes: Vec<VisibleNode>,
    pub sort_label: String,
}

#[derive(Deserialize)]
pub struct JumpTarget {
    pub target_type: JumpTargetType,
}

#[derive(Deserialize)]
pub enum JumpTargetType {
    Parameter { param_id: u32 },
    Dop { index: usize, name: String },
    TreeNodeByIndex { index: usize, short_name: String },
    Container { index: usize, short_name: String },
}

// ---------------------------------------------------------------------------
// Shared app state behind a Mutex
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagcommSortMode {
    #[default]
    IdAsc,
    IdDesc,
    NameAsc,
    NameDesc,
}

impl DiagcommSortMode {
    pub fn next(self) -> Self {
        match self {
            Self::IdAsc => Self::IdDesc,
            Self::IdDesc => Self::NameAsc,
            Self::NameAsc => Self::NameDesc,
            Self::NameDesc => Self::IdAsc,
        }
    }

    pub const fn status_label(self) -> &'static str {
        match self {
            Self::IdAsc => "Sort: ID \u{25b2}",
            Self::IdDesc => "Sort: ID \u{25bc}",
            Self::NameAsc => "Sort: Name \u{25b2}",
            Self::NameDesc => "Sort: Name \u{25bc}",
        }
    }
}

pub struct CoreState {
    pub all_nodes: Vec<TreeNode>,
    pub visible: Vec<usize>,
    pub ecu_name: String,
    pub is_diff_mode: bool,
    pub hide_unchanged: bool,
    pub search_stack: Vec<SearchEntry>,
    pub search_scope: SearchScope,
    pub diagcomm_sort: DiagcommSortMode,
}

#[derive(Clone, PartialEq)]
pub enum FilterOp {
    And,
    Or,
}

#[derive(Clone)]
pub struct SearchEntry {
    pub query: String,
    pub scope: SearchScope,
    pub op: FilterOp,
}

#[derive(Clone, Default, Serialize)]
pub enum SearchScope {
    #[default]
    All,
    Variants,
    FunctionalGroups,
    EcuSharedData,
    Services,
    DiagComms,
    Requests,
    Responses,
}

impl std::fmt::Display for SearchScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchScope::All => write!(f, "All"),
            SearchScope::Variants => write!(f, "Variants"),
            SearchScope::FunctionalGroups => write!(f, "Functional Groups"),
            SearchScope::EcuSharedData => write!(f, "ECU Shared Data"),
            SearchScope::Services => write!(f, "Services"),
            SearchScope::DiagComms => write!(f, "Diag-Comms"),
            SearchScope::Requests => write!(f, "Requests"),
            SearchScope::Responses => write!(f, "Responses"),
        }
    }
}

impl Default for CoreState {
    fn default() -> Self {
        Self {
            all_nodes: Vec::new(),
            visible: Vec::new(),
            ecu_name: String::new(),
            is_diff_mode: false,
            hide_unchanged: false,
            search_stack: Vec::new(),
            search_scope: SearchScope::default(),
            diagcomm_sort: DiagcommSortMode::IdAsc,
        }
    }
}

pub struct AppState(pub Mutex<TabManager>);

pub struct TabEntry {
    pub core: CoreState,
    pub file_path: String,
    pub display_name: String,
    pub is_diff: bool,
}

#[derive(Serialize)]
pub struct TabInfo {
    pub id: String,
    pub display_name: String,
    pub file_path: String,
    pub is_diff: bool,
    pub is_active: bool,
}

#[derive(Default)]
pub struct TabManager {
    pub tabs: HashMap<String, TabEntry>,
    pub active_tab: Option<String>,
    next_id: u64,
}

impl TabManager {
    fn next_tab_id(&mut self) -> String {
        self.next_id = self.next_id.saturating_add(1);
        format!("tab_{}", self.next_id)
    }

    pub fn active_core(&self) -> Result<&CoreState, String> {
        let tab_id = self
            .active_tab
            .as_deref()
            .ok_or_else(|| "No active tab".to_owned())?;
        self.tabs
            .get(tab_id)
            .map(|entry| &entry.core)
            .ok_or_else(|| "Active tab not found".to_owned())
    }

    pub fn active_core_mut(&mut self) -> Result<&mut CoreState, String> {
        let tab_id = self
            .active_tab
            .clone()
            .ok_or_else(|| "No active tab".to_owned())?;
        self.tabs
            .get_mut(&tab_id)
            .map(|entry| &mut entry.core)
            .ok_or_else(|| "Active tab not found".to_owned())
    }

    fn active_tab_id(&self) -> Result<String, String> {
        self.active_tab
            .clone()
            .ok_or_else(|| "No active tab".to_owned())
    }
}

/// Holds per-tab UDS translators, keyed by tab ID.
pub struct UdsState(
    pub tauri::async_runtime::Mutex<HashMap<String, mdd_core::uds::translator::UdsTranslator>>,
);

pub struct InitialFile(pub Mutex<Option<String>>);

pub struct LoadCancellation(pub Mutex<Option<Arc<AtomicBool>>>);

impl Default for AppState {
    fn default() -> Self {
        Self(Mutex::new(TabManager::default()))
    }
}

impl Default for LoadCancellation {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Compute the search include bitmap from the active search stack.
/// Returns `None` when no search filters are present (all nodes implicitly included).
fn compute_include(state: &CoreState) -> Option<Vec<bool>> {
    if state.search_stack.is_empty() {
        return None;
    }
    let all_true = vec![true; state.all_nodes.len()];
    let mut inc: Option<Vec<bool>> = None;
    for entry in &state.search_stack {
        let fresh = apply_search_filter(&state.all_nodes, &all_true, &entry.query, &entry.scope);
        inc = Some(match inc {
            None => fresh,
            Some(mut cur) => {
                match entry.op {
                    FilterOp::And => {
                        for (a, b) in cur.iter_mut().zip(fresh.iter()) {
                            *a = *a && *b;
                        }
                    }
                    FilterOp::Or => {
                        for (a, b) in cur.iter_mut().zip(fresh.iter()) {
                            *a = *a || *b;
                        }
                    }
                }
                cur
            }
        });
    }
    inc
}

/// Count direct children (depth = `header_depth + 1`) of the node at `header_idx`
/// that pass the active filters (search bitmap + hide-unchanged).
fn count_filtered_direct_children(
    all_nodes: &[TreeNode],
    include: Option<&[bool]>,
    hide_unchanged: bool,
    header_idx: usize,
    header_depth: usize,
) -> usize {
    let child_depth = header_depth.saturating_add(1);
    let mut count: usize = 0;
    for i in (header_idx.saturating_add(1))..all_nodes.len() {
        let Some(n) = all_nodes.get(i) else { break };
        if n.depth <= header_depth {
            break;
        }
        if n.depth == child_depth {
            let passes_search = include.is_none_or(|inc| inc.get(i).copied().unwrap_or(false));
            let passes_diff =
                !hide_unchanged || !matches!(n.diff_status, Some(DiffStatus::Unchanged));
            if passes_search && passes_diff {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

/// Filter the rows of a service-list overview table, keeping only rows whose
/// service matches the active search / diff filter.
///
/// Matching is done by **short name** (from the row's `TreeNodeByIndex` jump
/// target) looked up against the header's **actual direct children** in
/// `all_nodes`.  Using the stored jump-target index directly would be unreliable
/// because `resolve_all_indices` uses `or_insert` and can map a short name to a
/// different variant's node when services share names across layers.
///
/// Rows without a `TreeNodeByIndex` reference (separator rows, etc.) are always
/// kept.  Only `Table` content is filtered; `Composite` and `PlainText` are
/// returned as-is.  The section title count is updated to match.
fn filter_service_list_rows(
    section: &DetailSectionData,
    include: Option<&[bool]>,
    hide_unchanged: bool,
    all_nodes: &[TreeNode],
    header_idx: usize,
) -> DetailSectionData {
    let DetailContent::Table {
        header,
        rows,
        constraints,
        use_row_selection,
    } = &section.content
    else {
        return section.clone();
    };

    // Build short_name → passes_filter from the header's real direct children.
    let header_depth = all_nodes.get(header_idx).map_or(0, |n| n.depth);
    let child_depth = header_depth.saturating_add(1);
    let mut name_passes: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for i in (header_idx.saturating_add(1))..all_nodes.len() {
        let Some(n) = all_nodes.get(i) else { break };
        if n.depth <= header_depth {
            break;
        }
        if n.depth == child_depth {
            let passes = include.is_none_or(|inc| inc.get(i).copied().unwrap_or(false))
                && (!hide_unchanged || !matches!(n.diff_status, Some(DiffStatus::Unchanged)));
            let key = n.service_short_name().unwrap_or(n.text.as_str());
            name_passes.insert(key, passes);
        }
    }

    let filtered: Vec<DetailRow> = rows
        .iter()
        .filter(|row| {
            let sn = row
                .cells
                .iter()
                .filter_map(|cell| cell.jump_target.as_ref())
                .find_map(|jt| match &jt.target_type {
                    CellJumpTargetType::TreeNodeByIndex { short_name, .. } => {
                        Some(short_name.as_str())
                    }
                    _ => None,
                });
            match sn {
                None => true,
                Some(name) => name_passes.get(name).copied().unwrap_or(true),
            }
        })
        .cloned()
        .collect();

    let row_count = filtered.len();
    DetailSectionData {
        title: replace_header_count(&section.title, row_count),
        content: DetailContent::Table {
            header: header.clone(),
            rows: filtered,
            constraints: constraints.clone(),
            use_row_selection: *use_row_selection,
        },
        render_as_header: section.render_as_header,
        section_type: section.section_type,
        byte_pattern_rows: section.byte_pattern_rows.clone(),
    }
}

/// Strip a trailing `(…)` count suffix from a service-list header's display
/// text, leaving the base name.  Used when building node paths.
fn strip_count_suffix(text: &str) -> &str {
    if let Some(pos) = text.rfind('(')
        && text.ends_with(')')
    {
        return text.get(..pos).map_or(text, |s| s.trim_end());
    }
    text
}

/// Replace the trailing `(…)` of a service-list header's display text with `(new_count)`.
fn replace_header_count(text: &str, new_count: usize) -> String {
    if let Some(open) = text.rfind('(')
        && text.ends_with(')')
    {
        return format!("{}({new_count})", text.get(..open).unwrap_or(text));
    }
    format!("{text} ({new_count})")
}

fn build_visible(state: &CoreState) -> Vec<usize> {
    let mut visible = Vec::new();
    let mut collapsed_below: Option<usize> = None;

    let include = compute_include(state);

    for (i, node) in state.all_nodes.iter().enumerate() {
        // If search is active, skip nodes not in the include set
        if let Some(ref inc) = include
            && !inc.get(i).copied().unwrap_or(false)
        {
            continue;
        }

        // Skip nodes under collapsed parent
        if let Some(cd) = collapsed_below {
            if node.depth > cd {
                continue;
            }
            collapsed_below = None;
        }

        // Skip unchanged in diff mode when filter is active
        if state.hide_unchanged && matches!(node.diff_status, Some(DiffStatus::Unchanged)) {
            continue;
        }

        visible.push(i);

        if node.has_children && !node.expanded {
            collapsed_below = Some(node.depth);
        }
    }

    visible
}

fn apply_search_filter(
    nodes: &[TreeNode],
    include: &[bool],
    query: &str,
    scope: &SearchScope,
) -> Vec<bool> {
    let q = query.to_lowercase();
    let len = nodes.len();
    let mut new_include = vec![false; len];

    // Pass 1: Mark matching nodes and their children
    let mut skip_below: Option<usize> = None;
    for (i, &included) in include.iter().enumerate().take(len) {
        let Some(node) = nodes.get(i) else { continue };

        if let Some(depth) = skip_below {
            if node.depth > depth {
                if included && let Some(slot) = new_include.get_mut(i) {
                    *slot = true;
                }
                continue;
            }
            skip_below = None;
        }

        if !included {
            continue;
        }

        if node_matches_scope(node, scope) && node.text.to_lowercase().contains(&q) {
            if let Some(slot) = new_include.get_mut(i) {
                *slot = true;
            }
            skip_below = Some(node.depth);
        }
    }

    // Pass 2: Include ancestors of matched nodes
    let max_depth = nodes.iter().map(|n| n.depth).max().unwrap_or(0);
    let mut parent_at_depth = vec![0usize; max_depth.saturating_add(1)];

    for (i, node) in nodes.iter().enumerate() {
        if let Some(slot) = parent_at_depth.get_mut(node.depth) {
            *slot = i;
        }

        if new_include.get(i).copied().unwrap_or(false) && node.depth > 0 {
            for d in (0..node.depth).rev() {
                let Some(&ancestor) = parent_at_depth.get(d) else {
                    break;
                };
                if new_include.get(ancestor).copied().unwrap_or(false) {
                    break;
                }
                if let Some(slot) = new_include.get_mut(ancestor) {
                    *slot = true;
                }
            }
        }
    }

    new_include
}

fn node_matches_scope(node: &TreeNode, scope: &SearchScope) -> bool {
    match scope {
        SearchScope::All => true,
        SearchScope::Services => node.node_type.is_service(),
        SearchScope::DiagComms => node.node_type.is_diagcomm(),
        SearchScope::Requests => matches!(node.node_type, NodeType::Request),
        SearchScope::Responses => matches!(
            node.node_type,
            NodeType::PosResponse | NodeType::NegResponse
        ),
        SearchScope::Variants | SearchScope::FunctionalGroups | SearchScope::EcuSharedData => {
            matches!(
                node.node_type,
                NodeType::Container | NodeType::SectionHeader
            )
        }
    }
}

fn to_visible_nodes(state: &CoreState) -> Vec<VisibleNode> {
    let any_filter = !state.search_stack.is_empty() || state.hide_unchanged;
    let include = if any_filter {
        compute_include(state)
    } else {
        None
    };

    state
        .visible
        .iter()
        .filter_map(|&idx| {
            state.all_nodes.get(idx).map(|node| {
                let text = if any_filter && node.service_list_type().is_some() {
                    let count = count_filtered_direct_children(
                        &state.all_nodes,
                        include.as_deref(),
                        state.hide_unchanged,
                        idx,
                        node.depth,
                    );
                    replace_header_count(&node.text, count)
                } else {
                    node.text.clone()
                };

                VisibleNode {
                    index: idx,
                    depth: node.depth,
                    text,
                    expanded: node.expanded,
                    has_children: node.has_children,
                    node_type: node.node_type,
                    diff_status: node.diff_status,
                    is_sortable: node.service_list_type().is_some(),
                    old_text: node.old_text.clone(),
                }
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn load_mdd(
    path: String,
    state: State<'_, AppState>,
    cancel: State<'_, LoadCancellation>,
) -> Result<LoadResult, String> {
    // If the file is already open in a tab, switch to it
    {
        let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        let existing = manager
            .tabs
            .iter()
            .find(|(_, t)| t.file_path == path && !t.is_diff)
            .map(|(id, _)| id.clone());

        if let Some(tab_id) = existing {
            manager.active_tab = Some(tab_id.clone());
            let core = manager.active_core()?;
            return Ok(LoadResult {
                tab_id,
                ecu_name: core.ecu_name.clone(),
                node_count: core.all_nodes.len(),
                visible: to_visible_nodes(core),
                is_diff: core.is_diff_mode,
            });
        }
    }

    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut guard = cancel.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        *guard = Some(Arc::clone(&cancelled));
    }

    let (nodes, ecu_name, file_path) =
        tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
            let db = mdd_core::database::load_mdd(&path)
                .map_err(|e| format!("Failed to load: {e:#}"))?;
            let (nodes, ecu_name) = mdd_core::tree::build_tree(&db, &path);
            Ok((nodes, ecu_name, path))
        })
        .await
        .map_err(|e| format!("Thread error: {e}"))??;

    {
        let mut guard = cancel.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        *guard = None;
    }

    if cancelled.load(Ordering::Relaxed) {
        return Err("Cancelled".to_owned());
    }

    let node_count = nodes.len();
    let display_name = PathBuf::from(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(|| file_path.clone(), str::to_owned);

    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let tab_id = manager.next_tab_id();

    let mut core = CoreState {
        all_nodes: nodes,
        ecu_name: ecu_name.clone(),
        ..CoreState::default()
    };
    apply_default_sort(&mut core.all_nodes);
    mdd_core::tree::resolve_all_indices(&mut core.all_nodes);
    core.visible = build_visible(&core);

    let visible = to_visible_nodes(&core);

    manager.tabs.insert(
        tab_id.clone(),
        TabEntry {
            core,
            file_path,
            display_name,
            is_diff: false,
        },
    );
    manager.active_tab = Some(tab_id.clone());

    Ok(LoadResult {
        tab_id,
        ecu_name,
        node_count,
        visible,
        is_diff: false,
    })
}

#[tauri::command]
pub async fn load_diff(
    old_path: String,
    new_path: String,
    state: State<'_, AppState>,
    cancel: State<'_, LoadCancellation>,
) -> Result<LoadResult, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut guard = cancel.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        *guard = Some(Arc::clone(&cancelled));
    }

    let (nodes, ecu_name, old, new) =
        tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
            let db_old = mdd_core::database::load_mdd(&old_path)
                .map_err(|e| format!("Failed to load old: {e:#}"))?;
            let db_new = mdd_core::database::load_mdd(&new_path)
                .map_err(|e| format!("Failed to load new: {e:#}"))?;
            let (nodes, ecu_name) =
                mdd_core::diff::diff_tree::build_diff_tree(&db_old, &db_new, &old_path, &new_path);
            Ok((nodes, ecu_name, old_path, new_path))
        })
        .await
        .map_err(|e| format!("Thread error: {e}"))??;

    {
        let mut guard = cancel.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        *guard = None;
    }

    if cancelled.load(Ordering::Relaxed) {
        return Err("Cancelled".to_owned());
    }

    let node_count = nodes.len();

    let old_name = PathBuf::from(&old)
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(|| old.clone(), str::to_owned);
    let new_name = PathBuf::from(&new)
        .file_name()
        .and_then(|n| n.to_str())
        .map_or_else(|| new.clone(), str::to_owned);
    let display_name = format!("{old_name} \u{2194} {new_name}");

    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let tab_id = manager.next_tab_id();

    let mut core = CoreState {
        all_nodes: nodes,
        ecu_name: ecu_name.clone(),
        is_diff_mode: true,
        ..CoreState::default()
    };
    apply_default_sort(&mut core.all_nodes);
    mdd_core::tree::resolve_all_indices(&mut core.all_nodes);
    core.visible = build_visible(&core);

    let visible = to_visible_nodes(&core);

    manager.tabs.insert(
        tab_id.clone(),
        TabEntry {
            core,
            file_path: String::new(),
            display_name,
            is_diff: true,
        },
    );
    manager.active_tab = Some(tab_id.clone());

    Ok(LoadResult {
        tab_id,
        ecu_name,
        node_count,
        visible,
        is_diff: true,
    })
}

#[tauri::command]
pub fn cancel_load(state: State<'_, LoadCancellation>) -> Result<(), String> {
    let guard = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    if let Some(flag) = guard.as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn get_visible_nodes(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core()?;
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn get_node_detail(
    index: usize,
    state: State<'_, AppState>,
) -> Result<Vec<DetailSectionData>, String> {
    let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core()?;
    let node = core
        .all_nodes
        .get(index)
        .ok_or_else(|| format!("Node index {index} out of range"))?;

    // Only filter overview (service-list header) nodes when a filter is active.
    // Individual service/response detail sections are left untouched.
    let any_filter = !core.search_stack.is_empty() || core.hide_unchanged;
    if !any_filter || node.service_list_type().is_none() {
        return Ok(node.detail_sections.to_vec());
    }

    let include = compute_include(core);
    Ok(node
        .detail_sections
        .iter()
        .map(|section| {
            filter_service_list_rows(
                section,
                include.as_deref(),
                core.hide_unchanged,
                &core.all_nodes,
                index,
            )
        })
        .collect())
}
#[tauri::command]
pub fn get_node_variant(
    index: usize,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core()?;
    Ok(resolve_node_variant(&core.all_nodes, index))
}

/// Resolve the variant name for a node by walking its parent chain.
fn resolve_node_variant(all_nodes: &[TreeNode], index: usize) -> Option<String> {
    let mut idx = index;
    loop {
        let node = all_nodes.get(idx)?;
        if node.node_type == NodeType::Container
            && let NodePayload::Container { ref short_name, .. } = node.payload
        {
            return Some(short_name.clone());
        }
        idx = node.parent_idx?;
    }
}

#[tauri::command]
pub fn toggle_expand(index: usize, state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    if let Some(node) = core.all_nodes.get_mut(index)
        && node.has_children
    {
        node.expanded = !node.expanded;
    }
    core.visible = build_visible(core);
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn search(
    query: String,
    op: Option<String>,
    state: State<'_, AppState>,
) -> Result<SearchResult, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    if !query.is_empty() {
        let scope = core.search_scope.clone();
        let filter_op = if op.as_deref() == Some("or") {
            FilterOp::Or
        } else {
            FilterOp::And
        };
        core.search_stack.push(SearchEntry {
            query,
            scope,
            op: filter_op,
        });
    }
    let visible = build_visible(core);
    core.visible = visible;

    let match_count = core.search_stack.len();
    let scope = core.search_scope.to_string();
    Ok(SearchResult {
        visible: to_visible_nodes(core),
        match_count,
        scope,
    })
}

#[tauri::command]
pub fn clear_search(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    core.search_stack.clear();
    core.visible = build_visible(core);
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn cycle_search_scope(state: State<'_, AppState>) -> Result<String, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    core.search_scope = match core.search_scope {
        SearchScope::All => SearchScope::Variants,
        SearchScope::Variants => SearchScope::FunctionalGroups,
        SearchScope::FunctionalGroups => SearchScope::EcuSharedData,
        SearchScope::EcuSharedData => SearchScope::Services,
        SearchScope::Services => SearchScope::DiagComms,
        SearchScope::DiagComms => SearchScope::Requests,
        SearchScope::Requests => SearchScope::Responses,
        SearchScope::Responses => SearchScope::All,
    };
    Ok(core.search_scope.to_string())
}

#[tauri::command]
pub fn set_search_scope(scope: String, state: State<'_, AppState>) -> Result<String, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    core.search_scope = match scope.as_str() {
        "All" => SearchScope::All,
        "Variants" => SearchScope::Variants,
        "Functional Groups" => SearchScope::FunctionalGroups,
        "ECU Shared Data" => SearchScope::EcuSharedData,
        "Services" => SearchScope::Services,
        "Diag-Comms" => SearchScope::DiagComms,
        "Requests" => SearchScope::Requests,
        "Responses" => SearchScope::Responses,
        _ => return Err(format!("Unknown scope: {scope}")),
    };
    Ok(core.search_scope.to_string())
}

#[tauri::command]
pub fn toggle_sort(
    node_index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<ToggleSortResult, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;

    if let Some(idx) = node_index {
        // Sort only children of the specified node
        let Some(parent) = core.all_nodes.get(idx) else {
            return Err(format!("Node index {idx} out of range"));
        };
        let is_diagcomm =
            parent.service_list_type() == Some(mdd_core::tree::ServiceListType::DiagComms);

        if is_diagcomm {
            core.diagcomm_sort = core.diagcomm_sort.next();
            let mode = core.diagcomm_sort;
            sort_children_of(idx, &mut core.all_nodes, |groups| {
                sort_groups_by_mode(groups, mode);
            });
        } else {
            sort_children_of(idx, &mut core.all_nodes, |children| {
                children.sort_by(|a, b| {
                    let at = a.first().map(|n| n.text.to_lowercase());
                    let bt = b.first().map(|n| n.text.to_lowercase());
                    at.cmp(&bt)
                });
            });
        }
    } else {
        // No node specified: cycle DiagComm sort globally
        core.diagcomm_sort = core.diagcomm_sort.next();
        let mode = core.diagcomm_sort;
        sort_diagcomm_nodes(&mut core.all_nodes, mode);
    }

    mdd_core::tree::resolve_all_indices(&mut core.all_nodes);
    core.visible = build_visible(core);
    let sort_label = core.diagcomm_sort.status_label().to_owned();
    Ok(ToggleSortResult {
        nodes: to_visible_nodes(core),
        sort_label,
    })
}

/// Sort `DiagComm` children using the given mode.
fn sort_groups_by_mode(groups: &mut [Vec<TreeNode>], mode: DiagcommSortMode) {
    match mode {
        DiagcommSortMode::IdAsc => {
            groups.sort_by_key(|g| g.first().and_then(|n| extract_service_id(&n.text)));
        }
        DiagcommSortMode::IdDesc => {
            groups.sort_by(|a, b| {
                let a_id = a.first().and_then(|n| extract_service_id(&n.text));
                let b_id = b.first().and_then(|n| extract_service_id(&n.text));
                b_id.cmp(&a_id)
            });
        }
        DiagcommSortMode::NameAsc => {
            groups.sort_by(|a, b| {
                let a_name = a
                    .first()
                    .and_then(|n| n.service_short_name())
                    .unwrap_or_default();
                let b_name = b
                    .first()
                    .and_then(|n| n.service_short_name())
                    .unwrap_or_default();
                a_name.cmp(b_name)
            });
        }
        DiagcommSortMode::NameDesc => {
            groups.sort_by(|a, b| {
                let a_name = a
                    .first()
                    .and_then(|n| n.service_short_name())
                    .unwrap_or_default();
                let b_name = b
                    .first()
                    .and_then(|n| n.service_short_name())
                    .unwrap_or_default();
                b_name.cmp(a_name)
            });
        }
    }
}

/// Sort `DiagComm` sections with the given mode.
fn sort_diagcomm_nodes(nodes: &mut Vec<TreeNode>, mode: DiagcommSortMode) {
    let sections: Vec<(usize, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.service_list_type() == Some(mdd_core::tree::ServiceListType::DiagComms))
        .map(|(i, n)| {
            let depth = n.depth;
            let start = i.saturating_add(1);
            let end = nodes
                .iter()
                .skip(start)
                .position(|m| m.depth <= depth)
                .map_or(nodes.len(), |pos| start.saturating_add(pos));
            (start, end)
        })
        .collect();

    for (start, end) in sections.into_iter().rev() {
        if end <= start {
            continue;
        }
        let mut services: Vec<TreeNode> = nodes.drain(start..end).collect();
        match mode {
            DiagcommSortMode::IdAsc => {
                services.sort_by_key(|n| extract_service_id(&n.text));
            }
            DiagcommSortMode::IdDesc => {
                services.sort_by_key(|b| std::cmp::Reverse(extract_service_id(&b.text)));
            }
            DiagcommSortMode::NameAsc => {
                services.sort_by(|a, b| {
                    a.service_short_name()
                        .unwrap_or_default()
                        .cmp(b.service_short_name().unwrap_or_default())
                });
            }
            DiagcommSortMode::NameDesc => {
                services.sort_by(|a, b| {
                    b.service_short_name()
                        .unwrap_or_default()
                        .cmp(a.service_short_name().unwrap_or_default())
                });
            }
        }
        nodes.splice(start..start, services);
    }
}

/// Sort direct children of a single parent node.
/// `sort_fn` receives grouped subtrees (Vec of Vec<TreeNode>) and sorts them in place.
fn sort_children_of(
    parent_idx: usize,
    nodes: &mut Vec<TreeNode>,
    sort_fn: impl FnOnce(&mut Vec<Vec<TreeNode>>),
) {
    let Some(parent) = nodes.get(parent_idx) else {
        return;
    };
    let parent_depth = parent.depth;
    let children_start = parent_idx.saturating_add(1);
    let children_end = nodes
        .iter()
        .skip(children_start)
        .position(|n| n.depth <= parent_depth)
        .map_or(nodes.len(), |pos| children_start.saturating_add(pos));

    if children_end <= children_start {
        return;
    }

    let direct_child_depth = parent_depth.saturating_add(1);
    let all_children: Vec<TreeNode> = nodes.drain(children_start..children_end).collect();

    let mut groups: Vec<Vec<TreeNode>> = Vec::new();
    let mut current: Vec<TreeNode> = Vec::new();
    for node in all_children {
        if node.depth == direct_child_depth && !current.is_empty() {
            groups.push(std::mem::take(&mut current));
        }
        current.push(node);
    }
    if !current.is_empty() {
        groups.push(current);
    }

    sort_fn(&mut groups);

    let sorted: Vec<TreeNode> = groups.into_iter().flatten().collect();
    nodes.splice(children_start..children_start, sorted);
}

/// Apply ID-ascending sort to `DiagComms`, `Requests`,
/// `PosResponses`, `NegResponses` on initial load.
fn apply_default_sort(nodes: &mut Vec<TreeNode>) {
    sort_diagcomm_nodes(nodes, DiagcommSortMode::IdAsc);
    for list_type in [
        ServiceListType::Requests,
        ServiceListType::PosResponses,
        ServiceListType::NegResponses,
    ] {
        sort_service_section_by_id(nodes, list_type);
    }
}

/// Sort direct children of service-list sections (Requests / Responses) by service ID,
/// preserving each top-level child together with all its descendants as a group.
fn sort_service_section_by_id(nodes: &mut Vec<TreeNode>, list_type: ServiceListType) {
    let sections: Vec<(usize, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.service_list_type() == Some(list_type))
        .map(|(i, n)| {
            let depth = n.depth;
            let start = i.saturating_add(1);
            let end = nodes
                .iter()
                .skip(start)
                .position(|m| m.depth <= depth)
                .map_or(nodes.len(), |pos| start.saturating_add(pos));
            (start, end)
        })
        .collect();

    for (start, end) in sections.into_iter().rev() {
        if end <= start {
            continue;
        }
        let direct_depth = nodes.get(start).map_or(0, |n| n.depth);
        let all_children: Vec<TreeNode> = nodes.drain(start..end).collect();

        let mut groups: Vec<Vec<TreeNode>> = Vec::new();
        let mut current: Vec<TreeNode> = Vec::new();
        for node in all_children {
            if node.depth == direct_depth && !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            current.push(node);
        }
        if !current.is_empty() {
            groups.push(current);
        }

        groups.sort_by_key(|g| g.first().and_then(|n| extract_service_id(&n.text)));

        let sorted: Vec<TreeNode> = groups.into_iter().flatten().collect();
        nodes.splice(start..start, sorted);
    }
}

fn extract_service_id(text: &str) -> Option<u32> {
    let hex_part = text.strip_prefix("0x")?;
    let dash_pos = hex_part.find(" - ")?;
    u32::from_str_radix(hex_part[..dash_pos].trim(), 16).ok()
}

#[tauri::command]
pub fn expand_all(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    for node in &mut core.all_nodes {
        if node.has_children {
            node.expanded = true;
        }
    }
    core.visible = build_visible(core);
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn collapse_all(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    for node in &mut core.all_nodes {
        if node.has_children {
            node.expanded = node.depth == 0;
        }
    }
    core.visible = build_visible(core);
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn expand_first_level(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    for node in &mut core.all_nodes {
        if node.has_children && node.depth == 0 {
            node.expanded = true;
        }
    }
    core.visible = build_visible(core);
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn toggle_hide_unchanged(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;
    core.hide_unchanged = !core.hide_unchanged;
    core.visible = build_visible(core);
    Ok(to_visible_nodes(core))
}

#[tauri::command]
pub fn navigate_to(
    target: JumpTarget,
    state: State<'_, AppState>,
) -> Result<NavigateResult, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core_mut()?;

    let target_idx = resolve_jump_target(&core.all_nodes, &target.target_type)
        .ok_or_else(|| "Could not resolve navigation target".to_owned())?;

    // Expand all ancestors so the target becomes visible
    expand_ancestors(&mut core.all_nodes, target_idx);

    // Expand the target itself so its children are immediately visible
    if let Some(node) = core.all_nodes.get_mut(target_idx)
        && node.has_children
    {
        node.expanded = true;
    }

    core.visible = build_visible(core);

    let detail = core
        .all_nodes
        .get(target_idx)
        .map(|n| n.detail_sections.to_vec())
        .unwrap_or_default();

    Ok(NavigateResult {
        visible: to_visible_nodes(core),
        target_index: target_idx,
        detail,
    })
}

/// Resolve a jump target to a concrete node index.
fn resolve_jump_target(nodes: &[TreeNode], target: &JumpTargetType) -> Option<usize> {
    match target {
        JumpTargetType::TreeNodeByIndex { index, short_name } => {
            let lower = short_name.to_lowercase();
            let exact = |n: &TreeNode| {
                n.short_name().is_some_and(|sn| sn == short_name)
                    || n.service_short_name().is_some_and(|sn| sn == short_name)
                    || n.text == *short_name
            };
            let icase = |n: &TreeNode| {
                n.short_name().is_some_and(|sn| sn.to_lowercase() == lower)
                    || n.service_short_name()
                        .is_some_and(|sn| sn.to_lowercase() == lower)
                    || n.text.to_lowercase() == lower
            };
            // Prefer exact match at the hinted index, then exact anywhere, then case-insensitive
            if nodes.get(*index).is_some_and(exact) {
                Some(*index)
            } else {
                nodes
                    .iter()
                    .position(exact)
                    .or_else(|| nodes.iter().position(icase))
            }
        }
        JumpTargetType::Dop { index, name } => {
            if nodes
                .get(*index)
                .is_some_and(|n| n.short_name().is_some_and(|sn| sn == name) || n.text == *name)
            {
                Some(*index)
            } else {
                nodes
                    .iter()
                    .position(|n| n.short_name().is_some_and(|sn| sn == name) || n.text == *name)
            }
        }
        JumpTargetType::Parameter { param_id } => {
            nodes.iter().position(|n| n.param_id() == Some(*param_id))
        }
        JumpTargetType::Container { index, short_name } => {
            let exact = |n: &TreeNode| n.short_name().is_some_and(|sn| sn == short_name);
            let icase = |n: &TreeNode| {
                let lower = short_name.to_lowercase();
                n.short_name().is_some_and(|sn| sn.to_lowercase() == lower)
            };
            if nodes.get(*index).is_some_and(exact) {
                Some(*index)
            } else {
                nodes
                    .iter()
                    .position(exact)
                    .or_else(|| nodes.iter().position(icase))
            }
        }
    }
}

/// Expand all ancestor nodes so that `target_idx` becomes visible.
fn expand_ancestors(nodes: &mut [TreeNode], target_idx: usize) {
    let Some(target) = nodes.get(target_idx) else {
        return;
    };
    let target_depth = target.depth;
    if target_depth == 0 {
        return;
    }

    // Walk backward to find ancestors at each decreasing depth level.
    // Collect indices first, then mutate, to avoid borrow conflicts.
    let mut ancestors = Vec::new();
    let mut depth_needed = target_depth;
    for i in (0..target_idx).rev() {
        let Some(node) = nodes.get(i) else { continue };
        if node.depth < depth_needed {
            ancestors.push(i);
            depth_needed = node.depth;
            if depth_needed == 0 {
                break;
            }
        }
    }
    for idx in ancestors {
        if let Some(n) = nodes.get_mut(idx) {
            n.expanded = true;
        }
    }
}

#[tauri::command]
pub fn get_node_path(index: usize, state: State<'_, AppState>) -> Result<String, String> {
    let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let core = manager.active_core()?;
    let node = core
        .all_nodes
        .get(index)
        .ok_or_else(|| format!("Node index {index} out of range"))?;

    let node_text = if node.service_list_type().is_some() {
        strip_count_suffix(&node.text).to_owned()
    } else {
        node.text.clone()
    };
    let mut parts = vec![node_text];
    let mut depth_needed = node.depth;

    if depth_needed > 0 {
        for i in (0..index).rev() {
            let Some(ancestor) = core.all_nodes.get(i) else {
                continue;
            };
            if ancestor.depth < depth_needed {
                let ancestor_text = if ancestor.service_list_type().is_some() {
                    strip_count_suffix(&ancestor.text).to_owned()
                } else {
                    ancestor.text.clone()
                };
                parts.push(ancestor_text);
                depth_needed = ancestor.depth;
                if depth_needed == 0 {
                    break;
                }
            }
        }
    }

    parts.reverse();
    Ok(parts.join(" / "))
}

// ---------------------------------------------------------------------------
// Tab management commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn switch_tab(tab_id: String, state: State<'_, AppState>) -> Result<LoadResult, String> {
    let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    if !manager.tabs.contains_key(&tab_id) {
        return Err(format!("Tab {tab_id} not found"));
    }
    manager.active_tab = Some(tab_id.clone());
    let entry = manager
        .tabs
        .get(&tab_id)
        .ok_or_else(|| "Tab not found".to_owned())?;
    Ok(LoadResult {
        tab_id,
        ecu_name: entry.core.ecu_name.clone(),
        node_count: entry.core.all_nodes.len(),
        visible: to_visible_nodes(&entry.core),
        is_diff: entry.core.is_diff_mode,
    })
}

#[tauri::command]
pub async fn close_tab(
    tab_id: String,
    state: State<'_, AppState>,
    uds_state: State<'_, UdsState>,
) -> Result<Option<LoadResult>, String> {
    {
        let mut manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.tabs.remove(&tab_id);
        if manager.active_tab.as_deref() == Some(&tab_id) {
            manager.active_tab = manager.tabs.keys().next().cloned();
        }
    }

    let mut uds = uds_state.0.lock().await;
    uds.remove(&tab_id);
    drop(uds);

    let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    match &manager.active_tab {
        Some(active_id) => {
            let entry = manager
                .tabs
                .get(active_id)
                .ok_or_else(|| "Active tab not found".to_owned())?;
            Ok(Some(LoadResult {
                tab_id: active_id.clone(),
                ecu_name: entry.core.ecu_name.clone(),
                node_count: entry.core.all_nodes.len(),
                visible: to_visible_nodes(&entry.core),
                is_diff: entry.core.is_diff_mode,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub fn get_open_tabs(state: State<'_, AppState>) -> Result<Vec<TabInfo>, String> {
    let manager = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    Ok(manager
        .tabs
        .iter()
        .map(|(id, entry)| TabInfo {
            id: id.clone(),
            display_name: entry.display_name.clone(),
            file_path: entry.file_path.clone(),
            is_diff: entry.is_diff,
            is_active: manager.active_tab.as_deref() == Some(id.as_str()),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Recent files management
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct RecentFile {
    pub path: String,
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct RecentFilesResult {
    pub files: Vec<RecentFile>,
}

fn get_recent_files_path(app: &AppHandle) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {e}"))?;
    Ok(cache_dir.join("mdd-ui").join("recent-files.json"))
}

#[tauri::command]
pub fn get_recent_files(app: AppHandle) -> Result<RecentFilesResult, String> {
    let path = get_recent_files_path(&app)?;

    // Read recent files from cache
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(RecentFilesResult { files: Vec::new() });
    };

    let mut files: Vec<RecentFile> = serde_json::from_str(&content).unwrap_or_default();

    // Filter out files that don't exist
    files.retain(|f| PathBuf::from(&f.path).exists());

    // Write back the filtered list
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }
    let json = serde_json::to_string(&files)
        .map_err(|e| format!("Failed to serialize recent files: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write recent files: {e}"))?;

    Ok(RecentFilesResult { files })
}

#[tauri::command]
pub fn add_recent_file(path: String, app: AppHandle) -> Result<(), String> {
    let cache_path = get_recent_files_path(&app)?;

    // Create cache directory if it doesn't exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }

    // Read existing recent files
    let mut files: Vec<RecentFile> = if cache_path.exists() {
        let content = fs::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read recent files: {e}"))?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Remove the file if it already exists (to move it to the top)
    files.retain(|f| f.path != path);

    // Add the file to the top with current timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time is before UNIX epoch")
        .as_secs()
        .cast_signed();
    files.insert(0, RecentFile { path, timestamp });

    // Keep only the most recent 20 files
    files.truncate(20);

    // Write back to cache
    let json = serde_json::to_string(&files)
        .map_err(|e| format!("Failed to serialize recent files: {e}"))?;
    fs::write(&cache_path, json).map_err(|e| format!("Failed to write recent files: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn clear_recent_files(app: AppHandle) -> Result<(), String> {
    let path = get_recent_files_path(&app)?;

    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to remove recent files: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
pub fn clear_all_caches(app: AppHandle) -> Result<(), String> {
    let cache_dir = app
        .path()
        .cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {e}"))?
        .join("mdd-ui");

    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to clear cache directory: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
pub fn remove_recent_file(path: String, app: AppHandle) -> Result<(), String> {
    let cache_path = get_recent_files_path(&app)?;
    if !cache_path.exists() {
        return Ok(());
    }
    let content =
        fs::read_to_string(&cache_path).map_err(|e| format!("Failed to read recent files: {e}"))?;
    let mut files: Vec<RecentFile> = serde_json::from_str(&content).unwrap_or_default();
    files.retain(|f| f.path != path);
    let json = serde_json::to_string(&files)
        .map_err(|e| format!("Failed to serialize recent files: {e}"))?;
    fs::write(&cache_path, json).map_err(|e| format!("Failed to write recent files: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// UI preferences (font size, etc.)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct UiPrefs {
    pub font_size: u8,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_split_pct")]
    pub split_pct: u8,
    #[serde(default = "default_row_density")]
    pub row_density: String,
    #[serde(default)]
    pub default_hide_unchanged: bool,
    #[serde(default)]
    pub auto_expand_first_level: bool,
    #[serde(default = "default_max_recent_files")]
    pub max_recent_files: u8,
    #[serde(default)]
    pub wrap_table_text: bool,
    #[serde(default)]
    pub last_tab_title: Option<String>,
    #[serde(default)]
    pub auto_check_updates: bool,
}

fn default_theme() -> String {
    "dark".to_owned()
}
fn default_split_pct() -> u8 {
    35
}
fn default_row_density() -> String {
    "comfortable".to_owned()
}
fn default_max_recent_files() -> u8 {
    10
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            font_size: 13,
            theme: "dark".to_owned(),
            split_pct: 35,
            row_density: "comfortable".to_owned(),
            default_hide_unchanged: false,
            auto_expand_first_level: false,
            max_recent_files: 10,
            wrap_table_text: false,
            last_tab_title: None,
            auto_check_updates: false,
        }
    }
}

fn get_prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let cache_dir = app
        .path()
        .cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {e}"))?;
    Ok(cache_dir.join("mdd-ui").join("prefs.json"))
}

#[tauri::command]
pub fn get_ui_prefs(app: AppHandle) -> Result<UiPrefs, String> {
    let path = get_prefs_path(&app)?;
    if !path.exists() {
        return Ok(UiPrefs::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read prefs: {e}"))?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

#[tauri::command]
pub fn save_ui_prefs(prefs: UiPrefs, app: AppHandle) -> Result<(), String> {
    let path = get_prefs_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create cache directory: {e}"))?;
    }
    let json =
        serde_json::to_string(&prefs).map_err(|e| format!("Failed to serialize prefs: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write prefs: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// File association registration
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_initial_file(state: State<InitialFile>) -> Option<String> {
    state.0.lock().ok()?.take()
}

#[tauri::command]
pub fn register_mdd_association(_app: AppHandle) -> Result<String, String> {
    register_mdd_association_impl()
}

#[cfg(target_os = "macos")]
fn register_mdd_association_impl() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;

    let bundle_path = exe
        .ancestors()
        .find(|p| p.extension().is_some_and(|ext| ext == "app"))
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| {
            "Not running from an installed .app bundle. Build and install MDD UI first.".to_owned()
        })?;

    let lsregister_candidates = [
        "/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.\
         framework/Versions/A/Support/lsregister",
        "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/\
         Support/lsregister",
        "/System/Library/Frameworks/CoreServices.framework/Versions/A/Support/lsregister",
    ];
    let lsregister = lsregister_candidates
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .ok_or_else(|| "Cannot locate lsregister on this system".to_owned())?;
    let bundle_str = bundle_path
        .to_str()
        .ok_or_else(|| "Bundle path contains invalid UTF-8".to_owned())?;

    let output = std::process::Command::new(lsregister)
        .args(["-f", bundle_str])
        .output()
        .map_err(|e| format!("Failed to run lsregister: {e}"))?;

    if output.status.success() {
        Ok(
            "Registered with macOS Launch Services.\n\nTo set as default: right-click any .mdd \
             file \u{2192} Get Info \u{2192} Open With \u{2192} select MDD UI \u{2192} click \
             \u{201c}Change All\u{201d}."
                .to_owned(),
        )
    } else {
        Err(format!(
            "lsregister failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(target_os = "windows")]
fn register_mdd_association_impl() -> Result<String, String> {
    fn reg_add(key: &str, default_val: bool, name: &str, value: &str) -> Result<(), String> {
        let mut cmd = std::process::Command::new("reg");
        cmd.arg("add").arg(key);
        if default_val {
            cmd.arg("/ve");
        } else {
            cmd.args(["/v", name]);
        }
        cmd.args(["/d", value, "/f"]);
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run reg.exe: {e}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "reg.exe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    let exe = std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;
    let exe_str = exe.to_string_lossy();
    let prog_id = "io.github.alexmohr.mdd-ui.mddfile";
    let prog_key = format!(r"HKCU\Software\Classes\{prog_id}");
    let icon_key = format!(r"HKCU\Software\Classes\{prog_id}\DefaultIcon");
    let cmd_key = format!(r"HKCU\Software\Classes\{prog_id}\shell\open\command");
    let icon_val = format!("{exe_str},0");
    let cmd_val = format!("{exe_str} \"%1\"");

    reg_add(&prog_key, true, "", "MDD Database")?;
    reg_add(&icon_key, true, "", &icon_val)?;
    reg_add(&cmd_key, true, "", &cmd_val)?;
    reg_add(r"HKCU\Software\Classes\.mdd", true, "", prog_id)?;
    reg_add(
        r"HKCU\Software\Classes\.mdd",
        false,
        "Content Type",
        "application/x-mdd",
    )?;

    Ok("Registered as default handler for .mdd files.".to_owned())
}

#[cfg(target_os = "linux")]
fn register_mdd_association_impl() -> Result<String, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME environment variable not set".to_owned())?;
    let home_path = std::path::Path::new(&home);

    let mime_dir = home_path.join(".local/share/mime/packages");
    fs::create_dir_all(&mime_dir).map_err(|e| format!("Failed to create MIME directory: {e}"))?;

    let mime_content = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n  \
  <mime-type type=\"application/x-mdd\">\n    \
    <comment>MDD Diagnostic Database</comment>\n    \
    <glob pattern=\"*.mdd\"/>\n  \
  </mime-type>\n\
</mime-info>\n";
    fs::write(mime_dir.join("application-x-mdd.xml"), mime_content)
        .map_err(|e| format!("Failed to write MIME definition: {e}"))?;

    let _ = std::process::Command::new("update-mime-database")
        .arg(home_path.join(".local/share/mime"))
        .output();

    let exe = std::env::current_exe().map_err(|e| format!("Cannot locate executable: {e}"))?;
    let exe_str = exe.to_string_lossy();
    let apps_dir = home_path.join(".local/share/applications");
    fs::create_dir_all(&apps_dir)
        .map_err(|e| format!("Failed to create applications directory: {e}"))?;

    let desktop_content = format!(
        "[Desktop Entry]\nName=MDD UI\nComment=Diagnostic database browser\nExec={exe_str} \
         %f\nIcon=io.github.alexmohr.mdd-ui\nType=Application\nCategories=Utility;\\
         nMimeType=application/x-mdd;\n"
    );
    fs::write(
        apps_dir.join("io.github.alexmohr.mdd-ui.desktop"),
        desktop_content,
    )
    .map_err(|e| format!("Failed to write .desktop file: {e}"))?;

    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .output();

    let output = std::process::Command::new("xdg-mime")
        .args([
            "default",
            "io.github.alexmohr.mdd-ui.desktop",
            "application/x-mdd",
        ])
        .output()
        .map_err(|e| format!("xdg-mime not found: {e}. Install the xdg-utils package."))?;

    if output.status.success() {
        Ok("Registered as default handler for .mdd files (application/x-mdd).".to_owned())
    } else {
        Err(format!(
            "xdg-mime failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

// ---------------------------------------------------------------------------
// UDS translation commands
// ---------------------------------------------------------------------------

/// Load or reload the UDS translator for the given MDD path.
#[tauri::command]
pub async fn uds_load(
    path: String,
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let translator = mdd_core::uds::translator::UdsTranslator::new(&path)
        .await
        .map_err(|e| format!("Failed to init UDS translator: {e:#}"))?;
    let mut guard = state.0.lock().await;
    guard.insert(tab_id, translator);
    Ok(())
}

/// List all services available in the currently loaded MDD.
#[tauri::command]
pub async fn uds_list_services(
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<MatchedService>, String> {
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let guard = state.0.lock().await;
    let translator = guard
        .get(&tab_id)
        .ok_or_else(|| "UDS translator not initialised \u{2013} load an MDD first".to_owned())?;
    Ok(translator.list_services())
}

/// Look up which service(s) match a hex UDS byte string (e.g. `"22 F1 90"`).
#[tauri::command]
pub async fn uds_lookup(
    hex: String,
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<UdsLookupResult, String> {
    let bytes = mdd_core::uds::translator::parse_hex_string(&hex)
        .map_err(|e| format!("Invalid hex: {e:#}"))?;
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let guard = state.0.lock().await;
    let translator = guard
        .get(&tab_id)
        .ok_or_else(|| "UDS translator not initialised \u{2013} load an MDD first".to_owned())?;
    translator
        .lookup_service(&bytes)
        .map_err(|e| format!("{e:#}"))
}

/// Encode a JSON parameter map into raw UDS bytes for a named service.
#[tauri::command]
pub async fn uds_encode(
    service_name: String,
    json: serde_json::Value,
    variant_name: Option<String>,
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<UdsEncodeResult, String> {
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let mut guard = state.0.lock().await;
    let translator = guard
        .get_mut(&tab_id)
        .ok_or_else(|| "UDS translator not initialised \u{2013} load an MDD first".to_owned())?;
    if let Some(ref vn) = variant_name {
        translator
            .ensure_variant(vn)
            .await
            .map_err(|e| format!("{e:#}"))?;
    }
    translator
        .uds_encode(&service_name, &json)
        .await
        .map_err(|e| format!("{e:#}"))
}

/// Return the JSON Schema for a service's request and response parameters.
#[tauri::command]
pub async fn service_schema(
    service_name: String,
    variant_name: Option<String>,
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<ServiceSchemaResult, String> {
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let mut guard = state.0.lock().await;
    let translator = guard
        .get_mut(&tab_id)
        .ok_or_else(|| "UDS translator not initialised \u{2013} load an MDD first".to_owned())?;
    if let Some(ref vn) = variant_name {
        translator
            .ensure_variant(vn)
            .await
            .map_err(|e| format!("{e:#}"))?;
    }
    Ok(translator.service_schema(&service_name).await)
}

/// List all variants in the currently loaded MDD.
#[tauri::command]
pub async fn uds_list_variants(
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<VariantInfo>, String> {
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let guard = state.0.lock().await;
    let translator = guard
        .get(&tab_id)
        .ok_or_else(|| "UDS translator not initialised \u{2013} load an MDD first".to_owned())?;
    translator.list_variants().map_err(|e| format!("{e:#}"))
}

/// Switch the active variant by name using crafted detection responses.
#[tauri::command]
pub async fn uds_select_variant(
    variant_name: String,
    state: State<'_, UdsState>,
    app_state: State<'_, AppState>,
) -> Result<VariantInfo, String> {
    let tab_id = {
        let manager = app_state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
        manager.active_tab_id()?
    };
    let mut guard = state.0.lock().await;
    let translator = guard
        .get_mut(&tab_id)
        .ok_or_else(|| "UDS translator not initialised \u{2013} load an MDD first".to_owned())?;
    translator
        .select_variant(&variant_name)
        .await
        .map_err(|e| format!("{e:#}"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn register_mdd_association_impl() -> Result<String, String> {
    Err("File association registration is not supported on this platform.".to_owned())
}
