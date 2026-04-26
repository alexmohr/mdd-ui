<!-- SPDX-FileCopyrightText: 2026 Alexander Mohr -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { ref, watch, nextTick, reactive } from "vue";
import { useLlmStore } from "../stores/llm";
import type { LlmSettingsUpdate } from "../stores/llm";
import { useAppStore } from "../stores/app";
import { marked } from "marked";

// Extension: render [[name]] as a clickable navigation button
marked.use({
  extensions: [{
    name: "mddNav",
    level: "inline" as const,
    start(src: string) { return src.indexOf("[["); },
    tokenizer(src: string) {
      const m = /^\[\[([^\]]+)\]\]/.exec(src);
      if (m) return { type: "mddNav", raw: m[0], name: m[1] };
    },
    renderer(token) {
      const name = (token as unknown as { name: string }).name.replace(/"/g, "&quot;");
      return `<button class="mdd-nav" data-name="${name}">${name}</button>`;
    },
  }],
});

function renderMessage(content: string): string {
  return marked.parse(content, { async: false }) as string;
}

const store = useLlmStore();
const appStore = useAppStore();
const messagesEl = ref<HTMLElement | null>(null);
const inputText = ref("");
const copied = ref(false);

const form = reactive<LlmSettingsUpdate & { api_token: string }>({
  ghe_host: store.settings.ghe_host,
  llm_endpoint: store.settings.llm_endpoint,
  llm_model: store.settings.llm_model,
  auth_method: store.settings.auth_method,
  api_token: "",
  api_version: store.settings.api_version,
});

watch(
  () => store.settingsOpen,
  (open) => {
    if (open) {
      form.ghe_host = store.settings.ghe_host;
      form.llm_endpoint = store.settings.llm_endpoint;
      form.llm_model = store.settings.llm_model;
      form.auth_method = store.settings.auth_method;
      form.api_token = "";
      form.api_version = store.settings.api_version;
      if (store.isAuthenticated) void store.fetchModels();
    }
  },
);

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

async function saveSettings() {
  const endpoint =
    form.auth_method === "copilot"
      ? `https://copilot-api.${form.ghe_host}`
      : form.llm_endpoint;
  await store.saveSettings({
    ghe_host: form.ghe_host,
    llm_endpoint: endpoint,
    llm_model: form.llm_model,
    auth_method: form.auth_method,
    api_token: form.api_token || undefined,
    api_version: form.api_version || undefined,
  });
  store.settingsOpen = false;
}

async function copyCode() {
  if (!store.deviceFlowInfo) return;
  await navigator.clipboard.writeText(store.deviceFlowInfo.user_code);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}

async function copyUrl() {
  if (!store.deviceFlowInfo) return;
  await navigator.clipboard.writeText(store.deviceFlowInfo.verification_uri);
}

function cancelLogin() {
  store.stopPolling();
  store.loginState = "idle";
  store.deviceFlowInfo = null;
  store.error = "";
}

function close() {
  store.stopPolling();
  store.panelOpen = false;
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
    style="width: 420px"
  >
    <!-- Header -->
    <div
      class="flex items-center h-10 px-3 border-b border-neutral-800 shrink-0 gap-1"
    >
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
      <span class="flex-1 font-medium text-neutral-200 text-sm ml-1"
        >AI Assistant</span
      >
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
        class="p-1.5 rounded-md transition-colors"
        :class="
          store.settingsOpen
            ? 'bg-neutral-700 text-neutral-200'
            : 'text-neutral-500 hover:text-neutral-200 hover:bg-neutral-800'
        "
        title="Settings"
        @click="store.settingsOpen = !store.settingsOpen"
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

    <!-- Settings panel -->
    <div
      v-if="store.settingsOpen"
      class="border-b border-neutral-800 p-3 space-y-4 shrink-0 overflow-y-auto max-h-[55vh]"
    >
      <!-- Step 1 — Authentication -->
      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <span class="flex h-4 w-4 items-center justify-center rounded-full bg-blue-600 text-[10px] font-bold text-white shrink-0">1</span>
          <p class="text-[11px] text-neutral-300 font-medium">Authentication <span class="text-red-400">*</span></p>
        </div>
        <div>
          <label class="block text-[11px] text-neutral-400 mb-1">Method</label>
          <div class="relative">
            <select
              v-model="form.auth_method"
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

        <!-- GitHub Copilot — hardcoded app, no setup needed -->
        <template v-if="form.auth_method === 'copilot'">
          <p class="text-[10px] text-neutral-600 leading-relaxed">
            Uses GitHub's Copilot OAuth app — no Client ID or app registration needed.
            You will be shown a code to enter at the verification URL (handles SAML SSO).
          </p>
          <div>
            <label class="block text-[11px] text-neutral-400 mb-1">GHE Host <span class="text-red-400">*</span></label>
            <input
              v-model="form.ghe_host"
              type="text"
              placeholder="mercedes-benz.ghe.com"
              class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
            />
            <p class="mt-1 text-[10px] text-neutral-600">Domain only — no protocol or path</p>
          </div>
          <div class="pt-1">
            <div v-if="store.isAuthenticated && store.settings.auth_method === 'copilot'" class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-green-500 shrink-0"></div>
                <span class="text-xs text-neutral-300">Logged in via GitHub Copilot</span>
              </div>
              <button class="text-xs text-red-400 hover:text-red-300 transition-colors" @click="store.logout()">Logout</button>
            </div>
            <button
              v-else-if="store.loginState !== 'polling'"
              class="w-full py-1.5 rounded-md bg-neutral-800 hover:bg-neutral-700 border border-neutral-700 text-neutral-200 text-xs font-medium transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
              :disabled="!form.ghe_host"
              @click="store.startCopilotLogin(form.ghe_host)"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
              Login with GitHub Copilot
            </button>
            <p v-if="store.error && form.auth_method === 'copilot'" class="mt-1 text-[10px] text-red-400">{{ store.error }}</p>
          </div>
        </template>

        <!-- Azure OpenAI -->
        <template v-else-if="form.auth_method === 'azure'">
          <p class="text-[10px] text-neutral-600 leading-relaxed">
            Uses the <code class="text-neutral-500">api-key</code> header. Provide your Azure OpenAI API key and resource endpoint.
          </p>
          <div>
            <label class="block text-[11px] text-neutral-400 mb-1">API Key <span class="text-red-400">*</span></label>
            <input
              v-model="form.api_token"
              type="password"
              placeholder="Leave blank to keep existing key"
              autocomplete="off"
              class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
            />
            <p v-if="store.settings.has_token && store.settings.auth_method === 'azure'" class="mt-1 text-[10px] text-green-600 flex items-center gap-1">
              <span class="inline-block w-1.5 h-1.5 rounded-full bg-green-500"></span> Key is set
            </p>
          </div>
          <div>
            <label class="block text-[11px] text-neutral-400 mb-1">API Version <span class="text-neutral-600">(optional)</span></label>
            <input
              v-model="form.api_version"
              type="text"
              placeholder="2024-10-21"
              class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
            />
          </div>
        </template>

        <!-- OpenAI -->
        <template v-else-if="form.auth_method === 'openai'">
          <p class="text-[10px] text-neutral-600 leading-relaxed">
            Direct OpenAI API. Uses <code class="text-neutral-500">Authorization: Bearer</code> header.
          </p>
          <div>
            <label class="block text-[11px] text-neutral-400 mb-1">API Key <span class="text-red-400">*</span></label>
            <input
              v-model="form.api_token"
              type="password"
              placeholder="sk-…"
              autocomplete="off"
              class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
            />
            <p v-if="store.settings.has_token && store.settings.auth_method === 'openai'" class="mt-1 text-[10px] text-green-600 flex items-center gap-1">
              <span class="inline-block w-1.5 h-1.5 rounded-full bg-green-500"></span> Key is set
            </p>
          </div>
        </template>

        <!-- Bedrock -->
        <template v-else-if="form.auth_method === 'bedrock'">
          <p class="text-[10px] text-neutral-600 leading-relaxed">
            AWS Bedrock / GenAI Nexus proxy. Uses <code class="text-neutral-500">Authorization: Bearer</code> header.
          </p>
          <div>
            <label class="block text-[11px] text-neutral-400 mb-1">Bearer Token <span class="text-red-400">*</span></label>
            <input
              v-model="form.api_token"
              type="password"
              placeholder="Leave blank to keep existing token"
              autocomplete="off"
              class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 placeholder-neutral-600 focus:outline-none focus:border-blue-500 transition-colors"
            />
            <p v-if="store.settings.has_token && store.settings.auth_method === 'bedrock'" class="mt-1 text-[10px] text-green-600 flex items-center gap-1">
              <span class="inline-block w-1.5 h-1.5 rounded-full bg-green-500"></span> Token is set
            </p>
          </div>
        </template>

      </div>

      <!-- Step 2 — LLM Endpoint -->
      <div class="space-y-2">
        <div class="flex items-center gap-2">
          <span class="flex h-4 w-4 items-center justify-center rounded-full bg-blue-600 text-[10px] font-bold text-white shrink-0">2</span>
          <p class="text-[11px] text-neutral-300 font-medium">LLM Endpoint <span class="text-red-400">*</span></p>
        </div>
        <!-- Copilot: endpoint auto-derived from GHE host -->
        <div v-if="form.auth_method === 'copilot'">
          <p class="text-[10px] text-neutral-600">
            Endpoint: <code class="text-neutral-400">https://copilot-api.{{ form.ghe_host || '…' }}</code> (auto-configured)
          </p>
        </div>
        <div v-else>
          <label class="block text-[11px] text-neutral-400 mb-1">API Base URL <span class="text-red-400">*</span></label>
          <input
            v-model="form.llm_endpoint"
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
              v-model="form.llm_model"
              :disabled="store.modelsLoading"
              class="w-full bg-neutral-800 border border-neutral-700 rounded-md px-2.5 py-1.5 text-xs text-neutral-200 focus:outline-none focus:border-blue-500 transition-colors appearance-none pr-7 disabled:opacity-50"
            >
              <option value="" disabled>
                {{ store.modelsLoading ? 'Fetching models…' : store.availableModels.length === 0 ? '— authenticate first —' : '— select a model —' }}
              </option>
              <option
                v-if="form.llm_model && !store.availableModels.includes(form.llm_model)"
                :value="form.llm_model"
              >{{ form.llm_model }}</option>
              <option v-for="m in store.availableModels" :key="m" :value="m">{{ m }}</option>
            </select>
            <div class="pointer-events-none absolute inset-y-0 right-2 flex items-center">
              <svg v-if="store.modelsLoading" class="animate-spin text-neutral-500" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-500"><path d="m6 9 6 6 6-6"/></svg>
            </div>
          </div>
        </div>
      </div>

      <div class="flex gap-2 pt-1">
        <button
          class="flex-1 py-1.5 rounded-md bg-blue-600 hover:bg-blue-500 text-white text-xs font-medium transition-colors"
          @click="saveSettings"
        >
          Save Settings
        </button>
        <button
          class="px-3 py-1.5 rounded-md bg-neutral-800 hover:bg-neutral-700 text-neutral-300 text-xs transition-colors border border-neutral-700"
          @click="store.settingsOpen = false"
        >
          Cancel
        </button>
      </div>
    </div>

    <!-- Device flow pending -->
    <div
      v-if="store.loginState === 'polling' && store.deviceFlowInfo"
      class="border-b border-neutral-800 p-3 space-y-3 shrink-0"
    >
      <div class="flex items-center gap-2 text-xs text-neutral-400">
        <div
          class="w-2 h-2 rounded-full bg-amber-400 animate-pulse shrink-0"
        ></div>
        Waiting for GitHub authorization
      </div>
      <p class="text-[11px] text-neutral-500">
        1. Copy the code below, then open the verification URL.
      </p>
      <!-- User code -->
      <div
        class="flex items-center gap-2 bg-neutral-800 border border-neutral-700 rounded-lg p-2.5"
      >
        <span
          class="flex-1 text-center font-mono text-base font-bold tracking-widest text-neutral-100 select-all"
          >{{ store.deviceFlowInfo.user_code }}</span
        >
        <button
          class="px-2 py-1 rounded text-[11px] font-medium transition-colors shrink-0"
          :class="
            copied
              ? 'bg-green-700/30 text-green-400'
              : 'bg-neutral-700 hover:bg-neutral-600 text-neutral-300'
          "
          @click="copyCode"
        >
          {{ copied ? "Copied!" : "Copy" }}
        </button>
      </div>
      <!-- Verification URL -->
      <div class="flex items-center gap-2">
        <span class="text-[11px] text-neutral-500 truncate flex-1">{{
          store.deviceFlowInfo.verification_uri
        }}</span>
        <button
          class="text-[11px] text-blue-400 hover:text-blue-300 transition-colors shrink-0"
          @click="copyUrl"
        >
          Copy URL
        </button>
      </div>
      <button
        class="text-[11px] text-neutral-600 hover:text-neutral-400 transition-colors"
        @click="cancelLogin"
      >
        Cancel
      </button>
    </div>

    <!-- Not logged in nudge -->
    <div
      v-if="
        !store.isAuthenticated &&
        store.loginState === 'idle' &&
        !store.settingsOpen
      "
      class="border-b border-neutral-800 p-3 flex items-center justify-between shrink-0"
    >
      <span class="text-xs text-neutral-500">Not logged in</span>
      <button
        class="text-xs text-blue-400 hover:text-blue-300 transition-colors"
        @click="store.settingsOpen = true"
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
          Ask anything about the loaded MDD file.<br />The ECU structure is
          sent as context.
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
          >{{ msg.content }}</div>
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
          ></div>
          <div
            class="w-1.5 h-1.5 rounded-full bg-neutral-500 animate-bounce"
            style="animation-delay: 150ms"
          ></div>
          <div
            class="w-1.5 h-1.5 rounded-full bg-neutral-500 animate-bounce"
            style="animation-delay: 300ms"
          ></div>
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
.prose :deep(h1), .prose :deep(h2), .prose :deep(h3) { font-weight: 600; margin: 0.4em 0 0.2em; }
.prose :deep(h1) { font-size: 0.95rem; }
.prose :deep(h2) { font-size: 0.85rem; }
.prose :deep(h3) { font-size: 0.8rem; }
.prose :deep(p) { margin: 0.3em 0; }
.prose :deep(ul) { list-style: disc; padding-left: 1.2em; margin: 0.3em 0; }
.prose :deep(ol) { list-style: decimal; padding-left: 1.2em; margin: 0.3em 0; }
.prose :deep(li) { margin: 0.1em 0; }
.prose :deep(code) { background: rgba(255,255,255,0.08); padding: 0.1em 0.3em; border-radius: 3px; font-family: monospace; font-size: 0.9em; }
.prose :deep(pre) { background: rgba(0,0,0,0.35); padding: 0.6em 0.75em; border-radius: 6px; overflow-x: auto; margin: 0.4em 0; }
.prose :deep(pre code) { background: none; padding: 0; }
.prose :deep(strong) { font-weight: 600; }
.prose :deep(em) { font-style: italic; }
.prose :deep(blockquote) { border-left: 2px solid #4b5563; padding-left: 0.6em; color: #9ca3af; margin: 0.3em 0; }
.prose :deep(a) { color: #60a5fa; text-decoration: underline; }
.prose :deep(button.mdd-nav) { color: #60a5fa; text-decoration: underline; text-underline-offset: 2px; font-weight: 500; cursor: pointer; background: none; border: none; padding: 0; font-size: inherit; font-family: inherit; }
.prose :deep(button.mdd-nav:hover) { color: #93c5fd; }
</style>
