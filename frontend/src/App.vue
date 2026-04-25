<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

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
const isMac = navigator.platform.toLowerCase().includes('mac');

onMounted(async () => {
  await Promise.all([store.loadRecentFiles(), store.loadPrefs()]);
  window.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeydown);
});

async function openFile() {
  const path = await open({
    title: "Open MDD File",
    filters: [{ name: "MDD Files", extensions: ["mdd"] }],
  });
  if (path) await store.loadFile(path as string);
}

async function openRecentFile(path: string) {
  await store.loadFile(path);
}

function getFileName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

async function openDiff() {
  if (store.fileLoaded && store.filePath) {
    const newPath = await open({
      title: "Select NEW MDD File to Compare",
      filters: [{ name: "MDD Files", extensions: ["mdd"] }],
    });
    if (!newPath) return;
    await store.loadDiff(store.filePath, newPath as string);
  } else {
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
}

function handleKeydown(e: KeyboardEvent) {
  if (store.searchActive) return;
  const tag = (e.target as HTMLElement)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return;

  switch (e.key) {
    case "/": e.preventDefault(); store.searchActive = true; break;
    case "Backspace": e.preventDefault(); store.goBack(); break;
    case "s": store.toggleSort(); break;
    case "e": store.expandAll(); break;
    case "c": store.collapseAll(); break;
    case "u": if (store.isDiff) store.toggleHideUnchanged(); break;
    case "x": store.clearSearch(); break;
    case "+": case "=": store.increaseFontSize(); break;
    case "-": store.decreaseFontSize(); break;
  }
}

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
    class="flex flex-col h-screen bg-neutral-950 text-neutral-200 antialiased"
    :class="{ 'select-none': dragging }"
    :style="{ fontSize: store.fontSize + 'px' }"
  >
    <!-- Welcome screen -->
    <template v-if="!store.fileLoaded">
      <div class="flex-1 flex items-center justify-center" data-tauri-drag-region>
        <div class="text-center space-y-6">
          <div class="flex flex-col items-center gap-3">
            <h1 class="text-2xl font-semibold text-neutral-200 tracking-wide" style="font-family: Helvetica, Arial, sans-serif;">MDD UI</h1>
            <p class="text-neutral-600 text-sm">Diagnostic database browser</p>
          </div>
          <div class="flex gap-3 justify-center mt-4">
            <button
              class="px-5 py-2.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium transition-colors shadow-lg shadow-blue-600/20"
              @click="openFile"
            >
              Open File
            </button>
            <button
              class="px-5 py-2.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-sm font-medium transition-colors border border-neutral-700"
              @click="openDiff"
            >
              Compare Files
            </button>
          </div>
          <div v-if="store.recentFiles.length > 0" class="mt-8">
            <div class="text-neutral-500 text-xs uppercase tracking-wider mb-3">Recent Files</div>
            <div class="flex flex-col gap-2 items-center">
              <div
                v-for="file in store.recentFiles"
                :key="file.path"
                class="w-80 rounded-lg bg-neutral-900 border border-neutral-800 hover:border-neutral-700 transition-colors flex items-center group"
              >
                <button
                  class="flex-1 px-4 py-2 text-neutral-300 text-sm text-left flex items-center gap-3 min-w-0"
                  @click="openRecentFile(file.path)"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-500 group-hover:text-neutral-400 flex-shrink-0"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><path d="M14 2v6h6"/></svg>
                  <span class="truncate">{{ getFileName(file.path) }}</span>
                </button>
                <button
                  class="p-2 mr-1 rounded text-neutral-700 hover:text-red-400 transition-colors flex-shrink-0"
                  title="Remove from recent"
                  @click.stop="store.removeRecentFile(file.path)"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                </button>
              </div>
            </div>
          </div>
          <div class="text-neutral-600 text-xs mt-8">
            <kbd class="px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-400 border border-neutral-700 text-[11px]">/</kbd> search
            &nbsp;&middot;&nbsp;
            <kbd class="px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-400 border border-neutral-700 text-[11px]">Backspace</kbd> back
            &nbsp;&middot;&nbsp;
            <kbd class="px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-400 border border-neutral-700 text-[11px]">e</kbd> expand all
            &nbsp;&middot;&nbsp;
            <kbd class="px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-400 border border-neutral-700 text-[11px]">c</kbd> collapse all
          </div>
        </div>
      </div>
    </template>

    <!-- Main app layout -->
    <template v-else>
      <!-- Top bar -->
      <div
        class="flex items-center h-10 bg-neutral-900 border-b border-neutral-800/60 gap-2 shrink-0"
        :class="isMac ? 'pl-20 pr-3' : 'px-3'"
        data-tauri-drag-region
      >
        <!-- Close file / back to home -->
        <button
          class="p-1.5 rounded-md text-neutral-500 hover:text-white hover:bg-neutral-800 transition-colors"
          title="Close file"
          @click="store.closeFile()"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
        </button>

        <!-- Back -->
        <button
          class="p-1.5 rounded-md transition-colors"
          :class="store.canGoBack ? 'text-neutral-400 hover:text-white hover:bg-neutral-800' : 'text-neutral-700 cursor-default'"
          :disabled="!store.canGoBack"
          title="Back (Backspace)"
          @click="store.goBack()"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg>
        </button>

        <!-- Breadcrumbs -->
        <div class="flex items-center gap-1 text-xs text-neutral-500 overflow-hidden min-w-0 flex-1" data-tauri-drag-region>
          <template v-for="(crumb, i) in store.breadcrumbs" :key="crumb.index">
            <span v-if="i > 0" class="text-neutral-700">/</span>
            <button
              class="truncate max-w-48 hover:text-gray-300 transition-colors"
              :class="i === store.breadcrumbs.length - 1 ? 'text-neutral-200 font-medium' : 'text-neutral-500'"
              @click="store.selectNode(crumb.index)"
            >
              {{ crumb.text }}
            </button>
          </template>
          <span v-if="store.breadcrumbs.length === 0" class="text-neutral-500">{{ store.ecuName }}</span>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-1" data-tauri-drag-region>
          <button
            v-if="store.isDiff"
            class="px-2 py-1 rounded-md text-[11px] font-medium transition-colors"
            :class="store.hideUnchanged ? 'bg-amber-600/20 text-amber-400' : 'text-neutral-500 hover:text-neutral-300 hover:bg-neutral-800'"
            title="Toggle unchanged (u)"
            @click="store.toggleHideUnchanged()"
          >
            Hide unchanged
          </button>
          <div class="flex items-center gap-0.5 mr-1" data-tauri-drag-region>
            <button
              class="w-5 h-5 flex items-center justify-center rounded text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors text-[11px] font-bold"
              title="Decrease font size (-)"
              @click="store.decreaseFontSize()"
            >A-</button>
            <span class="text-[10px] text-neutral-600 w-5 text-center">{{ store.fontSize }}</span>
            <button
              class="w-5 h-5 flex items-center justify-center rounded text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors text-[11px] font-bold"
              title="Increase font size (+)"
              @click="store.increaseFontSize()"
            >A+</button>
          </div>
          <button
            class="p-1.5 rounded-md text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
            title="Open file"
            @click="openFile"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>
          </button>
          <button
            class="p-1.5 rounded-md text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
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
          class="w-1 cursor-col-resize bg-neutral-800/40 hover:bg-blue-500/40 active:bg-blue-500/60 transition-colors shrink-0"
          @mousedown.prevent="onSplitMouseDown"
        />
        <DetailPane class="flex-1 min-w-0" />
      </div>

      <!-- Status bar -->
      <StatusBar />
    </template>
  </div>
</template>
