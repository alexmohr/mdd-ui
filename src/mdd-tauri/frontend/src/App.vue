<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useAppStore } from "./stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import TreePane from "./components/TreePane.vue";
import DetailPane from "./components/DetailPane.vue";
import SearchBar from "./components/SearchBar.vue";
import StatusBar from "./components/StatusBar.vue";

const store = useAppStore();
const dragging = ref(false);

async function openFile() {
  const path = await open({
    title: "Open MDD File",
    filters: [{ name: "MDD Files", extensions: ["mdd"] }],
  });
  if (path) await store.loadFile(path as string);
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
  if (store.searchActive) return;
  const tag = (e.target as HTMLElement)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return;

  switch (e.key) {
    case "/": e.preventDefault(); store.searchActive = true; break;
    case "Backspace": e.preventDefault(); store.goBack(); break;
    case "e": store.expandAll(); break;
    case "c": store.collapseAll(); break;
    case "u": if (store.isDiff) store.toggleHideUnchanged(); break;
    case "x": store.clearSearch(); break;
  }
}

onMounted(() => window.addEventListener("keydown", handleKeydown));
onUnmounted(() => window.removeEventListener("keydown", handleKeydown));

function onSplitMouseDown() {
  dragging.value = true;
  const onMove = (e: MouseEvent) => {
    const pct = Math.round((e.clientX / window.innerWidth) * 100);
    store.splitPct = Math.max(15, Math.min(70, pct));
  };
  const onUp = () => {
    dragging.value = false;
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}
</script>

<template>
  <div
    class="flex flex-col h-screen bg-[#0c0e14] text-gray-300 text-[13px] antialiased"
    :class="{ 'select-none': dragging }"
  >
    <!-- Welcome screen -->
    <template v-if="!store.fileLoaded">
      <div class="flex-1 flex items-center justify-center">
        <div class="text-center space-y-6">
          <div class="text-4xl font-light text-gray-500 tracking-tight">MDD UI</div>
          <p class="text-gray-600 text-sm">Diagnostic database browser</p>
          <div class="flex gap-3 justify-center mt-4">
            <button
              class="px-5 py-2.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium transition-colors shadow-lg shadow-blue-600/20"
              @click="openFile"
            >
              Open File
            </button>
            <button
              class="px-5 py-2.5 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm font-medium transition-colors border border-gray-700"
              @click="openDiff"
            >
              Compare Files
            </button>
          </div>
          <div class="text-gray-700 text-xs mt-8">
            <kbd class="px-1.5 py-0.5 rounded bg-gray-800 text-gray-500 border border-gray-700 text-[11px]">/</kbd> search
            &nbsp;&middot;&nbsp;
            <kbd class="px-1.5 py-0.5 rounded bg-gray-800 text-gray-500 border border-gray-700 text-[11px]">Backspace</kbd> back
            &nbsp;&middot;&nbsp;
            <kbd class="px-1.5 py-0.5 rounded bg-gray-800 text-gray-500 border border-gray-700 text-[11px]">e</kbd> expand all
            &nbsp;&middot;&nbsp;
            <kbd class="px-1.5 py-0.5 rounded bg-gray-800 text-gray-500 border border-gray-700 text-[11px]">c</kbd> collapse all
          </div>
        </div>
      </div>
    </template>

    <!-- Main app layout -->
    <template v-else>
      <!-- Top bar -->
      <div class="flex items-center h-10 px-3 bg-[#10131a] border-b border-gray-800/60 gap-2 shrink-0">
        <!-- Back -->
        <button
          class="p-1.5 rounded-md transition-colors"
          :class="store.canGoBack ? 'text-gray-400 hover:text-white hover:bg-gray-800' : 'text-gray-700 cursor-default'"
          :disabled="!store.canGoBack"
          title="Back (Backspace)"
          @click="store.goBack()"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg>
        </button>

        <!-- Breadcrumbs -->
        <div class="flex items-center gap-1 text-xs text-gray-500 overflow-hidden min-w-0 flex-1">
          <template v-for="(crumb, i) in store.breadcrumbs" :key="crumb.index">
            <span v-if="i > 0" class="text-gray-700">/</span>
            <button
              class="truncate max-w-48 hover:text-gray-300 transition-colors"
              :class="i === store.breadcrumbs.length - 1 ? 'text-gray-300 font-medium' : 'text-gray-500'"
              @click="store.selectNode(crumb.index)"
            >
              {{ crumb.text }}
            </button>
          </template>
          <span v-if="store.breadcrumbs.length === 0" class="text-gray-600">{{ store.ecuName }}</span>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-1">
          <button
            v-if="store.isDiff"
            class="px-2 py-1 rounded-md text-[11px] font-medium transition-colors"
            :class="store.hideUnchanged ? 'bg-amber-600/20 text-amber-400' : 'text-gray-500 hover:text-gray-300 hover:bg-gray-800'"
            title="Toggle unchanged (u)"
            @click="store.toggleHideUnchanged()"
          >
            Hide unchanged
          </button>
          <button
            class="p-1.5 rounded-md text-gray-500 hover:text-gray-300 hover:bg-gray-800 transition-colors"
            title="Open file"
            @click="openFile"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>
          </button>
          <button
            class="p-1.5 rounded-md text-gray-500 hover:text-gray-300 hover:bg-gray-800 transition-colors"
            title="Compare files"
            @click="openDiff"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 3h5v5"/><path d="M8 3H3v5"/><path d="M12 22v-8.3a4 4 0 0 0-1.172-2.872L3 3"/><path d="m21 3-7.828 7.828A4 4 0 0 0 12 13.7V22"/></svg>
          </button>
        </div>
      </div>

      <!-- Search -->
      <SearchBar v-if="store.searchActive" />

      <!-- Resizable split -->
      <div class="flex flex-1 min-h-0">
        <TreePane :style="{ width: store.splitPct + '%' }" class="shrink-0" />
        <div
          class="w-1 cursor-col-resize bg-gray-800/40 hover:bg-blue-500/40 active:bg-blue-500/60 transition-colors shrink-0"
          @mousedown.prevent="onSplitMouseDown"
        />
        <DetailPane class="flex-1 min-w-0" />
      </div>

      <!-- Status bar -->
      <StatusBar />
    </template>
  </div>
</template>
