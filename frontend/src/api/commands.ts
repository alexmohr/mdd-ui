// SPDX-FileCopyrightText: 2026 Alexander Mohr
// SPDX-License-Identifier: Apache-2.0

import { invoke } from "@tauri-apps/api/core";

export interface VisibleNode {
  index: number;
  depth: number;
  text: string;
  expanded: boolean;
  has_children: boolean;
  node_type: string;
  diff_status: string | null;
  is_sortable: boolean;
}

export interface LoadResult {
  tab_id: string;
  ecu_name: string;
  node_count: number;
  visible: VisibleNode[];
  is_diff: boolean;
}

export interface TabInfo {
  id: string;
  display_name: string;
  file_path: string;
  is_diff: boolean;
  is_active: boolean;
}

export interface SearchResult {
  visible: VisibleNode[];
  match_count: number;
  scope: string;
}

export interface DetailSection {
  title: string;
  content: DetailContent;
  render_as_header: boolean;
  section_type: string;
  byte_pattern_rows?: DetailRow[] | null;
}

export type DetailContent =
  | { PlainText: string[] }
  | {
      Table: {
        header: DetailRow;
        rows: DetailRow[];
        constraints: unknown[];
        use_row_selection: boolean;
      };
    }
  | { Composite: DetailSection[] };

export interface DetailRow {
  cells: DetailCell[];
  indent: number;
  row_type: string;
  metadata: unknown | null;
  diff_status: string | null;
}

export interface DetailCell {
  text: string;
  cell_type: string;
  jump_target: JumpTarget | null;
}

export interface JumpTarget {
  target_type: JumpTargetType;
}

export type JumpTargetType =
  | { Parameter: { param_id: number } }
  | { Dop: { index: number; name: string } }
  | { TreeNodeByIndex: { index: number; short_name: string } };

export interface NavigateResult {
  visible: VisibleNode[];
  target_index: number;
  detail: DetailSection[];
}

export interface ToggleSortResult {
  nodes: VisibleNode[];
  sort_label: string;
}

export interface RecentFile {
  path: string;
  timestamp: number;
}

export interface RecentFilesResult {
  files: RecentFile[];
}

export async function loadMdd(path: string): Promise<LoadResult> {
  return invoke<LoadResult>("load_mdd", { path });
}

export async function loadDiff(
  oldPath: string,
  newPath: string,
): Promise<LoadResult> {
  return invoke<LoadResult>("load_diff", { oldPath, newPath });
}

export async function getVisibleNodes(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("get_visible_nodes");
}

export async function getNodeDetail(
  index: number,
): Promise<DetailSection[]> {
  return invoke<DetailSection[]>("get_node_detail", { index });
}

export async function toggleExpand(
  index: number,
): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("toggle_expand", { index });
}

export async function doSearch(query: string, op: 'and' | 'or' = 'and'): Promise<SearchResult> {
  return invoke<SearchResult>("search", { query, op });
}

export async function clearSearch(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("clear_search");
}

export async function cycleSearchScope(): Promise<string> {
  return invoke<string>("cycle_search_scope");
}

export async function setSearchScope(scope: string): Promise<string> {
  return invoke<string>("set_search_scope", { scope });
}

export async function toggleSort(nodeIndex?: number): Promise<ToggleSortResult> {
  return invoke<ToggleSortResult>("toggle_sort", { nodeIndex: nodeIndex ?? null });
}

export async function expandAll(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("expand_all");
}

export async function expandFirstLevel(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("expand_first_level");
}

export async function collapseAll(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("collapse_all");
}

export async function toggleHideUnchanged(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("toggle_hide_unchanged");
}

export async function navigateTo(
  target: JumpTarget,
): Promise<NavigateResult> {
  return invoke<NavigateResult>("navigate_to", { target });
}

export async function getNodePath(index: number): Promise<string> {
  return invoke<string>("get_node_path", { index });
}

export async function getRecentFiles(): Promise<RecentFilesResult> {
  return invoke<RecentFilesResult>("get_recent_files");
}

export async function addRecentFile(path: string): Promise<void> {
  return invoke("add_recent_file", { path });
}

export async function clearRecentFiles(): Promise<void> {
  return invoke("clear_recent_files");
}

export async function clearAllCaches(): Promise<void> {
  return invoke("clear_all_caches");
}

export async function removeRecentFile(path: string): Promise<void> {
  return invoke("remove_recent_file", { path });
}

export interface UiPrefs {
  font_size: number;
  theme: string;
  split_pct: number;
  row_density: string;
  default_hide_unchanged: boolean;
  auto_expand_first_level: boolean;
  max_recent_files: number;
  wrap_table_text: boolean;
  last_tab_title: string | null;
  auto_check_updates: boolean;
}

export async function getUiPrefs(): Promise<UiPrefs> {
  return invoke<UiPrefs>("get_ui_prefs");
}

export async function saveUiPrefs(prefs: UiPrefs): Promise<void> {
  return invoke("save_ui_prefs", { prefs });
}

export async function registerMddAssociation(): Promise<string> {
  return invoke<string>("register_mdd_association");
}

export async function getInitialFile(): Promise<string | null> {
  return invoke<string | null>("get_initial_file");
}

// ---------------------------------------------------------------------------
// Tab management
// ---------------------------------------------------------------------------

export async function switchTab(tabId: string): Promise<LoadResult> {
  return invoke<LoadResult>("switch_tab", { tabId });
}

export async function closeTab(tabId: string): Promise<LoadResult | null> {
  return invoke<LoadResult | null>("close_tab", { tabId });
}

export async function getOpenTabs(): Promise<TabInfo[]> {
  return invoke<TabInfo[]>("get_open_tabs");
}

// ---------------------------------------------------------------------------
// UDS translation
// ---------------------------------------------------------------------------

export interface MatchedService {
  name: string;
  service_type: string;
}

export interface UdsLookupResult {
  matched_services: MatchedService[];
  sid_name: string;
}

export interface UdsEncodeResult {
  service_name: string;
  hex_bytes: string;
  raw_bytes: number[];
}

export async function udsLoad(path: string): Promise<void> {
  return invoke("uds_load", { path });
}

export async function udsListServices(): Promise<MatchedService[]> {
  return invoke<MatchedService[]>("uds_list_services");
}

export async function udsLookup(hex: string): Promise<UdsLookupResult> {
  return invoke<UdsLookupResult>("uds_lookup", { hex });
}

export async function udsEncode(
  serviceName: string,
  json: unknown,
  variantName?: string | null,
): Promise<UdsEncodeResult> {
  return invoke<UdsEncodeResult>("uds_encode", {
    serviceName,
    json,
    variantName: variantName ?? null,
  });
}

export interface VariantInfo {
  name: string;
  is_base_variant: boolean;
  is_active: boolean;
}

export async function udsListVariants(): Promise<VariantInfo[]> {
  return invoke<VariantInfo[]>("uds_list_variants");
}

export async function udsSelectVariant(
  variantName: string,
): Promise<VariantInfo> {
  return invoke<VariantInfo>("uds_select_variant", { variantName });
}

export async function getNodeVariant(
  index: number,
): Promise<string | null> {
  return invoke<string | null>("get_node_variant", { index });
}
