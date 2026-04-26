// SPDX-FileCopyrightText: 2026 Alexander Mohr
// SPDX-License-Identifier: Apache-2.0

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface LlmSettingsView {
  ghe_host: string;
  client_id: string;
  llm_endpoint: string;
  llm_model: string;
  has_token: boolean;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export type LoginState = "idle" | "polling" | "authorized" | "error";

interface DeviceFlowInfo {
  user_code: string;
  verification_uri: string;
  device_code: string;
  interval: number;
}

export const useLlmStore = defineStore("llm", () => {
  const panelOpen = ref(false);
  const settingsOpen = ref(false);
  const settings = ref<LlmSettingsView>({
    ghe_host: "",
    client_id: "",
    llm_endpoint: "",
    llm_model: "gpt-4o",
    has_token: false,
  });
  const messages = ref<ChatMessage[]>([]);
  const loginState = ref<LoginState>("idle");
  const deviceFlowInfo = ref<DeviceFlowInfo | null>(null);
  const isLoading = ref(false);
  const error = ref("");
  const availableModels = ref<string[]>([]);
  const modelsLoading = ref(false);

  const isAuthenticated = computed(() => settings.value.has_token);

  async function fetchModels(): Promise<void> {
    if (!settings.value.has_token || !settings.value.llm_endpoint) return;
    modelsLoading.value = true;
    try {
      availableModels.value = await invoke<string[]>("fetch_llm_models");
    } catch (e) {
      console.warn("Failed to fetch models:", e);
    } finally {
      modelsLoading.value = false;
    }
  }

  async function loadSettings() {
    try {
      settings.value = await invoke<LlmSettingsView>("get_llm_settings");
      if (settings.value.has_token) {
        loginState.value = "authorized";
        await fetchModels();
      }
    } catch (e) {
      console.error("Failed to load LLM settings:", e);
    }
  }

  async function saveSettings(
    update: Omit<LlmSettingsView, "has_token">,
  ): Promise<void> {
    try {
      await invoke("save_llm_settings", { settings: update });
      settings.value = { ...settings.value, ...update };
    } catch (e) {
      error.value = `Failed to save settings: ${e}`;
    }
  }

  async function logout(): Promise<void> {
    try {
      await invoke("clear_llm_token");
      settings.value = { ...settings.value, has_token: false };
      loginState.value = "idle";
      deviceFlowInfo.value = null;
      stopPolling();
    } catch (e) {
      error.value = `Failed to logout: ${e}`;
    }
  }

  let pollTimer: ReturnType<typeof setTimeout> | null = null;

  function stopPolling() {
    if (pollTimer !== null) {
      clearTimeout(pollTimer);
      pollTimer = null;
    }
  }

  async function startLogin(): Promise<void> {
    if (!settings.value.ghe_host || !settings.value.client_id) {
      error.value =
        "Please configure GHE Host and OAuth Client ID in settings first.";
      settingsOpen.value = true;
      return;
    }
    stopPolling();
    loginState.value = "polling";
    error.value = "";
    try {
      const result = await invoke<{
        device_code: string;
        user_code: string;
        verification_uri: string;
        expires_in: number;
        interval: number;
      }>("start_ghe_device_flow", {
        gheHost: settings.value.ghe_host,
        clientId: settings.value.client_id,
      });
      deviceFlowInfo.value = {
        user_code: result.user_code,
        verification_uri: result.verification_uri,
        device_code: result.device_code,
        interval: result.interval,
      };
      schedulePoll(result.interval * 1000);
    } catch (e) {
      loginState.value = "error";
      error.value = `Login failed: ${e}`;
    }
  }

  function schedulePoll(intervalMs: number) {
    pollTimer = setTimeout(() => doPoll(intervalMs), intervalMs);
  }

  async function doPoll(intervalMs: number): Promise<void> {
    if (!deviceFlowInfo.value) return;
    try {
      const result = await invoke<{ status: string }>("poll_ghe_device_flow", {
        gheHost: settings.value.ghe_host,
        clientId: settings.value.client_id,
        deviceCode: deviceFlowInfo.value.device_code,
      });
      if (result.status === "authorized") {
        loginState.value = "authorized";
        settings.value = { ...settings.value, has_token: true };
        deviceFlowInfo.value = null;
        void fetchModels();
      } else if (result.status === "pending") {
        schedulePoll(intervalMs);
      } else if (result.status === "slow_down") {
        schedulePoll(Math.round(intervalMs * 1.5));
      } else {
        loginState.value = "error";
        error.value = `Authorization failed: ${result.status}`;
      }
    } catch (e) {
      loginState.value = "error";
      error.value = `Poll failed: ${e}`;
    }
  }

  async function sendMessage(content: string): Promise<void> {
    if (!content.trim()) return;
    messages.value.push({ role: "user", content });
    isLoading.value = true;
    error.value = "";
    try {
      const result = await invoke<{ content: string }>("llm_chat", {
        messages: messages.value.map((m) => ({
          role: m.role,
          content: m.content,
        })),
      });
      messages.value.push({ role: "assistant", content: result.content });
    } catch (e) {
      error.value = `${e}`;
    } finally {
      isLoading.value = false;
    }
  }

  function clearMessages() {
    messages.value = [];
    error.value = "";
  }

  return {
    panelOpen,
    settingsOpen,
    settings,
    messages,
    loginState,
    deviceFlowInfo,
    isLoading,
    error,
    availableModels,
    modelsLoading,
    isAuthenticated,
    loadSettings,
    saveSettings,
    logout,
    startLogin,
    sendMessage,
    clearMessages,
    stopPolling,
    fetchModels,
  };
});
