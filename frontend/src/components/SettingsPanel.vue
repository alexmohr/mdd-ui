<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { ref, reactive, watch, onMounted, onUnmounted } from "vue";
import { useSettingsStore } from "../stores/settings";
import { useAppStore } from "../stores/app";
import { useLlmStore } from "../stores/llm";
import type { LlmSettingsUpdate } from "../stores/llm";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

const store = useSettingsStore();
const appStore = useAppStore();
const llmStore = useLlmStore();
const activeCategory = ref("general");

const categories = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "behavior", label: "Behavior" },
  { id: "ai", label: "AI Assistant" },
  { id: "updates", label: "Updates" },
];

type UpdateCheckStatus = "idle" | "checking" | "up-to-date" | "available" | "done" | "error";
const updateStatus = ref<UpdateCheckStatus>("idle");
const updateVersion = ref("");
const updateError = ref("");
const isInstalling = ref(false);
const currentVersion = ref("");

// AI Assistant settings form
const aiForm = reactive<LlmSettingsUpdate & { api_token: string }>({
  ghe_host: llmStore.settings.ghe_host,
  llm_endpoint: llmStore.settings.llm_endpoint,
  llm_model: llmStore.settings.llm_model,
  auth_method: llmStore.settings.auth_method,
  api_token: "",
  api_version: llmStore.settings.api_version,
});
const copied = ref(false);

watch(
  () => activeCategory.value,
  (cat) => {
    if (cat === "ai") {
      aiForm.ghe_host = llmStore.settings.ghe_host;
      aiForm.llm_endpoint = llmStore.settings.llm_endpoint;
      aiForm.llm_model = llmStore.settings.llm_model;
      aiForm.auth_method = llmStore.settings.auth_method;
      aiForm.api_token = "";
      aiForm.api_version = llmStore.settings.api_version;
      if (llmStore.isAuthenticated) void llmStore.fetchModels();
    }
  },
);

async function saveAiSettings() {
  const endpoint =
    aiForm.auth_method === "copilot"
      ? `https://copilot-api.${aiForm.ghe_host}`
      : aiForm.llm_endpoint;
  await llmStore.saveSettings({
    ghe_host: aiForm.ghe_host,
    llm_endpoint: endpoint,
    llm_model: aiForm.llm_model,
    auth_method: aiForm.auth_method,
    api_token: aiForm.api_token || undefined,
    api_version: aiForm.api_version || undefined,
  });
}

async function copyCode() {
  if (!llmStore.deviceFlowInfo) return;
  await navigator.clipboard.writeText(llmStore.deviceFlowInfo.user_code);
  copied.value = true;
  setTimeout(() => { copied.value = false; }, 2000);
}

function cancelLogin() {
  llmStore.stopPolling();
  llmStore.loginState = "idle";
  llmStore.deviceFlowInfo = null;
  llmStore.error = "";
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    close();
  }
}

onMounted(async () => {
  currentVersion.value = await getVersion();
  window.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
});

async function checkForUpdates() {
  updateStatus.value = "checking";
  updateError.value = "";
  updateVersion.value = "";
  try {
    const update = await check();
    if (update) {
      updateVersion.value = update.version;
      updateStatus.value = "available";
    } else {
      updateStatus.value = "up-to-date";
    }
  } catch (e) {
    updateStatus.value = "error";
    updateError.value = `${e}`;
  }
}

async function installUpdate() {
  isInstalling.value = true;
  try {
    const update = await check();
    if (update) {
      await update.downloadAndInstall();
      updateStatus.value = "done";
    }
  } catch (e) {
    updateStatus.value = "error";
    updateError.value = `${e}`;
  } finally {
    isInstalling.value = false;
  }
}

function close() {
  store.open = false;
  store.resetRegisterStatus();
  store.resetClearCacheStatus();
}

async function handleClearAllCaches() {
  await store.doClearAllCaches();
  if (store.clearCacheStatus === 'success') {
    appStore.clearRecentFiles();
  }
}
</script>

<template>
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    @click.self="close"
  >
    <!-- Dialog -->
    <div
      class="w-full max-w-2xl bg-neutral-900 border border-neutral-800 rounded-xl shadow-2xl flex overflow-hidden"
      style="height: 80vh"
    >
      <!-- Left sidebar -->
      <div class="w-44 bg-neutral-950 border-r border-neutral-800 flex flex-col shrink-0">
        <div class="px-4 py-3 border-b border-neutral-800">
          <h2 class="text-sm font-semibold text-neutral-200">Settings</h2>
        </div>
        <nav class="flex-1 p-2 space-y-0.5">
          <button
            v-for="cat in categories"
            :key="cat.id"
            class="w-full text-left px-3 py-1.5 rounded-md text-sm transition-colors"
            :class="
              activeCategory === cat.id
                ? 'bg-blue-600/20 text-blue-300 font-medium'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800'
            "
            @click="activeCategory = cat.id"
          >
            {{ cat.label }}
          </button>
        </nav>
      </div>

      <!-- Content -->
      <div class="flex-1 flex flex-col min-w-0">
        <!-- Content header -->
        <div
          class="flex items-center justify-between px-5 py-3 border-b border-neutral-800 shrink-0"
        >
          <h3 class="text-sm font-medium text-neutral-200">
            {{ categories.find((c) => c.id === activeCategory)?.label }}
          </h3>
          <button
            class="p-1 rounded-md text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
            title="Close"
            @click="close"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>

        <!-- Content body -->
        <div class="flex-1 overflow-y-auto p-5 space-y-6 min-h-0">
          <!-- General -->
          <template v-if="activeCategory === 'general'">
            <!-- File Associations -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  File Associations
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Register MDD UI as the default application for
                  <code class="text-neutral-400 bg-neutral-800 px-1 rounded">.mdd</code> files on
                  your system.
                </p>
              </div>

              <div class="flex items-center gap-3">
                <button
                  class="px-3 py-1.5 rounded-md text-xs font-medium transition-colors shrink-0 disabled:opacity-50"
                  :class="
                    store.registerStatus === 'loading'
                      ? 'bg-neutral-800 text-neutral-500 cursor-not-allowed border border-neutral-700'
                      : 'bg-blue-600 hover:bg-blue-500 text-white'
                  "
                  :disabled="store.registerStatus === 'loading'"
                  @click="store.doRegisterMddAssociation()"
                >
                  <span
                    v-if="store.registerStatus === 'loading'"
                    class="flex items-center gap-1.5"
                  >
                    <svg
                      class="animate-spin"
                      xmlns="http://www.w3.org/2000/svg"
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                    </svg>
                    Registering…
                  </span>
                  <span v-else>Register as Default App</span>
                </button>

                <span
                  v-if="store.registerStatus === 'success'"
                  class="flex items-center gap-1.5 text-xs text-green-400"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="13"
                    height="13"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <path d="m9 11 3 3L22 4" />
                  </svg>
                  Done
                </span>
              </div>

              <!-- Success message -->
              <div
                v-if="store.registerStatus === 'success'"
                class="rounded-lg bg-green-900/20 border border-green-800/40 p-3 text-xs text-green-300 leading-relaxed whitespace-pre-wrap"
              >
                {{ store.registerMessage }}
              </div>

              <!-- Error message -->
              <div
                v-if="store.registerStatus === 'error'"
                class="rounded-lg bg-red-900/20 border border-red-800/40 p-3 text-xs text-red-400 leading-relaxed"
              >
                {{ store.registerMessage }}
              </div>

              <!-- Platform hints -->
              <div class="rounded-lg bg-neutral-800/50 border border-neutral-700/50 p-3 space-y-1.5">
                <p class="text-[11px] font-medium text-neutral-400">Platform notes</p>
                <ul class="space-y-1 text-[11px] text-neutral-600">
                  <li>
                    <span class="text-neutral-500">macOS —</span>
                    registers with Launch Services; then right-click a .mdd file → Get Info →
                    Open With → Change All.
                  </li>
                  <li>
                    <span class="text-neutral-500">Windows —</span>
                    writes per-user registry keys; no elevation required.
                  </li>
                  <li>
                    <span class="text-neutral-500">Linux —</span>
                    installs MIME type and .desktop file, then calls
                    <code class="text-neutral-500">xdg-mime</code>. Requires
                    <code class="text-neutral-500">xdg-utils</code>.
                  </li>
                </ul>
              </div>
            </section>

            <!-- Cache -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  Cache
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Clear all cached data, including recent files and saved preferences.
                </p>
              </div>

              <div class="flex items-center gap-3">
                <button
                  class="px-3 py-1.5 rounded-md text-xs font-medium transition-colors shrink-0 disabled:opacity-50"
                  :class="
                    store.clearCacheStatus === 'loading'
                      ? 'bg-neutral-800 text-neutral-500 cursor-not-allowed border border-neutral-700'
                      : 'bg-neutral-700 hover:bg-neutral-600 text-neutral-200 border border-neutral-600'
                  "
                  :disabled="store.clearCacheStatus === 'loading'"
                  @click="handleClearAllCaches()"
                >
                  <span
                    v-if="store.clearCacheStatus === 'loading'"
                    class="flex items-center gap-1.5"
                  >
                    <svg
                      class="animate-spin"
                      xmlns="http://www.w3.org/2000/svg"
                      width="12"
                      height="12"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    >
                      <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                    </svg>
                    Clearing…
                  </span>
                  <span v-else>Clear All Caches</span>
                </button>

                <span
                  v-if="store.clearCacheStatus === 'success'"
                  class="flex items-center gap-1.5 text-xs text-green-400"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="13"
                    height="13"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <path d="m9 11 3 3L22 4" />
                  </svg>
                  Cleared
                </span>
              </div>

              <!-- Error message -->
              <div
                v-if="store.clearCacheStatus === 'error'"
                class="rounded-lg bg-red-900/20 border border-red-800/40 p-3 text-xs text-red-400 leading-relaxed"
              >
                {{ store.clearCacheMessage }}
              </div>
            </section>

            <!-- Recent Files -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  Recent Files
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Number of recent files shown on the welcome screen.
                </p>
              </div>
              <div class="flex gap-2 flex-wrap">
                <button
                  v-for="n in [5, 10, 15, 20]"
                  :key="n"
                  class="px-3 py-1.5 rounded-lg border text-xs font-medium transition-colors"
                  :class="appStore.maxRecentFiles === n
                    ? 'bg-blue-600/20 border-blue-500/50 text-blue-300'
                    : 'border-neutral-700 text-neutral-400 hover:text-neutral-200 hover:border-neutral-600'"
                  @click="appStore.setMaxRecentFiles(n)"
                >{{ n }}</button>
              </div>
              <button
                class="px-3 py-1.5 rounded-md text-xs font-medium bg-neutral-800 hover:bg-neutral-700 text-neutral-300 border border-neutral-700 transition-colors"
                @click="appStore.clearRecentFiles()"
              >
                Clear recent files
              </button>
            </section>
          </template>

          <!-- Appearance -->
          <template v-if="activeCategory === 'appearance'">
            <!-- Font size -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  Font Size
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Adjust the interface text size. Keyboard shortcuts
                  <code class="text-neutral-400 bg-neutral-800 px-1 rounded">+</code> /
                  <code class="text-neutral-400 bg-neutral-800 px-1 rounded">-</code> also work.
                </p>
              </div>
              <div class="flex items-center gap-3">
                <button
                  class="w-6 h-6 flex items-center justify-center rounded text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition-colors text-sm font-bold shrink-0"
                  title="Decrease font size"
                  :disabled="appStore.fontSize <= 9"
                  :class="appStore.fontSize <= 9 ? 'opacity-30 cursor-not-allowed' : ''"
                  @click="appStore.decreaseFontSize()"
                >A-</button>
                <input
                  type="range"
                  min="9"
                  max="20"
                  :value="appStore.fontSize"
                  class="flex-1 h-1 rounded-full accent-blue-500 cursor-pointer"
                  @input="appStore.setFontSize(Number(($event.target as HTMLInputElement).value))"
                />
                <button
                  class="w-6 h-6 flex items-center justify-center rounded text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800 transition-colors text-sm font-bold shrink-0"
                  title="Increase font size"
                  :disabled="appStore.fontSize >= 20"
                  :class="appStore.fontSize >= 20 ? 'opacity-30 cursor-not-allowed' : ''"
                  @click="appStore.increaseFontSize()"
                >A+</button>
                <span class="text-xs text-neutral-400 w-6 text-right shrink-0">{{ appStore.fontSize }}</span>
              </div>
            </section>

            <!-- Theme -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  Theme
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Choose between a dark or light color scheme.
                </p>
              </div>
              <div class="flex gap-2">
                <button
                  class="flex items-center gap-2 px-3 py-2 rounded-lg border text-xs font-medium transition-colors"
                  :class="
                    appStore.theme === 'dark'
                      ? 'bg-blue-600/20 border-blue-500/50 text-blue-300'
                      : 'border-neutral-700 text-neutral-400 hover:text-neutral-200 hover:border-neutral-600'
                  "
                  @click="appStore.setTheme('dark')"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>
                  Dark
                </button>
                <button
                  class="flex items-center gap-2 px-3 py-2 rounded-lg border text-xs font-medium transition-colors"
                  :class="
                    appStore.theme === 'light'
                      ? 'bg-blue-600/20 border-blue-500/50 text-blue-300'
                      : 'border-neutral-700 text-neutral-400 hover:text-neutral-200 hover:border-neutral-600'
                  "
                  @click="appStore.setTheme('light')"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>
                  Light
                </button>
              </div>
            </section>

            <!-- Row Density -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  Row Density
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Controls the height of rows in the tree explorer.
                </p>
              </div>
              <div class="flex gap-2">
                <button
                  v-for="[id, label] in [['compact', 'Compact'], ['comfortable', 'Comfortable'], ['spacious', 'Spacious']]"
                  :key="id"
                  class="px-3 py-2 rounded-lg border text-xs font-medium transition-colors"
                  :class="appStore.rowDensity === id
                    ? 'bg-blue-600/20 border-blue-500/50 text-blue-300'
                    : 'border-neutral-700 text-neutral-400 hover:text-neutral-200 hover:border-neutral-600'"
                  @click="appStore.setRowDensity(id as 'compact' | 'comfortable' | 'spacious')"
                >{{ label }}</button>
              </div>
            </section>
          </template>

          <!-- Behavior -->
          <template v-if="activeCategory === 'behavior'">
            <!-- Auto-expand first level -->
            <section class="space-y-3">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                    Auto-expand first level
                  </h4>
                  <p class="text-xs text-neutral-500 leading-relaxed">
                    Expand top-level nodes automatically when a file is opened.
                  </p>
                </div>
                <button
                  class="relative w-9 h-5 rounded-full transition-colors shrink-0 mt-0.5 overflow-hidden"
                  :class="appStore.autoExpandFirstLevel ? 'bg-blue-600' : 'bg-neutral-700'"
                  @click="appStore.setAutoExpandFirstLevel(!appStore.autoExpandFirstLevel)"
                >
                  <span
                    class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
                    :class="appStore.autoExpandFirstLevel ? 'translate-x-4' : 'translate-x-0'"
                  />
                </button>
              </div>
            </section>

            <!-- Default hide unchanged -->
            <section class="space-y-3">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                    Hide unchanged nodes in diff
                  </h4>
                  <p class="text-xs text-neutral-500 leading-relaxed">
                    Automatically hide unchanged nodes when comparing two files.
                  </p>
                </div>
                <button
                  class="relative w-9 h-5 rounded-full transition-colors shrink-0 mt-0.5 overflow-hidden"
                  :class="appStore.defaultHideUnchanged ? 'bg-blue-600' : 'bg-neutral-700'"
                  @click="appStore.setDefaultHideUnchanged(!appStore.defaultHideUnchanged)"
                >
                  <span
                    class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
                    :class="appStore.defaultHideUnchanged ? 'translate-x-4' : 'translate-x-0'"
                  />
                </button>
              </div>
            </section>

            <!-- Wrap table cell text -->
            <section class="space-y-3">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                    Wrap table cell text
                  </h4>
                  <p class="text-xs text-neutral-500 leading-relaxed">
                    Wrap long values in detail-pane table cells instead of truncating.
                  </p>
                </div>
                <button
                  class="relative w-9 h-5 rounded-full transition-colors shrink-0 mt-0.5 overflow-hidden"
                  :class="appStore.wrapTableText ? 'bg-blue-600' : 'bg-neutral-700'"
                  @click="appStore.setWrapTableText(!appStore.wrapTableText)"
                >
                  <span
                    class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
                    :class="appStore.wrapTableText ? 'translate-x-4' : 'translate-x-0'"
                  />
                </button>
              </div>
            </section>
          </template>

          <!-- AI Assistant -->
          <template v-if="activeCategory === 'ai'">
            <!-- Device flow pending -->
            <div
              v-if="llmStore.loginState === 'polling' && llmStore.deviceFlowInfo"
              class="space-y-3 rounded-lg bg-neutral-800/50 border border-neutral-700/50 p-3"
            >
              <div class="flex items-center gap-2 text-xs text-neutral-400">
                <div class="w-2 h-2 rounded-full bg-amber-400 animate-pulse shrink-0"></div>
                Waiting for GitHub authorization
              </div>
              <p class="text-[11px] text-neutral-500">
                1. Copy the code below, then open the verification URL.
              </p>
              <div class="flex items-center gap-2 bg-neutral-800 border border-neutral-700 rounded-lg p-2.5">
                <span class="flex-1 text-center font-mono text-base font-bold tracking-widest text-neutral-100 select-all">{{ llmStore.deviceFlowInfo.user_code }}</span>
                <button
                  class="px-2 py-1 rounded text-[11px] font-medium transition-colors shrink-0"
                  :class="copied ? 'bg-green-700/30 text-green-400' : 'bg-neutral-700 hover:bg-neutral-600 text-neutral-300'"
                  @click="copyCode"
                >{{ copied ? "Copied!" : "Copy" }}</button>
              </div>
              <div class="flex items-center gap-2">
                <button
                  class="text-[11px] text-blue-400 hover:text-blue-300 underline transition-colors truncate flex-1 text-left"
                  @click="openUrl(llmStore.deviceFlowInfo.verification_uri)"
                >{{ llmStore.deviceFlowInfo.verification_uri }} ↗</button>
              </div>
              <button
                class="text-[11px] text-neutral-600 hover:text-neutral-400 transition-colors"
                @click="cancelLogin"
              >Cancel</button>
            </div>

            <!-- Step 1 — Authentication -->
            <section class="space-y-3">
              <div class="flex items-center gap-2">
                <span class="flex h-4 w-4 items-center justify-center rounded-full bg-blue-600 text-[10px] font-bold text-white shrink-0">1</span>
                <h4 class="text-xs font-semibold text-neutral-300">Authentication <span class="text-red-400">*</span></h4>
              </div>
              <div>
                <label class="block text-[11px] text-neutral-400 mb-1">Method</label>
                <div class="relative">
                  <select
                    v-model="aiForm.auth_method"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-blue-500 transition-colors appearance-none pr-7"
                  >
                    <option value="copilot">GitHub Copilot (GHE)</option>
                    <option value="azure">Azure OpenAI</option>
                    <option value="openai">OpenAI</option>
                    <option value="bedrock">AWS Bedrock</option>
                  </select>
                  <div class="pointer-events-none absolute inset-y-0 right-2 flex items-center">
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-500"><path d="m6 9 6 6 6-6"/></svg>
                  </div>
                </div>
              </div>

              <!-- GitHub Copilot -->
              <template v-if="aiForm.auth_method === 'copilot'">
                <p class="text-[10px] text-neutral-600 leading-relaxed">
                  Uses GitHub's Copilot OAuth app — no Client ID or app registration needed.
                  You will be shown a code to enter at the verification URL (handles SAML SSO).
                </p>
                <div>
                  <label class="block text-[11px] text-neutral-400 mb-1">GHE Host <span class="text-red-400">*</span></label>
                  <input
                    v-model="aiForm.ghe_host"
                    type="text"
                    placeholder="mercedes-benz.ghe.com"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
                  />
                  <p class="mt-1 text-[10px] text-neutral-600">Domain only — no protocol or path</p>
                </div>
                <div class="pt-1">
                  <div v-if="llmStore.isAuthenticated && llmStore.settings.auth_method === 'copilot'" class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <div class="w-2 h-2 rounded-full bg-green-500 shrink-0"></div>
                      <span class="text-xs text-neutral-300">Logged in via GitHub Copilot</span>
                    </div>
                    <button class="text-xs text-red-400 hover:text-red-300 transition-colors" @click="llmStore.logout()">Logout</button>
                  </div>
                  <button
                    v-else-if="llmStore.loginState !== 'polling'"
                    class="w-full py-1.5 rounded-md bg-neutral-800 hover:bg-neutral-700 border border-neutral-700 text-neutral-200 text-xs font-medium transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
                    :disabled="!aiForm.ghe_host"
                    @click="llmStore.startCopilotLogin(aiForm.ghe_host)"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
                    Login with GitHub Copilot
                  </button>
                  <p v-if="llmStore.error && aiForm.auth_method === 'copilot'" class="mt-1 text-[10px] text-red-400">{{ llmStore.error }}</p>
                </div>
              </template>

              <!-- Azure OpenAI -->
              <template v-else-if="aiForm.auth_method === 'azure'">
                <p class="text-[10px] text-neutral-600 leading-relaxed">
                  Uses the <code class="text-neutral-500">api-key</code> header. Provide your Azure OpenAI API key and resource endpoint.
                </p>
                <div>
                  <label class="block text-[11px] text-neutral-400 mb-1">API Key <span class="text-red-400">*</span></label>
                  <input
                    v-model="aiForm.api_token"
                    type="password"
                    placeholder="Leave blank to keep existing key"
                    autocomplete="off"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
                  />
                  <p v-if="llmStore.settings.has_token && llmStore.settings.auth_method === 'azure'" class="mt-1 text-[10px] text-green-600 flex items-center gap-1">
                    <span class="inline-block w-1.5 h-1.5 rounded-full bg-green-500"></span> Key is set
                  </p>
                </div>
                <div>
                  <label class="block text-[11px] text-neutral-400 mb-1">API Version <span class="text-neutral-600">(optional)</span></label>
                  <input
                    v-model="aiForm.api_version"
                    type="text"
                    placeholder="2024-10-21"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
                  />
                </div>
              </template>

              <!-- OpenAI -->
              <template v-else-if="aiForm.auth_method === 'openai'">
                <p class="text-[10px] text-neutral-600 leading-relaxed">
                  Direct OpenAI API. Uses <code class="text-neutral-500">Authorization: Bearer</code> header.
                </p>
                <div>
                  <label class="block text-[11px] text-neutral-400 mb-1">API Key <span class="text-red-400">*</span></label>
                  <input
                    v-model="aiForm.api_token"
                    type="password"
                    placeholder="sk-…"
                    autocomplete="off"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
                  />
                  <p v-if="llmStore.settings.has_token && llmStore.settings.auth_method === 'openai'" class="mt-1 text-[10px] text-green-600 flex items-center gap-1">
                    <span class="inline-block w-1.5 h-1.5 rounded-full bg-green-500"></span> Key is set
                  </p>
                </div>
              </template>

              <!-- Bedrock -->
              <template v-else-if="aiForm.auth_method === 'bedrock'">
                <p class="text-[10px] text-neutral-600 leading-relaxed">
                  AWS Bedrock / GenAI Nexus proxy. Uses <code class="text-neutral-500">Authorization: Bearer</code> header.
                </p>
                <div>
                  <label class="block text-[11px] text-neutral-400 mb-1">Bearer Token <span class="text-red-400">*</span></label>
                  <input
                    v-model="aiForm.api_token"
                    type="password"
                    placeholder="Leave blank to keep existing token"
                    autocomplete="off"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
                  />
                  <p v-if="llmStore.settings.has_token && llmStore.settings.auth_method === 'bedrock'" class="mt-1 text-[10px] text-green-600 flex items-center gap-1">
                    <span class="inline-block w-1.5 h-1.5 rounded-full bg-green-500"></span> Token is set
                  </p>
                </div>
              </template>
            </section>

            <!-- Step 2 — LLM Endpoint -->
            <section class="space-y-3">
              <div class="flex items-center gap-2">
                <span class="flex h-4 w-4 items-center justify-center rounded-full bg-blue-600 text-[10px] font-bold text-white shrink-0">2</span>
                <h4 class="text-xs font-semibold text-neutral-300">LLM Endpoint <span class="text-red-400">*</span></h4>
              </div>
              <div v-if="aiForm.auth_method === 'copilot'">
                <p class="text-[10px] text-neutral-600">
                  Endpoint: <code class="text-neutral-400">https://copilot-api.{{ aiForm.ghe_host || '…' }}</code> (auto-configured)
                </p>
              </div>
              <div v-else>
                <label class="block text-[11px] text-neutral-400 mb-1">API Base URL <span class="text-red-400">*</span></label>
                <input
                  v-model="aiForm.llm_endpoint"
                  type="text"
                  placeholder="https://llm.mycompany.com/v1"
                  class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
                />
                <p class="mt-1 text-[10px] text-neutral-600">OpenAI-compatible — exposes <code class="text-neutral-500">/models</code> and <code class="text-neutral-500">/chat/completions</code></p>
              </div>
              <div>
                <label class="block text-[11px] text-neutral-400 mb-1">Model <span class="text-red-400">*</span></label>
                <div class="relative">
                  <select
                    v-model="aiForm.llm_model"
                    :disabled="llmStore.modelsLoading"
                    class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-blue-500 transition-colors appearance-none pr-7 disabled:opacity-50"
                  >
                    <option value="" disabled>
                      {{ llmStore.modelsLoading ? 'Fetching models…' : llmStore.availableModels.length === 0 ? '— authenticate first —' : '— select a model —' }}
                    </option>
                    <option
                      v-if="aiForm.llm_model && !llmStore.availableModels.includes(aiForm.llm_model)"
                      :value="aiForm.llm_model"
                    >{{ aiForm.llm_model }}</option>
                    <option v-for="m in llmStore.availableModels" :key="m" :value="m">{{ m }}</option>
                  </select>
                  <div class="pointer-events-none absolute inset-y-0 right-2 flex items-center">
                    <svg v-if="llmStore.modelsLoading" class="animate-spin text-neutral-500" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
                    <svg v-else xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-500"><path d="m6 9 6 6 6-6"/></svg>
                  </div>
                </div>
              </div>
            </section>

            <!-- Save button -->
            <section>
              <button
                class="w-full py-1.5 rounded-md bg-blue-600 hover:bg-blue-500 text-white text-xs font-medium transition-colors"
                @click="saveAiSettings"
              >Save AI Settings</button>
            </section>
          </template>

          <!-- Updates -->
          <template v-if="activeCategory === 'updates'">
            <!-- Current version -->
            <section class="space-y-1">
              <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider">Current Version</h4>
              <p class="text-xs text-neutral-400 font-mono">
                {{ currentVersion || '…' }}
              </p>
            </section>

            <!-- Auto-check toggle -->
            <section class="space-y-3">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                    Automatically check for updates
                  </h4>
                  <p class="text-xs text-neutral-500 leading-relaxed">
                    Check for new releases on startup. Disabled by default.
                  </p>
                </div>
                <button
                  class="relative w-9 h-5 rounded-full transition-colors shrink-0 mt-0.5 overflow-hidden"
                  :class="appStore.autoCheckUpdates ? 'bg-blue-600' : 'bg-neutral-700'"
                  @click="appStore.setAutoCheckUpdates(!appStore.autoCheckUpdates)"
                >
                  <span
                    class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
                    :class="appStore.autoCheckUpdates ? 'translate-x-4' : 'translate-x-0'"
                  />
                </button>
              </div>
            </section>

            <!-- Manual check -->
            <section class="space-y-3">
              <div>
                <h4 class="text-xs font-semibold text-neutral-300 uppercase tracking-wider mb-1">
                  Check for Updates
                </h4>
                <p class="text-xs text-neutral-500 leading-relaxed">
                  Manually check for a new release on GitHub.
                </p>
              </div>

              <div class="flex items-center gap-3">
                <button
                  class="px-3 py-1.5 rounded-md text-xs font-medium transition-colors shrink-0 disabled:opacity-50"
                  :class="
                    updateStatus === 'checking' || isInstalling
                      ? 'bg-neutral-800 text-neutral-500 cursor-not-allowed border border-neutral-700'
                      : 'bg-blue-600 hover:bg-blue-500 text-white'
                  "
                  :disabled="updateStatus === 'checking' || isInstalling"
                  @click="checkForUpdates"
                >
                  <span v-if="updateStatus === 'checking'" class="flex items-center gap-1.5">
                    <svg class="animate-spin" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
                    Checking…
                  </span>
                  <span v-else>Check Now</span>
                </button>

                <button
                  v-if="updateStatus === 'available'"
                  class="px-3 py-1.5 rounded-md text-xs font-medium transition-colors shrink-0 bg-green-600 hover:bg-green-500 text-white disabled:opacity-50"
                  :disabled="isInstalling"
                  @click="installUpdate"
                >
                  <span v-if="isInstalling" class="flex items-center gap-1.5">
                    <svg class="animate-spin" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
                    Installing…
                  </span>
                  <span v-else>Install v{{ updateVersion }}</span>
                </button>

                <span v-if="updateStatus === 'up-to-date'" class="flex items-center gap-1.5 text-xs text-green-400">
                  <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><path d="m9 11 3 3L22 4"/></svg>
                  Up to date
                </span>
              </div>

              <div
                v-if="updateStatus === 'done'"
                class="rounded-lg bg-green-900/20 border border-green-800/40 p-3 text-xs text-green-300 leading-relaxed"
              >
                Update installed. Please restart MDD UI to apply the changes.
              </div>

              <div
                v-if="updateStatus === 'available'"
                class="rounded-lg bg-blue-900/20 border border-blue-800/40 p-3 text-xs text-blue-300 leading-relaxed"
              >
                Version <strong>{{ updateVersion }}</strong> is available.
              </div>

              <div
                v-if="updateStatus === 'error'"
                class="rounded-lg bg-red-900/20 border border-red-800/40 p-3 text-xs text-red-400 leading-relaxed"
              >
                {{ updateError }}
              </div>
            </section>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
