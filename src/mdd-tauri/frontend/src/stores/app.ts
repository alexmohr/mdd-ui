import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type {
  VisibleNode,
  DetailSection,
  JumpTarget,
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

  async function selectNode(index: number) {
    if (selectedIndex.value !== null && selectedIndex.value !== index) {
      const prev = selectedNode.value;
      if (prev) pushHistory(prev.index, prev.text);
    }
    selectedIndex.value = index;
    selectedTab.value = 0;
    try {
      detailSections.value = await api.getNodeDetail(index);
    } catch (e) {
      detailSections.value = [];
      status.value = `Error: ${e}`;
    }
  }

  async function goBack() {
    const entry = history.value.pop();
    if (!entry) return;
    selectedIndex.value = entry.index;
    selectedTab.value = 0;
    try {
      detailSections.value = await api.getNodeDetail(entry.index);
    } catch (e) {
      detailSections.value = [];
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

  function increaseFontSize() { fontSize.value = Math.min(20, fontSize.value + 1); }
  function decreaseFontSize() { fontSize.value = Math.max(9, fontSize.value - 1); }

  async function toggleSort(nodeIndex?: number) {
    try {
      const idx = nodeIndex ?? selectedIndex.value ?? undefined;
      nodes.value = await api.toggleSort(idx);
      status.value = "Sorted";
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

  return {
    nodes, ecuName, nodeCount, isDiff, selectedIndex, selectedNode,
    detailSections, selectedTab, searchQuery, searchScope, searchActive,
    status, loading, history, canGoBack, breadcrumbs, splitPct,
    fileLoaded, filePath, hideUnchanged, fontSize,
    loadFile, loadDiff, selectNode, goBack, toggleExpand, search,
    clearSearch, cycleScope, expandAll, collapseAll, toggleSort, toggleHideUnchanged,
    increaseFontSize, decreaseFontSize,
    navigateTo,
  };
});
