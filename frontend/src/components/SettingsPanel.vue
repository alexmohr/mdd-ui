<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useSettingsStore } from "../stores/settings";
import { useAppStore } from "../stores/app";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";

const store = useSettingsStore();
const appStore = useAppStore();
const activeCategory = ref("general");

const categories = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "behavior", label: "Behavior" },
  { id: "updates", label: "Updates" },
];

type UpdateCheckStatus = "idle" | "checking" | "up-to-date" | "available" | "done" | "error";
const updateStatus = ref<UpdateCheckStatus>("idle");
const updateVersion = ref("");
const updateError = ref("");
const isInstalling = ref(false);
const currentVersion = ref("");

onMounted(async () => {
  currentVersion.value = await getVersion();
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
