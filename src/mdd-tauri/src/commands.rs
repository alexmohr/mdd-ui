// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use std::sync::Mutex;

use mdd_core::tree::{
    DiffStatus, DetailSectionData, NodeType, TreeNode,
};
use serde::{Deserialize, Serialize};
use tauri::State;

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

pub struct CoreState {
    pub all_nodes: Vec<TreeNode>,
    pub visible: Vec<usize>,
    pub ecu_name: String,
    pub is_diff_mode: bool,
    pub hide_unchanged: bool,
    pub search_stack: Vec<SearchEntry>,
    pub search_scope: SearchScope,
    pub diagcomm_sort_by_id: bool,
}

#[derive(Clone)]
pub struct SearchEntry {
    pub query: String,
    pub scope: SearchScope,
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
            diagcomm_sort_by_id: true,
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
        let mut inc = vec![true; state.all_nodes.len()];
        for entry in &state.search_stack {
            inc = apply_search_filter(&state.all_nodes, &inc, &entry.query, &entry.scope);
        }
        Some(inc)
    } else {
        None
    };

    for (i, node) in state.all_nodes.iter().enumerate() {
        // If search is active, skip nodes not in the include set
        if let Some(ref inc) = include {
            if !inc.get(i).copied().unwrap_or(false) {
                continue;
            }
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
                if included {
                    if let Some(slot) = new_include.get_mut(i) {
                        *slot = true;
                    }
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
                let Some(&ancestor) = parent_at_depth.get(d) else { break };
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
            matches!(node.node_type, NodeType::Container | NodeType::SectionHeader)
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
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn load_mdd(path: String, state: State<'_, AppState>) -> Result<LoadResult, String> {
    let db = mdd_core::database::load_mdd(&path)
        .map_err(|e| format!("Failed to load: {e:#}"))?;
    let (nodes, ecu_name) = mdd_core::tree::build_tree(&db, &path);
    let node_count = nodes.len();

    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.all_nodes = nodes;
    core.ecu_name = ecu_name.clone();
    core.is_diff_mode = false;
    core.hide_unchanged = false;
    core.search_stack.clear();
    core.diagcomm_sort_by_id = true;
    core.visible = build_visible(&core);

    Ok(LoadResult {
        ecu_name,
        node_count,
        visible: to_visible_nodes(&core),
        is_diff: false,
    })
}

#[tauri::command]
pub fn load_diff(
    old_path: String,
    new_path: String,
    state: State<'_, AppState>,
) -> Result<LoadResult, String> {
    let db_old = mdd_core::database::load_mdd(&old_path)
        .map_err(|e| format!("Failed to load old: {e:#}"))?;
    let db_new = mdd_core::database::load_mdd(&new_path)
        .map_err(|e| format!("Failed to load new: {e:#}"))?;
    let (nodes, ecu_name) =
        mdd_core::diff::diff_tree::build_diff_tree(&db_old, &db_new, &old_path, &new_path);
    let node_count = nodes.len();

    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.all_nodes = nodes;
    core.ecu_name = ecu_name.clone();
    core.is_diff_mode = true;
    core.hide_unchanged = false;
    core.search_stack.clear();
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
pub fn toggle_expand(
    index: usize,
    state: State<'_, AppState>,
) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    if let Some(node) = core.all_nodes.get_mut(index) {
        if node.has_children {
            node.expanded = !node.expanded;
        }
    }
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

#[tauri::command]
pub fn search(query: String, state: State<'_, AppState>) -> Result<SearchResult, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    if !query.is_empty() {
        let scope = core.search_scope.clone();
        core.search_stack.push(SearchEntry {
            query,
            scope,
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
pub fn toggle_sort(state: State<'_, AppState>) -> Result<Vec<VisibleNode>, String> {
    let mut core = state.0.lock().map_err(|e| format!("Lock error: {e}"))?;
    core.diagcomm_sort_by_id = !core.diagcomm_sort_by_id;
    let by_id = core.diagcomm_sort_by_id;
    sort_diagcomm_nodes(&mut core.all_nodes, by_id);
    sort_all_children_by_name(&mut core.all_nodes);
    mdd_core::tree::resolve_all_indices(&mut core.all_nodes);
    core.visible = build_visible(&core);
    Ok(to_visible_nodes(&core))
}

/// Sort DiagComm sections by ID or name.
fn sort_diagcomm_nodes(nodes: &mut Vec<TreeNode>, by_id: bool) {
    let sections: Vec<(usize, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            n.service_list_type()
                == Some(mdd_core::tree::ServiceListType::DiagComms)
        })
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
        if by_id {
            services.sort_by_key(|n| extract_service_id(&n.text));
        } else {
            services.sort_by(|a, b| {
                let a_name = a.service_short_name().unwrap_or_default();
                let b_name = b.service_short_name().unwrap_or_default();
                a_name.cmp(b_name)
            });
        }
        nodes.splice(start..start, services);
    }
}

/// Sort direct children of every non-DiagComm parent node alphabetically.
/// Preserves subtrees: each direct child and all its descendants move together.
fn sort_all_children_by_name(nodes: &mut Vec<TreeNode>) {
    // Find all parent nodes that have children (skip DiagComm headers, already sorted)
    let parents: Vec<(usize, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            n.has_children
                && n.service_list_type().is_none()
        })
        .map(|(i, n)| (i, n.depth))
        .collect();

    // Process in reverse order to keep indices stable
    for (parent_idx, parent_depth) in parents.into_iter().rev() {
        let children_start = parent_idx.saturating_add(1);
        let children_end = nodes
            .iter()
            .skip(children_start)
            .position(|n| n.depth <= parent_depth)
            .map_or(nodes.len(), |pos| children_start.saturating_add(pos));

        if children_end <= children_start {
            continue;
        }

        let direct_child_depth = parent_depth.saturating_add(1);
        let all_children: Vec<TreeNode> = nodes.drain(children_start..children_end).collect();

        // Group into subtrees (each direct child + its descendants)
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

        groups.sort_by(|a, b| {
            let a_text = a.first().map(|n| n.text.to_lowercase());
            let b_text = b.first().map(|n| n.text.to_lowercase());
            a_text.cmp(&b_text)
        });

        let sorted: Vec<TreeNode> = groups.into_iter().flatten().collect();
        nodes.splice(children_start..children_start, sorted);
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
            // Verify the index still points to the right node; fallback to name search
            if nodes.get(*index).is_some_and(|n| {
                n.short_name().is_some_and(|sn| sn == short_name)
                    || n.service_short_name().is_some_and(|sn| sn == short_name)
                    || n.text == *short_name
            }) {
                Some(*index)
            } else {
                // Fallback: search by name
                nodes.iter().position(|n| {
                    n.short_name().is_some_and(|sn| sn == short_name)
                        || n.service_short_name().is_some_and(|sn| sn == short_name)
                        || n.text == *short_name
                })
            }
        }
        JumpTargetType::Dop { index, name } => {
            if nodes.get(*index).is_some_and(|n| {
                n.short_name().is_some_and(|sn| sn == name) || n.text == *name
            }) {
                Some(*index)
            } else {
                nodes.iter().position(|n| {
                    n.short_name().is_some_and(|sn| sn == name) || n.text == *name
                })
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
