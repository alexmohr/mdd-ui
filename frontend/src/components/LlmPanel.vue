<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useLlmStore } from "../stores/llm";
import { useSettingsStore } from "../stores/settings";
import { useAppStore } from "../stores/app";
import { marked } from "marked";

// Extension: render [[name]] as a clickable navigation button
marked.use({
  extensions: [
    {
      name: "mddNav",
      level: "inline" as const,
      start(src: string) {
        return src.indexOf("[[");
      },
      tokenizer(src: string) {
        const m = /^\[\[([^\]]+)\]\]/.exec(src);
        if (m) return { type: "mddNav", raw: m[0], name: m[1] };
      },
      renderer(token) {
        const name = (token as unknown as { name: string }).name.replace(/"/g, "&quot;");
        return `<button class="mdd-nav" data-name="${name}">${name}</button>`;
      },
    },
  ],
});

function renderMessage(content: string): string {
  return marked.parse(content, { async: false }) as string;
}

const store = useLlmStore();
const settingsStore = useSettingsStore();
const appStore = useAppStore();
const messagesEl = ref<HTMLElement | null>(null);
const inputText = ref("");
const panelWidth = ref(420);
const resizing = ref(false);

function onResizeMouseDown(e: MouseEvent) {
  e.preventDefault();
  resizing.value = true;
  const onMove = (ev: MouseEvent) => {
    const w = window.innerWidth - ev.clientX;
    panelWidth.value = Math.max(280, Math.min(Math.round(window.innerWidth * 0.75), w));
  };
  const onUp = () => {
    resizing.value = false;
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", onUp);
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
}

watch(
  () => store.messages.length,
  async () => {
    await nextTick();
    messagesEl.value?.scrollTo({
      top: messagesEl.value.scrollHeight,
      behavior: "smooth",
    });
  },
);

async function send() {
  const text = inputText.value.trim();
  if (!text || store.isLoading) return;
  inputText.value = "";
  await store.sendMessage(text);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    send();
  }
}

function close() {
  store.stopPolling();
  store.panelOpen = false;
}

function openSettings() {
  settingsStore.open = true;
}

async function navigateToNode(name: string) {
  // The backend resolves TreeNodeByIndex by name fallback when index doesn't match,
  // so we can navigate directly without a search round-trip.
  await appStore.navigateTo({ target_type: { TreeNodeByIndex: { index: 0, short_name: name } } });
}

function onMessageAreaClick(e: MouseEvent) {
  const btn = (e.target as HTMLElement).closest<HTMLElement>(".mdd-nav");
  if (btn?.dataset.name) void navigateToNode(btn.dataset.name);
}
</script>

<template>
  <div
    class="fixed top-0 right-0 h-screen flex flex-col bg-neutral-900 border-l border-neutral-800 z-50 text-sm"
    :class="{ 'select-none': resizing }"
    :style="{ width: panelWidth + 'px' }"
  >
    <!-- Resize handle -->
    <div
      class="absolute top-0 left-0 h-full w-1 cursor-col-resize bg-neutral-800/40 hover:bg-blue-500/40 active:bg-blue-500/60 transition-colors z-10"
      @mousedown.prevent="onResizeMouseDown"
    />
    <!-- Header -->
    <div class="flex items-center h-10 px-3 border-b border-neutral-800 shrink-0 gap-1">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="15"
        height="15"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        class="text-blue-400 shrink-0"
      >
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
      </svg>
      <span class="flex-1 font-medium text-neutral-200 text-sm ml-1">AI Assistant</span>
      <!-- Clear -->
      <button
        class="p-1.5 rounded-md text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
        title="Clear conversation"
        @click="store.clearMessages()"
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
          <path d="M3 6h18" />
          <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
          <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
        </svg>
      </button>
      <!-- Settings -->
      <button
        class="p-1.5 rounded-md text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
        title="Settings"
        @click="openSettings"
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
          <path
            d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
          />
          <circle cx="12" cy="12" r="3" />
        </svg>
      </button>
      <!-- Close -->
      <button
        class="p-1.5 rounded-md text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800 transition-colors"
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

    <!-- Not logged in nudge -->
    <div
      v-if="!store.isAuthenticated && store.loginState === 'idle'"
      class="border-b border-neutral-800 p-3 flex items-center justify-between shrink-0"
    >
      <span class="text-xs text-neutral-500">Not logged in</span>
      <button
        class="text-xs text-blue-400 hover:text-blue-300 transition-colors"
        @click="openSettings"
      >
        Configure &amp; Login →
      </button>
    </div>

    <!-- Messages -->
    <div
      ref="messagesEl"
      class="flex-1 overflow-y-auto p-3 space-y-3 min-h-0"
      @click="onMessageAreaClick"
    >
      <div v-if="store.messages.length === 0" class="text-center py-8">
        <p class="text-neutral-600 text-xs">
          Ask anything about the loaded MDD file.<br />The ECU structure is sent as context.
        </p>
      </div>

      <div
        v-for="(msg, i) in store.messages"
        :key="i"
        class="flex flex-col gap-1"
        :class="msg.role === 'user' ? 'items-end' : 'items-start'"
      >
        <span class="text-[10px] text-neutral-600 px-1">
          {{ msg.role === "user" ? "You" : "AI" }}
        </span>
        <!-- User message -->
        <div
          v-if="msg.role === 'user'"
          class="max-w-[90%] rounded-xl rounded-br-sm px-3 py-2 text-xs leading-relaxed whitespace-pre-wrap break-words bg-blue-600/20 text-blue-100"
        >
          {{ msg.content }}
        </div>
        <!-- AI message: rendered markdown with navigation links -->
        <div
          v-else
          class="prose max-w-[90%] rounded-xl rounded-bl-sm px-3 py-2 text-xs bg-neutral-800 text-neutral-200"
          v-html="renderMessage(msg.content)"
        />
      </div>

      <!-- Typing indicator -->
      <div v-if="store.isLoading" class="flex items-start gap-1">
        <div class="bg-neutral-800 rounded-xl rounded-bl-sm px-3 py-2.5 flex gap-1">
          <div
            class="w-1.5 h-1.5 rounded-full bg-neutral-500 animate-bounce"
            style="animation-delay: 0ms"
          />
          <div
            class="w-1.5 h-1.5 rounded-full bg-neutral-500 animate-bounce"
            style="animation-delay: 150ms"
          />
          <div
            class="w-1.5 h-1.5 rounded-full bg-neutral-500 animate-bounce"
            style="animation-delay: 300ms"
          />
        </div>
      </div>
    </div>

    <!-- Error bar -->
    <div
      v-if="store.error"
      class="px-3 py-2 bg-red-900/20 border-t border-red-800/30 text-red-400 text-xs leading-relaxed shrink-0 flex items-start gap-2"
    >
      <span class="flex-1">{{ store.error }}</span>
      <button
        class="text-red-500 hover:text-red-300 shrink-0 transition-colors"
        @click="store.error = ''"
      >
        <svg
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
          <path d="M18 6 6 18" />
          <path d="m6 6 12 12" />
        </svg>
      </button>
    </div>

    <!-- Input area -->
    <div class="p-2 border-t border-neutral-800 flex gap-2 shrink-0">
      <textarea
        v-model="inputText"
        :disabled="!store.isAuthenticated || store.isLoading"
        placeholder="Ask about the MDD file… (Enter to send)"
        rows="2"
        class="flex-1 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-xs text-neutral-200 placeholder-neutral-600 resize-none focus:outline-none focus:border-blue-500 transition-colors disabled:opacity-40"
        @keydown="onKeydown"
      />
      <button
        :disabled="!store.isAuthenticated || store.isLoading || !inputText.trim()"
        class="px-3 py-2 rounded-lg text-xs font-medium transition-colors self-end shrink-0"
        :class="
          store.isAuthenticated && !store.isLoading && inputText.trim()
            ? 'bg-blue-600 hover:bg-blue-500 text-white'
            : 'bg-neutral-800 text-neutral-600 cursor-not-allowed border border-neutral-700'
        "
        @click="send"
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
          <path d="m22 2-7 20-4-9-9-4Z" />
          <path d="M22 2 11 13" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.prose :deep(h1),
.prose :deep(h2),
.prose :deep(h3) {
  font-weight: 600;
  margin: 0.4em 0 0.2em;
}
.prose :deep(h1) {
  font-size: 0.95rem;
}
.prose :deep(h2) {
  font-size: 0.85rem;
}
.prose :deep(h3) {
  font-size: 0.8rem;
}
.prose :deep(p) {
  margin: 0.3em 0;
}
.prose :deep(ul) {
  list-style: disc;
  padding-left: 1.2em;
  margin: 0.3em 0;
}
.prose :deep(ol) {
  list-style: decimal;
  padding-left: 1.2em;
  margin: 0.3em 0;
}
.prose :deep(li) {
  margin: 0.1em 0;
}
.prose :deep(code) {
  background: rgba(255, 255, 255, 0.08);
  padding: 0.1em 0.3em;
  border-radius: 3px;
  font-family: monospace;
  font-size: 0.9em;
}
.prose :deep(pre) {
  background: rgba(0, 0, 0, 0.35);
  padding: 0.6em 0.75em;
  border-radius: 6px;
  overflow-x: auto;
  margin: 0.4em 0;
}
.prose :deep(pre code) {
  background: none;
  padding: 0;
}
.prose :deep(strong) {
  font-weight: 600;
}
.prose :deep(em) {
  font-style: italic;
}
.prose :deep(blockquote) {
  border-left: 2px solid #4b5563;
  padding-left: 0.6em;
  color: #9ca3af;
  margin: 0.3em 0;
}
.prose :deep(a) {
  color: #60a5fa;
  text-decoration: underline;
}
.prose :deep(button.mdd-nav) {
  color: #60a5fa;
  text-decoration: underline;
  text-underline-offset: 2px;
  font-weight: 500;
  cursor: pointer;
  background: none;
  border: none;
  padding: 0;
  font-size: inherit;
  font-family: inherit;
}
.prose :deep(button.mdd-nav:hover) {
  color: #93c5fd;
}
</style>
