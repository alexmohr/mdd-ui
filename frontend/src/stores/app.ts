// SPDX-FileCopyrightText: 2026 Alexander Mohr
// SPDX-License-Identifier: Apache-2.0

import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import type {
  VisibleNode,
  DetailSection,
  JumpTarget,
  RecentFile,
  TabInfo,
} from "../api/commands";
import * as api from "../api/commands";

export interface HistoryEntry {
  index: number;
  text: string;
}

export interface SearchFilter {
  query: string;
  scope: string;
  op: 'and' | 'or';
}

interface TabUiState {
  selectedIndex: number | null;
  detailSections: DetailSection[];
  detailTabIndex: number;
  history: HistoryEntry[];
  forwardHistory: HistoryEntry[];
  searchFilters: SearchFilter[];
  searchQuery: string;
  searchScope: string;
  searchActive: boolean;
  sortLabel: string;
  lastTabKey: { title: string; section_type: string } | null;
  hideUnchanged: boolean;
  udsReady: boolean;
  udsLoading: boolean;
  udsError: string | null;
}

export const useAppStore = defineStore("app", () => {
  const nodes = ref<VisibleNode[]>([]);
  const ecuName = ref("");
  const nodeCount = ref(0);
  const isDiff = ref(false);
  const selectedIndex = ref<number | null>(null);
  const detailSections = ref<DetailSection[]>([]);
  const selectedTab = ref(0);
  const searchQuery = ref("");
  const searchScope = ref("All");
  const searchActive = ref(false);
  const status = ref("");
  const loading = ref(false);
  const history = ref<HistoryEntry[]>([]);
  const forwardHistory = ref<HistoryEntry[]>([]);
  const searchFilters = ref<SearchFilter[]>([]);
  const splitPct = ref(35);
  const fileLoaded = ref(false);
  const filePath = ref("");
  const hideUnchanged = ref(false);
  const fontSize = ref(13);
  const theme = ref<'dark' | 'light'>('dark');
  const sortLabel = ref("ID\u25b2");
  const recentFiles = ref<RecentFile[]>([]);
  const rowDensity = ref<'compact' | 'comfortable' | 'spacious'>('comfortable');
  const defaultHideUnchanged = ref(false);
  const autoExpandFirstLevel = ref(false);
  const maxRecentFiles = ref(10);
  const wrapTableText = ref(false);
  const lastTabTitle = ref<string | null>(null);
  const autoCheckUpdates = ref(false);
  const lastTabKey = ref<{ title: string; section_type: string } | null>(null);
  const udsLoading = ref(false);
  const udsReady = ref(false);
  const udsError = ref<string | null>(null);
  let _udsLoadPromise: Promise<void> | null = null;

  const openTabs = ref<TabInfo[]>([]);
  const activeTabId = ref<string | null>(null);
  const tabUiStates = new Map<string, TabUiState>();

  function saveCurrentTabUiState() {
    const id = activeTabId.value;
    if (!id) return;
    tabUiStates.set(id, {
      selectedIndex: selectedIndex.value,
      detailSections: detailSections.value,
      detailTabIndex: selectedTab.value,
      history: [...history.value],
      forwardHistory: [...forwardHistory.value],
      searchFilters: [...searchFilters.value],
      searchQuery: searchQuery.value,
      searchScope: searchScope.value,
      searchActive: searchActive.value,
      sortLabel: sortLabel.value,
      lastTabKey: lastTabKey.value,
      hideUnchanged: hideUnchanged.value,
      udsReady: udsReady.value,
      udsLoading: udsLoading.value,
      udsError: udsError.value,
    });
  }

  function restoreTabUiState(tabId: string) {
    const saved = tabUiStates.get(tabId);
    if (saved) {
      selectedIndex.value = saved.selectedIndex;
      detailSections.value = saved.detailSections;
      selectedTab.value = saved.detailTabIndex;
      history.value = saved.history;
      forwardHistory.value = saved.forwardHistory;
      searchFilters.value = saved.searchFilters;
      searchQuery.value = saved.searchQuery;
      searchScope.value = saved.searchScope;
      searchActive.value = saved.searchActive;
      sortLabel.value = saved.sortLabel;
      lastTabKey.value = saved.lastTabKey;
      hideUnchanged.value = saved.hideUnchanged;
      udsReady.value = saved.udsReady;
      udsLoading.value = saved.udsLoading;
      udsError.value = saved.udsError;
    } else {
      selectedIndex.value = null;
      detailSections.value = [];
      selectedTab.value = 0;
      history.value = [];
      forwardHistory.value = [];
      searchFilters.value = [];
      searchQuery.value = "";
      searchScope.value = "All";
      searchActive.value = false;
      sortLabel.value = "ID\u25b2";
      lastTabKey.value = null;
      hideUnchanged.value = false;
      udsReady.value = false;
      udsLoading.value = false;
      udsError.value = null;
    }
  }

  function applyLoadResult(result: api.LoadResult) {
    nodes.value = result.visible;
    ecuName.value = result.ecu_name;
    nodeCount.value = result.node_count;
    isDiff.value = result.is_diff;
    activeTabId.value = result.tab_id;
    fileLoaded.value = true;
  }

  async function refreshOpenTabs() {
    try {
      openTabs.value = await api.getOpenTabs();
    } catch {
    }
  }

  const selectedNode = computed(() =>
    nodes.value.find((n: VisibleNode) => n.index === selectedIndex.value) ?? null,
  );

  const canGoBack = computed(() => history.value.length > 0);

  const rowHeightPx = computed(() => {
    switch (rowDensity.value) {
      case 'compact': return 20;
      case 'spacious': return 30;
      default: return 24;
    }
  });

  const displayedRecentFiles = computed(() => recentFiles.value.slice(0, maxRecentFiles.value));
  const canGoForward = computed(() => forwardHistory.value.length > 0);

  const breadcrumbs = computed(() => {
    if (selectedIndex.value === null) return [];
    const crumbs: { index: number; text: string }[] = [];
    const idx = selectedIndex.value;
    const node = nodes.value.find((n: VisibleNode) => n.index === idx);
    if (!node) return [];
    crumbs.push({ index: node.index, text: node.text });
    let currentDepth = node.depth;
    const allVisible = nodes.value;
    const nodePos = allVisible.findIndex((n: VisibleNode) => n.index === idx);
    for (let i = nodePos - 1; i >= 0; i--) {
      const n = allVisible[i];
      if (n.depth < currentDepth) {
        crumbs.unshift({ index: n.index, text: n.text });
        currentDepth = n.depth;
        if (currentDepth === 0) break;
      }
    }
    return crumbs;
  });

  function _pushBack(index: number, text: string) {
    if (history.value.length > 50) history.value.shift();
    history.value.push({ index, text });
  }

  function persistPrefs() {
    api.saveUiPrefs({
      font_size: fontSize.value,
      theme: theme.value,
      split_pct: splitPct.value,
      row_density: rowDensity.value,
      default_hide_unchanged: defaultHideUnchanged.value,
      auto_expand_first_level: autoExpandFirstLevel.value,
      max_recent_files: maxRecentFiles.value,
      wrap_table_text: wrapTableText.value,
      last_tab_title: lastTabTitle.value,
      auto_check_updates: autoCheckUpdates.value,
    }).catch(() => {});
  }

  let _splitPctTimer: ReturnType<typeof setTimeout> | null = null;
  watch(splitPct, () => {
    if (_splitPctTimer) clearTimeout(_splitPctTimer);
    _splitPctTimer = setTimeout(persistPrefs, 500);
  });

  let _tabTitleTimer: ReturnType<typeof setTimeout> | null = null;

  function pushHistory(index: number, text: string) {
    _pushBack(index, text);
    forwardHistory.value = [];
  }

  async function loadFile(path: string) {
    loading.value = true;
    try {
      saveCurrentTabUiState();
      const result = await api.loadMdd(path);
      applyLoadResult(result);
      selectedIndex.value = null;
      detailSections.value = [];
      history.value = [];
      forwardHistory.value = [];
      searchFilters.value = [];
      filePath.value = path;
      status.value = `${result.node_count} nodes`;
      if (autoExpandFirstLevel.value) {
        nodes.value = await api.expandFirstLevel();
      }
      await api.addRecentFile(path);
      await loadRecentFiles();
      await refreshOpenTabs();
      udsLoading.value = true;
      udsReady.value = false;
      udsError.value = null;
      _udsLoadPromise = api.udsLoad(path).then(() => {
        udsReady.value = true;
      }).catch((e) => {
        udsError.value = String(e);
      }).finally(() => {
        udsLoading.value = false;
      });
    } catch (e) {
      status.value = `Error: ${e}`;
    } finally {
      loading.value = false;
    }
  }

  async function loadDiff(oldPath: string, newPath: string) {
    loading.value = true;
    try {
      saveCurrentTabUiState();
      const result = await api.loadDiff(oldPath, newPath);
      applyLoadResult(result);
      selectedIndex.value = null;
      detailSections.value = [];
      history.value = [];
      forwardHistory.value = [];
      searchFilters.value = [];
      filePath.value = "";
      status.value = `Diff: ${result.node_count} nodes`;
      if (defaultHideUnchanged.value) {
        nodes.value = await api.toggleHideUnchanged();
        hideUnchanged.value = true;
      }
      if (autoExpandFirstLevel.value) {
        nodes.value = await api.expandFirstLevel();
      }
      await refreshOpenTabs();
    } catch (e) {
      status.value = `Error: ${e}`;
    } finally {
      loading.value = false;
    }
  }

  function tabSectionsOf(sections: import('../api/commands').DetailSection[]): import('../api/commands').DetailSection[] {
    const first = sections[0];
    if (sections.length > 1 && first?.render_as_header && "PlainText" in first.content) {
      return sections.slice(1);
    }
    return sections;
  }

  function restoreTab(
    sections: import('../api/commands').DetailSection[],
    key: { title: string; section_type: string } | null,
  ) {
    if (!key) { selectedTab.value = 0; return; }
    const tabs = tabSectionsOf(sections);
    // Prefer exact title match, fall back to same section type
    let idx = key.title ? tabs.findIndex(t => t.title === key.title) : -1;
    if (idx < 0 && key.section_type) {
      idx = tabs.findIndex(t => t.section_type === key.section_type);
    }
    selectedTab.value = idx >= 0 ? idx : 0;
    if (idx >= 0) {
      const matched = tabs[idx];
      lastTabKey.value = { title: matched.title, section_type: matched.section_type };
      lastTabTitle.value = matched.title;
      if (_tabTitleTimer) clearTimeout(_tabTitleTimer);
      _tabTitleTimer = setTimeout(persistPrefs, 500);
    }
    // fallback to 0: intentionally do NOT update lastTabKey so the memory is preserved
  }

  function setSelectedTab(index: number) {
    selectedTab.value = index;
    const tabs = tabSectionsOf(detailSections.value);
    const tab = tabs[index];
    if (tab) {
      lastTabKey.value = { title: tab.title, section_type: tab.section_type };
      lastTabTitle.value = tab.title;
      if (_tabTitleTimer) clearTimeout(_tabTitleTimer);
      _tabTitleTimer = setTimeout(persistPrefs, 500);
    }
  }

  async function selectNode(index: number) {
    if (selectedIndex.value !== null && selectedIndex.value !== index) {
      const prev = selectedNode.value;
      if (prev) pushHistory(prev.index, prev.text);
    }
    const prevKey = lastTabKey.value;
    selectedIndex.value = index;
    try {
      const sections = await api.getNodeDetail(index);
      detailSections.value = sections;
      restoreTab(sections, prevKey);
    } catch (e) {
      detailSections.value = [];
      selectedTab.value = 0;
      status.value = `Error: ${e}`;
    }
  }

  async function goBack() {
    const entry = history.value.pop();
    if (!entry) return;
    if (selectedIndex.value !== null && selectedNode.value) {
      forwardHistory.value.push({ index: selectedNode.value.index, text: selectedNode.value.text });
    }
    const prevKey = lastTabKey.value;
    try {
      const result = await api.navigateTo({
        target_type: { TreeNodeByIndex: { index: entry.index, short_name: entry.text } },
      });
      nodes.value = result.visible;
      selectedIndex.value = result.target_index;
      detailSections.value = result.detail;
      restoreTab(result.detail, prevKey);
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function goForward() {
    const entry = forwardHistory.value.pop();
    if (!entry) return;
    if (selectedIndex.value !== null && selectedNode.value) {
      _pushBack(selectedNode.value.index, selectedNode.value.text);
    }
    const prevKey = lastTabKey.value;
    try {
      const result = await api.navigateTo({
        target_type: { TreeNodeByIndex: { index: entry.index, short_name: entry.text } },
      });
      nodes.value = result.visible;
      selectedIndex.value = result.target_index;
      detailSections.value = result.detail;
      restoreTab(result.detail, prevKey);
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function toggleExpand(index: number) {
    try {
      nodes.value = await api.toggleExpand(index);
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function search(query: string, op: 'and' | 'or' = 'and') {
    try {
      const result = await api.doSearch(query, op);
      nodes.value = result.visible;
      searchScope.value = result.scope;
      status.value = `${result.match_count} filter(s) active`;
      searchFilters.value.push({ query, scope: result.scope, op });
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function clearSearch() {
    try {
      nodes.value = await api.clearSearch();
      searchFilters.value = [];
      status.value = "";
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function _replayFilters(filters: SearchFilter[]) {
    nodes.value = await api.clearSearch();
    searchFilters.value = [];
    for (const f of filters) {
      await api.setSearchScope(f.scope);
      const result = await api.doSearch(f.query, f.op);
      nodes.value = result.visible;
      searchScope.value = result.scope;
      searchFilters.value.push(f);
    }
    status.value = filters.length > 0 ? `${filters.length} filter(s) active` : "";
  }

  async function removeSearchFilter(idx: number) {
    try {
      await _replayFilters(searchFilters.value.filter((_, i) => i !== idx));
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function toggleFilterOp(idx: number) {
    if (idx === 0) return;
    const updated = searchFilters.value.map((f, i) =>
      i === idx ? { ...f, op: (f.op === 'and' ? 'or' : 'and') as 'and' | 'or' } : f
    );
    try {
      await _replayFilters(updated);
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function cycleScope() {
    try {
      searchScope.value = await api.cycleSearchScope();
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function setScope(scope: string) {
    try {
      searchScope.value = await api.setSearchScope(scope);
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function nextChange() {
    if (!isDiff.value) return;
    const ns = nodes.value;
    const curPos = selectedIndex.value === null ? -1 : ns.findIndex(n => n.index === selectedIndex.value);
    for (let i = curPos + 1; i < ns.length; i++) {
      if (ns[i].diff_status !== null && ns[i].diff_status !== 'Unchanged') {
        await selectNode(ns[i].index); return;
      }
    }
    for (let i = 0; i <= Math.max(0, curPos - 1); i++) {
      if (ns[i].diff_status !== null && ns[i].diff_status !== 'Unchanged') {
        await selectNode(ns[i].index); return;
      }
    }
  }

  async function prevChange() {
    if (!isDiff.value) return;
    const ns = nodes.value;
    const curPos = selectedIndex.value === null ? ns.length : ns.findIndex(n => n.index === selectedIndex.value);
    for (let i = curPos - 1; i >= 0; i--) {
      if (ns[i].diff_status !== null && ns[i].diff_status !== 'Unchanged') {
        await selectNode(ns[i].index); return;
      }
    }
    for (let i = ns.length - 1; i > Math.max(0, curPos); i--) {
      if (ns[i].diff_status !== null && ns[i].diff_status !== 'Unchanged') {
        await selectNode(ns[i].index); return;
      }
    }
  }

  async function expandAll() {
    try { nodes.value = await api.expandAll(); } catch (e) { status.value = `Error: ${e}`; }
  }

  async function collapseAll() {
    try { nodes.value = await api.collapseAll(); } catch (e) { status.value = `Error: ${e}`; }
  }

  function increaseFontSize() {
    fontSize.value = Math.min(20, fontSize.value + 1);
    persistPrefs();
  }
  function decreaseFontSize() {
    fontSize.value = Math.max(9, fontSize.value - 1);
    persistPrefs();
  }
  function setFontSize(size: number) {
    fontSize.value = Math.max(9, Math.min(20, size));
    persistPrefs();
  }

  function setTheme(t: 'dark' | 'light') {
    theme.value = t;
    persistPrefs();
  }

  function setRowDensity(d: 'compact' | 'comfortable' | 'spacious') {
    rowDensity.value = d;
    persistPrefs();
  }
  function setDefaultHideUnchanged(v: boolean) {
    defaultHideUnchanged.value = v;
    persistPrefs();
  }
  function setAutoExpandFirstLevel(v: boolean) {
    autoExpandFirstLevel.value = v;
    persistPrefs();
  }
  function setMaxRecentFiles(n: number) {
    maxRecentFiles.value = n;
    persistPrefs();
  }
  function setWrapTableText(v: boolean) {
    wrapTableText.value = v;
    persistPrefs();
  }
  function setAutoCheckUpdates(v: boolean) {
    autoCheckUpdates.value = v;
    persistPrefs();
  }

  /** Await UDS translator initialisation (resolves immediately if already done). */
  function waitForUds(): Promise<void> {
    return _udsLoadPromise ?? Promise.resolve();
  }

  async function toggleSort(nodeIndex?: number) {
    try {
      const idx = nodeIndex ?? selectedIndex.value ?? undefined;
      const result = await api.toggleSort(idx);
      nodes.value = result.nodes;
      status.value = result.sort_label;
      sortLabel.value = result.sort_label.replace("Sort: ", "").replace("Name ", "N").replace(" ", "");
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function toggleHideUnchanged() {
    try {
      nodes.value = await api.toggleHideUnchanged();
      hideUnchanged.value = !hideUnchanged.value;
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function navigateTo(target: JumpTarget) {
    if (selectedIndex.value !== null) {
      const prev = selectedNode.value;
      if (prev) pushHistory(prev.index, prev.text);
    }
    try {
      const result = await api.navigateTo(target);
      nodes.value = result.visible;
      selectedIndex.value = result.target_index;
      detailSections.value = result.detail;
      selectedTab.value = 0;
    } catch (e) {
      status.value = `Navigation failed: ${e}`;
    }
  }

  async function loadRecentFiles() {
    try {
      const result = await api.getRecentFiles();
      recentFiles.value = result.files;
    } catch (e) {
      console.error("Failed to load recent files:", e);
    }
  }

  async function loadPrefs() {
    try {
      const prefs = await api.getUiPrefs();
      fontSize.value = prefs.font_size;
      theme.value = (prefs.theme as 'dark' | 'light') ?? 'dark';
      if (prefs.split_pct >= 15 && prefs.split_pct <= 70) splitPct.value = prefs.split_pct;
      rowDensity.value = (prefs.row_density as 'compact' | 'comfortable' | 'spacious') ?? 'comfortable';
      defaultHideUnchanged.value = prefs.default_hide_unchanged ?? false;
      autoExpandFirstLevel.value = prefs.auto_expand_first_level ?? false;
      maxRecentFiles.value = prefs.max_recent_files ?? 10;
      wrapTableText.value = prefs.wrap_table_text ?? false;
      lastTabTitle.value = prefs.last_tab_title ?? null;
      lastTabKey.value = lastTabTitle.value ? { title: lastTabTitle.value, section_type: '' } : null;
      autoCheckUpdates.value = prefs.auto_check_updates ?? false;
    } catch (e) {
      console.error("Failed to load prefs:", e);
    }
  }

  function closeFile() {
    nodes.value = [];
    ecuName.value = "";
    nodeCount.value = 0;
    isDiff.value = false;
    selectedIndex.value = null;
    detailSections.value = [];
    history.value = [];
    forwardHistory.value = [];
    searchFilters.value = [];
    fileLoaded.value = false;
    filePath.value = "";
    status.value = "";
    hideUnchanged.value = false;
    searchActive.value = false;
    searchQuery.value = "";
    udsLoading.value = false;
    udsReady.value = false;
    udsError.value = null;
    _udsLoadPromise = null;
    openTabs.value = [];
    activeTabId.value = null;
    tabUiStates.clear();
  }

  async function switchTab(tabId: string) {
    if (tabId === activeTabId.value) return;
    loading.value = true;
    try {
      saveCurrentTabUiState();
      const result = await api.switchTab(tabId);
      applyLoadResult(result);
      filePath.value = openTabs.value.find(t => t.id === tabId)?.file_path ?? "";
      restoreTabUiState(tabId);
      status.value = `${result.node_count} nodes`;
      await refreshOpenTabs();
    } catch (e) {
      status.value = `Error: ${e}`;
    } finally {
      loading.value = false;
    }
  }

  async function closeTabById(tabId: string) {
    try {
      saveCurrentTabUiState();
      tabUiStates.delete(tabId);
      const result = await api.closeTab(tabId);
      if (result) {
        applyLoadResult(result);
        filePath.value = openTabs.value.find(t => t.id === result.tab_id)?.file_path ?? "";
        restoreTabUiState(result.tab_id);
        status.value = `${result.node_count} nodes`;
      } else {
        closeFile();
      }
      await refreshOpenTabs();
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function closeOtherTabs(keepTabId: string) {
    const toClose = openTabs.value.filter(t => t.id !== keepTabId).map(t => t.id);
    for (const id of toClose) {
      tabUiStates.delete(id);
      await api.closeTab(id);
    }
    if (activeTabId.value !== keepTabId) {
      const result = await api.switchTab(keepTabId);
      applyLoadResult(result);
      restoreTabUiState(keepTabId);
    }
    await refreshOpenTabs();
  }

  async function switchToAdjacentTab(direction: -1 | 1) {
    const tabs = openTabs.value;
    if (tabs.length < 2) return;
    const currentIdx = tabs.findIndex(t => t.id === activeTabId.value);
    if (currentIdx < 0) return;
    const nextIdx = (currentIdx + direction + tabs.length) % tabs.length;
    const nextTab = tabs[nextIdx];
    if (nextTab) await switchTab(nextTab.id);
  }

  async function removeRecentFile(path: string) {
    try {
      await api.removeRecentFile(path);
      recentFiles.value = recentFiles.value.filter(f => f.path !== path);
    } catch (e) {
      console.error("Failed to remove recent file:", e);
    }
  }

  async function clearRecentFiles() {
    try {
      await api.clearRecentFiles();
      recentFiles.value = [];
    } catch (e) {
      console.error("Failed to clear recent files:", e);
    }
  }

  return {
    nodes, ecuName, nodeCount, isDiff, selectedIndex, selectedNode,
    detailSections, selectedTab, searchQuery, searchScope, searchActive,
    status, loading, history, canGoBack, canGoForward, breadcrumbs, splitPct,
    fileLoaded, filePath, hideUnchanged, fontSize, theme, sortLabel, recentFiles,
    rowDensity, rowHeightPx, defaultHideUnchanged, autoExpandFirstLevel,
    maxRecentFiles, wrapTableText, lastTabTitle, displayedRecentFiles, autoCheckUpdates,
    udsLoading, udsReady, udsError, waitForUds,
    openTabs, activeTabId,
    loadFile, loadDiff, selectNode, goBack, goForward, toggleExpand, search, searchFilters,
    clearSearch, removeSearchFilter, toggleFilterOp, cycleScope, setScope, expandAll, collapseAll, toggleSort, toggleHideUnchanged,
    increaseFontSize, decreaseFontSize, setFontSize, setTheme,
    setRowDensity, setDefaultHideUnchanged, setAutoExpandFirstLevel, setMaxRecentFiles, setWrapTableText, setSelectedTab,
    setAutoCheckUpdates,
    navigateTo, loadRecentFiles, loadPrefs, clearRecentFiles, removeRecentFile, closeFile,
    nextChange, prevChange,
    switchTab, closeTabById, closeOtherTabs, switchToAdjacentTab,
  };
});
