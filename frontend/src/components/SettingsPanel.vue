<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { ref } from "vue";
import { useSettingsStore } from "../stores/settings";

const store = useSettingsStore();
const activeCategory = ref("general");

const categories = [{ id: "general", label: "General" }];

function close() {
  store.open = false;
  store.resetRegisterStatus();
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
      style="max-height: 80vh"
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
            class="w-full text-left px-3 py-1.5 rounded-md text-xs transition-colors"
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
          </template>
        </div>
      </div>
    </div>
  </div>
</template>
