import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type {
  VisibleNode,
  DetailSection,
} from "../api/commands";
import * as api from "../api/commands";

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
  const status = ref("No file loaded");
  const loading = ref(false);

  const selectedNode = computed(() =>
    nodes.value.find((n) => n.index === selectedIndex.value) ?? null,
  );

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
      status.value = `Loaded ${result.ecu_name} (${result.node_count} nodes)`;
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
      status.value = `Diff: ${result.ecu_name} (${result.node_count} nodes)`;
    } catch (e) {
      status.value = `Error: ${e}`;
    } finally {
      loading.value = false;
    }
  }

  async function selectNode(index: number) {
    selectedIndex.value = index;
    selectedTab.value = 0;
    try {
      detailSections.value = await api.getNodeDetail(index);
    } catch (e) {
      detailSections.value = [];
      status.value = `Error loading details: ${e}`;
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
      status.value = `Search: ${result.match_count} filter(s) active`;
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function clearSearch() {
    try {
      nodes.value = await api.clearSearch();
      status.value = "Search cleared";
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
    try {
      nodes.value = await api.expandAll();
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function collapseAll() {
    try {
      nodes.value = await api.collapseAll();
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  async function toggleHideUnchanged() {
    try {
      nodes.value = await api.toggleHideUnchanged();
    } catch (e) {
      status.value = `Error: ${e}`;
    }
  }

  return {
    nodes,
    ecuName,
    nodeCount,
    isDiff,
    selectedIndex,
    selectedNode,
    detailSections,
    selectedTab,
    searchQuery,
    searchScope,
    searchActive,
    status,
    loading,
    loadFile,
    loadDiff,
    selectNode,
    toggleExpand,
    search,
    clearSearch,
    cycleScope,
    expandAll,
    collapseAll,
    toggleHideUnchanged,
  };
});
