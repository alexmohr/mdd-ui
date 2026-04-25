<script setup lang="ts">
import { useAppStore } from "./stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import TreePane from "./components/TreePane.vue";
import DetailPane from "./components/DetailPane.vue";
import SearchBar from "./components/SearchBar.vue";
import StatusBar from "./components/StatusBar.vue";

const store = useAppStore();

async function openFile() {
  const path = await open({
    title: "Open MDD File",
    filters: [{ name: "MDD Files", extensions: ["mdd"] }],
  });
  if (path) {
    await store.loadFile(path as string);
  }
}

async function openDiff() {
  const oldPath = await open({
    title: "Select OLD MDD File",
    filters: [{ name: "MDD Files", extensions: ["mdd"] }],
  });
  if (!oldPath) return;
  const newPath = await open({
    title: "Select NEW MDD File",
    filters: [{ name: "MDD Files", extensions: ["mdd"] }],
  });
  if (!newPath) return;
  await store.loadDiff(oldPath as string, newPath as string);
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "/" && !store.searchActive) {
    e.preventDefault();
    store.searchActive = true;
  }
}
</script>

<template>
  <div
    class="flex flex-col h-screen bg-gray-950 text-gray-200 font-mono text-sm"
    @keydown="handleKeydown"
    tabindex="0"
  >
    <!-- Toolbar -->
    <div class="flex items-center gap-2 px-3 py-1.5 bg-gray-900 border-b border-gray-800">
      <button
        class="px-3 py-1 rounded bg-blue-700 hover:bg-blue-600 text-white text-xs font-medium"
        @click="openFile"
      >
        Open MDD
      </button>
      <button
        class="px-3 py-1 rounded bg-indigo-700 hover:bg-indigo-600 text-white text-xs font-medium"
        @click="openDiff"
      >
        Diff
      </button>
      <div v-if="store.ecuName" class="ml-2 text-gray-400 text-xs">
        {{ store.ecuName }}
        <span class="text-gray-600">({{ store.nodeCount }} nodes)</span>
      </div>
      <div class="flex-1" />
      <template v-if="store.nodes.length > 0">
        <button
          class="px-2 py-0.5 rounded text-xs text-gray-400 hover:text-white hover:bg-gray-800"
          title="Expand all (e)"
          @click="store.expandAll()"
        >
          ⊞
        </button>
        <button
          class="px-2 py-0.5 rounded text-xs text-gray-400 hover:text-white hover:bg-gray-800"
          title="Collapse all (c)"
          @click="store.collapseAll()"
        >
          ⊟
        </button>
        <button
          v-if="store.isDiff"
          class="px-2 py-0.5 rounded text-xs"
          :class="store.nodes.length ? 'text-yellow-400 hover:text-yellow-300' : 'text-gray-600'"
          title="Toggle unchanged (u)"
          @click="store.toggleHideUnchanged()"
        >
          Δ
        </button>
      </template>
    </div>

    <!-- Search bar -->
    <SearchBar v-if="store.searchActive" />

    <!-- Main content -->
    <div class="flex flex-1 min-h-0">
      <TreePane class="w-2/5 min-w-64 border-r border-gray-800" />
      <DetailPane class="flex-1" />
    </div>

    <!-- Status bar -->
    <StatusBar />
  </div>
</template>
