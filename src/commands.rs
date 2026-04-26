// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

// Tauri commands require owned types for JSON deserialization and state injection.
#![allow(clippy::needless_pass_by_value)]

use std::{fs, path::PathBuf, sync::Mutex};

use mdd_core::tree::{DetailSectionData, DiffStatus, NodeType, ServiceListType, TreeNode};
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
}

#[derive(Serialize)]
pub struct LoadResult {
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

pub struct AppState(pub Mutex<CoreState>);

impl Default for AppState {
    fn default() -> Self {
        Self(Mutex::new(CoreState::default()))
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn build_visible(state: &CoreState) -> Vec<usize> {
    let mut visible = Vec::new();
    let mut collapsed_below: Option<usize> = None;

    let has_search = !state.search_stack.is_empty();

    // When searching, first compute include flags
    let include = if has_search {
        let all_true = vec![true; state.all_nodes.len()];
        let mut inc: Option<Vec<bool>> = None;
        for entry in &state.search_stack {
            let fresh =
                apply_search_filter(&state.all_nodes, &all_true, &entry.query, &entry.scope);
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
    } else {
        None
    };

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
    state
        .visible
        .iter()
        .filter_map(|&idx| {
            state.all_nodes.get(idx).map(|node| VisibleNode {
                index: idx,
                depth: node.depth,
                text: node.text.clone(),
                expanded: node.expanded,
                has_children: node.has_children,
                node_type: node.node_type,
                diff_status: node.diff_status,
                is_sortable: node.service_list_type().is_some(),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn load_mdd(path: String, state: State<'_, AppState>) -> Result<LoadResult, String> {
    let (nodes, ecu_name) = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let db =
            mdd_core::database::load_mdd(&path).map_err(|e| format!("Failed to load: {e:#}"))?;
        Ok(mdd_core::tree::build_tree(&db, &path))
    })
    .await
    .map_err(|e| format!("Thread error: {e}"))??;
    let node_count = nodes.len();

    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.all_nodes = nodes;
    core.ecu_name.clone_from(&ecu_name);
    core.is_diff_mode = false;
    core.hide_unchanged = false;
    core.search_stack.clear();
    core.diagcomm_sort = DiagcommSortMode::IdAsc;
    apply_default_sort(&mut core.all_nodes);
    mdd_core::tree::resolve_all_indices(&mut core.all_nodes);
    core.visible = build_visible(&core);

    Ok(LoadResult {
        ecu_name,
        node_count,
        visible: to_visible_nodes(&core),
        is_diff: false,
    })
}

#[tauri::command]
pub async fn load_diff(
    old_path: String,
    new_path: String,
    state: State<'_, AppState>,
) -> Result<LoadResult, String> {
    let (nodes, ecu_name) = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let db_old = mdd_core::database::load_mdd(&old_path)
            .map_err(|e| format!("Failed to load old: {e:#}"))?;
        let db_new = mdd_core::database::load_mdd(&new_path)
            .map_err(|e| format!("Failed to load new: {e:#}"))?;
        Ok(mdd_core::diff::diff_tree::build_diff_tree(
            &db_old, &db_new, &old_path, &new_path,
        ))
    })
    .await
    .map_err(|e| format!("Thread error: {e}"))??;
    let node_count = nodes.len();

    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.all_nodes = nodes;
    core.ecu_name.clone_from(&ecu_name);
    core.is_diff_mode = true;
    core.hide_unchanged = false;
    core.search_stack.clear();
    core.diagcomm_sort = DiagcommSortMode::IdAsc;
    apply_default_sort(&mut core.all_nodes);
    mdd_core::tree::resolve_all_indices(&mut core.all_nodes);
    core.visible = build_visible(&core);

    Ok(LoadResult {
        ecu_name,
        node_count,
        visible: to_visible_nodes(&core),
        is_diff: true,
    })
}

#[tauri::command]
pub fn get_visible_nodes(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn get_node_detail(
    index: usize,
    state: State<'_, AppState>,
) -> Result<Vec<DetailSectionData>, String> {
    let core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let node = core
        .all_nodes
        .get(index)
        .ok_or_else(|| format!("Node index {index} out of range"))?;
    Ok(node.detail_sections.to_vec())
}

#[tauri::command]
pub fn toggle_expand(index: usize, state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    if let Some(node) = core.all_nodes.get_mut(index)
        && node.has_children
    {
        node.expanded = !node.expanded;
    }
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn search(
    query: String,
    op: Option<String>,
    state: State<'_, AppState>,
) -> Result<SearchResult, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
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
    let visible = build_visible(&core);
    core.visible = visible;

    let match_count = core.search_stack.len();
    let scope = core.search_scope.to_string();
    Ok(SearchResult {
        visible: to_visible_nodes(&core),
        match_count,
        scope,
    })
}

#[tauri::command]
pub fn clear_search(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.search_stack.clear();
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn cycle_search_scope(state: State<'_, AppState>) -> Result<String, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
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
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
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
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;

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
    core.visible = build_visible(&core);
    let sort_label = core.diagcomm_sort.status_label().to_owned();
    Ok(ToggleSortResult {
        nodes: to_visible_nodes(&core),
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
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    for node in &mut core.all_nodes {
        if node.has_children {
            node.expanded = true;
        }
    }
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn collapse_all(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    for node in &mut core.all_nodes {
        if node.has_children {
            node.expanded = node.depth == 0;
        }
    }
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn expand_first_level(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    for node in &mut core.all_nodes {
        if node.has_children && node.depth == 0 {
            node.expanded = true;
        }
    }
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn toggle_hide_unchanged(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.hide_unchanged = !core.hide_unchanged;
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn navigate_to(
    target: JumpTarget,
    state: State<'_, AppState>,
) -> Result<NavigateResult, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;

    let target_idx = resolve_jump_target(&core.all_nodes, &target.target_type)
        .ok_or_else(|| "Could not resolve navigation target".to_owned())?;

    // Expand all ancestors so the target becomes visible
    expand_ancestors(&mut core.all_nodes, target_idx);
    core.visible = build_visible(&core);

    let detail = core
        .all_nodes
        .get(target_idx)
        .map(|n| n.detail_sections.to_vec())
        .unwrap_or_default();

    Ok(NavigateResult {
        visible: to_visible_nodes(&core),
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
    let core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    let node = core
        .all_nodes
        .get(index)
        .ok_or_else(|| format!("Node index {index} out of range"))?;

    let mut parts = vec![node.text.clone()];
    let mut depth_needed = node.depth;

    if depth_needed > 0 {
        for i in (0..index).rev() {
            let Some(ancestor) = core.all_nodes.get(i) else {
                continue;
            };
            if ancestor.depth < depth_needed {
                parts.push(ancestor.text.clone());
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

    let lsregister =
        "/System/Library/Frameworks/CoreServices.framework/Versions/A/Support/lsregister";
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

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn register_mdd_association_impl() -> Result<String, String> {
    Err("File association registration is not supported on this platform.".to_owned())
}
