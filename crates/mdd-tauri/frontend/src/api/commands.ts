import { invoke } from "@tauri-apps/api/core";

export interface VisibleNode {
  index: number;
  depth: number;
  text: string;
  expanded: boolean;
  has_children: boolean;
  node_type: string;
  diff_status: string | null;
}

export interface LoadResult {
  ecu_name: string;
  node_count: number;
  visible: VisibleNode[];
  is_diff: boolean;
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
  jump_target: unknown | null;
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

export async function doSearch(query: string): Promise<SearchResult> {
  return invoke<SearchResult>("search", { query });
}

export async function clearSearch(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("clear_search");
}

export async function cycleSearchScope(): Promise<string> {
  return invoke<string>("cycle_search_scope");
}

export async function toggleSort(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("toggle_sort");
}

export async function expandAll(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("expand_all");
}

export async function collapseAll(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("collapse_all");
}

export async function toggleHideUnchanged(): Promise<VisibleNode[]> {
  return invoke<VisibleNode[]>("toggle_hide_unchanged");
}
