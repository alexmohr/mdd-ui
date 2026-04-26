// SPDX-FileCopyrightText: 2026 Alexander Mohr
// SPDX-License-Identifier: Apache-2.0

import { ref } from "vue";
import { defineStore } from "pinia";
import { registerMddAssociation } from "../api/commands";

export type RegisterStatus = "idle" | "loading" | "success" | "error";

export const useSettingsStore = defineStore("settings", () => {
  const open = ref(false);
  const registerStatus = ref<RegisterStatus>("idle");
  const registerMessage = ref("");

  async function doRegisterMddAssociation(): Promise<void> {
    registerStatus.value = "loading";
    registerMessage.value = "";
    try {
      const msg = await registerMddAssociation();
      registerStatus.value = "success";
      registerMessage.value = msg;
    } catch (e) {
      registerStatus.value = "error";
      registerMessage.value = `${e}`;
    }
  }

  function resetRegisterStatus(): void {
    registerStatus.value = "idle";
    registerMessage.value = "";
  }

  return {
    open,
    registerStatus,
    registerMessage,
    doRegisterMddAssociation,
    resetRegisterStatus,
  };
});
