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
  auth_method: string;
  has_token: boolean;
}

export interface LlmSettingsUpdate {
  ghe_host: string;
  client_id: string;
  llm_endpoint: string;
  llm_model: string;
  auth_method: string;
  api_token?: string;
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
    auth_method: "ghe",
    has_token: false,
  });
  const messages = ref<ChatMessage[]>([]);
  const loginState = ref<LoginState>("idle");
  const deviceFlowInfo = ref<DeviceFlowInfo | null>(null);
  const isLoading = ref(false);
  const error = ref("");
  const availableModels = ref<string[]>([]);
  const modelsLoading = ref(false);

  const isAuthenticated = computed(
    () => settings.value.auth_method === "none" || settings.value.has_token,
  );

  async function fetchModels(): Promise<void> {
    if (!settings.value.llm_endpoint) return;
    if (settings.value.auth_method !== "none" && !settings.value.has_token) return;
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
        client_id: update.client_id,
        llm_endpoint: update.llm_endpoint,
        llm_model: update.llm_model,
        auth_method: update.auth_method,
      };
      if (update.auth_method === "token" && update.api_token) {
        settings.value.has_token = true;
      } else if (update.auth_method === "none") {
        settings.value.has_token = false;
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

  // NOTE: This Client ID belongs to the opencode project (https://github.com/sst/opencode).
  // We borrow it temporarily because it is already approved on enterprise GHE instances
  // that have Copilot enabled, which avoids requiring users to go through a corporate
  // OAuth App approval process for mdd-ui.
  // Replace this with mdd-ui's own Client ID (Ov23liMhCri4BIE67Zeh) once it has been
  // approved by the relevant enterprise admins.
  const MDD_UI_CLIENT_ID = "Ov23li8tweQw6odWQebz";

  async function startCopilotLogin(gheHost: string): Promise<void> {
    if (!gheHost) {
      error.value = "Please enter your GHE host first.";
      return;
    }
    settings.value = { ...settings.value, ghe_host: gheHost, client_id: MDD_UI_CLIENT_ID };
    // Device flow goes directly to the enterprise host
    await runDeviceFlow(gheHost, MDD_UI_CLIENT_ID);
  }

  async function startLogin(): Promise<void> {
    if (!settings.value.ghe_host || !settings.value.client_id) {
      error.value =
        "Please configure GHE Host and OAuth Client ID in settings first.";
      settingsOpen.value = true;
      return;
    }
    await runDeviceFlow(settings.value.ghe_host, settings.value.client_id);
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

  async function importGhCliToken(gheHost: string): Promise<void> {
    isLoading.value = true;
    error.value = "";
    try {
      await invoke("import_gh_cli_token", { gheHost });
      settings.value = { ...settings.value, has_token: true };
      loginState.value = "authorized";
      void fetchModels();
    } catch (e) {
      error.value = `${e}`;
      loginState.value = "error";
    } finally {
      isLoading.value = false;
    }
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
    importGhCliToken,
    startCopilotLogin,
  };
});
