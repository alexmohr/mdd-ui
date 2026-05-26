// SPDX-FileCopyrightText: 2026 Alexander Mohr
// SPDX-License-Identifier: Apache-2.0

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface LlmSettingsView {
  ghe_host: string;
  llm_endpoint: string;
  llm_model: string;
  auth_method: string;
  has_token: boolean;
  api_version?: string;
}

export interface LlmSettingsUpdate {
  ghe_host: string;
  llm_endpoint: string;
  llm_model: string;
  auth_method: string;
  api_token?: string;
  api_version?: string;
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
  const settings = ref<LlmSettingsView>({
    ghe_host: "",
    llm_endpoint: "",
    llm_model: "gpt-4o",
    auth_method: "copilot",
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
    // For copilot, the endpoint is auto-derived; for others, check it's set.
    if (settings.value.auth_method !== "copilot" && !settings.value.llm_endpoint) return;
    if (!settings.value.has_token) return;
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

  async function saveSettings(update: LlmSettingsUpdate): Promise<void> {
    try {
      await invoke("save_llm_settings", { settings: update });
      settings.value = {
        ...settings.value,
        ghe_host: update.ghe_host,
        llm_endpoint: update.llm_endpoint,
        llm_model: update.llm_model,
        auth_method: update.auth_method,
        api_version: update.api_version,
      };
      if (["azure", "openai", "bedrock"].includes(update.auth_method) && update.api_token) {
        settings.value.has_token = true;
      }
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
  let currentAuthHost = ""; // host used for device flow (may differ from ghe_host for copilot)

  function stopPolling() {
    if (pollTimer !== null) {
      clearTimeout(pollTimer);
      pollTimer = null;
    }
  }

  // VS Code Copilot extension's GitHub App Client ID.
  // Pre-approved on every GHE instance with Copilot enabled — no admin approval needed.
  const COPILOT_CLIENT_ID = "Iv1.b507a08c87ecfe98";

  async function startCopilotLogin(gheHost: string): Promise<void> {
    if (!gheHost) {
      error.value = "Please enter your GHE host first.";
      return;
    }
    settings.value = { ...settings.value, ghe_host: gheHost };
    await runDeviceFlow(gheHost, COPILOT_CLIENT_ID);
  }

  async function runDeviceFlow(authHost: string, clientId: string): Promise<void> {
    stopPolling();
    loginState.value = "polling";
    error.value = "";
    currentAuthHost = authHost;
    try {
      const result = await invoke<{
        device_code: string;
        user_code: string;
        verification_uri: string;
        expires_in: number;
        interval: number;
      }>("start_ghe_device_flow", {
        gheHost: authHost,
        clientId,
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
        gheHost: currentAuthHost,
        clientId: COPILOT_CLIENT_ID,
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
    sendMessage,
    clearMessages,
    stopPolling,
    fetchModels,
    startCopilotLogin,
  };
});
