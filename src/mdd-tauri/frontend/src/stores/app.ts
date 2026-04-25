import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type {
  VisibleNode,
  DetailSection,
  JumpTarget,
  RecentFile,
} from "../api/commands";
import * as api from "../api/commands";

export interface HistoryEntry {
  index: number;
  text: string;
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
  const splitPct = ref(35);
  const fileLoaded = ref(false);
  const filePath = ref("");
  const hideUnchanged = ref(false);
  const fontSize = ref(13);
  const sortLabel = ref("ID\u25b2");
  const recentFiles = ref<RecentFile[]>([]);

  const selectedNode = computed(() =>
    nodes.value.find((n: VisibleNode) => n.index === selectedIndex.value) ?? null,
  );

  const canGoBack = computed(() => history.value.length > 0);

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

  function pushHistory(index: number, text: string) {
    if (history.value.length > 50) history.value.shift();
    history.value.push({ index, text });
  }

  async function loadFile(path: string) {
    loading.value = true;
    try {
      const result = await api.loadMdd(path);
      nodes.value = result.visible;
      ecuName.value = result.ecu_name;
      nodeCount.value = result.node_count;
      isDiff.value = result.is_diff;
      selectedIndex.value = null;
      detailSections.value = [];
      history.value = [];
      fileLoaded.value = true;
      filePath.value = path;
      status.value = `${result.node_count} nodes`;
      await api.addRecentFile(path);
      await loadRecentFiles();
    } catch (e) {
      status.value = `Error: ${e}`;
    } finally {
      loading.value = false;
    }
  }

  async function loadDiff(oldPath: string, newPath: string) {
    loading.value = true;
    try {
      const result = await api.loadDiff(oldPath, newPath);
      nodes.value = result.visible;
      ecuName.value = result.ecu_name;
      nodeCount.value = result.node_count;
      isDiff.value = result.is_diff;
      selectedIndex.value = null;
      detailSections.value = [];
      history.value = [];
      fileLoaded.value = true;
      filePath.value = "";
      status.value = `Diff: ${result.node_count} nodes`;
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

  function activeTabTitle(): string | null {
    const tabs = tabSectionsOf(detailSections.value);
    return tabs[selectedTab.value]?.title ?? null;
  }

  function restoreTab(sections: import('../api/commands').DetailSection[], title: string | null) {
    if (title === null) { selectedTab.value = 0; return; }
    const tabs = tabSectionsOf(sections);
    const idx = tabs.findIndex(t => t.title === title);
    selectedTab.value = idx >= 0 ? idx : 0;
  }

  async function selectNode(index: number) {
    if (selectedIndex.value !== null && selectedIndex.value !== index) {
      const prev = selectedNode.value;
      if (prev) pushHistory(prev.index, prev.text);
    }
    const prevTitle = activeTabTitle();
    selectedIndex.value = index;
    try {
      const sections = await api.getNodeDetail(index);
      detailSections.value = sections;
      restoreTab(sections, prevTitle);
    } catch (e) {
      detailSections.value = [];
      selectedTab.value = 0;
      status.value = `Error: ${e}`;
    }
  }

  async function goBack() {
    const entry = history.value.pop();
    if (!entry) return;
    const prevTitle = activeTabTitle();
    try {
      const result = await api.navigateTo({
        target_type: { TreeNodeByIndex: { index: entry.index, short_name: entry.text } },
      });
      nodes.value = result.visible;
      selectedIndex.value = result.target_index;
      detailSections.value = result.detail;
      restoreTab(result.detail, prevTitle);
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

  async function search(query: string) {
    try {
      const result = await api.doSearch(query);
      nodes.value = result.visible;
      searchScope.value = result.scope;
      status.value = `${result.match_count} filter(s) active`;
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function clearSearch() {
    try {
      nodes.value = await api.clearSearch();
      status.value = "";
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

  async function expandAll() {
    try { nodes.value = await api.expandAll(); } catch (e) { status.value = `Error: ${e}`; }
  }

  async function collapseAll() {
    try { nodes.value = await api.collapseAll(); } catch (e) { status.value = `Error: ${e}`; }
  }

  function increaseFontSize() {
    fontSize.value = Math.min(20, fontSize.value + 1);
    api.saveUiPrefs({ font_size: fontSize.value }).catch(() => {});
  }
  function decreaseFontSize() {
    fontSize.value = Math.max(9, fontSize.value - 1);
    api.saveUiPrefs({ font_size: fontSize.value }).catch(() => {});
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
    fileLoaded.value = false;
    filePath.value = "";
    status.value = "";
    hideUnchanged.value = false;
    searchActive.value = false;
    searchQuery.value = "";
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
    status, loading, history, canGoBack, breadcrumbs, splitPct,
    fileLoaded, filePath, hideUnchanged, fontSize, sortLabel, recentFiles,
    loadFile, loadDiff, selectNode, goBack, toggleExpand, search,
    clearSearch, cycleScope, expandAll, collapseAll, toggleSort, toggleHideUnchanged,
    increaseFontSize, decreaseFontSize,
    navigateTo, loadRecentFiles, loadPrefs, clearRecentFiles, removeRecentFile, closeFile,
  };
});
